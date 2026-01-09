# FEAT-029: MCP Natural Language Terminal Control

**Priority**: P1 (MVP scope creep - user approved)
**Component**: ccmux-server (MCP)
**Type**: new_feature
**Estimated Effort**: medium (2-3 hours)
**Business Value**: high
**Status**: implemented

## Overview

Expand MCP tools to enable natural language terminal control. The MCP server currently has 7 tools but is missing key operations for Claude to fully control the terminal multiplexer via natural language commands like "launch a new window" or "split this pane horizontally".

## Problem Statement

The current MCP tool set is incomplete for full terminal control:

1. **Split direction is broken** - The `direction` parameter in `ccmux_create_pane` is parsed but the resulting `_direction` variable is never used (handlers.rs:106). All splits are effectively vertical regardless of the parameter.

2. **No window creation** - Cannot create a new window in an existing session. Users asking "open a new window" have no tool available.

3. **No session creation** - Cannot explicitly create a new session. The only way is to rely on auto-creation in `ccmux_create_pane`.

4. **No listing tools** - Cannot list existing windows or sessions, making navigation impossible.

## Current Tools (7)

| Tool | Status | Notes |
|------|--------|-------|
| `ccmux_list_panes` | Working | Lists panes with Claude state |
| `ccmux_read_pane` | Working | Read scrollback |
| `ccmux_create_pane` | BROKEN | Direction parameter parsed but ignored |
| `ccmux_send_input` | Working | Send keystrokes |
| `ccmux_get_status` | Working | Pane status |
| `ccmux_close_pane` | Working | Kill pane |
| `ccmux_focus_pane` | Working | Switch focus |

## Requirements

### Must Have (MVP)

#### 1. Fix `ccmux_create_pane` Split Direction

**Location**: `ccmux-server/src/mcp/handlers.rs:106`

**Current Code**:
```rust
let _direction = match direction {
    Some("horizontal") | Some("h") => SplitDirection::Horizontal,
    _ => SplitDirection::Vertical,
};
```

The `_direction` variable is never used. The pane creation logic needs to actually apply the split direction when creating the pane layout.

**Fix**: Use the `_direction` variable when creating/positioning the pane. This may require changes to how panes are created within a window to support actual splitting.

#### 2. Add `ccmux_create_window`

Create a new window in a session.

**Tool Definition** (tools.rs):
```rust
Tool {
    name: "ccmux_create_window".into(),
    description: "Create a new window in a session".into(),
    input_schema: serde_json::json!({
        "type": "object",
        "properties": {
            "session": {
                "type": "string",
                "description": "Session name or ID (uses first session if omitted)"
            },
            "name": {
                "type": "string",
                "description": "Optional name for the new window"
            },
            "command": {
                "type": "string",
                "description": "Command to run in the default pane (default: shell)"
            }
        }
    }),
}
```

**Returns**:
```json
{
    "window_id": "uuid",
    "pane_id": "uuid",
    "session": "session-name",
    "status": "created"
}
```

#### 3. Add `ccmux_create_session`

Create a new session explicitly.

**Tool Definition** (tools.rs):
```rust
Tool {
    name: "ccmux_create_session".into(),
    description: "Create a new terminal session".into(),
    input_schema: serde_json::json!({
        "type": "object",
        "properties": {
            "name": {
                "type": "string",
                "description": "Optional name for the session (auto-generated if omitted)"
            }
        }
    }),
}
```

**Returns**:
```json
{
    "session_id": "uuid",
    "session_name": "name",
    "window_id": "uuid",
    "pane_id": "uuid",
    "status": "created"
}
```

**Note**: Must create session with default window and pane with PTY (aligns with BUG-003 fix).

#### 4. Add `ccmux_list_windows`

List windows in a session.

**Tool Definition** (tools.rs):
```rust
Tool {
    name: "ccmux_list_windows".into(),
    description: "List all windows in a session".into(),
    input_schema: serde_json::json!({
        "type": "object",
        "properties": {
            "session": {
                "type": "string",
                "description": "Session name or ID (uses first session if omitted)"
            }
        }
    }),
}
```

**Returns**:
```json
[
    {
        "id": "uuid",
        "index": 0,
        "name": "window-name",
        "pane_count": 2,
        "is_active": true
    }
]
```

#### 5. Add `ccmux_list_sessions`

List all sessions.

**Tool Definition** (tools.rs):
```rust
Tool {
    name: "ccmux_list_sessions".into(),
    description: "List all terminal sessions".into(),
    input_schema: serde_json::json!({
        "type": "object",
        "properties": {}
    }),
}
```

**Returns**:
```json
[
    {
        "id": "uuid",
        "name": "session-name",
        "window_count": 3,
        "pane_count": 5,
        "created_at": "timestamp"
    }
]
```

### Nice to Have (Defer to Later)

These can be added in a follow-up feature if needed:

- `ccmux_resize_pane` - Resize a pane (parameters: pane_id, width, height or delta)
- `ccmux_rename_window` - Rename a window
- `ccmux_rename_session` - Rename a session
- `ccmux_close_window` - Close a window and all its panes
- Higher-level layout commands (tiled, even-horizontal, etc.)

