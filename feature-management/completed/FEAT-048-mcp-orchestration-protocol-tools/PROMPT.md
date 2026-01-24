# FEAT-048: Expose orchestration protocol via MCP tools

**Priority**: P2
**Component**: fugue-server
**Type**: new_feature
**Estimated Effort**: medium
**Business Value**: high
**Status**: unblocked (FEAT-028 complete)

## Overview

Add MCP tools for the existing orchestration message types, enabling agents to communicate directly without going through shell commands. fugue already has OrchestrationMessage types in `fugue-protocol/src/messages.rs`:

- **Message Format**: Generic `OrchestrationMessage { msg_type: String, payload: serde_json::Value }` for flexible, workflow-defined messages
- **Targets** (updated with FEAT-028 tag-based routing):
  - `Tagged(String)` - Send to sessions with a specific tag (e.g., "orchestrator", "worker", "evaluator")
  - `Session(Uuid)` - Send to specific session by ID
  - `Broadcast` - Broadcast to all sessions in same repo
  - `Worktree(String)` - Send to sessions in specific worktree

Currently these are only accessible via the daemon protocol (ClientMessage::SendOrchestration), not MCP.

## Architecture Context (Post-FEAT-028)

FEAT-028 replaced the binary orchestrator/worker model with flexible tag-based routing:
- Sessions have `tags: HashSet<String>` for classification
- `is_orchestrator()` now checks if tags contain "orchestrator"
- `OrchestrationTarget::Tagged("orchestrator")` replaces the old `OrchestrationTarget::Orchestrator`
- Multiple tags per session enable nuanced routing (e.g., "orchestrator", "primary", "evaluator")

## Benefits

- Agents become first-class participants in orchestration rather than passive recipients of nudges
- This is the "Orchestrator API Surface" Steve Yegge called for - enabling true agent-to-agent coordination
- Eliminates need for shell command workarounds for agent communication
- Provides structured, typed communication between agents

## Current Protocol Types (Post-FEAT-028)

From `fugue-protocol/src/messages.rs`:

```rust
/// Generic orchestration message with user-defined semantics
/// Allows workflows to define their own message types and payloads
pub struct OrchestrationMessage {
    /// User-defined message type identifier (e.g., "status_update", "task.assigned", "gas_town.auction")
    pub msg_type: String,
    /// Message payload as JSON - structure is defined by the workflow
    pub payload: serde_json::Value,
}

/// Target for orchestration messages (FEAT-028 tag-based routing)
pub enum OrchestrationTarget {
    /// Send to sessions with a specific tag (e.g., "orchestrator", "worker")
    Tagged(String),
    /// Send to specific session by ID
    Session(Uuid),
    /// Broadcast to all sessions in same repo
    Broadcast,
    /// Send to sessions in specific worktree
    Worktree(String),
}
```

**Note**: The old enum-based `OrchestrationMessage` with fixed variants (StatusUpdate, TaskAssignment, etc.) has been replaced with a flexible `msg_type` + `payload` structure. Workflows define their own message semantics.

### Common Message Type Conventions

While the protocol is now generic, these are common conventions:
- `status.update` - Worker status changes (idle, working, blocked, etc.)
- `task.assigned` - Task assignment from orchestrator
- `task.complete` - Task completion notification
- `help.request` - Worker requesting orchestrator assistance
- `sync.request` - Request state synchronization

## Implementation Tasks

### Section 1: Core MCP Tool

- [ ] Add `fugue_send_orchestration` MCP tool in `fugue-server/src/mcp/tools.rs`
- [ ] Define schema for target parameter (FEAT-028 tag-based routing):
  - `{tag: "orchestrator"}` - Send to sessions tagged "orchestrator"
  - `{tag: "worker"}` - Send to sessions tagged "worker"
  - `{session: "uuid"}` - Send to specific session
  - `{broadcast: true}` - Broadcast to all sessions in same repo
  - `{worktree: "path"}` - Send to sessions in specific worktree
- [ ] Define schema for message parameter with `msg_type` and `payload` fields
- [ ] Add handler that constructs and sends ClientMessage::SendOrchestration to daemon

### Section 1b: Tag Management Tools

- [ ] Add `fugue_set_tags` MCP tool to add/remove tags on a session
- [ ] Add `fugue_get_tags` MCP tool to retrieve session tags
- [ ] Allow orchestrator to self-identify by adding "orchestrator" tag

### Section 2: Convenience Tools

- [ ] Add `fugue_report_status(status, message)` - shorthand for status.update message
  - Auto-fills session_id from current session context
  - status: one of "idle", "working", "waiting_for_input", "blocked", "complete", "error"
  - message: optional string
  - Automatically targets `Tagged("orchestrator")`
