# FEAT-046: MCP Focus/Select Control

**Priority**: P1
**Component**: ccmux-server (MCP)
**Type**: enhancement
**Estimated Effort**: small
**Business Value**: high
**Technical Complexity**: low
**Status**: new

## Overview

Add MCP tools for explicit focus/selection control at all three levels (session, window, pane), and change the default behavior of `ccmux_create_pane` to NOT auto-switch focus.

## Problem Statement

Currently, when `ccmux_create_pane` is called via MCP:
1. A new pane is created
2. Focus automatically switches to the new pane in the TUI

This is problematic for orchestration workflows where an LLM wants to:
- Create multiple panes without losing its place
- Spawn worker panes while continuing to monitor the original
- Set up a layout and then explicitly choose which pane to focus

Additionally, there's no MCP tool to:
- Switch focus to a specific pane
- Switch to a different window (tab)
- Switch to a different session

## Requirements

### Part 1: Change `ccmux_create_pane` Default Behavior

**Current behavior**: Auto-switches focus to new pane
**New behavior**: Keep focus on current pane (default), with opt-in to switch

#### Schema Update

Add new optional parameter:
```json
{
    "select": {
        "type": "boolean",
        "default": false,
        "description": "If true, switch focus to the new pane after creation. Default: false (keep focus on current pane)."
    }
}
```

### Part 2: `ccmux_select_pane` Tool

Switch focus to a specific pane.

#### Schema

```json
{
    "name": "ccmux_select_pane",
    "description": "Switch focus to a specific pane",
    "input_schema": {
        "type": "object",
        "properties": {
            "pane_id": {
                "type": "string",
                "description": "UUID of the pane to focus"
            }
        },
        "required": ["pane_id"]
    }
}
```

#### Response

```json
{
    "status": "focused",
    "pane_id": "uuid",
    "session_id": "uuid",
    "window_id": "uuid"
}
```

### Part 3: `ccmux_select_window` Tool

Switch to a specific window (tab) within the current or specified session.

#### Schema

```json
{
    "name": "ccmux_select_window",
    "description": "Switch to a specific window (tab)",
    "input_schema": {
        "type": "object",
        "properties": {
            "window": {
                "type": "string",
                "description": "Window UUID, name, or index (0-9)"
            },
            "session": {
                "type": "string",
                "description": "Optional session UUID or name. Uses active session if omitted."
            }
        },
        "required": ["window"]
    }
}
```

#### Response

```json
{
    "status": "focused",
    "window_id": "uuid",
    "window_name": "name",
    "window_index": 0,
    "session_id": "uuid"
}
```

### Part 4: `ccmux_select_session` Tool

Switch to a different session entirely.

#### Schema

```json
{
    "name": "ccmux_select_session",
    "description": "Switch to a different session",
    "input_schema": {
        "type": "object",
        "properties": {
            "session": {
                "type": "string",
                "description": "Session UUID or name"
            }
        },
        "required": ["session"]
    }
}
```

#### Response

```json
{
    "status": "focused",
    "session_id": "uuid",
    "session_name": "name",
    "window_id": "uuid",
    "pane_id": "uuid"
}
```

### Part 5: Update `ccmux_list_panes` Response

Add a `focused` field to indicate which pane currently has focus.

#### Updated Response Item

```json
{
    "id": "uuid",
    "session": "session-uuid",
    "window": 0,
    "window_name": "0",
    "index": 0,
    "rows": 46,
    "cols": 186,
    "is_claude": true,
    "claude_state": {...},
    "focused": true  // NEW: indicates this pane has focus
}
```

## Use Cases

### 1. Create Worker Panes Without Losing Focus

```
LLM: ccmux_create_pane(direction="vertical", cwd="/project/worktree-1")
     ccmux_create_pane(direction="vertical", cwd="/project/worktree-2")
     ccmux_create_pane(direction="vertical", cwd="/project/worktree-3")
     // Focus stays on original pane throughout
```

### 2. Orchestrator Spawns Workers Then Monitors

```
LLM: ccmux_create_pane(direction="vertical", command="claude 'work on task 1'")
     ccmux_create_pane(direction="vertical", command="claude 'work on task 2'")
     // Still focused on orchestrator pane
     // Can read worker outputs without switching
```

### 3. Explicit Navigation

