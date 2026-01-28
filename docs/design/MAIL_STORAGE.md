# Mail Storage Format

> Filesystem-based asynchronous messaging for agent-to-agent communication

## Overview

The mail system provides **persistent, asynchronous** communication between agents. Unlike real-time tools (`fugue_send_orchestration`, `fugue_report_status`), mail messages:

- **Persist to disk** - survive session restarts
- **Fire and forget** - sender doesn't wait for recipient
- **Self-serve** - recipient checks mail when convenient
- **Auditable** - complete history in plain text files

## When to Use Mail vs Real-Time

| Use Case | Tool |
|----------|------|
| Status updates (recipient online) | `fugue_report_status` |
| Immediate coordination | `fugue_send_orchestration` |
| Task handoffs across sessions | **Mail** |
| Reports for later review | **Mail** |
| Messages when recipient offline | **Mail** |
| Notifications that don't need immediate action | **Mail** |

## Directory Structure

```
.mail/
├── orchestrator/                    # Mailbox for "orchestrator" tag
│   ├── 20260128T153000Z_worker-bug-069_status.md
│   ├── 20260128T153200Z_watchdog_alert.md
│   └── read/                        # Processed messages
│       └── 20260128T150000Z_worker-feat-104_status.md
├── nexus/                           # Mailbox for "nexus" tag
│   └── 20260128T153500Z_orch-fugue_report.md
├── worker-feat-104/                 # Mailbox for specific session
│   ├── 20260128T154000Z_orchestrator_task.md
│   └── archive/                     # Historical messages
│       └── 20260128T120000Z_orchestrator_task.md
└── __broadcast/                     # Messages to all sessions
    └── 20260128T160000Z_nexus_announcement.md
```

### Directory Naming

| Type | Directory Name | Example |
|------|---------------|---------|
| Session name | Exact match | `worker-feat-104/` |
| Tag | Tag name | `orchestrator/` |
| Broadcast | `__broadcast` | `__broadcast/` |

### Subdirectories

| Subdirectory | Purpose |
|--------------|---------|
| (root) | Unread messages |
| `read/` | Messages marked as read but not archived |
| `archive/` | Historical messages for reference |

## Message Format

Messages are Markdown files with YAML frontmatter:

```markdown
---
from: worker-bug-069
to: orchestrator
type: status
timestamp: 2026-01-28T15:30:00Z
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

### Frontmatter Schema

#### Required Fields

| Field | Type | Description |
|-------|------|-------------|
| `from` | string | Sender session name |
| `to` | string | Recipient (session name, tag, or `__broadcast`) |
| `type` | enum | Message type (see below) |
| `timestamp` | ISO 8601 | When message was created |

#### Optional Fields

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `needs_response` | boolean | `false` | Whether sender expects a reply |
| `priority` | enum | `normal` | `urgent`, `normal`, `low` |
| `tags` | string[] | `[]` | Tags for filtering (task IDs, topics) |
| `in_reply_to` | string | - | Filename of message being replied to |
| `thread_id` | string | - | Groups related messages into conversation |
| `expires` | ISO 8601 | - | When message can be auto-deleted |
| `read_at` | ISO 8601 | - | When recipient read the message |

### Message Types

| Type | Purpose | Typical Sender |
|------|---------|----------------|
| `status` | Progress/completion update | Worker |
| `alert` | Attention needed | Watchdog |
| `task` | Work assignment | Orchestrator |
| `question` | Need input/clarification | Any |
| `response` | Reply to question | Any |
| `report` | Summary/aggregation | Any |
| `handoff` | Transferring responsibility | Worker |

## Filename Convention

```
{timestamp}_{from}_{type}.md
```

### Format Details

- **timestamp**: ISO 8601 compact format: `YYYYMMDDTHHMMSSZ`
  - Always UTC (Z suffix)
  - No separators (colons invalid in filenames)
- **from**: Sender session name (hyphens allowed)
- **type**: Message type

### Examples

```
20260128T153000Z_worker-bug-069_status.md
20260128T153200Z_watchdog_alert.md
20260128T154000Z_orchestrator_task.md
20260128T155500Z_orch-fugue_question.md
```

### Why This Convention

1. **Chronological sorting**: Timestamp prefix = `ls` shows oldest first
2. **Sender identification**: Quick visual scan shows who sent what
3. **Type filtering**: `*.status.md` or `*_alert.md` globs work
4. **Uniqueness**: Timestamp + sender + type nearly guarantees uniqueness
5. **No special characters**: Works on all filesystems

## Mailbox Resolution

When sending a message to recipient `X`:

```
1. Is X a session name that exists?
   → Use .mail/{session-name}/

2. Is X a known tag?
   → Use .mail/{tag}/