- [ ] Add `fugue_request_help(context)` - shorthand for help.request message
  - Auto-fills session_id from current session context
  - Automatically targets `Tagged("orchestrator")`
- [ ] Add `fugue_broadcast(message)` - shorthand for broadcast message
  - Auto-fills from_session_id from current session context
  - Automatically uses `Broadcast` target

### Section 3: Subscription/Notification

- [ ] Add `fugue_subscribe_orchestration` MCP tool (or use notifications)
- [ ] Investigate MCP notification mechanism for async message delivery
- [ ] Consider polling alternative if notifications are complex

### Section 4: Testing

- [ ] Add unit tests for tool schema validation
- [ ] Add integration tests for message routing
- [ ] Test all OrchestrationMessage variants
- [ ] Test all OrchestrationTarget variants
- [ ] Test error cases (NoRepository, NoRecipients)

### Section 5: Documentation

- [ ] Document tool schemas in MCP tool listing
- [ ] Add usage examples for common orchestration patterns
- [ ] Update CLAUDE.md with orchestration tool guidance

## Acceptance Criteria

- [ ] `fugue_send_orchestration` tool is available and functional
- [ ] All OrchestrationMessage variants can be sent via MCP
- [ ] All OrchestrationTarget variants are supported
- [ ] Convenience tools simplify common operations
- [ ] Agents can receive orchestration messages (via subscription or polling)
- [ ] All tests passing
- [ ] Documentation updated

## Tool Schema Design (Updated for FEAT-028)

### fugue_send_orchestration

```json
{
  "name": "fugue_send_orchestration",
  "description": "Send orchestration message to other sessions using tag-based routing",
  "inputSchema": {
    "type": "object",
    "properties": {
      "target": {
        "oneOf": [
          {"type": "object", "properties": {"tag": {"type": "string", "description": "Send to sessions with this tag (e.g., 'orchestrator', 'worker')"}}, "required": ["tag"]},
          {"type": "object", "properties": {"session": {"type": "string", "format": "uuid"}}, "required": ["session"]},
          {"type": "object", "properties": {"broadcast": {"const": true}}, "required": ["broadcast"]},
          {"type": "object", "properties": {"worktree": {"type": "string"}}, "required": ["worktree"]}
        ]
      },
      "msg_type": {
        "type": "string",
        "description": "Message type identifier (e.g., 'status.update', 'task.assigned', 'gas_town.auction')"
      },
      "payload": {
        "type": "object",
        "description": "Message payload - structure defined by the workflow/message type"
      }
    },
    "required": ["target", "msg_type", "payload"]
  }
}
```

### fugue_set_tags (New - Tag Management)

```json
{
  "name": "fugue_set_tags",
  "description": "Add or remove tags on a session for routing purposes",
  "inputSchema": {
    "type": "object",
    "properties": {
      "session": {
        "type": "string",
        "description": "Session UUID or name. Uses active session if omitted."
      },
      "add": {
        "type": "array",
        "items": {"type": "string"},
        "description": "Tags to add (e.g., ['orchestrator', 'primary'])"
      },
      "remove": {
        "type": "array",
        "items": {"type": "string"},
        "description": "Tags to remove"
      }
    }
  }
}
```

### fugue_get_tags (New - Tag Query)

```json
{
  "name": "fugue_get_tags",
  "description": "Get tags from a session",
  "inputSchema": {
    "type": "object",
    "properties": {
      "session": {
        "type": "string",
        "description": "Session UUID or name. Uses active session if omitted."
      }
    }
  }
}
```

### fugue_report_status (convenience)

```json
{
  "name": "fugue_report_status",
  "description": "Report current session status to orchestrator (sends to sessions tagged 'orchestrator')",
  "inputSchema": {
    "type": "object",
    "properties": {
      "status": {
        "type": "string",
        "enum": ["idle", "working", "waiting_for_input", "blocked", "complete", "error"]
      },
      "message": {
        "type": "string",
        "description": "Optional status message"
      }
    },
    "required": ["status"]
  }
}
```

## Dependencies

- **FEAT-028** (Orchestration Flexibility Refactor): COMPLETED - Tag-based routing now available
- **FEAT-050** (Session Metadata Storage): COMPLETED - Can be used for additional orchestration state

## Notes

- The daemon already handles SendOrchestration messages, so this is primarily an MCP surface layer
- Need to determine how to get current session context for auto-fill features
- Consider rate limiting for broadcast messages
- Error handling should surface NoRepository and NoRecipients errors clearly
- Tags are stored in `Session.tags: HashSet<String>` - need to expose via MCP
- Consider using FEAT-050's metadata storage for orchestration state that doesn't fit in tags
