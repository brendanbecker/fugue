# FEAT-122: Add "parent" target type for orchestration messages

**Priority**: P1
**Component**: fugue-server, fugue-protocol
**Type**: enhancement
**Estimated Effort**: small
**Business Value**: high

## Overview

Add a `{"target": {"parent": true}}` option to `fugue_send_orchestration` that automatically resolves the sender's parent session using the `child:<parent-name>` lineage tag pattern.

## Problem Statement

Currently, `fugue_report_status` and `fugue_request_help` hardcode the target as `OrchestrationTarget::Tagged("orchestrator")`. This causes problems in multi-orchestrator scenarios:

1. A watchdog spawned by `orch-claude` reports status to **all** sessions tagged "orchestrator"
2. If `orch-gemini` is also running, it receives irrelevant status updates
3. There's no way for a child session to reliably message only its parent

The `child:<parent>` lineage tag pattern already exists (documented in CLAUDE.md) but isn't being leveraged for message routing.

## Current Behavior

```rust
// handlers.rs:1144 - report_status hardcodes target
let target = OrchestrationTarget::Tagged("orchestrator".to_string());

// handlers.rs:1204 - request_help hardcodes target
let target = OrchestrationTarget::Tagged("orchestrator".to_string());
```

MCP tool only accepts:
```json
{"target": {"tag": "orchestrator"}}     // All orchestrators
{"target": {"session": "<uuid>"}}       // Specific session by UUID
{"target": {"broadcast": true}}         // All sessions
{"target": {"worktree": "/path"}}       // Sessions in worktree
```

## Desired Behavior

Add parent target resolution:
```json
{"target": {"parent": true}}
```

This would:
1. Get the sender session's tags
2. Find the `child:<parent-name>` tag
3. Resolve `<parent-name>` to a session UUID
4. Route the message to that specific session

### Automatic Parent Resolution

`fugue_report_status` and `fugue_request_help` should use Parent target by default instead of broadcasting to all orchestrators.

## Implementation

### Section 1: Protocol Changes

**File**: `fugue-protocol/src/messages.rs`

