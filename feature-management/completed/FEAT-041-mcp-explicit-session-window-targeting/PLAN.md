# Implementation Plan: FEAT-041

**Work Item**: [FEAT-041: MCP Explicit Session and Window Targeting for fugue_create_pane](PROMPT.md)
**Component**: fugue-server (MCP)
**Priority**: P1
**Created**: 2026-01-10

## Overview

The `fugue_create_pane` MCP tool lacks explicit session and window targeting parameters. While the handler already supports these parameters, the MCP bridge hardcodes them to `None`. This enhancement exposes the existing capability via the MCP tool schema.

## Architecture Decisions

### Decision 1: Parameter Names

**Choice**: Use `session` and `window` as parameter names (not `session_id`/`window_id`).

**Rationale**:
- Matches user mental model - they think in terms of sessions/windows, not IDs
- Allows both UUID and name-based targeting
- Consistent with tmux-style interface
- More discoverable and intuitive

**Trade-offs**:
- Requires lookup logic to resolve name to ID
- Slightly more complex than direct ID parameters

### Decision 2: Filter Resolution Strategy

**Choice**: Support both UUID and name matching, try UUID first.

**Implementation**:
```rust
fn resolve_session_filter(session: Option<String>) -> Option<SessionFilter> {
    session.map(|s| {
        if let Ok(uuid) = Uuid::parse_str(&s) {
            SessionFilter::ById(uuid)
        } else {
            SessionFilter::ByName(s)
        }
    })
}
```

**Rationale**:
- UUIDs are unambiguous, should take priority
- Names are user-friendly for interactive use
- Single parameter handles both cases
- No need for separate `session_id` and `session_name` parameters

**Trade-offs**:
- Session names that look like UUIDs could be misinterpreted (very rare edge case)

### Decision 3: Response Enhancement

**Choice**: Always include `session_id` and `window_id` in response, even when using defaults.

**Rationale**:
- Claude needs to know which session was actually used
- Enables verification of targeting
- Useful for debugging (especially BUG-010)
- Small overhead for significant value

**Response Structure**:
```json
{
  "pane_id": "uuid",
  "session_id": "uuid",
  "window_id": "uuid",
  "dimensions": {"cols": 80, "rows": 24}
}
```

**Trade-offs**:
- Slightly larger response payload
- Requires handler to return session/window info

### Decision 4: Error Handling

**Choice**: Return clear error messages for invalid session/window targets.

**Implementation**:
```rust
if session_not_found {
    return Err(ToolError::NotFound(format!(
        "Session not found: {}", session_arg
    )));
}
```

**Rationale**:
- Clear errors help Claude understand and retry
- Better than silent fallback to wrong session
- Consistent with MCP tool error conventions

## Affected Components

| Component | Type of Change | Risk Level |
|-----------|----------------|------------|
| fugue-server/src/mcp/tools.rs | Add schema properties | Low |
| fugue-server/src/mcp/bridge.rs | Parse and pass arguments | Low |
| fugue-server/src/mcp/server.rs | Update ToolParams parsing | Low |

## Implementation Order

### Phase 1: Schema Update (tools.rs)

1. Locate `fugue_create_pane` tool definition
2. Add `session` property to input_schema
3. Add `window` property to input_schema
4. Update description to mention targeting capability
5. **Deliverable**: Tool schema includes new parameters

### Phase 2: Parsing Update (server.rs)

1. Locate `ToolParams::CreatePane` struct
2. Add `session: Option<String>` field
3. Add `window: Option<String>` field
4. Update deserialization to populate new fields
5. **Deliverable**: Arguments are parsed into struct

### Phase 3: Bridge Update (bridge.rs)

1. Locate `tool_create_pane()` function
2. Add filter resolution logic for session
3. Add filter resolution logic for window
4. Pass resolved filters to `handle_create_pane_with_options()`
5. **Deliverable**: Filters are passed to handler

### Phase 4: Response Enhancement

1. Update response construction to include `session_id`
2. Update response construction to include `window_id`
3. Ensure handler returns necessary context
4. **Deliverable**: Response includes session/window info

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Breaking existing callers | Low | Low | New params are optional, defaults preserved |
| Incorrect filter resolution | Low | Medium | Test both UUID and name matching |
| Handler doesn't return session info | Low | Low | Handler already has this context |
| Performance impact | Very Low | Very Low | Single lookup per call |

## Rollback Strategy

If implementation causes issues:
1. New parameters can be ignored by returning to hardcoded `None`
2. Response enhancement can be removed independently
3. No changes to handler internals required

## Testing Strategy

### Unit Tests

- `resolve_session_filter()` with UUID input
- `resolve_session_filter()` with name input
- `resolve_session_filter()` with None input
- `resolve_window_filter()` with UUID input
- `resolve_window_filter()` with name input

### Integration Tests

- Create pane with explicit session UUID
- Create pane with explicit session name
- Create pane with explicit window UUID
- Create pane with invalid session (error case)
- Create pane with no targeting (default behavior)
- Verify response includes session_id

### Manual Testing

- Call `fugue_create_pane` from Claude with session targeting
- Verify pane appears in correct session
- Verify response includes correct session_id

## Implementation Notes

### Schema Addition in tools.rs

```rust
Tool {
    name: "fugue_create_pane".into(),
    description: "Create a new pane in a session. If session/window not specified, uses active session.".into(),
    input_schema: serde_json::json!({
        "type": "object",
        "properties": {
            "session": {
                "type": "string",
                "description": "Target session (UUID or name). Uses active session if omitted."
            },
            "window": {
                "type": "string",
                "description": "Target window (UUID or name). Uses first window in session if omitted."
            },
            "command": {
                "type": "string",
                "description": "Command to execute in the new pane"
            },
            // ... existing properties
        }
    }),
}
```

### Filter Resolution in bridge.rs

```rust
fn resolve_session_filter(session: Option<String>) -> Option<SessionFilter> {
    session.map(|s| {
        match Uuid::parse_str(&s) {
            Ok(uuid) => SessionFilter::ById(uuid),
            Err(_) => SessionFilter::ByName(s),
        }
    })
}

fn resolve_window_filter(window: Option<String>) -> Option<WindowFilter> {
    window.map(|w| {
        match Uuid::parse_str(&w) {
            Ok(uuid) => WindowFilter::ById(uuid),
            Err(_) => WindowFilter::ByName(w),
        }
    })
}

pub async fn tool_create_pane(args: CreatePaneArgs) -> ToolResult {
    let session_filter = resolve_session_filter(args.session);
    let window_filter = resolve_window_filter(args.window);

    let result = handle_create_pane_with_options(
        session_filter,
        window_filter,
        args.command,
        // ... other args
    ).await?;

    Ok(json!({
        "pane_id": result.pane_id,
        "session_id": result.session_id,
        "window_id": result.window_id,
        "dimensions": result.dimensions
    }))
}
```

---
*This plan should be updated as implementation progresses.*
