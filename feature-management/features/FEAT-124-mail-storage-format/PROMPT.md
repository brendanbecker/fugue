# FEAT-124: Filesystem-Based Mail Storage Format

**Priority**: P1
**Component**: mail
**Effort**: Small
**Status**: new

## Summary

Define a filesystem-based mail storage format for asynchronous agent-to-agent communication. Messages are stored in `.mail/{recipient}/` directories with YAML frontmatter metadata.

## Problem

Current agent communication relies on:
- `fugue_send_orchestration` - requires recipient to be online and polling
- `fugue_send_input` - synchronous, interrupts recipient

Neither supports true async "fire and forget" messaging where:
- Sender writes message and continues working
- Recipient checks mail when convenient
- Messages persist across session restarts

## Proposed Format

### Directory Structure

```
.mail/
├── orchestrator/           # Mailbox for sessions tagged "orchestrator"
│   ├── 2024-01-28T15-30-00_worker-bug-069_status.md
│   └── 2024-01-28T15-32-00_watchdog_alert.md
├── nexus/                  # Mailbox for sessions tagged "nexus"
│   └── 2024-01-28T15-35-00_orch-fugue_report.md
└── worker-feat-104/        # Mailbox for specific session
    └── 2024-01-28T15-40-00_orchestrator_task.md
```

### Message Format

```markdown
---
from: worker-bug-069
to: orchestrator
type: status
timestamp: 2024-01-28T15:30:00Z
needs_response: false
priority: normal
tags: [BUG-069, fugue]
---

## Status Update

BUG-069 investigation complete. Root cause identified:
- Messages delivered to wrong session due to attached_session fallback
- Fix implemented: made worker_id optional in poll_messages

Ready for review and merge.
```

### Frontmatter Fields

| Field | Required | Description |
|-------|----------|-------------|
| `from` | Yes | Sender session name or tag |
| `to` | Yes | Recipient session name or tag |
| `type` | Yes | Message type: `status`, `alert`, `task`, `question`, `response` |
| `timestamp` | Yes | ISO 8601 timestamp |
| `needs_response` | No | Boolean, default false |
| `priority` | No | `urgent`, `normal`, `low` (default: normal) |
| `tags` | No | List of relevant tags for filtering |
| `in_reply_to` | No | Filename of message being replied to |
| `thread_id` | No | Thread identifier for conversation grouping |

### Filename Convention

```
{timestamp}_{from}_{type}.md

Examples:
2024-01-28T15-30-00_worker-bug-069_status.md
2024-01-28T15-32-00_watchdog_alert.md
2024-01-28T15-35-00_orch-fugue_task.md
```

## Mailbox Resolution

Recipients can be:
1. **Session name**: Exact match (e.g., `worker-bug-069`)
2. **Tag**: Any session with that tag (e.g., `orchestrator`)
3. **Role**: Predefined roles (`nexus`, `orchestrator`, `watchdog`)

When sending to a tag, the message goes to `.mail/{tag}/` directory. Any session with that tag can read from it.

## Message Lifecycle

1. **Created**: Sender writes to `.mail/{recipient}/`
2. **Unread**: File exists, not yet processed
3. **Read**: Recipient reads and optionally moves to `.mail/{recipient}/read/`
4. **Archived**: Old messages moved to `.mail/{recipient}/archive/`

## Acceptance Criteria

- [ ] Directory structure documented and standardized
- [ ] YAML frontmatter schema defined
- [ ] Filename convention documented
- [ ] Message lifecycle states defined
- [ ] Mailbox resolution rules documented (name vs tag)
- [ ] Example messages provided

## Related

- FEAT-125: MCP Mail Commands (tools to send/read)
- FEAT-126: Watchdog Mail Checking (integration)
- FEAT-104: Watchdog Orchestration Skill