Add Parent variant to OrchestrationTarget:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum OrchestrationTarget {
    /// Send to sessions with a specific tag
    Tagged(String),
    /// Send to specific session by ID
    Session(Uuid),
    /// Broadcast to all sessions in same repo
    Broadcast,
    /// Send to sessions in specific worktree
    Worktree(String),
    /// Send to parent session (resolved from child:<parent> tag)
    Parent,
}
```

### Section 2: MCP Tool Schema Update

**File**: `fugue-server/src/mcp/tools.rs`

Update the `fugue_send_orchestration` target schema to include parent option:

```json
"target": {
  "description": "Target for the message. Use ONE of: {\"tag\": \"..\"}, {\"session\": \"uuid\"}, {\"broadcast\": true}, {\"worktree\": \"path\"}, {\"parent\": true}",
  "oneOf": [
    // ... existing options ...
    {
      "properties": {
        "parent": {
          "const": true,
          "description": "Send to parent session (resolved from child:<name> tag)",
          "type": "boolean"
        }
      },
      "required": ["parent"],
      "type": "object"
    }
  ]
}
```

### Section 3: Handler Implementation

**File**: `fugue-server/src/mcp/bridge/handlers.rs`

#### 3a. Update target parsing in `tool_send_orchestration`

```rust
// Around line 1033
let orchestration_target = if let Some(tag) = target.get("tag").and_then(|v| v.as_str()) {
    OrchestrationTarget::Tagged(tag.to_string())
} else if let Some(session) = target.get("session").and_then(|v| v.as_str()) {
    let session_id = Uuid::parse_str(session)
        .map_err(|e| McpError::InvalidParams(format!("Invalid session UUID: {}", e)))?;
    OrchestrationTarget::Session(session_id)
} else if target.get("broadcast").and_then(|v| v.as_bool()).unwrap_or(false) {
    OrchestrationTarget::Broadcast
} else if let Some(worktree) = target.get("worktree").and_then(|v| v.as_str()) {
    OrchestrationTarget::Worktree(worktree.to_string())
} else if target.get("parent").and_then(|v| v.as_bool()).unwrap_or(false) {
    OrchestrationTarget::Parent
} else {
    return Err(McpError::InvalidParams(
        "Invalid target: must specify 'tag', 'session', 'broadcast', 'worktree', or 'parent'".into(),
    ));
};
```

#### 3b. Handle Parent resolution in daemon

**File**: `fugue-server/src/session/manager.rs` (or wherever orchestration routing happens)

When processing `OrchestrationTarget::Parent`:

```rust
OrchestrationTarget::Parent => {
    // Get sender's session tags
    let sender_tags = get_session_tags(sender_session_id)?;

    // Find child:<parent> tag
    let parent_name = sender_tags.iter()
        .find_map(|tag| tag.strip_prefix("child:"))
        .ok_or_else(|| "No child:<parent> tag found on sender session")?;

    // Resolve parent name to session UUID
    let parent_session = find_session_by_name(parent_name)
        .ok_or_else(|| format!("Parent session '{}' not found", parent_name))?;

    // Route to parent
    send_to_session(parent_session.id, message)?;
}
```

### Section 4: Update Status Reporting Tools

**File**: `fugue-server/src/mcp/bridge/handlers.rs`

#### 4a. Update `tool_report_status` (line ~1141)

```rust
pub async fn tool_report_status(
    &mut self,
    status: &str,
    message: Option<&str>,
) -> Result<ToolResult, McpError> {
    let current_issue_id = self.get_current_issue_id().await;

    // Changed: Use Parent target instead of Tagged("orchestrator")
    let target = OrchestrationTarget::Parent;

    // ... rest unchanged ...
}
```

#### 4b. Update `tool_request_help` (line ~1203)

```rust
pub async fn tool_request_help(&mut self, context: &str) -> Result<ToolResult, McpError> {
    // Changed: Use Parent target instead of Tagged("orchestrator")
    let target = OrchestrationTarget::Parent;

    // ... rest unchanged ...
}
```

### Section 5: Fallback Behavior

If no `child:<parent>` tag exists on the sender, the Parent target should:

1. **Option A (Recommended)**: Fall back to `Tagged("orchestrator")` for backwards compatibility
2. **Option B**: Return an error explaining no parent tag was found

Implement Option A:

```rust
OrchestrationTarget::Parent => {
    let sender_tags = get_session_tags(sender_session_id)?;

    if let Some(parent_name) = sender_tags.iter().find_map(|tag| tag.strip_prefix("child:")) {
        if let Some(parent_session) = find_session_by_name(parent_name) {
            return send_to_session(parent_session.id, message);
        }
        // Parent name found but session doesn't exist - fall through to Tagged
        log::warn!("Parent session '{}' not found, falling back to 'orchestrator' tag", parent_name);
    }

    // Fallback: send to all orchestrators
    send_to_tagged("orchestrator", message)?;
}
```

## Files to Modify

| File | Changes |
|------|---------|
| `fugue-protocol/src/messages.rs` | Add `Parent` variant to `OrchestrationTarget` |
| `fugue-server/src/mcp/tools.rs` | Update tool schema for parent target |
| `fugue-server/src/mcp/bridge/handlers.rs` | Parse parent target, update report_status/request_help |
| `fugue-server/src/session/manager.rs` | Handle Parent target resolution |

## Implementation Tasks

### Section 1: Protocol Changes
- [ ] Add `Parent` variant to `OrchestrationTarget` enum
- [ ] Run `cargo build` to verify protocol compiles

### Section 2: MCP Tool Schema
- [ ] Update `fugue_send_orchestration` schema with parent option
- [ ] Update tool description to document parent target

### Section 3: Handler Implementation
- [ ] Add parent parsing in `tool_send_orchestration`
- [ ] Implement parent resolution logic in session manager
- [ ] Add fallback to Tagged("orchestrator") if no parent found

### Section 4: Update Status Tools
- [ ] Change `tool_report_status` to use `OrchestrationTarget::Parent`
- [ ] Change `tool_request_help` to use `OrchestrationTarget::Parent`

### Section 5: Testing
- [ ] Test: session with `child:orch-claude` tag sends to `orch-claude`
- [ ] Test: session without child tag falls back to `orchestrator` tag
- [ ] Test: parent session not found falls back gracefully
- [ ] Test: `report_status` routes to parent orchestrator only

## Acceptance Criteria

- [ ] `{"target": {"parent": true}}` resolves sender's `child:<parent>` tag
- [ ] Parent name resolves to session UUID and routes correctly
- [ ] Missing child tag falls back to `Tagged("orchestrator")`
- [ ] Missing parent session falls back gracefully with warning
- [ ] `fugue_report_status` uses Parent target by default
- [ ] `fugue_request_help` uses Parent target by default
- [ ] Multi-orchestrator scenarios route to correct parent only

## Notes

### Lineage Tag Pattern

The `child:<parent>` tag pattern is documented in the user's CLAUDE.md:

```
**Lineage tags** (track parent-child relationships):
- Format: `child:<parent-session-name>`
- Example: worker spawned by `orch-claude` gets tag `child:orch-claude`
- Enables easy lookup of all children for a given orchestrator
```

This feature makes the lineage tag pattern functional for message routing, not just metadata.

### Alternative: Watchdog Uses send_input

For urgent notifications (like "worker stuck"), the watchdog could use `fugue_send_input` directly to inject messages into the orchestrator's terminal. This is complementary - `send_input` for interrupts, `send_orchestration` with Parent for structured messages.
