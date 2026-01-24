# Implementation Plan: BUG-032

**Work Item**: [BUG-032: MCP Handlers Missing TUI Broadcasts](PROMPT.md)
**Component**: fugue-server
**Priority**: P0
**Created**: 2026-01-11

## Overview

Four MCP handlers in `mcp_bridge.rs` modify session state (create panes, windows, layouts; resize panes) but only return responses to the MCP caller without broadcasting to TUI clients. This fix adds the missing `ResponseWithBroadcast` returns.

## Architecture Decisions

### Approach: Per-Handler Broadcast Fix

Each handler will be modified to use `HandlerResult::ResponseWithBroadcast` instead of `HandlerResult::Response`. This is the established pattern in the codebase (see `handle_create_pane_with_options`).

**Rationale**:
- Consistent with existing broadcast infrastructure
- No new message types needed for most handlers
- Minimal changes to existing code paths
- Already tested pattern (19 broadcast-related tests pass)

### Trade-offs

| Option | Pros | Cons |
|--------|------|------|
| Per-handler broadcast (chosen) | Simple, consistent, tested | Multiple messages for complex operations |
| Batch state sync message | Single message for all changes | New infrastructure, more complex |
| Event sourcing | Complete state sync | Major refactor, overkill |

### Message Types

| Handler | Existing Broadcast Message | Notes |
|---------|---------------------------|-------|
| `split_pane` | `PaneCreated` | Reuse existing message |
| `create_window` | Need `WindowCreated` | May need new message type |
| `create_layout` | Multiple `PaneCreated` | Need to handle multiple broadcasts |
| `resize_pane_delta` | `PaneResized` | Already exists in protocol |

## Affected Components

| Component | Type of Change | Risk Level |
|-----------|----------------|------------|
| `fugue-server/src/handlers/mcp_bridge.rs` | Primary - add broadcasts | Medium |
| `fugue-protocol/src/messages.rs` | May add `WindowCreated` | Low |
| `fugue-client/src/ui/app.rs` | May add `WindowCreated` handler | Low |
| `fugue-server/src/main.rs` | No change (handles broadcasts) | None |

## Detailed Implementation

### 1. handle_split_pane (Line 852)

**Current code:**
```rust
HandlerResult::Response(ServerMessage::PaneSplit {
    new_pane_id,
    original_pane_id: pane_id,
    session_id,
    session_name,
    window_id,
    direction: direction_str.to_string(),
})
```

**Fixed code:**
```rust
// Need to collect pane_info before returning
let pane_info = { /* get pane info from session_manager */ };

HandlerResult::ResponseWithBroadcast {
    response: ServerMessage::PaneSplit {
        new_pane_id,
        original_pane_id: pane_id,
        session_id,
        session_name,
        window_id,
        direction: direction_str.to_string(),
    },
    session_id,
    broadcast: ServerMessage::PaneCreated { pane: pane_info, direction },
}
```

**Challenge**: Need to re-acquire session_manager lock to get `pane_info` after PTY spawn.

### 2. handle_create_window_with_options (Line 719)

**Current code:**
```rust
HandlerResult::Response(ServerMessage::WindowCreatedWithDetails {
    window_id,
    pane_id,
    session_name,
})
```

**Fixed code (option A - single broadcast):**
```rust
// Broadcast the default pane creation (window implicitly created)
HandlerResult::ResponseWithBroadcast {
    response: ServerMessage::WindowCreatedWithDetails { ... },
    session_id,
    broadcast: ServerMessage::PaneCreated { pane: pane_info, direction: SplitDirection::Vertical },
}
```

**Alternative (option B - new WindowCreated message):**
Would require adding `WindowCreated` to protocol and TUI handler.

### 3. handle_create_layout (Line 1073)

**Current code:**
```rust
HandlerResult::Response(ServerMessage::LayoutCreated {
    session_id,
    session_name,
    window_id,
    pane_ids,
})
```

**Challenge**: Multiple panes created - need to broadcast all of them.

**Options:**
1. Broadcast only the `LayoutCreated` message (TUI needs handler)
2. Broadcast first pane, let TUI request full state
3. Return multiple broadcasts (not currently supported by HandlerResult)

**Recommended**: Option 1 - Add `LayoutCreated` broadcast handling to TUI that triggers a full state refresh.

### 4. handle_resize_pane_delta (Line 923)

**Current code:**
```rust
HandlerResult::Response(ServerMessage::PaneResized {
    pane_id,
    new_cols,
    new_rows,
})
```

**Fixed code:**
```rust
HandlerResult::ResponseWithBroadcast {
    response: ServerMessage::PaneResized { pane_id, new_cols, new_rows },
    session_id, // Need to capture earlier in function
    broadcast: ServerMessage::PaneResized { pane_id, new_cols, new_rows },
}
```

**Challenge**: Need to capture `session_id` earlier in the handler (currently only pane_id is used).

## Dependencies

- None external
- Internal: `ResponseWithBroadcast` pattern already established
- `PaneCreated` and `PaneResized` messages already exist

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Regression in MCP responses | Low | High | Comprehensive testing |
| Lock contention (re-acquiring session_manager) | Medium | Medium | Follow split_pane pattern, minimize lock scope |
| TUI doesn't handle new broadcasts | Low | Medium | Check existing handlers first |
| Performance impact from extra broadcasts | Low | Low | Broadcasts are already per-operation |

## Rollback Strategy

If implementation causes issues:
1. Revert commits associated with this work item
2. Verify system returns to previous state (broken broadcast but working MCP)
3. Document what went wrong in comments.md

## Testing Strategy

1. **Unit tests**: Verify each handler returns `ResponseWithBroadcast`
2. **Integration tests**: Similar to `test_mcp_pane_creation_broadcasts_to_tui`
3. **Manual testing**:
   - Start TUI
   - Run MCP operations from another session
   - Verify TUI updates immediately

## Implementation Order

1. `handle_resize_pane_delta` - simplest, same message for response and broadcast
2. `handle_split_pane` - similar to existing `create_pane` pattern
3. `handle_create_window_with_options` - decide on window broadcast approach
4. `handle_create_layout` - most complex, may need TUI changes

## Implementation Notes

<!-- Add notes during implementation -->

---
*This plan should be updated as implementation progresses.*