3. Is X "__broadcast"?
   → Use .mail/__broadcast/

4. Otherwise:
   → Create .mail/{X}/ (assumes future session/tag)
```

### Tag-Based Mailboxes

Multiple sessions can share a tag-based mailbox:

```
Sessions tagged "worker":
  - worker-feat-104
  - worker-bug-069
  - worker-feat-105

Message to "worker" goes to: .mail/worker/

Any session tagged "worker" can read from .mail/worker/
```

**Important**: Tag mailboxes are shared. If multiple workers read from `.mail/worker/`, coordinate who processes which messages (e.g., first to read moves to `read/`).

### Session-Specific Mailboxes

For direct messages, use the exact session name:

```
Message to "worker-feat-104" goes to: .mail/worker-feat-104/

Only worker-feat-104 should read from this mailbox.
```

## Message Lifecycle

```
┌─────────┐    write     ┌────────┐    read     ┌──────┐
│ Created │ ──────────▶  │ Unread │ ─────────▶  │ Read │
└─────────┘              └────────┘              └──────┘
                                                    │
                                                    │ archive
                                                    ▼
                                               ┌──────────┐
                                               │ Archived │
                                               └──────────┘
```

### States

| State | Location | Description |
|-------|----------|-------------|
| Created | `.mail/{recipient}/` | Sender writes file |
| Unread | `.mail/{recipient}/` | File exists, not processed |
| Read | `.mail/{recipient}/read/` | Recipient processed, kept for reference |
| Archived | `.mail/{recipient}/archive/` | Old messages, may be pruned |

### Processing Messages

**Reading mail:**
```bash
# List unread messages
ls .mail/orchestrator/*.md

# Read a message
cat .mail/orchestrator/20260128T153000Z_worker-bug-069_status.md

# Mark as read (move to read/)
mv .mail/orchestrator/20260128T153000Z_worker-bug-069_status.md \
   .mail/orchestrator/read/
```

**Archiving:**
```bash
# Archive old read messages
mv .mail/orchestrator/read/*.md .mail/orchestrator/archive/
```

**Checking for mail:**
```bash
# Any unread messages?
ls .mail/orchestrator/*.md 2>/dev/null

# Count unread
ls .mail/orchestrator/*.md 2>/dev/null | wc -l
```

## Concurrency Considerations

### Multiple Readers (Tag Mailboxes)

When multiple sessions read from a shared mailbox:

1. **Claim before processing**: Move file to `read/` atomically before processing
2. **Check if already claimed**: If `mv` fails, another reader got it
3. **Process then archive**: After handling, move to `archive/`

```bash
# Atomic claim (fails if file doesn't exist)
mv .mail/worker/20260128T153000Z_orch_task.md \
   .mail/worker/read/ 2>/dev/null && \
   echo "Claimed, processing..." || \
   echo "Already claimed by another reader"
```

### Multiple Writers

Multiple senders writing to same mailbox is safe:
- Unique filenames (timestamp + sender) prevent collisions
- Filesystem write is atomic for small files

## Integration with Existing Tools

### Relationship to fugue_send_orchestration

| Aspect | `fugue_send_orchestration` | Mail |
|--------|---------------------------|------|
| Delivery | Real-time, in-memory | Filesystem |
| Recipient state | Must be polling | Can be offline |
| Persistence | Lost on restart | Survives restart |
| Speed | Immediate | On next check |
| Use case | Coordination | Handoffs, reports |

### Hybrid Pattern

Use both for reliability:

```python
# Send real-time AND persist to mail
fugue_send_orchestration(target={"tag": "orchestrator"}, ...)
write_mail(to="orchestrator", ...)  # Backup if recipient missed real-time
```

## Implementation Notes

### MCP Tools (FEAT-125)

Future MCP tools will provide:
- `fugue_mail_send` - Create and write message
- `fugue_mail_list` - List messages in mailbox
- `fugue_mail_read` - Read message content
- `fugue_mail_mark_read` - Move to read/
- `fugue_mail_archive` - Move to archive/

### Watchdog Integration (FEAT-126)

Watchdog can check mail as part of monitoring loop:
1. Check `.mail/orchestrator/` for urgent alerts
2. Process any `priority: urgent` messages immediately
3. Forward to orchestrator via real-time channel

## Examples

See [FEAT-124 example files](../../feature-management/features/FEAT-124-mail-storage-format/examples/) for complete message examples.

## Related

- [AGENT_COOPERATION.md](../AGENT_COOPERATION.md) - Real-time status protocol
- [WATCHDOG_MONITOR.md](../WATCHDOG_MONITOR.md) - Monitoring pattern
- FEAT-125: MCP Mail Commands
- FEAT-126: Watchdog Mail Checking
