# Watchdog Monitor Agent

> Dedicated agent for monitoring worker sessions and alerting orchestrators only when action is needed

## Overview

The watchdog monitor is a lightweight agent that continuously monitors worker sessions and sends targeted alerts to orchestrators. Unlike polling-based approaches where the orchestrator checks on workers (consuming context), the watchdog proactively monitors and **only notifies when intervention is required**.

**Key principle**: No news is good news. The watchdog sends NO messages when all workers are healthy.

## Architecture

```
┌─────────────────┐                              ┌─────────────────┐
│   Orchestrator  │◄─────── (only on alert) ────│    Watchdog     │
│   (session-0)   │                              │   (__watchdog)  │
│                 │                              │                 │
│  tag:orchestrator                              │  tag:watchdog   │
└─────────────────┘                              └────────┬────────┘
        ▲                                                 │
        │                                                 │ periodic "check"
        │                                                 ▼
        │                                        ┌─────────────────┐
        │                                        │    Workers      │
        └─────── (action taken) ────────────────►│  tag:worker     │
                                                 │                 │
                                                 │  - bug-066-*    │
                                                 │  - feat-103-*   │
                                                 └─────────────────┘
```

## Setup

### 1. Create Watchdog Session

```json
{
  "tool": "fugue_create_session",
  "input": {
    "name": "__watchdog",
    "command": "claude --dangerously-skip-permissions",
    "tags": ["watchdog", "featmgmt"]
  }
}
```

### 2. Start Native Timer

```json
{
  "tool": "fugue_watchdog_start",
  "input": {
    "pane_id": "<watchdog_pane_uuid>",
    "interval_secs": 30,
    "message": "check"
  }
}
```

### 3. Send Initial Prompt

