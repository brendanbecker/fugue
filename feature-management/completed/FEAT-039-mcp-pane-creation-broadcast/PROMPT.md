# FEAT-039: MCP Pane Creation Broadcast - Sync TUI Clients on MCP Splits

**Priority**: P1
**Component**: fugue-server
**Type**: enhancement
**Estimated Effort**: small
**Business Value**: high
**Status**: new

## Overview

When panes are created via MCP tools (e.g., `fugue_create_pane`), TUI clients attached to the same session don't see the split because the server doesn't broadcast the `PaneCreated` message to them. This breaks the multi-client experience where MCP and TUI clients should stay in sync.

## Problem Statement

The `handle_create_pane_with_options` function in `fugue-server/src/handlers/mcp_bridge.rs` returns `HandlerResult::Response(ServerMessage::PaneCreatedWithDetails {...})` instead of `HandlerResult::ResponseWithBroadcast`. This means:

1. **MCP client receives**: The `PaneCreatedWithDetails` response with full details
2. **TUI clients receive**: Nothing - they are unaware a new pane was created
3. **Result**: TUI displays become stale; users must manually switch panes or reconnect to see MCP-created splits

## Root Cause

In `fugue-server/src/handlers/mcp_bridge.rs` around line 356:

```rust
HandlerResult::Response(ServerMessage::PaneCreatedWithDetails {
    pane_id,
    session_id,
    session_name,
    window_id,
    direction: direction_str.to_string(),
})
```

This returns only a `Response`, not a `ResponseWithBroadcast`. Compare with the regular pane handler in `fugue-server/src/handlers/pane.rs` around line 119:

```rust
HandlerResult::ResponseWithBroadcast {
    response: ServerMessage::PaneCreated {
        pane: pane_info.clone(),
    },
    session_id,
    broadcast: ServerMessage::PaneCreated { pane: pane_info },
}
```

The regular pane handler correctly broadcasts to all clients attached to the session.

## Solution

Modify `handle_create_pane_with_options` to return `ResponseWithBroadcast` that includes:

1. **Response**: `PaneCreatedWithDetails` for the MCP client (retains existing behavior)
2. **Broadcast**: `PaneCreated` for TUI clients (new behavior)

This requires constructing a `PaneInfo` struct similar to what the regular pane handler does.

## Implementation Details

### Changes to mcp_bridge.rs

Before (current):
```rust
HandlerResult::Response(ServerMessage::PaneCreatedWithDetails {
    pane_id,
    session_id,
    session_name,
    window_id,
    direction: direction_str.to_string(),
})
```

After (fixed):
```rust
// Build PaneInfo for broadcast
let pane_info = PaneInfo {
    id: pane_id,
    window_id,
    session_id,
    index: pane_index,  // Need to get this from the created pane
    title: None,
    cwd: None,
};

HandlerResult::ResponseWithBroadcast {
    response: ServerMessage::PaneCreatedWithDetails {
        pane_id,
        session_id,
        session_name,
        window_id,
        direction: direction_str.to_string(),
    },
    session_id,
    broadcast: ServerMessage::PaneCreated { pane: pane_info },
}
```

### Getting Pane Index

The pane index is needed for `PaneInfo`. The `create_pane` method on `Window` returns the created `Pane`, which should have an `index` field. Ensure this is captured when creating the pane.

## Affected Files

| File | Change |
|------|--------|
| `fugue-server/src/handlers/mcp_bridge.rs` | Change `handle_create_pane_with_options` return type to include broadcast |

## Reference Implementation

See `fugue-server/src/handlers/pane.rs:handle_create_pane()` for the correct pattern:

```rust
// Broadcast to all clients attached to this session
HandlerResult::ResponseWithBroadcast {
    response: ServerMessage::PaneCreated {
        pane: pane_info.clone(),
    },
    session_id,
    broadcast: ServerMessage::PaneCreated { pane: pane_info },
}
```

## Implementation Tasks

### Section 1: Update Return Type
- [ ] Locate `handle_create_pane_with_options` in `mcp_bridge.rs`
- [ ] Capture pane index when creating the pane
- [ ] Construct `PaneInfo` struct with pane details
- [ ] Change return from `HandlerResult::Response` to `HandlerResult::ResponseWithBroadcast`
- [ ] Include both `PaneCreatedWithDetails` response and `PaneCreated` broadcast

### Section 2: Testing
- [ ] Test MCP pane creation still returns correct response to MCP client
- [ ] Test TUI client receives `PaneCreated` broadcast after MCP creates pane
- [ ] Test split rendering in TUI updates after MCP split
- [ ] Test multiple TUI clients all receive the broadcast
- [ ] Verify no regression in existing MCP functionality

### Section 3: Verification
- [ ] Start fugue server
- [ ] Attach TUI client to a session
- [ ] Use MCP tool to create a pane (e.g., `fugue_create_pane`)
- [ ] Verify TUI client immediately shows the new pane split
- [ ] Verify pane navigation works for the new pane

## Acceptance Criteria

- [ ] MCP `fugue_create_pane` still returns `PaneCreatedWithDetails` to the MCP client
- [ ] TUI clients attached to the session receive `PaneCreated` broadcast
- [ ] TUI split pane rendering (FEAT-038) updates to show MCP-created panes
- [ ] No changes required in client code (existing `PaneCreated` handler works)
- [ ] All existing tests pass
- [ ] New test covers MCP-to-TUI broadcast scenario

## Dependencies

- **FEAT-038**: Split Pane Rendering - TUI must handle `PaneCreated` to render splits (implemented)

## Notes

- This is a small change with high impact on user experience
- The fix aligns MCP behavior with the regular pane handler pattern
- Consider also auditing other MCP handlers for similar broadcast issues (e.g., session creation, window creation)
- Future enhancement: Add broadcast for `PaneClosed` from MCP if not already implemented
