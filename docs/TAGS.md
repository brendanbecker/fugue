# Session Tags

Tags are arbitrary strings used for message routing and role identification. This document tracks conventional tags used across fugue workflows.

## Special Tags (Hardcoded)

| Tag | Behavior |
|-----|----------|
| `orchestrator` | Target for `fugue_report_status` and `fugue_request_help` messages |

## Conventional Tags

| Tag | Purpose |
|-----|---------|
| `worker` | Session doing implementation work; should not delegate further |
| `watchdog` | Session monitoring other sessions for timeouts/failures |

## Usage Examples

```bash
# Mark session as orchestrator
fugue_set_tags --add orchestrator

# Mark session as worker (prevents cascade spawning per CLAUDE.md)
fugue_set_tags --add worker

# Mark session as watchdog
fugue_set_tags --add watchdog
```

## Tag Routing

Use `fugue_send_orchestration` with tag-based targeting:

```json
{"target": {"tag": "orchestrator"}, "msg_type": "status.update", "payload": {...}}
{"target": {"tag": "watchdog"}, "msg_type": "heartbeat", "payload": {...}}
```