The watchdog needs a system prompt defining its behavior (see [Agent System Prompt](#agent-system-prompt) below).

## Agent Preset

Add to `~/.fugue/config/fugue.toml`:

```toml
[presets.watchdog-monitor]
harness = "claude"
description = "Worker monitoring agent - alerts only when action needed"

[presets.watchdog-monitor.config]
model = "claude-3-haiku-20240307"  # Cheap and fast for monitoring
dangerously_skip_permissions = true  # Only uses MCP tools
system_prompt = """
You are a watchdog monitor agent. Your job is to observe worker sessions and alert the orchestrator ONLY when intervention is required.

CRITICAL: Send NO messages when workers are healthy. Silence means all is well.
"""
```

### Alternative: Full Haiku Preset

```toml
[presets.haiku-watchdog]
harness = "claude"
description = "Haiku-based watchdog for cost-effective monitoring"

[presets.haiku-watchdog.config]
model = "claude-3-haiku-20240307"
dangerously_skip_permissions = true
context_limit = 50000  # Watchdog doesn't need large context
```

## Agent System Prompt

The following prompt defines the watchdog's complete behavior. Send this as the initial message after creating the watchdog session:

```markdown
You are a watchdog monitor agent for fugue multi-agent orchestration. Your job is to observe worker sessions and alert the orchestrator ONLY when intervention is required.

## Trigger

You receive periodic "check" messages from a background timer (every 30 seconds by default).

## On Each "check" Message

### Step 1: Discover Workers

Call `fugue_get_worker_status()` to get all sessions with their current state.
Alternatively, call `fugue_list_panes()` and filter to sessions with the "worker" tag.

### Step 2: Assess Each Worker

For each discovered worker, determine its state:

| State | Detection | Action |
|-------|-----------|--------|
| `working` | Activity changed since last check | No alert needed |
| `stuck` | No activity for 3+ consecutive checks | ALERT |
| `error` | Status is "error" or error patterns in output | ALERT |
| `complete` | Status is "complete" | ALERT |
| `needs_input` | `is_awaiting_input` or `is_awaiting_confirmation` is true | ALERT |

### Step 3: Track State (Mental Model)

Maintain a mental model of each worker:
- Previous activity fingerprint (last output line + status)
- Consecutive unchanged count
- Last known status

Compare current state to remembered state to detect "stuck" workers.

### Step 4: Send Alerts (Only When Needed)

If ANY worker needs attention, send alert via `fugue_send_orchestration`:

```json
{
  "tool": "fugue_send_orchestration",
  "input": {
    "target": {"tag": "orchestrator"},
    "msg_type": "worker.<state>",
    "payload": { /* state-specific context */ }
  }
}
```

### Step 5: Stay Silent When Healthy

If ALL workers are working normally:
- Do NOT send any message
- Do NOT log "all healthy" status
- Simply wait for the next "check"

## Critical Behavior Rules

1. **NEVER** send "all workers healthy" or "check complete" messages
2. **NEVER** interrupt the orchestrator unless action is required
3. **BE CONCISE** in alerts - include only what orchestrator needs to act
4. **TRACK STATE** between checks to accurately detect stuck workers
5. **RESET BASELINE** if you restart - first check establishes baseline, no alerts

## State Tracking

For each worker, remember:
- `worker_id`: Session UUID
- `worker_name`: Session name (e.g., "feat-103-worker")
- `last_fingerprint`: Hash of (last_output_line + status)
- `unchanged_count`: How many consecutive checks with same fingerprint
- `last_status`: Previous status value

## Stuck Detection Algorithm

```
On each check:
  current_fingerprint = hash(last_output_line + status)

  if current_fingerprint == last_fingerprint:
    unchanged_count += 1
  else:
    unchanged_count = 0
    last_fingerprint = current_fingerprint

  if unchanged_count >= STUCK_THRESHOLD (default: 3):
    send worker.stuck alert
```

## Available MCP Tools

- `fugue_get_worker_status()` - Get all workers with status
- `fugue_list_panes()` - List all panes
- `fugue_get_status(pane_id)` - Get detailed pane status
- `fugue_read_pane(pane_id, lines)` - Read pane output
- `fugue_send_orchestration(target, msg_type, payload)` - Send alert

## Example Check Cycle

1. Receive "check" message
2. Call `fugue_get_worker_status()`
3. Worker "feat-103" shows status "complete" → send `worker.complete` alert
4. Worker "bug-066" has same output as last 3 checks → send `worker.stuck` alert
5. Worker "feat-107" is working normally → no alert
6. Done. Wait for next "check".
```

## Detection Logic

### Working (Healthy)

No alert needed when:
- Worker status is "working" or "idle"
- Output has changed since last check
- No error indicators present

### Stuck Detection

A worker is stuck when its state hasn't changed for N consecutive poll intervals.

**Fingerprint calculation:**
```
fingerprint = hash(
  last_output_line (trimmed) +
  reported_status +
  timestamp_bucket (optional, for coarse time tracking)
)
```

**Threshold**: Default 3 intervals (90 seconds at 30s polling)

**Alert payload:**
```json
{
  "msg_type": "worker.stuck",
  "payload": {
    "worker_id": "550e8400-e29b-41d4-a716-446655440000",
    "worker_name": "feat-103-worker",
    "last_activity": "2025-01-15T10:30:00Z",
    "duration_secs": 270,
    "intervals_stuck": 3,
    "last_output_preview": "Processing file 42 of 100..."
  }
}
```

### Error Detection

Detect errors from:
1. **Reported status**: Worker called `fugue_report_status(status: "error")`
2. **Output patterns**: Error keywords in last N lines of output

**Error patterns to watch:**
- `Error:`, `ERROR:`, `error:`
- `panic:`, `PANIC:`
- `fatal:`, `FATAL:`
- `failed`, `Failed`, `FAILED`
- Exit codes in shell prompts (e.g., `[1]`, `exit 1`)

**Alert payload:**
```json
{
  "msg_type": "worker.error",
  "payload": {
    "worker_id": "550e8400-e29b-41d4-a716-446655440000",
    "worker_name": "bug-066-worker",
    "error_source": "status",
    "error_context": "Build failed with exit code 1",
    "output_tail": [
      "error[E0308]: mismatched types",
      "  --> src/main.rs:42:5",
      "npm ERR! code ELIFECYCLE"
    ]
  }
}
```

### Complete Detection

Worker reports completion via `fugue_report_status(status: "complete")`.

**Alert payload:**
```json
{
  "msg_type": "worker.complete",
  "payload": {
    "worker_id": "550e8400-e29b-41d4-a716-446655440000",
    "worker_name": "feat-103-worker",
    "outcome": "success",
    "completion_message": "Feature implemented and all tests passing"
  }
}
```

### Needs Input Detection

Check pane state for input prompts:
- `is_awaiting_input: true` from `fugue_get_status`
- `is_awaiting_confirmation: true`
- Permission prompt patterns in output

**Prompt patterns:**
- `Do you want to proceed?`
- `(y/n)`
- `[Y/n]`
- `Press Enter to continue`
- `Allow?` / `Approve?`

**Alert payload:**
```json
{
  "msg_type": "worker.needs_input",
  "payload": {
    "worker_id": "550e8400-e29b-41d4-a716-446655440000",
    "worker_name": "bug-066-worker",
    "prompt_type": "confirmation",
    "prompt_preview": "Do you want to proceed with the refactoring? (y/n)"
  }
}
```

## Message Type Reference

| Message Type | Trigger | Key Payload Fields |
|--------------|---------|-------------------|
| `worker.stuck` | No activity for N intervals | `worker_id`, `worker_name`, `duration_secs`, `intervals_stuck`, `last_output_preview` |
| `worker.error` | Error status or output detected | `worker_id`, `worker_name`, `error_source`, `error_context`, `output_tail` |
| `worker.complete` | Worker reports complete | `worker_id`, `worker_name`, `outcome`, `completion_message` |
| `worker.needs_input` | Awaiting user input | `worker_id`, `worker_name`, `prompt_type`, `prompt_preview` |

## Orchestrator Alert Handling

When the orchestrator receives an alert via `fugue_poll_messages`, here's how to respond:

### worker.stuck

```json
{
  "recommended_actions": [
    {"action": "investigate", "tool": "fugue_read_pane", "input": {"pane_id": "<id>", "lines": 100}},
    {"action": "nudge", "tool": "fugue_send_input", "input": {"pane_id": "<id>", "input": "continue"}},
    {"action": "restart", "tool": "fugue_close_pane", "input": {"pane_id": "<id>"}}
  ]
}
```

### worker.error

```json
{
  "recommended_actions": [
    {"action": "investigate", "tool": "fugue_read_pane", "input": {"pane_id": "<id>", "lines": 200}},
    {"action": "retry", "tool": "fugue_send_input", "input": {"pane_id": "<id>", "input": "retry"}},
    {"action": "abandon", "tool": "fugue_close_pane", "input": {"pane_id": "<id>"}}
  ]
}
```

### worker.complete

```json
{
  "recommended_actions": [
    {"action": "collect_output", "tool": "fugue_read_pane", "input": {"pane_id": "<id>"}},
    {"action": "merge_branch", "shell": "git merge <worker-branch>"},
    {"action": "cleanup", "tool": "fugue_kill_session", "input": {"session": "<name>"}}
  ]
}
```

### worker.needs_input

```json
{
  "recommended_actions": [
    {"action": "approve", "tool": "fugue_send_input", "input": {"pane_id": "<id>", "key": "Enter"}},
    {"action": "decline", "tool": "fugue_send_input", "input": {"pane_id": "<id>", "input": "n", "submit": true}},
    {"action": "review_context", "tool": "fugue_read_pane", "input": {"pane_id": "<id>", "lines": 50}}
  ]
}
```

## Configuration

### Environment Variables

```bash
# Override poll interval
FUGUE_WATCHDOG_POLL_INTERVAL=30

# Override stuck threshold (number of unchanged intervals)
FUGUE_WATCHDOG_STUCK_THRESHOLD=3

# Tags to identify workers
FUGUE_WATCHDOG_WORKER_TAGS=worker

# Tag to identify orchestrator for alerts
FUGUE_WATCHDOG_ORCHESTRATOR_TAG=orchestrator

# Lines of output to include in alerts
FUGUE_WATCHDOG_OUTPUT_PREVIEW_LINES=5
```

### Recommended Defaults

| Setting | Default | Notes |
|---------|---------|-------|
| Poll interval | 30s | Balance between responsiveness and cost |
| Stuck threshold | 3 intervals | 90 seconds before alerting |
| Output preview lines | 5 | Enough context without bloating messages |
| Worker tags | `["worker"]` | Filter which sessions to monitor |

## Example Full Workflow

### Orchestrator Setup

```json
// 1. Create watchdog session
{"tool": "fugue_create_session", "input": {
  "name": "__watchdog",
  "command": "claude --dangerously-skip-permissions",
  "tags": ["watchdog"]
}}

// 2. Send system prompt to watchdog
{"tool": "fugue_send_input", "input": {
  "pane_id": "<watchdog_pane_id>",
  "input": "<watchdog system prompt from above>",
  "submit": true
}}

// 3. Start timer
{"tool": "fugue_watchdog_start", "input": {
  "pane_id": "<watchdog_pane_id>",
  "interval_secs": 30,
  "message": "check"
}}

// 4. Spawn workers...

// 5. Poll for alerts periodically
{"tool": "fugue_poll_messages", "input": {"worker_id": "<orchestrator_session>"}}
```

### Cleanup

```json
// Stop timer
{"tool": "fugue_watchdog_stop"}

// Kill watchdog session
{"tool": "fugue_kill_session", "input": {"session": "__watchdog"}}
```

## Integration with FEAT-104

This feature (FEAT-110) defines the watchdog agent's **internal behavior**. It complements FEAT-104 (Watchdog Orchestration Skill) which provides **skill commands** for managing the watchdog:

| FEAT-104 | FEAT-110 |
|----------|----------|
| `/orchestrate monitor start` | Watchdog agent behavior |
| Skill-based CLI interface | Agent prompt and preset |
| Timer management commands | Detection and alert logic |

## Troubleshooting

### Watchdog not detecting workers

1. Verify workers have `worker` tag: `fugue_get_tags(session: "<worker>")`
2. Check watchdog is receiving "check" messages: `fugue_read_pane(<watchdog_pane_id>)`
3. Verify timer is running: `fugue_watchdog_status()`

### False "stuck" alerts

- Increase `STUCK_THRESHOLD` intervals
- Verify workers are reporting status changes
- Check if output naturally pauses (e.g., long build)

### Missing alerts

- Verify watchdog session is running
- Check orchestrator is tagged: `fugue_get_tags()`
- Poll messages: `fugue_poll_messages()`

## Related Documents

- [AGENT_COOPERATION.md](./AGENT_COOPERATION.md) - Status reporting protocol
- [TAGS.md](./TAGS.md) - Session tag conventions
- [CONFIGURATION.md](./architecture/CONFIGURATION.md) - Preset configuration
