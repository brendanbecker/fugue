# FEAT-112: Orchestrator Context-Aware Handoff

**Priority**: P1
**Component**: skill/orchestration
**Effort**: Medium
**Status**: new

## Summary

Enable orchestrators to automatically perform a handoff cycle (update HANDOFF.md, commit, `/clear`) when their context exceeds a threshold (e.g., 100k tokens). This prevents context exhaustion during long-running orchestration sessions and ensures state is persisted before clearing.

## Related Features

- **FEAT-110**: Watchdog Monitor Agent (triggers the check)
- **FEAT-111**: Watchdog Auto-Clear (watchdog's own context management)
- **FEAT-104**: Watchdog Orchestration Skill

## Problem Statement

Long-running orchestrators accumulate context as they:
1. Spawn and monitor workers
2. Receive status updates and help requests
3. Make decisions and track progress
4. Handle errors and retries

Without context management:
- Orchestrator hits context limit and becomes unresponsive
- Important state is lost on forced clear or crash
- No clean handoff for session continuation

## Solution: Context-Aware Handoff Skill

When the orchestrator's context exceeds a threshold, it should:

1. **Update HANDOFF.md** - Capture current state (active workers, progress, pending tasks)
2. **Commit** - Persist the handoff document
3. **Clear** - Reset context
4. **Resume** - Reload from HANDOFF.md and continue

```
┌─────────────────────────────────────────────────────────────┐
│              Orchestrator Context Lifecycle                 │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│   ┌──────────┐    ┌──────────┐    ┌──────────┐             │
│   │  Normal  │───►│  Check   │───►│ Continue │             │
│   │ Operation│    │ Context  │    │ Working  │             │
│   └──────────┘    └────┬─────┘    └──────────┘             │
│                        │                                    │
│                   [> threshold]                             │
│                        │                                    │
│                        ▼                                    │
│   ┌──────────┐    ┌──────────┐    ┌──────────┐             │
│   │  Update  │───►│  Commit  │───►│  Clear   │             │
│   │ HANDOFF  │    │          │    │          │             │
│   └──────────┘    └──────────┘    └────┬─────┘             │
│                                        │                    │
│                                        ▼                    │
│                                   ┌──────────┐             │
│                                   │  Resume  │──► Normal   │
│                                   │ from DOC │   Operation │
│                                   └──────────┘             │
└─────────────────────────────────────────────────────────────┘
```

## Implementation Approach

### Option A: Skill-Based (Recommended)

Create an orchestrator skill that:
1. Is invoked at the start of each watchdog check cycle
2. Checks current context usage (from agent state or estimate)
3. If over threshold, performs handoff cycle before processing

**Challenge**: Skills don't persist through `/clear`. Solutions:
- Put skill invocation in CLAUDE.md (always loaded)
- Put skill invocation in system prompt via preset
- Re-invoke skill as first action after clear

### Option B: Watchdog-Initiated

The watchdog detects orchestrator context size and sends different messages:
- Normal: "check workers"
- High context: "handoff and clear, then check workers"

**Challenge**: Watchdog needs access to orchestrator's token count.

### Option C: Hybrid

- Watchdog queries orchestrator's agent state via `fugue_get_status`
- If token count > threshold, sends "handoff" trigger
- Orchestrator skill handles the actual handoff process
- After clear, watchdog sends normal "check workers"

## Context Detection

### From Agent State

fugue tracks agent state including token usage (when detectable):

```json
{
  "tool": "fugue_get_status",
  "input": {"pane_id": "<orchestrator_pane>"}
}
```

Response includes `agent_state.context_tokens` if available.

### From Output Parsing

Claude Code displays context in status line:
```
> context: 45.2k tokens (23%)
```

The agent state detector can parse this.

### Fallback: Estimate

Count messages/tool calls as rough proxy if exact count unavailable.

## Handoff Document Structure

Three-part system for managing handoff state:

```
docs/
├── HANDOFF.md                    # Current active state
├── HANDOFF-template.md           # Structure template
└── HANDOFF-archive/
    ├── HANDOFF-2025-01-20-1430.md
    ├── HANDOFF-2025-01-20-1215.md
    └── HANDOFF-2025-01-19-0930.md
```

### HANDOFF-template.md

Defines the structure orchestrators should follow:

```markdown
# Orchestration Handoff

## Session Info

- **Timestamp**: <YYYY-MM-DD HH:MM>
- **Reason**: <context threshold | manual | error recovery>
- **Context at handoff**: <token count if known>

## Active Workers

| Session | Task | Status | Branch | Last Update |
|---------|------|--------|--------|-------------|
| | | | | |

## Pending Tasks

- [ ] <task description>

## Blocked Items

| Item | Blocker | Action Needed |
|------|---------|---------------|
| | | |

## Recent Events

<!-- Last 5-10 significant events -->

## Next Actions

<!-- What the next session should do first -->

## Notes

<!-- Any context that would be lost without capture -->
```

### HANDOFF.md

The **living document** that bridges sessions. Updated before each clear, read after each clear. This is the continuity mechanism - it persists and evolves across many clear cycles.

### HANDOFF-archive/

Directory for archiving old handoffs when:
- Starting a completely new phase of work
- Current handoff is no longer relevant
- Deliberate decision to start fresh

Archiving is a **manual/deliberate action**, not part of the regular clear cycle.

### Regular Handoff Cycle (on context threshold)

```
Before clear:
  1. Update docs/HANDOFF.md with current state (in-place)
  2. git add && git commit
  3. /clear

After clear:
  1. Read docs/HANDOFF.md to restore context
  2. Resume operations
```

### Archive Cycle (manual or automatic)

**Manual** - when deliberately starting a new phase:
```
  1. mv docs/HANDOFF.md → docs/HANDOFF-archive/HANDOFF-<timestamp>.md
  2. Create fresh docs/HANDOFF.md from template
  3. git add && git commit
```

**Automatic** - when HANDOFF.md exceeds size threshold:
```
Before clear (if HANDOFF.md > size_threshold):
  1. mv docs/HANDOFF.md → docs/HANDOFF-archive/HANDOFF-<timestamp>.md
  2. Create fresh docs/HANDOFF.md from template with current state
  3. git add && git commit
  4. /clear
```

This prevents HANDOFF.md from growing unbounded across many sessions while preserving history in the archive.

## Skill Design

### Pre-Check Hook

Add to orchestrator prompt or CLAUDE.md:

```markdown
## Context Management

Before responding to any watchdog check:

1. Check your context usage (estimate or from status)
2. If over 100k tokens:
   a. Update docs/HANDOFF.md with current state
   b. Commit: "docs: orchestrator handoff checkpoint"
   c. Send "/clear"
   d. After clear, read HANDOFF.md to restore context
   e. Then proceed with the check

This ensures state is never lost during long sessions.
```

### Post-Clear Recovery

After `/clear`, the orchestrator needs to:
1. Read HANDOFF.md
2. Understand current worker states
3. Resume monitoring

This can be triggered by:
- The next watchdog "check workers" message
- CLAUDE.md instructions to always check HANDOFF.md on startup

## Configuration

| Setting | Default | Description |
|---------|---------|-------------|
| `context_threshold` | `100000` | Token count triggering handoff |
| `handoff_path` | `docs/HANDOFF.md` | Path to active handoff document |
| `template_path` | `docs/HANDOFF-template.md` | Path to handoff template |
| `archive_dir` | `docs/HANDOFF-archive/` | Directory for archived handoffs |
| `handoff_size_threshold` | `500` | Lines in HANDOFF.md triggering auto-archive |
| `auto_commit` | `true` | Commit handoff before clear |

## Acceptance Criteria

- [ ] Orchestrator detects when context exceeds threshold
- [ ] HANDOFF-template.md exists and defines structure
- [ ] HANDOFF.md updated in-place with current state before clear
- [ ] Changes committed before clear
- [ ] Orchestrator resumes correctly after clear by reading HANDOFF.md
- [ ] Worker monitoring continues uninterrupted
- [ ] No state loss during handoff cycle
- [ ] Configurable threshold
- [ ] Works with watchdog trigger cycle
- [ ] Manual archive mechanism available (move to HANDOFF-archive/ when starting fresh)
- [ ] Auto-archive when HANDOFF.md exceeds size threshold
- [ ] Archived files preserve history for reference

## Testing

### Manual Testing

1. Start orchestrator with multiple workers
2. Run until context approaches threshold
3. Trigger watchdog check
4. Verify:
   - HANDOFF.md updated
   - Commit created
   - Context cleared
   - Monitoring resumes

### Edge Cases

- Handoff during active worker communication
- Multiple rapid checks near threshold
- Worker completes during handoff cycle
- Git conflicts in HANDOFF.md

## Open Questions

1. **Skill persistence**: How to ensure handoff logic survives clear?
   - CLAUDE.md approach seems most reliable

2. **Token detection reliability**: How accurate is context tracking?
   - May need fallback heuristics

3. **Handoff atomicity**: What if clear happens mid-handoff?
   - Need to complete commit before clear

4. **Resume trigger**: What tells orchestrator to read HANDOFF.md after clear?
   - Could be in system prompt or watchdog's next message
