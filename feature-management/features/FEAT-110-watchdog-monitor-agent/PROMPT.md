# FEAT-110: Watchdog Monitor Agent

**Priority**: P1
**Component**: orchestration/watchdog
**Effort**: Medium
**Status**: new

## Summary

Create a dedicated watchdog agent that actively monitors worker sessions and only notifies orchestrators when action is needed. Unlike a polling-based approach where the orchestrator checks on workers, this watchdog proactively monitors and sends targeted alerts, preserving orchestrator context for actual decision-making.

## Problem Statement

Current orchestration patterns require the orchestrator to:
1. Periodically poll worker status (context-expensive)
2. Process "all good" responses (wasted tokens)
3. Manually track which workers need attention

This results in:
- Orchestrator context bloat from routine status checks
- Delayed detection of stuck or errored workers
- Orchestrator distraction from higher-level planning

## Architecture

```
┌─────────────────┐                              ┌─────────────────┐
│   Orchestrator  │◄─────── (only on alert) ────│    Watchdog     │
│   (session-0)   │                              │   (__watchdog)  │
│                 │                              │                 │
│  tag:orchestrator│                              │  tag:watchdog   │
└─────────────────┘                              └────────┬────────┘
        ▲                                                 │
        │                                                 │ poll interval
        │                                                 ▼
        │                                        ┌─────────────────┐
        │                                        │    Workers      │
        └─────── (action taken) ────────────────►│  tag:worker     │
                                                 │                 │
                                                 │  - bug-066-*    │
                                                 │  - feat-103-*   │
                                                 └─────────────────┘
```

**Key principle**: Watchdog sends NO messages when all workers are healthy. Only notifies orchestrator when intervention is required.

## Core Behavior

### 1. Worker Discovery

Watchdog discovers workers by:
1. Querying sessions with configured `worker_tags` (e.g., `["worker"]`)
2. Optionally filtering by session name pattern (e.g., `*-worker`)

```json
// Discovery query
fugue_get_worker_status()  // Returns all workers with status
```

### 2. Status Assessment

For each discovered worker, assess:

| State | Detection Logic | Requires Alert? |
|-------|-----------------|-----------------|
| `working` | Activity changed since last poll | No |
| `stuck` | No activity change for N intervals | **Yes** |
| `error` | Status contains error indicator | **Yes** |
| `complete` | Status is "complete" | **Yes** |
| `needs_input` | `is_awaiting_input` or `is_awaiting_confirmation` | **Yes** |

### 3. Alert Routing

When alert condition detected, send to orchestrator via:

```json
{
  "tool": "fugue_send_orchestration",
  "input": {
    "target": {"tag": "orchestrator"},
    "msg_type": "worker.stuck",  // or worker.error, worker.complete, worker.needs_input
    "payload": {
      "worker_id": "session-uuid",
      "worker_name": "bug-066-worker",
      // ... state-specific context
    }
  }
}
```

## Detection Logic Detail

### Stuck Detection

Track activity fingerprint per worker:
```
activity_fingerprint = hash(last_output_line + status + timestamp_bucket)
```

If fingerprint unchanged for `stuck_threshold_intervals` consecutive polls:
```json
{
  "msg_type": "worker.stuck",
  "payload": {
    "worker_id": "uuid",
    "worker_name": "feat-103-worker",
    "last_activity": "2024-01-15T10:30:00Z",
    "duration_secs": 270,
    "intervals_stuck": 3,
    "last_output_preview": "Processing file 42 of 100..."
  }
}
```

### Error Detection

Check worker status for error indicators:
- `status: "error"` in fugue_report_status
- Error patterns in last N lines of output

```json
{
  "msg_type": "worker.error",
  "payload": {
    "worker_id": "uuid",
    "worker_name": "bug-066-worker",
    "error_context": "Build failed with exit code 1",
    "output_tail": ["Error: Cannot find module 'foo'", "npm ERR! ..."]
  }
}
```

### Complete Detection

Worker reports completion via `fugue_report_status(status: "complete")`:

```json
{
  "msg_type": "worker.complete",
  "payload": {
    "worker_id": "uuid",
    "worker_name": "feat-103-worker",
    "outcome": "success",
    "completion_message": "Feature implemented and tests passing"
  }
}
```

### Needs Input Detection

Check pane state for input prompts:
- `fugue_get_status` returns `is_awaiting_input: true`
- `is_awaiting_confirmation: true`

```json
{
  "msg_type": "worker.needs_input",
  "payload": {
    "worker_id": "uuid",
    "worker_name": "bug-066-worker",
    "prompt_type": "confirmation",  // or "input", "permission"
    "prompt_preview": "Do you want to proceed with the refactoring? (y/n)"
  }
}
```

## Configuration

### Agent Preset

```toml
# ~/.fugue/config.toml

[presets.watchdog]
harness = "claude"
description = "Dedicated worker monitoring agent"

[presets.watchdog.config]
model = "haiku"  # Cheap, fast - just monitoring
dangerously_skip_permissions = true  # Only uses MCP tools
```

### Watchdog Configuration

```toml
[watchdog]
# Polling interval in seconds
poll_interval_secs = 30

# Number of unchanged intervals before declaring stuck
stuck_threshold_intervals = 3

# Tags to identify workers to monitor
worker_tags = ["worker"]

# Tag to identify orchestrator for alerts
orchestrator_tag = "orchestrator"

# Optional: session name pattern (regex)
# worker_pattern = ".*-worker$"

# Number of output lines to include in alerts
output_preview_lines = 5
```

### Environment Overrides

