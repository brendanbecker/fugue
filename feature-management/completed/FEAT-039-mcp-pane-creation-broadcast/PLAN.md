# Implementation Plan: FEAT-039

**Work Item**: [FEAT-039: MCP Pane Creation Broadcast - Sync TUI Clients on MCP Splits](PROMPT.md)
**Component**: fugue-server
**Priority**: P1
**Created**: 2026-01-09

## Overview

Modify the MCP bridge's `handle_create_pane_with_options` function to broadcast `PaneCreated` messages to TUI clients when panes are created via MCP tools. This ensures MCP and TUI clients stay synchronized.

## Architecture Decisions

### Approach: Add Broadcast to Existing Handler

The fix is straightforward - change the return type from `HandlerResult::Response` to `HandlerResult::ResponseWithBroadcast`. This follows the established pattern used by `handle_create_pane` in `pane.rs`.

**Why this approach**:
- Minimal code change
- Follows existing patterns
- No protocol changes required
- TUI clients already handle `PaneCreated` messages

### Message Types

**MCP Client receives**: `ServerMessage::PaneCreatedWithDetails` (unchanged)
- Contains: `pane_id`, `session_id`, `session_name`, `window_id`, `direction`
- Used by MCP for structured tool responses

**TUI Clients receive**: `ServerMessage::PaneCreated` (new)
- Contains: `PaneInfo { id, window_id, session_id, index, title, cwd }`
- Standard message TUI clients already handle

### Data Flow

```
MCP Tool Call: fugue_create_pane
       |
       v
mcp_bridge.rs: handle_create_pane_with_options()
       |
       v
Creates pane via session_manager
       |
       v
Returns HandlerResult::ResponseWithBroadcast
       |
       +---> response: PaneCreatedWithDetails -> MCP Client
       |
       +---> broadcast: PaneCreated -> All TUI Clients (via session_id)
```

## Affected Components

| Component | Type of Change | Risk Level |
|-----------|----------------|------------|
| `fugue-server/src/handlers/mcp_bridge.rs` | Minor - change return type | Low |

## Dependencies

- FEAT-038 (Split Pane Rendering) - TUI must render new panes (implemented)

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Missing pane index | Medium | Low | Ensure index is captured from created pane |
| Broadcast to wrong session | Low | Medium | Use correct session_id from pane creation |
| Performance overhead | Low | Low | Broadcast is already efficient |

## Implementation Phases

### Phase 1: Capture Pane Data

Currently `handle_create_pane_with_options` creates the pane but doesn't capture all fields needed for `PaneInfo`. Need to:

1. Get pane index from created pane
2. Optionally get cwd if available
3. Title can be None initially

### Phase 2: Change Return Type

1. Import `PaneInfo` if not already imported
2. Construct `PaneInfo` struct
3. Change `HandlerResult::Response` to `HandlerResult::ResponseWithBroadcast`
4. Include both response and broadcast messages

### Phase 3: Test

1. Unit test for broadcast behavior
2. Integration test with MCP + TUI clients
3. Manual verification

## Code Changes

### mcp_bridge.rs - handle_create_pane_with_options

```rust
// Current (line ~356):
HandlerResult::Response(ServerMessage::PaneCreatedWithDetails {
    pane_id,
    session_id,
    session_name,
    window_id,
    direction: direction_str.to_string(),
})

// New:
// Build PaneInfo for TUI broadcast
let pane_info = PaneInfo {
    id: pane_id,
    window_id,
    session_id,
    index: pane_index,  // Capture from pane creation
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

## Testing Strategy

### Unit Test

Add test case in `mcp_bridge.rs` tests:

```rust
#[tokio::test]
async fn test_create_pane_with_options_broadcasts_to_tui() {
    // Setup MCP bridge context
    // Call handle_create_pane_with_options
    // Assert returns ResponseWithBroadcast
    // Assert broadcast contains PaneCreated
}
```

### Integration Test

1. Start server
2. Connect TUI client, attach to session
3. Connect MCP client to same server
4. MCP creates pane via tool call
5. Assert TUI client receives PaneCreated

### Manual Test

1. `fugue attach` (TUI)
2. In separate terminal, use MCP tool to split pane
3. Verify TUI immediately shows split

## Rollback Strategy

If issues occur:
1. Revert to `HandlerResult::Response` (single line change)
2. Document issues in comments.md
3. TUI clients will still work but won't see MCP splits

## Open Questions

1. **Should other MCP handlers also broadcast?**
   - Session creation?
   - Window creation?
   - Pane closure?
   - Answer: Audit and create follow-up issues if needed

2. **Should broadcast include direction?**
   - TUI `PaneCreated` handler may not use direction
   - FEAT-038 tracks pending splits locally
   - Answer: Current approach is fine; direction in `PaneCreatedWithDetails` response

## Implementation Notes

<!-- Add notes during implementation -->

---
*This plan should be updated as implementation progresses.*
