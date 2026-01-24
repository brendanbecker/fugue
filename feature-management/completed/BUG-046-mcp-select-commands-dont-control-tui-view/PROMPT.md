# BUG-046: MCP select_session/select_window commands don't control TUI view

**Priority**: P2
**Component**: handlers/session.rs, registry
**Severity**: medium
**Status**: new

## Problem Statement

When calling `fugue_select_session` or `fugue_select_window` via MCP, the command returns success but the TUI view does not change. The MCP bridge and TUI are separate clients with independent focus states, so MCP commands only update the MCP bridge's focus, not the human's TUI view.

## Symptoms

1. `fugue_select_session` returns `{"session_id": "...", "status": "selected"}` successfully
2. `fugue_select_window` returns `{"window_id": "...", "status": "selected"}` successfully
3. The TUI remains on the previous session/window - no visual change occurs
4. User must manually switch sessions/windows in TUI despite MCP "success"

## Root Cause

Per-client focus architecture (FEAT-078) isolates focus state per client. In `session.rs:770`:

```rust
self.registry.update_client_focus(self.client_id, Some(session_id), ...)
```

When MCP calls `SelectSession`:
1. Daemon receives request from MCP bridge client (client_id = MCP bridge)
2. Daemon updates MCP bridge's focus state in registry
3. Daemon sends `SessionFocused` response to MCP bridge only
4. TUI client's focus state is unchanged
5. TUI receives no message and doesn't update its view

The MCP bridge and TUI are registered as separate clients with separate `ClientId`s and independent `ClientFocusState`.

## Affected Components

| File | Function | Issue |
|------|----------|-------|
| `fugue-server/src/handlers/session.rs` | `handle_select_session()` | Updates requesting client's focus only |
| `fugue-server/src/handlers/session.rs` | `handle_select_window()` | Updates requesting client's focus only |
| `fugue-server/src/registry.rs` | `update_client_focus()` | Per-client focus by design |

## Steps to Reproduce

1. Start fugue with TUI attached to a session
2. Create additional sessions via MCP: `fugue_create_session(name: "test")`
3. Call `fugue_select_session(session_id: "<new-session-id>")`
4. Observe: MCP returns success, but TUI still shows original session
5. Manually press session-switch key in TUI - now it changes

## Expected vs Actual Behavior

**Expected**: When MCP calls `select_session`, the attached TUI's view should change to that session, matching what a human would see if they pressed the session-switch key.

**Actual**: MCP command succeeds silently. TUI view unchanged. Human must manually switch.

## Design Considerations

### Option 1: Target TUI clients from MCP commands

MCP select commands should find attached TUI clients and update their focus instead of (or in addition to) the MCP bridge's focus.

```rust
// In handle_select_session:
// Find all TUI clients attached to this session's server
let tui_clients = self.registry.clients_of_type(ClientType::Tui);
for client_id in tui_clients {
    self.registry.update_client_focus(client_id, Some(session_id), ...);
    // Also send SessionFocused to each TUI client
}
```

### Option 2: Broadcast focus changes to TUI

Instead of `HandlerResult::Response`, use broadcast to all TUI clients:

```rust
HandlerResult::Targeted {
    client_types: vec![ClientType::Tui],
    message: ServerMessage::SessionFocused { session_id },
}
```

### Option 3: Add target parameter to MCP tools

Allow MCP to specify which client type to control:

```rust
fugue_select_session(session_id: "...", target: "tui")
```

### Recommendation

Option 1 is most intuitive - MCP is acting on behalf of the human, so it should control what the human sees (TUI), not just the MCP bridge's internal state.

## Implementation Tasks

### Section 1: Investigation
- [ ] Verify ClientType::Tui is correctly assigned to TUI clients on connect
- [ ] Confirm registry can query clients by type
- [ ] Trace message flow when TUI client receives SessionFocused

### Section 2: Implementation
- [ ] Add `clients_of_type(ClientType)` method to registry if not exists
- [ ] Modify `handle_select_session` to update TUI client focus
- [ ] Modify `handle_select_window` to update TUI client focus
- [ ] Send focus notification to affected TUI clients

### Section 3: Testing
- [ ] Test MCP select_session changes TUI view
- [ ] Test MCP select_window changes TUI view
- [ ] Test with multiple TUI clients attached
- [ ] Test that MCP-only focus tracking still works for non-TUI scenarios

### Section 4: Verification
- [ ] QA demo select commands now change visible view
- [ ] No regression in TUI-initiated focus changes
- [ ] Session navigation works end-to-end via MCP

## Acceptance Criteria

- [ ] `fugue_select_session` via MCP changes the TUI's displayed session
- [ ] `fugue_select_window` via MCP changes the TUI's displayed window
- [ ] Human sees visual confirmation of the switch
- [ ] MCP response still indicates success
- [ ] Works with multiple attached TUI clients (all switch, or primary switches)

## Related

- **FEAT-078**: Per-client focus state (introduced the isolation)
- **FEAT-079**: Client type tracking (provides TUI vs MCP distinction)
- **BUG-043**: MCP handlers fail to unwrap Sequenced (related MCP issue, now fixed)

## Notes

This is an architectural design decision: should MCP commands control the TUI view or just the MCP client's conceptual view? The QA demo and user expectations suggest MCP should control what the human sees, making the agent's actions visible.
