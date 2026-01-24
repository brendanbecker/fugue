# BUG-032: MCP Handlers Missing TUI Broadcasts

**Priority**: P0
**Component**: fugue-server (handlers/mcp_bridge.rs)
**Severity**: critical
**Status**: new

## Problem Statement

Multiple MCP handlers in `fugue-server/src/handlers/mcp_bridge.rs` create panes, windows, or layouts but fail to broadcast state changes to TUI clients. This causes the TUI to become out of sync with the daemon's actual state.

When an MCP client (e.g., Claude orchestrator) creates a split pane, window, or layout, the operation succeeds on the server but the TUI never receives the notification. The user sees stale state until they disconnect and reconnect.

## Evidence

### Affected Handlers

| Handler | Line | Current Return | Should Be |
|---------|------|----------------|-----------|
| `handle_split_pane` | 852 | `Response(PaneSplit)` | `ResponseWithBroadcast` + `PaneCreated` |
| `handle_create_window_with_options` | 719 | `Response(WindowCreatedWithDetails)` | `ResponseWithBroadcast` + `WindowCreated` + `PaneCreated` |
| `handle_create_layout` | 1073 | `Response(LayoutCreated)` | `ResponseWithBroadcast` + multiple `PaneCreated` |
| `handle_resize_pane_delta` | 923 | `Response(PaneResized)` | `ResponseWithBroadcast` + `PaneResized` |

### Working Reference Pattern

The correct pattern is demonstrated in `handle_create_pane_with_options` (line 447):

```rust
HandlerResult::ResponseWithBroadcast {
    response: ServerMessage::PaneCreatedWithDetails {
        pane_id,
        session_id,
        session_name,
        window_id,
        direction: direction_str.to_string(),
    },
    session_id,
    broadcast: ServerMessage::PaneCreated { pane: pane_info, direction },
}
```

## Steps to Reproduce

1. Start fugue TUI: `fugue`
2. Attach to or create a session
3. From another Claude session, call `fugue_split_pane` on the current pane
4. **Observe**: The daemon shows 2 panes (verified via `fugue_list_panes`)
5. **Observe**: The TUI still shows only 1 pane at full width
6. Press `Ctrl+B o` - the new pane is not accessible because TUI doesn't know about it

## Expected Behavior

All MCP handlers that modify session state should:
1. Return a response to the MCP caller (for tool result)
2. Broadcast the state change to all TUI clients attached to the session

TUI should immediately render:
- New split panes from `split_pane`
- New windows from `create_window`
- Complete layouts from `create_layout`
- Updated dimensions from `resize_pane_delta`

## Actual Behavior

The four affected handlers only return `HandlerResult::Response`, which sends the result to the MCP caller but does not broadcast to TUI clients. The TUI remains stale.

## Root Cause

These handlers were implemented to return responses to the MCP caller but forgot to also broadcast state changes to connected TUI clients. Compare to `handle_create_pane_with_options` (line 447) which correctly uses `ResponseWithBroadcast`.

This is likely an oversight from when these handlers were added - they may have been written before the broadcast pattern was established for MCP handlers, or during rapid feature development.

## Implementation Tasks

### Section 1: Fix handle_split_pane

- [ ] Collect `pane_info` for the new pane after creation (similar to create_pane handler)
- [ ] Change return from `HandlerResult::Response` to `HandlerResult::ResponseWithBroadcast`
- [ ] Set `broadcast` to `ServerMessage::PaneCreated { pane: pane_info, direction }`
- [ ] Ensure `session_id` is passed for routing the broadcast

### Section 2: Fix handle_create_window_with_options

- [ ] Collect window info and pane info after creation
- [ ] Change return from `HandlerResult::Response` to `HandlerResult::ResponseWithBroadcast`
- [ ] Broadcast should include `WindowCreated` message (may need to add this message type)
- [ ] Also broadcast `PaneCreated` for the default pane in the new window
- [ ] Note: May need to return multiple broadcasts or combine into single update message

### Section 3: Fix handle_create_layout

- [ ] Collect `PaneInfo` for each pane created during layout construction
- [ ] Change return from `HandlerResult::Response` to `HandlerResult::ResponseWithBroadcast`
- [ ] Broadcast multiple `PaneCreated` messages (or a single `LayoutCreated` broadcast)
- [ ] Ensure all panes in the layout are included in the broadcast

### Section 4: Fix handle_resize_pane_delta

- [ ] Change return from `HandlerResult::Response` to `HandlerResult::ResponseWithBroadcast`
- [ ] Set `broadcast` to `ServerMessage::PaneResized { pane_id, new_cols, new_rows }`
- [ ] Ensure TUI handles `PaneResized` broadcast (may need client-side update)

### Section 5: Testing

- [ ] Add integration test for `split_pane` broadcast (similar to `test_mcp_pane_creation_broadcasts_to_tui`)
- [ ] Add integration test for `create_window` broadcast
- [ ] Add integration test for `create_layout` broadcast
- [ ] Add integration test for `resize_pane_delta` broadcast
- [ ] Manual verification: TUI updates immediately after each MCP operation

### Section 6: Documentation

- [ ] Update handler documentation to note broadcast requirement
- [ ] Add comment explaining why `ResponseWithBroadcast` is needed for state-modifying handlers

## Acceptance Criteria

- [ ] `fugue_split_pane` results in immediate TUI update showing the new pane
- [ ] `fugue_create_window` results in TUI awareness of the new window
- [ ] `fugue_create_layout` results in TUI rendering the complete layout
- [ ] `fugue_resize_pane_delta` results in TUI updating pane dimensions
- [ ] All existing tests continue to pass
- [ ] New integration tests cover MCP-to-TUI broadcast for all four handlers
- [ ] `Ctrl+B o` cycles through all panes including those created via MCP

## Related Issues

- **BUG-010**: MCP pane broadcast not received by TUI (fixed the infrastructure)
- **BUG-026**: Focus management broken (fixed select_pane/select_window broadcasts)
- These handlers were missed when BUG-010/BUG-026 were fixed

## Files to Modify

| File | Changes |
|------|---------|
| `fugue-server/src/handlers/mcp_bridge.rs` | Add broadcasts to 4 handlers |
| `fugue-protocol/src/messages.rs` | May need `WindowCreated` broadcast message |
| `fugue-client/src/ui/app.rs` | May need `WindowCreated` handler |

## Notes

- The `ResponseWithBroadcast` pattern is well-established in the codebase
- Focus changes (BUG-026) added `FocusChanged` broadcast - similar pattern needed here
- Consider whether a single "StateUpdated" message would be cleaner than multiple broadcasts
- Layout creation may need special handling to batch multiple pane creations
