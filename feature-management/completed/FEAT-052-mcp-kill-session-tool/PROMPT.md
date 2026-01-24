# FEAT-052: Add fugue_kill_session MCP tool

**Priority**: P1
**Component**: fugue-server (MCP)
**Type**: enhancement
**Estimated Effort**: small
**Business Value**: high
**Status**: new

## Overview

Expose the existing `DestroySession` protocol message as an MCP tool so agents can terminate sessions programmatically. This enables orchestration patterns like Gas Town's worker management, which currently uses `tmux kill-session -t <name>`.

## Problem Statement

The fugue daemon already has full session lifecycle support including `ClientMessage::DestroySession { session_id: Uuid }` (defined in `fugue-protocol/src/messages.rs:149-150`), but this capability is not exposed through the MCP interface.

### Current State

- Sessions can be created via `fugue_create_session` MCP tool
- Sessions can be listed via `fugue_list_panes` MCP tool
- Sessions **cannot** be killed/destroyed via MCP
- Orchestration workflows requiring session termination must use alternative methods

### Use Case: Gas Town Integration

Gas Town (and similar orchestration patterns) needs to:
1. Spawn worker sessions programmatically
2. Assign tasks to workers
3. **Terminate workers when complete or errored**

Step 3 is blocked by the lack of an MCP kill session tool.

## Requirements

### New MCP Tool: `fugue_kill_session`

**Tool Definition**:
```rust
Tool {
    name: "fugue_kill_session".into(),
    description: "Kill/destroy a fugue session and all its windows and panes".into(),
    input_schema: serde_json::json!({
        "type": "object",
        "properties": {
            "session": {
                "type": "string",
                "description": "Session UUID or name to kill"
            }
        },
        "required": ["session"]
    }),
}
```

### Handler Implementation

The handler should:
1. Parse the `session` parameter (string)
2. Resolve session by UUID or name (reuse existing resolution logic)
3. Send `ClientMessage::DestroySession { session_id }` to the daemon
4. Return success with session info, or error if session not found

### Response Format

**Success**:
```json
{
  "success": true,
  "message": "Session killed",
  "session_id": "uuid-here",
  "session_name": "session-name"
}
```

**Error (session not found)**:
```json
{
  "error": "Session not found: <session-param>"
}
```

## Implementation

### Files to Modify

| File | Changes |
|------|---------|
| `fugue-server/src/mcp/tools.rs` | Add `fugue_kill_session` tool definition |
| `fugue-server/src/mcp/handlers.rs` | Add handler that resolves session and sends DestroySession |

### Existing Code to Reference

**Session Resolution** (already implemented for other tools):
- `fugue_create_pane` resolves sessions by UUID or name
- Pattern: check if string is valid UUID, else search by name

**DestroySession Message** (already defined):
```rust
// fugue-protocol/src/messages.rs:149-150
ClientMessage::DestroySession { session_id: Uuid }
```

## Implementation Tasks

### Section 1: Tool Definition
- [ ] Add `fugue_kill_session` tool to `get_tool_definitions()` in tools.rs
- [ ] Use schema with required `session` string parameter

### Section 2: Handler Implementation
- [ ] Add `handle_kill_session` function in handlers.rs
- [ ] Parse `session` parameter from tool arguments
- [ ] Resolve session by UUID or name (reuse existing helper)
- [ ] Send `ClientMessage::DestroySession` to daemon
- [ ] Return success response with session details
- [ ] Handle session-not-found error

### Section 3: Testing
- [ ] Test killing session by UUID
- [ ] Test killing session by name
- [ ] Test error response for non-existent session
- [ ] Test that session is actually removed after kill

### Section 4: Documentation
- [ ] Add clear tool description
- [ ] Update CHANGELOG

## Acceptance Criteria

- [ ] `fugue_kill_session` tool is registered and visible to MCP clients
- [ ] Can kill session by UUID
- [ ] Can kill session by name
- [ ] Appropriate error returned for non-existent session
- [ ] Session and all its windows/panes are destroyed
- [ ] All existing tests pass

## Example Usage

**Kill by name**:
```json
{
  "tool": "fugue_kill_session",
  "arguments": {
    "session": "worker-1"
  }
}
```

**Kill by UUID**:
```json
{
  "tool": "fugue_kill_session",
  "arguments": {
    "session": "550e8400-e29b-41d4-a716-446655440000"
  }
}
```

## Dependencies

None - the underlying `DestroySession` capability already exists in the protocol and daemon.

## Notes

- This is a thin MCP wrapper around existing functionality
- The daemon already handles all cleanup (PTY termination, state removal, etc.)
- Consider whether to add a `force` parameter for edge cases (not required initially)
- Future enhancement: could add `fugue_kill_pane` and `fugue_kill_window` tools
