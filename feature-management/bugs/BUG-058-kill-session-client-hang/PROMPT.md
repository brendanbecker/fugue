# BUG-058: ccmux_kill_session causes client hang

**Priority**: P2
**Component**: mcp, client
**Severity**: medium
**Status**: completed

## Problem

When calling `ccmux_kill_session` via MCP, the ccmux client hangs. The TUI remains responsive to keybindings (Ctrl+b commands work), but the client appears frozen and doesn't return control properly.

## Reproduction Steps

1. Create a session: `ccmux_create_session(name: "test-session")`
2. Kill the session: `ccmux_kill_session(session: "test-session")`
3. Observe: Client hangs

## Expected Behavior

Session is killed and client continues operating normally.

## Actual Behavior

- Session is killed on the daemon side (success response returned)
- Client hangs/freezes
- TUI keybindings (Ctrl+b) still work
- Workaround: Exit to session picker (Ctrl+b s) and return

### Additional Observations (2026-01-18)

When killing multiple sessions in quick succession (e.g., cleaning up 8 idle agent sessions):
- Session picker workaround (Ctrl+b s) may not be sufficient
- Required full client restart to reattach to session
- Daemon remained operational (tmux commands still worked)
- Suggests client state corruption accumulates with multiple rapid kill operations

## Analysis

The TUI event loop is running (keybindings work), but something is blocking the main render/update cycle. Likely causes:

### 1. Waiting for Killed Session Update
The client may be waiting for a state update from a session that no longer exists. The daemon killed the session but the client is still subscribed to events from it.

### 2. Channel Deadlock
The response channel for the kill_session operation may be waiting for something that the killed session was supposed to provide.

### 3. Missing Session Removal Event
The client may not be receiving or processing the session removal notification, leaving it in an inconsistent state.

## Investigation Steps

### Section 1: Add Logging
- [x] Add debug logging to client session subscription handling
- [x] Log when session removal events are received
- [x] Log what the client is waiting on when it hangs

### Section 2: Trace the Hang
- [x] Run with RUST_LOG=debug and capture logs during hang
- [x] Check if daemon sends session removal notification
- [x] Check if client receives and processes it

### Section 3: Test Variations
- [x] Test killing non-current session (does it still hang?)
- [x] Test killing session from within that session
- [x] Test killing session from a different client
- [x] Test killing multiple sessions in rapid succession (8+ sessions)
- [x] Check if hang severity correlates with number of sessions killed

## Acceptance Criteria

- [x] `ccmux_kill_session` completes without hanging the client
- [x] Client remains fully responsive after session kill
- [x] Session removal is properly propagated to all clients
- [x] No regressions in session management

## Related Files

- `ccmux-server/src/mcp/bridge/handlers.rs` - kill_session handler
- `ccmux-server/src/session/manager.rs` - Session management
- `ccmux-client/src/ui/app.rs` - Client state management
- `ccmux-protocol/src/messages.rs` - Session removal messages

## Notes

Discovered during multi-agent orchestration demo on 2026-01-18.
Fixed by broadcasting `SessionEnded` to attached clients before detaching them in `handle_destroy_session`.
