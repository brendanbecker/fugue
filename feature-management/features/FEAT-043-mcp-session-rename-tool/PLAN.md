# Implementation Plan: FEAT-043

**Work Item**: [FEAT-043: MCP Session Rename Tool](PROMPT.md)
**Component**: ccmux-server (MCP)
**Priority**: P2
**Created**: 2026-01-10

## Overview

Add the ability to rename sessions via MCP to replace auto-generated UUID-based names with human-friendly names like "Orchestrator" or "Worker-1".

## Architecture Decisions

### Decision 1: Name Storage Location

**Choice**: Store the name directly on the Session struct.

**Rationale**:
- Sessions already have a `name` field (likely initialized to UUID-based name)
- No additional data structures needed
- Natural place for the information
- Aligns with how windows/panes handle naming

**Trade-offs**:
- Must ensure persistence layer captures name field
- Name changes need to propagate to any cached session info

### Decision 2: Session Resolution

**Choice**: Accept both UUID and current name for the `session` parameter.

**Implementation**:
```rust
fn resolve_session(session: &str) -> Result<SessionId, Error> {
    // Try UUID first
    if let Ok(uuid) = Uuid::parse_str(session) {
        return session_manager.get_session_by_id(uuid);
    }
    // Fall back to name lookup
    session_manager.get_session_by_name(session)
}
```

**Rationale**:
- Consistent with existing MCP tool patterns (FEAT-041)
- Flexible for callers
- UUIDs are unambiguous, names are user-friendly

**Trade-offs**:
- Need to ensure name lookup is case-sensitive or define case handling

### Decision 3: Uniqueness Enforcement

**Choice**: Enforce strict uniqueness at the SessionManager level.

**Implementation**:
```rust
pub fn rename_session(&mut self, session_id: SessionId, new_name: String) -> Result<(), Error> {
    // Check for duplicates (excluding the session being renamed)
    if self.sessions.values()
        .any(|s| s.id != session_id && s.name == new_name)
    {
        return Err(Error::DuplicateName(new_name));
    }

    // Perform rename
    if let Some(session) = self.sessions.get_mut(&session_id) {
        session.name = new_name;
        Ok(())
    } else {
        Err(Error::SessionNotFound(session_id))
    }
}
```

**Rationale**:
- Prevents confusion when targeting sessions by name
- Clear error messages help users understand the issue
- Centralized enforcement in SessionManager

**Trade-offs**:
- Names that "look like UUIDs" could theoretically conflict (unlikely edge case)

### Decision 4: Response Format

**Choice**: Return both previous and new name in response.

**Implementation**:
```json
{
  "success": true,
  "session_id": "uuid",
  "previous_name": "old-name",
  "new_name": "new-name"
}
```

**Rationale**:
- Confirms the operation completed
- Shows what changed for verification
- Useful for logging/audit purposes

**Trade-offs**:
- Slightly larger response payload

## Affected Components

| Component | Type of Change | Risk Level |
|-----------|----------------|------------|
| ccmux-server/src/mcp/tools.rs | Add tool definition | Low |
| ccmux-server/src/mcp/bridge.rs | Add handler function | Low |
| ccmux-server/src/mcp/server.rs | Add routing | Low |
| ccmux-server/src/session/mod.rs | Add rename_session method | Low |
| ccmux-server/src/persistence/ | Verify name persistence | Low |

## Implementation Order

### Phase 1: Session Manager API

1. Locate Session struct and verify `name` field exists
2. Add `rename_session()` method to SessionManager
3. Implement uniqueness check
4. Handle session not found error
5. **Deliverable**: SessionManager can rename sessions programmatically

### Phase 2: MCP Tool Definition

1. Add `ccmux_rename_session` to tools.rs
2. Define input schema with `session` and `name` parameters
3. Mark both as required
4. Write clear description
5. **Deliverable**: Tool appears in MCP tool list

### Phase 3: MCP Handler

1. Add `tool_rename_session()` function in bridge.rs
2. Parse session argument (UUID or name)
3. Parse name argument
4. Call SessionManager::rename_session()
5. Return success or error response
6. **Deliverable**: Tool can be invoked and processes requests

### Phase 4: Routing Integration

1. Add case for `ccmux_rename_session` in server.rs
2. Route to handler function
3. **Deliverable**: Full end-to-end functionality

### Phase 5: Persistence Verification

1. Verify session name is included in checkpoint serialization
2. Verify WAL captures rename operations
3. Test restart scenario
4. **Deliverable**: Names persist correctly

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Breaking session lookup | Low | Medium | Test both UUID and name lookup after rename |
| Persistence not capturing name | Low | Medium | Verify serialization includes name field |
| Race condition in uniqueness check | Very Low | Low | SessionManager should be single-threaded |
| Performance impact | Very Low | Very Low | Simple string comparison |

## Rollback Strategy

If implementation causes issues:
1. Remove tool definition from tools.rs
2. Remove handler from bridge.rs
3. Remove routing from server.rs
4. SessionManager changes can remain (unused but harmless)
5. No data migration needed - names revert to auto-generated

## Testing Strategy

### Unit Tests

- `rename_session()` with valid session ID and new name
- `rename_session()` with invalid session ID (error)
- `rename_session()` with duplicate name (error)
- `rename_session()` to same name (no-op success)
- Session name lookup after rename

### Integration Tests

- MCP call with session UUID
- MCP call with session current name
- MCP call with invalid session (error)
- MCP call with duplicate name (error)
- `list_sessions` shows new name
- Other tools can target by new name
- Persistence across restart

### Manual Testing

- Rename session via MCP
- Verify `ccmux_list_sessions` shows new name
- Create pane in renamed session using new name
- Restart server and verify name persists

## Implementation Notes

### Tool Definition in tools.rs

```rust
Tool {
    name: "ccmux_rename_session".into(),
    description: "Rename a session for easier identification. Session names must be unique.".into(),
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

### Handler in bridge.rs

```rust
pub async fn tool_rename_session(
    session_manager: &mut SessionManager,
    session: String,
    name: String,
) -> ToolResult {
    // Resolve session by UUID or name
    let session_id = resolve_session(&session, session_manager)?;

    // Get previous name for response
    let previous_name = session_manager
        .get_session(session_id)
        .map(|s| s.name.clone())
        .ok_or_else(|| ToolError::NotFound("Session not found".into()))?;

    // Perform rename
    session_manager.rename_session(session_id, name.clone())?;

    // Trigger persistence (WAL entry)
    session_manager.mark_dirty(session_id);

    Ok(json!({
        "success": true,
        "session_id": session_id.to_string(),
        "previous_name": previous_name,
        "new_name": name
    }))
}
```

### SessionManager Method

```rust
impl SessionManager {
    pub fn rename_session(&mut self, session_id: SessionId, new_name: String) -> Result<(), Error> {
        // Check uniqueness
        for (id, session) in &self.sessions {
            if *id != session_id && session.name == new_name {
                return Err(Error::DuplicateSessionName(new_name));
            }
        }

        // Perform rename
        match self.sessions.get_mut(&session_id) {
            Some(session) => {
                session.name = new_name;
                Ok(())
            }
            None => Err(Error::SessionNotFound(session_id)),
        }
    }
}
```

---
*This plan should be updated as implementation progresses.*