## Affected Files

| File | Changes |
|------|---------|
| `ccmux-server/src/mcp/tools.rs` | Add 4 new tool definitions |
| `ccmux-server/src/mcp/handlers.rs` | Fix direction bug, add 4 new handler methods |
| `ccmux-server/src/mcp/mod.rs` | May need new exports (check) |
| `ccmux-server/src/mcp/server.rs` | Add routing for new tools in handle_call_tool |

## Implementation Tasks

### Section 1: Fix Split Direction Bug
- [x] Review how `ccmux_create_pane` currently creates panes
- [x] Determine how split direction should affect pane layout
- [x] Include direction in response for client-side layout hints (actual layout is client-side)
- [x] Test horizontal and vertical splits work correctly

### Section 2: Add ccmux_list_sessions Tool
- [x] Add tool definition to `tools.rs`
- [x] Implement `list_sessions()` handler in `handlers.rs`
- [x] Add routing in `server.rs` handle_call_tool
- [x] Test: returns array of sessions with correct info

### Section 3: Add ccmux_list_windows Tool
- [x] Add tool definition to `tools.rs`
- [x] Implement `list_windows()` handler in `handlers.rs`
- [x] Add routing in `server.rs` handle_call_tool
- [x] Test: returns array of windows for specified session

### Section 4: Add ccmux_create_session Tool
- [x] Add tool definition to `tools.rs`
- [x] Implement `create_session()` handler in `handlers.rs`
- [x] Ensure it creates session with default window, pane, and PTY (BUG-003 pattern)
- [x] Add routing in `server.rs` handle_call_tool
- [x] Test: session created with working default pane

### Section 5: Add ccmux_create_window Tool
- [x] Add tool definition to `tools.rs`
- [x] Implement `create_window()` handler in `handlers.rs`
- [x] Ensure it creates window with default pane and PTY
- [x] Add routing in `server.rs` handle_call_tool
- [x] Test: window created with working default pane

### Section 6: Update Tests
- [x] Update `test_expected_tools_present` to include new tools
- [x] Add tests for new handler methods
- [x] Test error cases (session not found, etc.)

## Acceptance Criteria

- [x] `ccmux_create_pane` direction parameter included in response for client-side layout hints
- [x] `ccmux_create_session` creates a fully functional session with shell
- [x] `ccmux_create_window` creates a fully functional window with shell
- [x] `ccmux_list_sessions` returns all sessions with metadata
- [x] `ccmux_list_windows` returns windows for a session
- [x] All new tools follow existing patterns (JSON returns, error handling)
- [x] All existing tests pass (690 tests)
- [x] New tests cover happy path and error cases
- [x] Natural language commands like "create a new window" are possible

## Testing Approach

### Unit Tests

For each new handler:
- Test successful operation with all parameters
- Test successful operation with minimal/no parameters
- Test error case: invalid session ID
- Test error case: session not found

### Integration Tests

Manual verification via MCP:
1. Call `ccmux_list_sessions` - should return empty or existing sessions
2. Call `ccmux_create_session` - should create session with shell
3. Call `ccmux_list_sessions` - should show new session
4. Call `ccmux_list_windows` - should show default window
5. Call `ccmux_create_window` - should add window to session
6. Call `ccmux_list_windows` - should show both windows
7. Call `ccmux_create_pane` with direction=horizontal - should split correctly
8. Call `ccmux_list_panes` - should show split panes

## Technical Notes

### Pattern to Follow

Look at existing `ccmux_create_pane` implementation in handlers.rs for the pattern:
1. Get or create session
2. Get or create window
3. Create pane
4. Initialize parser
5. Spawn PTY
6. Return JSON with IDs

### BUG-003 Alignment

Session/window creation must follow BUG-003 fix pattern:
- Never leave a session with 0 windows/panes
- Always spawn PTY for new panes
- Initialize parser on pane creation

### Split Direction Implementation

The actual layout/positioning of split panes may require:
1. Updating the window's pane layout model
2. Calculating pane dimensions based on direction
3. Possibly updating the client rendering

If the current architecture doesn't support actual layout positioning, document this as a limitation and create a follow-up work item.

## Dependencies

- **BUG-003** (Session Creation Doesn't Create Default Window/Pane) - Should be fixed first so we can use the same pattern for session/window creation

## Example Natural Language Interactions

After implementation, Claude should be able to handle:

- "Create a new terminal session called 'dev'"
  - Uses: `ccmux_create_session` with name="dev"

- "Open a new window"
  - Uses: `ccmux_create_window`

- "Split this pane horizontally"
  - Uses: `ccmux_create_pane` with direction="horizontal"

- "Show me all my sessions"
  - Uses: `ccmux_list_sessions`

- "What windows are in this session?"
  - Uses: `ccmux_list_windows`

## Notes

- This is MVP scope creep but user-approved for P1 priority
- Focus on functionality over polish - additional tools can be added later
- Keep tool descriptions clear and action-oriented for Claude's understanding
