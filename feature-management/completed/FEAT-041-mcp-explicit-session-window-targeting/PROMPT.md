# FEAT-041: MCP Explicit Session and Window Targeting for fugue_create_pane

**Priority**: P1
**Component**: fugue-server (MCP)
**Type**: enhancement
**Estimated Effort**: small
**Business Value**: high
**Status**: new

## Overview

The `fugue_create_pane` MCP tool lacks explicit session and window targeting parameters. While the handler (`handle_create_pane_with_options`) already supports `session_filter` and `window_filter` parameters, the MCP bridge hardcodes these to `None`, always using the "first session" heuristic. This prevents Claude from orchestrating panes across multiple sessions.

## Problem Statement

When MCP creates panes via `fugue_create_pane`, it cannot explicitly target a specific session or window. The current implementation in the MCP bridge always passes `None` for session and window filters, causing the handler to fall back to the "first session" heuristic.

### Current Behavior

- `fugue_create_pane` has no `session` or `window` parameters in its schema
- The bridge function `tool_create_pane()` hardcodes `session_filter: None` and `window_filter: None`
- Handler uses first available session when no filter is provided
- MCP response does not include `session_id`, so Claude cannot verify which session was used

### Impact

- Cannot create panes in specific sessions
- Cannot orchestrate multi-session workflows (e.g., "create background session, spawn 4 workers")
- May create panes in wrong session when multiple sessions exist
- Related to BUG-010 debugging - explicit targeting could help isolate broadcast issues

## Requirements

### New Parameters for fugue_create_pane

Add optional `session` and `window` parameters to the tool schema:

```json
{
  "session": {
    "type": "string",
    "description": "Target session (UUID or name). Uses active session if omitted."
  },
  "window": {
    "type": "string",
    "description": "Target window (UUID or name). Uses first window in session if omitted."
  }
}
```

### Enhanced Response

The response should include `session_id` so Claude knows which session was used:

```json
{
  "pane_id": "uuid",
  "session_id": "uuid",
  "window_id": "uuid",
  "dimensions": {"cols": 80, "rows": 24}
}
```

## Files Affected

| File | Changes |
|------|---------|
| `fugue-server/src/mcp/tools.rs` | Add `session` and `window` properties to `fugue_create_pane` schema |
| `fugue-server/src/mcp/bridge.rs` | Update `tool_create_pane()` to accept and pass session/window arguments |
| `fugue-server/src/mcp/server.rs` | Update `ToolParams::CreatePane` parsing to include new fields |

## Implementation Tasks

### Section 1: Schema Update
- [ ] Add `session` property to `fugue_create_pane` input_schema
- [ ] Add `window` property to `fugue_create_pane` input_schema
- [ ] Update tool description to mention session/window targeting

### Section 2: Bridge Update
- [ ] Update `tool_create_pane()` function signature to accept session/window
- [ ] Parse session argument and convert to `SessionFilter` (by UUID or name)
- [ ] Parse window argument and convert to `WindowFilter` (by UUID or name)
- [ ] Pass session_filter and window_filter to `handle_create_pane_with_options()`

### Section 3: Response Enhancement
- [ ] Update response to include `session_id`
- [ ] Update response to include `window_id`
- [ ] Ensure response includes the actually-used session (not just if specified)

### Section 4: Testing
- [ ] Test create pane with explicit session UUID
- [ ] Test create pane with explicit session name
- [ ] Test create pane with explicit window UUID
- [ ] Test create pane with explicit window name
- [ ] Test create pane with no session/window (existing behavior preserved)
- [ ] Test response includes session_id
- [ ] Test invalid session/window returns appropriate error

### Section 5: Documentation
- [ ] Update tool description in tools.rs
- [ ] Add code comments explaining filter resolution

## Acceptance Criteria

- [ ] `fugue_create_pane` accepts optional `session` parameter (UUID or name)
- [ ] `fugue_create_pane` accepts optional `window` parameter (UUID or name)
- [ ] When session specified, pane is created in the target session
- [ ] When window specified, pane is created in the target window
- [ ] When not specified, existing behavior preserved (use first/active session)
- [ ] MCP response includes `session_id` so Claude knows which session was used
- [ ] Invalid session/window returns clear error message
- [ ] All existing tests pass

## Example Usage

### Create Pane in Specific Session

```json
{
  "tool": "fugue_create_pane",
  "arguments": {
    "session": "dev-session",
    "command": "npm run dev"
  }
}
```

### Create Pane in Specific Window

```json
{
  "tool": "fugue_create_pane",
  "arguments": {
    "session": "550e8400-e29b-41d4-a716-446655440000",
    "window": "main-window",
    "command": "tail -f logs/app.log"
  }
}
```

### Response with Session Context

```json
{
  "pane_id": "123e4567-e89b-12d3-a456-426614174000",
  "session_id": "550e8400-e29b-41d4-a716-446655440000",
  "window_id": "789e0123-e89b-12d3-a456-426614174000",
  "dimensions": {
    "cols": 120,
    "rows": 40
  }
}
```

## Dependencies

None - this enhances existing functionality.

## Related Work Items

- **BUG-010**: MCP Pane Broadcast Not Received - explicit targeting may help debug this issue
- **FEAT-036**: Session-aware MCP Commands - related session targeting improvements
- **FEAT-039**: MCP Pane Creation Broadcast - the broadcast being investigated

## Notes

- The handler already supports `session_filter` and `window_filter` - this is purely about exposing them via MCP
- Session/window can be matched by UUID or name, consistent with other MCP tools
- This is a low-risk enhancement since the underlying handler logic already exists
- Consider adding similar targeting to other MCP tools (create_window, list_panes, etc.) as follow-up
