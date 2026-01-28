# FEAT-126: Watchdog Mail Checking Integration

**Priority**: P2
**Component**: orchestration/watchdog
**Effort**: Small
**Status**: complete

## Summary

Integrate mail checking into the watchdog monitoring cycle. The watchdog should check its mailbox and the orchestrator's mailbox for messages that need attention, forwarding alerts as needed.

## Problem

With the mail system (FEAT-124, FEAT-125), agents can send async messages. But if no one checks the mail, messages go unread. The watchdog is already running periodic checks - it should also check mailboxes.

## Proposed Behavior

### Watchdog Mail Check Cycle

Add to the existing watchdog cycle:

```
poll workers → detect issues → CHECK MAIL → notify (if needed) → clear
```

### What Watchdog Checks

1. **Orchestrator's mailbox** (`.mail/orchestrator/`)
   - Messages needing response (`needs_response: true`)
   - Urgent priority messages
   - Task assignments

2. **Watchdog's own mailbox** (`.mail/watchdog/` or `.mail/__watchdog/`)
   - Direct commands from orchestrator
   - Configuration updates

### Alert Types for Mail

New message types for watchdog → orchestrator alerts:

```json
{
  "msg_type": "mail.urgent",
  "payload": {
    "mailbox": "orchestrator",
    "count": 2,
    "messages": [
      {
        "from": "worker-bug-069",
        "type": "question",
        "subject": "Need clarification on scope",
        "needs_response": true
      }
    ]
  }
}
```

```json
{
  "msg_type": "mail.pending_responses",
  "payload": {
    "mailbox": "orchestrator",
    "count": 3,
    "oldest": "2024-01-28T14:00:00Z",
    "messages": [...]
  }
}
```

### Watchdog System Prompt Addition

Add to watchdog prompt (FEAT-110):

```markdown
## Mail Checking

On each check cycle, also check mailboxes:

1. `fugue_mail_check(mailbox: "orchestrator", needs_response: true)`
2. `fugue_mail_check(mailbox: "orchestrator", priority: "urgent")`
3. `fugue_mail_check(mailbox: "__watchdog")`

If mail needs attention:
- Urgent messages → send `mail.urgent` alert
- Messages awaiting response for >1 hour → send `mail.pending_responses` alert
- Direct commands to watchdog → execute them

Do NOT alert for:
- Read messages
- Low priority status updates
- Messages that don't need response
```

### Configuration

Add to watchdog config:

```toml
[watchdog.mail]
# Enable mail checking
enabled = true

# Mailboxes to monitor (in addition to own mailbox)
watch_mailboxes = ["orchestrator"]

# Alert threshold for pending responses (seconds)
pending_response_threshold = 3600

# Check for urgent messages
check_urgent = true

# Check for needs_response messages
check_needs_response = true
```

## Implementation Notes

### Minimal Alerts

Following FEAT-110's principle: only alert when action needed.

**Alert:**
- Urgent messages exist
- Messages need response and are older than threshold
- Direct commands for watchdog

**Don't alert:**
- Normal status updates (orchestrator reads when ready)
- Already-read messages
- Low priority informational messages

### Mail Check Frequency

Mail check happens every watchdog cycle (default 30s). This is frequent enough for urgent messages but not so frequent as to be wasteful.

### Watchdog's Own Mail

The watchdog can receive commands via mail:
- `type: command` - Execute a specific action
- `type: config` - Update watchdog configuration
- `type: query` - Return specific information

Example command message:
```yaml
---
from: orchestrator
to: __watchdog
type: command
---
Check worker-feat-104 immediately and report status.
```

## Acceptance Criteria

- [x] Watchdog checks orchestrator's mailbox each cycle
- [x] Watchdog checks its own mailbox each cycle
- [x] Alerts sent for urgent messages
- [x] Alerts sent for stale needs_response messages
- [x] Configurable thresholds and mailboxes
- [x] Watchdog can receive and execute commands via mail
- [x] No alerts for routine/read messages

## Implementation Notes

### What Was Implemented

Updated `.claude/skills/orchestrate.md` with:

1. **Extended Watchdog Prompt**: Added STEP 2 (Check Mailboxes) to the monitoring cycle
   - Checks orchestrator mailbox for urgent and needs_response messages
   - Checks __watchdog mailbox for direct commands

2. **New Alert Types**: Defined JSON formats for:
   - `mail.urgent` - urgent priority messages that need immediate attention
   - `mail.pending_responses` - messages awaiting response for >1 hour

3. **Mail Alert Rules**: Clear guidance on what triggers alerts vs what doesn't

4. **Configuration Options**: Added environment variables and TOML config:
   - `WATCHDOG_MAIL_ENABLED` - toggle mail checking
   - `WATCHDOG_MAIL_BOXES` - which mailboxes to monitor
   - `WATCHDOG_MAIL_PENDING_THRESHOLD` - seconds before pending alert
   - `WATCHDOG_MAIL_CHECK_URGENT` - toggle urgent checking
   - `WATCHDOG_MAIL_CHECK_NEEDS_RESPONSE` - toggle needs_response checking

5. **Command Handling**: Instructions for processing watchdog commands received via mail

### Dependencies

This feature requires FEAT-124 (Mail Storage Format) and FEAT-125 (MCP Mail Commands) to be implemented for the mail tools to exist. The watchdog prompt is ready and will work once those tools are available:
- `fugue_mail_check` - used to check mailboxes
- `fugue_mail_read` - used to read command messages
- `fugue_mail_send` - used to reply to queries

## Related

- FEAT-124: Mail Storage Format
- FEAT-125: MCP Mail Commands
- FEAT-110: Watchdog Monitor Agent (base watchdog behavior)
- FEAT-111: Watchdog Auto-Clear Cycle
