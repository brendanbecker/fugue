# BUG-057: Agent detection cross-contamination between panes

**Priority**: P3
**Component**: agents
**Severity**: low
**Status**: fixed

## Problem

After running Gemini CLI in one pane, another pane running Claude Code was incorrectly detected as Gemini. The detection for the Claude pane changed from:

```json
{
  "is_claude": true,
  "state": "claude",
  "claude_state": { "activity": "Thinking" }
}
```

To:

```json
{
  "is_claude": false,
  "state": "agent",
  "state": {
    "agent_type": "gemini",
    "model": "Gemini 3",
    "activity": "AwaitingConfirmation"
  }
}
```

## Reproduction Steps

1. Start fugue with Claude Code in a pane (session-0)
2. Start Gemini CLI in the `__orchestration__` session pane
3. Call `fugue_list_panes()` or `fugue_get_status()` for the Claude pane
4. Observe that the Claude pane now shows as Gemini

## Expected Behavior

Each pane should have independent agent detection. Running Gemini in one pane should not affect the detection state of another pane running Claude.

## Hypothesis

### 1. Shared Detection State
The agent detection might be storing state globally rather than per-pane. When Gemini is detected, it might be overwriting a shared state variable.

### 2. Screen Buffer Contamination
The detection logic might be reading screen content from the wrong pane, or there's cross-contamination in the PTY output buffers being analyzed.

### 3. Detection Priority Issue
If both Claude and Gemini patterns are detected, there may be a priority issue where the more recently matched pattern wins regardless of which pane it came from.

## Investigation Steps

### Section 1: Review Agent Detection Architecture
- [ ] Check `fugue-server/src/agents/` for detection state storage
- [ ] Verify each pane has isolated detection state
- [ ] Review how detection results are associated with pane IDs

### Section 2: Add Logging
- [ ] Add debug logging to agent detection with pane_id
- [ ] Log which screen buffer content is being analyzed
- [ ] Trace how detection state is updated

### Section 3: Reproduce and Isolate
- [ ] Confirm bug is reproducible
- [ ] Test if issue occurs with Claude in both panes
- [ ] Test if issue occurs in reverse (Claude detected as Gemini after Claude runs)

## Acceptance Criteria

- [ ] Each pane has independent agent detection state
- [ ] Running one agent type in a pane doesn't affect other panes
- [ ] Detection correctly identifies agent per-pane

## Related Files

- `fugue-server/src/agents/mod.rs` - Agent detection module
- `fugue-server/src/agents/claude.rs` - Claude detection
- `fugue-server/src/agents/gemini.rs` - Gemini detection (FEAT-098)

## Notes

Discovered during QA testing of FEAT-094 through FEAT-098 on 2026-01-17.