```
LLM: ccmux_list_panes()  // Find pane IDs
     ccmux_select_pane(pane_id="abc-123")  // Switch to specific pane
```

### 4. Session Switching

```
LLM: ccmux_list_sessions()  // See all sessions
     ccmux_select_session(session="development")  // Switch sessions
```

### 5. Window/Tab Navigation

```
LLM: ccmux_select_window(window="1")  // Switch to second tab
     ccmux_select_window(window="tests")  // Switch by name
```

## Implementation Approach

### Server-Side Changes

1. **Track active pane per client** - The server needs to know which pane is focused for each connected TUI client

2. **Add focus control message type** - New protocol message to tell TUI to change focus

3. **Update `ccmux_create_pane` handler** - Don't send focus message unless `select: true`

4. **Add new tool handlers** - Implement `ccmux_select_pane`, `ccmux_select_window`, `ccmux_select_session`

### Protocol Changes

Add new message type:
```rust
pub enum ServerMessage {
    // ... existing variants ...
    FocusPane { pane_id: Uuid },
    FocusWindow { window_id: Uuid },
    FocusSession { session_id: Uuid },
}
```

### Client-Side Changes

Handle focus messages:
```rust
ServerMessage::FocusPane { pane_id } => {
    self.focus_pane(pane_id);
}
ServerMessage::FocusWindow { window_id } => {
    self.focus_window(window_id);
}
ServerMessage::FocusSession { session_id } => {
    self.switch_session(session_id);
}
```

## Files Affected

| File | Changes |
|------|---------|
| `ccmux-server/src/mcp/tools.rs` | Add tool definitions |
| `ccmux-server/src/mcp/handlers.rs` | Implement handlers |
| `ccmux-server/src/handlers/mcp_bridge.rs` | Route focus commands, update create_pane |
| `ccmux-protocol/src/messages.rs` | Add focus message types |
| `ccmux-client/src/ui/app.rs` | Handle focus messages |
| `ccmux-server/src/session/manager.rs` | Track active pane/window per client |

## Implementation Tasks

### Section 1: Protocol Changes
- [ ] Add `FocusPane`, `FocusWindow`, `FocusSession` message types to protocol
- [ ] Update serialization/deserialization

### Section 2: Update ccmux_create_pane
- [ ] Add `select` parameter to tool schema
- [ ] Default to NOT sending focus message
- [ ] Only send focus message when `select: true`

### Section 3: Implement ccmux_select_pane
- [ ] Add tool definition
- [ ] Implement handler that sends FocusPane message
- [ ] Return current focus state in response

### Section 4: Implement ccmux_select_window
- [ ] Add tool definition
- [ ] Implement handler that sends FocusWindow message
- [ ] Support window by UUID, name, or index

### Section 5: Implement ccmux_select_session
- [ ] Add tool definition
- [ ] Implement handler that sends FocusSession message
- [ ] Switches TUI to different session

### Section 6: Update ccmux_list_panes
- [ ] Add `focused` field to response items
- [ ] Track which pane is currently focused

### Section 7: Client-Side Handling
- [ ] Handle FocusPane message
- [ ] Handle FocusWindow message
- [ ] Handle FocusSession message
- [ ] Update UI state appropriately

### Section 8: Testing
- [ ] Test create_pane no longer auto-focuses
- [ ] Test create_pane with select: true does focus
- [ ] Test ccmux_select_pane switches focus
- [ ] Test ccmux_select_window switches tabs
- [ ] Test ccmux_select_session switches sessions
- [ ] Test focused field in list_panes response

## Acceptance Criteria

- [ ] `ccmux_create_pane` does NOT auto-switch focus by default
- [ ] `ccmux_create_pane` with `select: true` DOES switch focus
- [ ] `ccmux_select_pane` switches focus to specified pane
- [ ] `ccmux_select_window` switches to specified window/tab
- [ ] `ccmux_select_session` switches to specified session
- [ ] `ccmux_list_panes` includes `focused` field
- [ ] All existing tests pass
- [ ] New tools have test coverage

## Dependencies

None - this is a foundational MCP feature.

## Notes

- This feature is critical for multi-agent orchestration where the orchestrator needs to spawn workers without losing its own focus
- The "select" naming follows tmux conventions (`select-pane`, `select-window`)
- Future enhancement: add relative navigation (next/prev pane, next/prev window)