```bash
FUGUE_WATCHDOG_POLL_INTERVAL=30
FUGUE_WATCHDOG_STUCK_THRESHOLD=3
FUGUE_WATCHDOG_WORKER_TAGS=worker,secondary-worker
FUGUE_WATCHDOG_ORCHESTRATOR_TAG=orchestrator
```

## Watchdog Agent System Prompt

```markdown
You are a worker monitoring agent. Your job is to watch worker sessions and alert the orchestrator ONLY when action is needed.

## Trigger
You receive periodic "check" messages from a background timer.

## On each "check":

1. **Discover workers**: `fugue_get_worker_status()` for all sessions
2. **Filter to workers**: Only sessions with tag "worker" (configurable)
3. **For each worker, check**:
   - `fugue_get_status(pane_id)` for detailed state
   - Compare to previous state (track in memory)

4. **Classify each worker**:
   - `working`: Activity changed since last check → OK, no alert
   - `stuck`: Same state for 3+ intervals → ALERT
   - `error`: Status "error" or error patterns → ALERT
   - `complete`: Status "complete" → ALERT
   - `needs_input`: Awaiting input/confirmation → ALERT

5. **If ANY worker needs attention**:
   - `fugue_send_orchestration` to tag:orchestrator with worker.* message
   - Include actionable context

6. **If ALL workers healthy**:
   - Say nothing. Do not alert. Do not log success.
   - Simply wait for next check.

## Critical Behavior
- NEVER send "all workers healthy" messages
- NEVER interrupt orchestrator unless action required
- Be concise in alerts - include only what orchestrator needs to act
- Track state between checks to detect stuck workers

## State Tracking
Maintain per-worker:
- last_activity_fingerprint
- consecutive_unchanged_count
- last_status
```

## Integration with FEAT-104

This feature complements FEAT-104 (Watchdog Orchestration Skill):

| FEAT-104 | FEAT-110 |
|----------|----------|
| `/orchestrate monitor start` command | Dedicated watchdog agent behavior |
| Skill-based interface | Agent prompt and preset |
| Background timer mechanism | Polling and detection logic |
| Worker spawn/collect commands | Worker monitoring only |

FEAT-110 defines the watchdog agent's **internal behavior**, while FEAT-104 defines the **orchestration skill commands** that manage it.

## Implementation Notes

### Using Native Watchdog Timer (FEAT-104)

The existing `fugue_watchdog_start` MCP tool can trigger the watchdog:

```json
// Start watchdog timer (from orchestrator or skill)
{
  "tool": "fugue_watchdog_start",
  "input": {
    "pane_id": "<watchdog_pane_uuid>",
    "interval_secs": 30,
    "message": "check"
  }
}
```

### State Persistence

Watchdog maintains in-session state:
- Use Claude's conversation context to track previous states
- On first "check", establish baseline (no alerts)
- Subsequent checks compare to remembered state

### Handling Watchdog Restart

If watchdog session restarts:
- First poll after restart establishes new baseline
- Workers unchanged from baseline after N intervals → stuck
- Avoids false "stuck" alerts immediately after restart

## Message Type Reference

| Message Type | Trigger | Payload Fields |
|--------------|---------|----------------|
| `worker.stuck` | No activity for N intervals | `worker_id`, `worker_name`, `last_activity`, `duration_secs`, `intervals_stuck`, `last_output_preview` |
| `worker.error` | Error status/output detected | `worker_id`, `worker_name`, `error_context`, `output_tail` |
| `worker.complete` | Worker reports complete | `worker_id`, `worker_name`, `outcome`, `completion_message` |
| `worker.needs_input` | Awaiting user input | `worker_id`, `worker_name`, `prompt_type`, `prompt_preview` |

## Acceptance Criteria

- [ ] Watchdog agent preset defined in example config
- [ ] System prompt documented for watchdog agent behavior
- [ ] Stuck detection works with configurable threshold
- [ ] Error detection catches status-reported and output-based errors
- [ ] Complete detection triggers on fugue_report_status(status: "complete")
- [ ] Needs input detection checks is_awaiting_input/is_awaiting_confirmation
- [ ] Alerts sent ONLY when action needed (no "all good" messages)
- [ ] Alert messages contain actionable context
- [ ] Configuration documented with defaults
- [ ] Integration with fugue_watchdog_start timer documented
- [ ] Example orchestrator handling of watchdog alerts provided

## Example Orchestrator Alert Handling

When orchestrator receives alert via `fugue_poll_messages`:

```json
// worker.stuck
{
  "action": "check_on_worker",
  "options": [
    "fugue_read_pane to see current output",
    "fugue_send_input to nudge worker",
    "fugue_close_pane to kill and respawn"
  ]
}

// worker.needs_input
{
  "action": "provide_input",
  "options": [
    "fugue_send_input with 'y' to confirm",
    "fugue_send_input with 'n' to decline",
    "fugue_read_pane to see full prompt context"
  ]
}

// worker.complete
{
  "action": "collect_results",
  "options": [
    "fugue_read_pane to get final output",
    "Merge worker branch",
    "fugue_kill_session to cleanup"
  ]
}

// worker.error
{
  "action": "investigate",
  "options": [
    "fugue_read_pane for full error context",
    "Fix issue and retry",
    "fugue_close_pane and respawn with different approach"
  ]
}
```

## Related

- FEAT-104: Watchdog Orchestration Skill (commands to manage watchdog)
- FEAT-097: fugue_get_worker_status / fugue_poll_messages
- FEAT-105: Universal Agent Presets (watchdog preset)
- FEAT-102: Agent Status Pane (visual monitoring complement)
- AGENTS.md: Status reporting conventions
