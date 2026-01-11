# FEAT-043: MCP Session Rename Tool

**Priority**: P2
**Component**: ccmux-server (MCP)
**Type**: enhancement
**Estimated Effort**: small
**Business Value**: medium
**Status**: new

## Overview

Add the ability to rename sessions via MCP. Currently sessions get auto-generated names like `session-c21daf06519f410aaeadd1bb0ed9705f` which are not human-friendly. Users should be able to rename sessions to meaningful names like "Orchestrator", "Worker-1", etc.

## Problem Statement

When ccmux creates sessions, they receive auto-generated names based on UUIDs. These names are:
- Hard to remember and distinguish
- Not meaningful in multi-session workflows
- Difficult to reference in MCP commands
- Cluttered in `ccmux_list_sessions` output

### Current Behavior

Sessions are named with UUID-based identifiers:
```
session-c21daf06519f410aaeadd1bb0ed9705f
session-a8b2c3d4e5f6789012345678abcdef01
```

### Desired Behavior

Users can rename sessions to meaningful names:
```
Orchestrator
Worker-1
Worker-2
dev-backend
```

## Requirements

### 1. New MCP Tool: `ccmux_rename_session`

Add a new tool to rename sessions:

```rust
Tool {
    name: "ccmux_rename_session".into(),
    description: "Rename a session for easier identification".into(),
    input_schema: serde_json::json!({
        "type": "object",
        "properties": {
            "session": {
                "type": "string",
                "description": "Session to rename (UUID or current name)"
            },
            "name": {
                "type": "string",
                "description": "New display name for the session"
            }
        },
        "required": ["session", "name"]
    }),
}
```

### 2. Uniqueness Constraint

Session names must be unique:
- Reject rename if target name is already in use by another session
- Return clear error message: "Session name 'X' is already in use"
- Allow renaming to same name (no-op, or refresh updated_date)

### 3. Updated List Output

`ccmux_list_sessions` should show the user-assigned name:

```json
{
  "sessions": [
    {
      "id": "c21daf06-519f-410a-aeadd-1bb0ed9705f",
      "name": "Orchestrator",
      "window_count": 1,
      "pane_count": 3,
      "attached_clients": 1
    },
    {
      "id": "a8b2c3d4-e5f6-7890-1234-5678abcdef01",
      "name": "Worker-1",
      "window_count": 1,
      "pane_count": 1,
      "attached_clients": 0
    }
  ]
}
```

### 4. Persistence

Session names must persist across server restarts:
- Update the session's persistence state when renamed
- Load name from persistence on session restore
- Ensure WAL captures rename operations

## Use Cases

### Multi-Agent Orchestration

Name sessions by their role in the agent system:
- "Orchestrator" - Main coordinating agent
- "Worker-1", "Worker-2", "Worker-3" - Worker agents
- "Evaluator" - Quality assurance agent
- "Monitor" - Logging/monitoring agent

### Project Organization

Name sessions by project or task:
- "frontend-dev" - Frontend development session
- "backend-api" - Backend API work
- "db-migrations" - Database migration work
- "testing" - Test execution session

### Easy Identification

Instead of:
```
ccmux_create_pane --session session-c21daf06519f410a...
```

Users can:
```
ccmux_create_pane --session Orchestrator
```

## Files Affected

| File | Changes |
|------|---------|
| `ccmux-server/src/mcp/tools.rs` | Add `ccmux_rename_session` tool definition |
| `ccmux-server/src/mcp/bridge.rs` | Implement rename handler function |
| `ccmux-server/src/mcp/server.rs` | Add routing for new tool |
| `ccmux-server/src/session/mod.rs` | Add `rename_session()` method to SessionManager |
| `ccmux-server/src/session/session.rs` | Add `rename()` or `set_name()` method to Session |
| `ccmux-server/src/persistence/` | Ensure name changes are persisted |

## Implementation Tasks

### Section 1: Session Manager API
- [ ] Add `rename_session(session_id: SessionId, new_name: String) -> Result<(), Error>` to SessionManager
- [ ] Implement uniqueness check in rename_session
- [ ] Return appropriate error for duplicate names
- [ ] Update session's internal name field

### Section 2: MCP Tool Definition
- [ ] Add `ccmux_rename_session` to tools.rs with schema
- [ ] Define input parameters: `session` (UUID or name), `name` (new name)
- [ ] Mark both parameters as required

### Section 3: MCP Handler
- [ ] Add `tool_rename_session()` handler in bridge.rs
- [ ] Resolve session by UUID or name
- [ ] Call SessionManager::rename_session()
- [ ] Return success response with updated session info

### Section 4: Persistence
- [ ] Verify session name is included in checkpoint state
- [ ] Ensure WAL captures rename operations
- [ ] Test that renamed sessions restore correctly after restart

### Section 5: Testing
- [ ] Test rename with session UUID
- [ ] Test rename with session name
- [ ] Test rename to duplicate name (should fail)
- [ ] Test rename to same name (should succeed as no-op)
- [ ] Test list_sessions shows new name
- [ ] Test session targeting works with new name
- [ ] Test persistence across restart

### Section 6: Documentation
- [ ] Update tool description to be clear and helpful
- [ ] Add code comments explaining uniqueness logic
- [ ] Update CHANGELOG

## Acceptance Criteria

- [ ] New `ccmux_rename_session` tool available in MCP
- [ ] Session can be renamed by UUID
- [ ] Session can be renamed by current name
- [ ] Duplicate names are rejected with clear error
- [ ] `ccmux_list_sessions` shows updated name
- [ ] Other MCP tools can target session by new name
- [ ] Session name persists across server restart
- [ ] All existing tests pass

## Example Usage

### Rename a Session

**Request**:
```json
{
  "tool": "ccmux_rename_session",
  "arguments": {
    "session": "session-c21daf06519f410aaeadd1bb0ed9705f",
    "name": "Orchestrator"
  }
}
```

**Response**:
```json
{
  "success": true,
  "session_id": "c21daf06-519f-410a-aeadd-1bb0ed9705f",
  "previous_name": "session-c21daf06519f410aaeadd1bb0ed9705f",
  "new_name": "Orchestrator"
}
```

### Rename Using Current Name

**Request**:
```json
{
  "tool": "ccmux_rename_session",
  "arguments": {
    "session": "Orchestrator",
    "name": "Main-Orchestrator"
  }
}
```

### Duplicate Name Error

**Request**:
```json
{
  "tool": "ccmux_rename_session",
  "arguments": {
    "session": "Worker-2",
    "name": "Orchestrator"
  }
}
```

**Response**:
```json
{
  "error": true,
  "message": "Session name 'Orchestrator' is already in use"
}
```

## Related Work Items

- **FEAT-036**: Session-aware MCP Commands with Window/Pane Naming - covers window/pane naming, this extends to sessions
- **FEAT-041**: MCP Explicit Session and Window Targeting - can use renamed sessions as targets

## Dependencies

None - this is a standalone enhancement to existing session management.

## Notes

- Consider adding `name` parameter to session creation for initial naming
- Session names could potentially include validation (no special chars, length limit)
- Future enhancement: auto-naming based on first command or working directory
