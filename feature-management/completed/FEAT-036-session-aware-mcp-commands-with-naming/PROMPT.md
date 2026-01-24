# FEAT-036: Session-aware MCP Commands with Window/Pane Naming

**Priority**: P1
**Component**: fugue-server (MCP)
**Type**: enhancement
**Estimated Effort**: medium
**Business Value**: high
**Status**: new

## Overview

MCP commands should intelligently default to the active session (one with attached clients) when no session is explicitly specified. Additionally, add the ability to name windows and panes for easier identification and management.

## Problem Statement

The fugue MCP tools (like `fugue_create_pane`) currently default to the wrong session when multiple sessions exist. When a user calls `create_pane` without specifying a session, it picks the first session in the list rather than the session that has an attached client.

### Current Behavior

- `fugue_list_windows` says "uses first session if omitted"
- `fugue_create_window` says "uses first session if omitted"
- `fugue_create_pane` creates pane "in the current session" but doesn't define what "current" means

### Observed Issue

With two sessions:
- Session A: 0 attached clients (orphaned/detached)
- Session B: 1 attached client (user's active session)

Calling `fugue_create_pane` without specifying a session created the pane in Session A (the orphaned session) instead of Session B (where the user is working).

### Impact

- Panes created in wrong session
- User confusion about where new windows/panes appear
- No way to easily organize and identify windows/panes by name

## Requirements

### Part 1: Session-aware Defaults

All session-scoped MCP operations should default to the **active session** (the one with attached clients) when no session is explicitly specified.

#### Affected Tools

| Tool | Current Default | New Default |
|------|-----------------|-------------|
| `fugue_list_windows` | First session | Active session (with attached clients) |
| `fugue_create_window` | First session | Active session |
| `fugue_create_pane` | Ambiguous "current session" | Active session |
| `fugue_list_panes` | Optional session filter | Active session if filter omitted |

#### Implementation Details

1. **Track attached client count per session**
   - Add `attached_clients: usize` to session metadata
   - Increment when client connects to session
   - Decrement when client disconnects

2. **Implement `get_active_session()` helper**
   ```rust
   fn get_active_session(&self) -> Option<SessionId> {
       // Return session with most attached clients
       // If tie, prefer most recently active
       // If no attached clients, fall back to most recent session
   }
   ```

3. **Update tool descriptions**
   - Change "uses first session if omitted" to "uses active session if omitted"
   - Document what "active session" means (session with attached clients)

4. **Return which session was used in responses**
   - Always include `session_id` and `session_name` in tool responses
   - This helps users verify the correct session was targeted

### Part 2: Window/Pane Naming

Add the ability to name windows and panes for easier identification and management.

#### New Parameters

**`fugue_create_pane`** - Add `name` parameter:
```json
{
  "name": {
    "type": "string",
    "description": "Optional display name for the pane"
  }
}
```

**`fugue_create_window`** - Already has `name` parameter (verify it works)

#### New Tools

**`fugue_rename_pane`**:
```rust
Tool {
    name: "fugue_rename_pane".into(),
    description: "Rename a pane for easier identification".into(),
    input_schema: serde_json::json!({
        "type": "object",
        "properties": {
            "pane_id": {
                "type": "string",
                "description": "UUID of the pane to rename"
            },
            "name": {
                "type": "string",
                "description": "New display name for the pane"
            }
        },
        "required": ["pane_id", "name"]
    }),
}
```

**`fugue_rename_window`**:
```rust
Tool {
    name: "fugue_rename_window".into(),
    description: "Rename a window for easier identification".into(),
    input_schema: serde_json::json!({
        "type": "object",
        "properties": {
            "window_id": {
                "type": "string",
                "description": "UUID of the window to rename"
            },
            "name": {
                "type": "string",
                "description": "New display name for the window"
            }
        },
        "required": ["window_id", "name"]
    }),
}
```

#### Updated List Output

**`fugue_list_panes`** response should include name:
```json
{
  "id": "uuid",
  "name": "pane-name",  // Add this field
  "window_id": "uuid",
  "session_id": "uuid",
  ...
}
```

**`fugue_list_windows`** response should include name:
```json
{
  "id": "uuid",
  "name": "window-name",  // Verify this is present
  "index": 0,
  "pane_count": 2,
  ...
}
```

## Affected Files

| File | Changes |
|------|---------|
| `fugue-server/src/mcp/tools.rs` | Add name param to create_pane, add rename tools |
| `fugue-server/src/mcp/handlers.rs` | Implement active session logic, rename handlers |
| `fugue-server/src/mcp/server.rs` | Add routing for rename tools |
| `fugue-server/src/session/mod.rs` | Add attached_clients tracking, name field to Pane |
| `fugue-server/src/session/window.rs` | Add name field to Pane if not present |
| `fugue-server/src/session/pane.rs` | Add name field if not present |

## Implementation Tasks

### Section 1: Session-aware Defaults
- [ ] Add `attached_clients` counter to Session struct
- [ ] Implement client attach/detach tracking
- [ ] Implement `get_active_session()` helper method
- [ ] Update `fugue_list_windows` to use active session
- [ ] Update `fugue_create_window` to use active session
- [ ] Update `fugue_create_pane` to use active session
- [ ] Update `fugue_list_panes` to use active session when no filter
- [ ] Update tool descriptions to say "active session"
- [ ] Ensure all responses include `session_id` and `session_name`

### Section 2: Window/Pane Naming
- [ ] Add `name` field to Pane struct (if not present)
- [ ] Add `name` parameter to `fugue_create_pane`
- [ ] Verify `fugue_create_window` name parameter works
- [ ] Implement `fugue_rename_pane` tool
- [ ] Implement `fugue_rename_window` tool
- [ ] Update `fugue_list_panes` to include name in output
- [ ] Update `fugue_list_windows` to include name in output

### Section 3: Testing
- [ ] Test active session selection with multiple sessions
- [ ] Test active session selection with no attached clients (fallback)
- [ ] Test pane naming on creation
- [ ] Test pane rename
- [ ] Test window rename
- [ ] Test list output includes names
- [ ] Update existing tests for new behavior

### Section 4: Documentation
- [ ] Update tool descriptions in tools.rs
- [ ] Add code comments explaining active session logic
- [ ] Update CHANGELOG

## Acceptance Criteria

- [ ] With multiple sessions, MCP commands default to session with attached clients
- [ ] When no session has attached clients, fallback to most recent session
- [ ] All tool responses include `session_id` and `session_name` for verification
- [ ] Panes can be created with an optional name
- [ ] Panes can be renamed after creation
- [ ] Windows can be renamed after creation
- [ ] `fugue_list_panes` shows pane names
- [ ] `fugue_list_windows` shows window names
- [ ] All existing tests pass
- [ ] New tests cover session selection logic and naming

## Example Scenarios

### Scenario 1: Active Session Selection

**Setup**:
- Session "dev" with 1 attached client
- Session "orphan" with 0 attached clients

**Call**: `fugue_create_pane` with no session parameter

**Expected**: Pane created in "dev" session (has attached client)

### Scenario 2: Fallback When No Active Sessions

**Setup**:
- Session "recent" (created 5 min ago, 0 clients)
- Session "old" (created 1 hour ago, 0 clients)

**Call**: `fugue_create_pane` with no session parameter

**Expected**: Pane created in "recent" session (most recent)

### Scenario 3: Named Pane Creation

**Call**:
```json
{
  "name": "claude_main",
  "command": "claude"
}
```

**Expected**: Pane created with name "claude_main" visible in `fugue_list_panes`

## Dependencies

- **FEAT-029**: MCP Natural Language Terminal Control (provides base MCP tools)

## Notes

- This is a UX improvement that prevents user confusion when working with multiple sessions
- The naming feature aids organization in complex multi-pane workflows
- Consider whether names should be auto-generated (e.g., from command) if not specified
- Pane names could potentially be derived from Claude detection state (e.g., "claude_thinking")
