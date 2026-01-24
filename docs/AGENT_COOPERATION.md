# Agent Cooperation Model

> How AI agents communicate status and coordinate within fugue

## Overview

fugue enables multi-agent workflows where an **orchestrator** coordinates multiple **worker** agents. This requires agents to cooperate by reporting their status using MCP tools.

**Key insight**: Rather than complex heuristics to detect agent state, agents simply tell fugue what they're doing.

## Status Protocol

### Available Statuses

| Status | Meaning | When to Use |
|--------|---------|-------------|
| `idle` | Not working, ready for tasks | Session started, between tasks |
| `working` | Actively processing | Started a task, making progress |
| `waiting_for_input` | Need user/orchestrator input | Asked a question, need clarification |
| `blocked` | Cannot proceed | Tool approval pending, external dependency |
| `complete` | Task finished successfully | Work item done |
| `error` | Task failed | Unrecoverable error encountered |

### Reporting Status

Use `fugue_report_status` to report state changes:

```json
{
  "tool": "fugue_report_status",
  "input": {
    "status": "working",
    "message": "Implementing fugue_expect polling loop"
  }
}
```

The `message` field is optional but recommended for context.

### Status Flow Example

```
Agent starts:     idle
User sends task:  working
Need clarification: waiting_for_input
User responds:    working
Hit a blocker:    blocked
Resolved:         working
Task done:        complete
```

## Message Routing

### Tags

Sessions can have tags for routing:

```json
// Orchestrator tags itself
{"tool": "fugue_set_tags", "input": {"add": ["orchestrator"]}}

// Worker tags itself
{"tool": "fugue_set_tags", "input": {"add": ["worker", "feat-096"]}}
```

### Sending Messages

```json
// To orchestrator
{"tool": "fugue_send_orchestration", "input": {
  "target": {"tag": "orchestrator"},
  "msg_type": "progress.update",
  "payload": {"percent": 75}
}}

// To specific session
{"tool": "fugue_send_orchestration", "input": {
  "target": {"session": "uuid-here"},
  "msg_type": "task.handoff",
  "payload": {"next_step": "review"}
}}

// Broadcast to all
{"tool": "fugue_broadcast", "input": {
  "msg_type": "announcement",
  "payload": {"text": "Build complete"}
}}
```

### Convenience Tools

```json
// Quick status report to orchestrator
{"tool": "fugue_report_status", "input": {"status": "working"}}

// Request help from orchestrator
{"tool": "fugue_request_help", "input": {"context": "Cannot resolve merge conflict"}}
```

## Orchestrator Responsibilities

The orchestrator session should:

1. **Tag itself**: `fugue_set_tags` with `["orchestrator"]`
2. **Monitor workers**: Periodically check `fugue_get_status` or `fugue_list_panes`
3. **Handle help requests**: Watch for `help.request` messages
4. **Aggregate progress**: Track which workers are done

### Monitoring Pattern

```json
// List all panes with their status
{"tool": "fugue_list_panes"}

// Get detailed status of specific pane
{"tool": "fugue_get_status", "input": {"pane_id": "uuid"}}
```

## Worker Responsibilities

Worker agents should:

1. **Report status changes**: Call `fugue_report_status` at transitions
2. **Tag themselves**: Optional but helpful for routing
3. **Request help when stuck**: Use `fugue_request_help`
4. **Report completion**: Status `complete` when done

### Minimal Compliance

At minimum, workers should report:
- `working` when starting a task
- `complete` when done
- `error` if failed

This enables basic orchestration awareness.

### Full Compliance

For better orchestration, also report:
- `waiting_for_input` when blocked on user input
- `blocked` with message when stuck
- Progress updates via `fugue_send_orchestration`

## Message Types (Convention)

| msg_type | Payload | Purpose |
|----------|---------|---------|
| `status.update` | `{status, message}` | General status |
| `progress.update` | `{percent, current_step}` | Progress indicator |
| `task.complete` | `{result, duration_ms}` | Task finished |
| `task.failed` | `{error, context}` | Task failed |
| `help.request` | `{context}` | Need assistance |
| `help.response` | `{guidance}` | Providing assistance |

These are conventions, not enforced schemas.

## Integration with CLAUDE.md

Add to your project's CLAUDE.md:

```markdown
## fugue Status Reporting

When running inside fugue (check for fugue MCP tools), report status:
- `fugue_report_status status:"working"` - when starting work
- `fugue_report_status status:"waiting_for_input"` - when need input
- `fugue_report_status status:"complete"` - when done
```

## Detecting fugue Environment

Agents can check if fugue MCP tools are available:

1. Look for `fugue_*` tools in available tool list
2. Check environment variable: `FUGUE_SESSION_ID` (if set)
3. Try calling `fugue_list_sessions` (will fail if not in fugue)

## Example: Multi-Agent Feature Development

```
Orchestrator (main session):
  1. Creates worktrees for 3 features
  2. Spawns worker sessions in each worktree
  3. Tags self as "orchestrator"
  4. Monitors worker status

Worker 1 (feat-096):
  1. Reports status:"working"
  2. Implements feature
  3. Hits blocker, reports status:"blocked"
  4. Orchestrator notices, provides guidance
  5. Continues, reports status:"working"
  6. Finishes, reports status:"complete"

Worker 2 (feat-094):
  1. Reports status:"working"
  2. Needs clarification, reports status:"waiting_for_input"
  3. Orchestrator sends guidance via message
  4. Continues...

Orchestrator:
  1. Sees all workers complete
  2. Aggregates results
  3. Coordinates merge
```

## Future Enhancements

- **Push notifications**: Orchestrator notified immediately on status change
- **Status history**: Track status transitions over time
- **Automatic escalation**: Blocked status for N minutes triggers alert
- **Status dashboard**: Visual overview of all agent states
