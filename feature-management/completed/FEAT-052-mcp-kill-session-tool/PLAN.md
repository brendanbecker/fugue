# Implementation Plan: FEAT-052

**Work Item**: [FEAT-052: Add fugue_kill_session MCP tool](PROMPT.md)
**Component**: fugue-server (MCP)
**Priority**: P1
**Created**: 2026-01-10

## Overview

Expose the existing `DestroySession` protocol message as an MCP tool so agents can terminate sessions programmatically. This is a straightforward wrapper around existing daemon functionality.

## Architecture Decisions

### Decision 1: Session Resolution

**Choice**: Reuse existing session resolution pattern from other MCP tools.

**Rationale**:
- `fugue_create_pane` and other tools already resolve sessions by UUID or name
- Consistent behavior across all session-targeting tools
- No new code paths needed

**Implementation**:
- Check if input is valid UUID format
- If yes, look up by UUID
- If no, search sessions by name

### Decision 2: Response Format

**Choice**: Return success/error JSON matching existing MCP tool patterns.

**Rationale**:
- Consistent with other fugue MCP tools
- Provides session context for verification
- Clear error messages for debugging

**Response Structure**:
```json
{
  "success": true,
  "message": "Session killed",
  "session_id": "uuid",
  "session_name": "name"
}
```

### Decision 3: Error Handling

**Choice**: Return MCP error for session not found, let daemon handle all other errors.

**Rationale**:
- Session lookup is the only MCP-layer validation needed
- Daemon already handles cleanup and edge cases
- Keeps MCP layer thin

**Error Cases**:
- Session not found -> MCP error response
- Daemon errors -> Propagate as MCP error

## Affected Components

| Component | Type of Change | Risk Level |
|-----------|----------------|------------|
| fugue-server/src/mcp/tools.rs | Add tool definition | Low |
| fugue-server/src/mcp/handlers.rs | Add handler function | Low |

## Implementation Order

### Phase 1: Tool Definition
1. Add tool schema to `get_tool_definitions()` in tools.rs
2. **Deliverable**: Tool visible in MCP tool list

### Phase 2: Handler Implementation
1. Add handler function in handlers.rs
2. Implement session resolution
3. Send DestroySession message to daemon
4. Format response
5. **Deliverable**: Functional kill session capability

### Phase 3: Testing
1. Manual testing with MCP client
2. Unit tests for handler
3. **Deliverable**: Verified functionality

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Breaking existing session resolution | Low | Medium | Reuse existing code paths |
| Daemon DestroySession not working | Low | High | Capability is already used by client |
| Unintended session termination | Low | Medium | Require explicit session parameter |

## Rollback Strategy

If implementation causes issues:
1. Remove tool from `get_tool_definitions()` - tool becomes invisible
2. Handler code can remain (unused)
3. No protocol or daemon changes needed

## Testing Strategy

### Unit Tests
- Handler with valid UUID
- Handler with valid name
- Handler with invalid session (error path)

### Integration Tests
- Create session, kill by UUID, verify removed
- Create session, kill by name, verify removed

### Manual Testing
- Use MCP client to list sessions, kill one, verify removal

## Implementation Notes

### Handler Pattern

Follow existing handler pattern in handlers.rs:

```rust
pub async fn handle_kill_session(
    &self,
    args: serde_json::Value,
) -> Result<serde_json::Value, McpError> {
    let session_param = args.get("session")
        .and_then(|v| v.as_str())
        .ok_or_else(|| McpError::InvalidParams("session parameter required".into()))?;

    // Resolve session by UUID or name
    let session_id = self.resolve_session(session_param)?;
    let session_name = self.get_session_name(session_id)?;

    // Send DestroySession to daemon
    self.send_to_daemon(ClientMessage::DestroySession { session_id }).await?;

    Ok(serde_json::json!({
        "success": true,
        "message": "Session killed",
        "session_id": session_id.to_string(),
        "session_name": session_name
    }))
}
```

### Tool Definition

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

---
*This plan should be updated as implementation progresses.*
