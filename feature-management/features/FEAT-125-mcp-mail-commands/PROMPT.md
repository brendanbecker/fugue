# FEAT-125: MCP Mail Commands

**Priority**: P1
**Component**: mcp/mail
**Effort**: Medium
**Status**: complete

## Summary

Add MCP tools for agents to send, check, read, and manage filesystem-based mail. These tools interact with the `.mail/` directory structure defined in FEAT-124.

## Proposed Tools

### `fugue_mail_send`

Send a message to another agent's mailbox.

```json
{
  "name": "fugue_mail_send",
  "description": "Send an async message to another agent's mailbox",
  "parameters": {
    "to": {
      "type": "string",
      "description": "Recipient session name or tag (e.g., 'orchestrator', 'worker-bug-069')",
      "required": true
    },
    "type": {
      "type": "string",
      "enum": ["status", "alert", "task", "question", "response"],
      "description": "Message type",
      "required": true
    },
    "subject": {
      "type": "string",
      "description": "Brief subject line",
      "required": true
    },
    "body": {
      "type": "string",
      "description": "Message body (markdown)",
      "required": true
    },
    "needs_response": {
      "type": "boolean",
      "description": "Whether a response is expected",
      "default": false
    },
    "priority": {
      "type": "string",
      "enum": ["urgent", "normal", "low"],
      "default": "normal"
    },
    "tags": {
      "type": "array",
      "items": {"type": "string"},
      "description": "Tags for filtering/categorization"
    },
    "in_reply_to": {
      "type": "string",
      "description": "Filename of message being replied to"
    }
  }
}
```

**Response:**
```json
{
  "success": true,
  "filename": "2024-01-28T15-30-00_worker-bug-069_status.md",
  "mailbox": ".mail/orchestrator/"
}
```

### `fugue_mail_check`

Check for new mail without reading content.

```json
{
  "name": "fugue_mail_check",
  "description": "Check mailbox for unread messages",
  "parameters": {
    "mailbox": {
      "type": "string",
      "description": "Mailbox to check (defaults to caller's session name and tags)"
    },
    "type": {
      "type": "string",
      "description": "Filter by message type"
    },
    "priority": {
      "type": "string",
      "description": "Filter by priority"
    },
    "needs_response": {
      "type": "boolean",
      "description": "Filter to messages expecting response"
    }
  }
}
```

**Response:**
```json
{
  "mailbox": "orchestrator",
  "unread_count": 3,
  "messages": [
    {
      "filename": "2024-01-28T15-30-00_worker-bug-069_status.md",
      "from": "worker-bug-069",
      "type": "status",
      "subject": "BUG-069 Complete",
      "priority": "normal",
      "needs_response": false,
      "timestamp": "2024-01-28T15:30:00Z"
    }
  ]
}
```

### `fugue_mail_read`

Read a specific message.

```json
{
  "name": "fugue_mail_read",
  "description": "Read a message from mailbox",
  "parameters": {
    "mailbox": {
      "type": "string",
      "description": "Mailbox containing the message",
      "required": true
    },
    "filename": {
      "type": "string",
      "description": "Message filename to read",
      "required": true
    },
    "mark_read": {
      "type": "boolean",
      "description": "Move to read/ subdirectory",
      "default": true
    }
  }
}
```

**Response:**
```json
{
  "filename": "2024-01-28T15-30-00_worker-bug-069_status.md",
  "from": "worker-bug-069",
  "to": "orchestrator",
  "type": "status",
  "timestamp": "2024-01-28T15:30:00Z",
  "needs_response": false,
  "priority": "normal",
  "tags": ["BUG-069", "fugue"],
  "subject": "BUG-069 Complete",
  "body": "## Status Update\n\nBUG-069 investigation complete..."
}
```

### `fugue_mail_list`

List all messages in a mailbox with optional filters.

```json
{
  "name": "fugue_mail_list",
  "description": "List messages in mailbox",
  "parameters": {
    "mailbox": {
      "type": "string",
      "description": "Mailbox to list"
    },
    "include_read": {
      "type": "boolean",
      "description": "Include read messages",
      "default": false
    },
    "from": {
      "type": "string",
      "description": "Filter by sender"
    },
    "type": {
      "type": "string",
      "description": "Filter by type"
    },
    "since": {
      "type": "string",
      "description": "ISO timestamp, only messages after this time"
    },
    "limit": {
      "type": "integer",
      "description": "Max messages to return",
      "default": 50
    }
  }
}
```

### `fugue_mail_delete`

Delete or archive a message.

```json
{
  "name": "fugue_mail_delete",
  "description": "Delete or archive a message",
  "parameters": {
    "mailbox": {
      "type": "string",
      "required": true
    },
    "filename": {
      "type": "string",
      "required": true
    },
    "archive": {
      "type": "boolean",
      "description": "Move to archive instead of deleting",
      "default": true
    }
  }
}
```

## Implementation Notes

### Caller Identity

Tools need to know the caller's identity for:
- Default mailbox resolution (check own mail)
- Setting `from` field automatically

Options:
1. Require explicit session name in requests
2. Infer from MCP connection context (if FEAT-BUG-073 is fixed)
3. Use attached session as fallback

### File Operations

All file operations should be atomic:
- Write to temp file, then rename
- Use file locking for concurrent access
- Handle missing directories gracefully (create on first write)

### Mailbox Discovery

When checking mail, agent should check:
1. `.mail/{session_name}/` - direct messages
2. `.mail/{tag}/` for each tag the session has

## Acceptance Criteria

- [x] `fugue_mail_send` creates properly formatted messages
- [x] `fugue_mail_check` returns unread message summaries
- [x] `fugue_mail_read` returns full message content
- [x] `fugue_mail_list` supports filtering options
- [x] `fugue_mail_delete` archives by default
- [x] Automatic `from` field based on caller identity
- [x] Tools handle missing directories gracefully
- [x] Atomic file operations prevent corruption

## Related

- FEAT-124: Mail Storage Format (defines the format)
- FEAT-126: Watchdog Mail Checking (integration)
- BUG-073: get_tags wrong session (affects caller identity)
