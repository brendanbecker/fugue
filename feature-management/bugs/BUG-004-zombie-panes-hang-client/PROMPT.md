# BUG-004: Client Hangs When Reattaching to Session with Dead Pane

**Priority**: P1 (High)
**Component**: ccmux-server
**Status**: resolved
**Created**: 2026-01-09
**Resolved**: 2026-01-09
**Discovered During**: Manual Testing

## Summary

Client hangs (becomes unresponsive to input) when attaching to a session whose pane's shell process has exited. The session and pane remain in server state as zombies, causing clients to attach successfully but receive no output.

## Reproduction Steps

1. Start server: `./target/release/ccmux-server`
2. Start client and create/enter a session: `./target/release/ccmux`
3. Exit the shell in the pane (type `exit` or Ctrl+D)
4. Detach from client (Ctrl+B, d) or the client returns to session select
5. Attempt to reattach to the same session
6. Client appears to attach but is completely unresponsive to input

## Expected Behavior

- When a pane's shell process exits, the pane should be cleaned up
- Empty windows should be removed
- Sessions with no panes should be removed
- Client should not be able to attach to zombie sessions

## Actual Behavior

- PTY output poller exits when shell dies (logs: "PTY output poller exiting")
- Pane remains in session state as zombie
- Session remains visible in session list
- Client can "attach" but receives no output and hangs
- Ctrl+C does not work; must kill client externally

## Server Logs (Before Fix)

```
2026-01-09T18:58:42 INFO AttachSession request from Client(1)
2026-01-09T18:58:42 INFO Client attached to session (1 windows, 1 panes)
2026-01-09T18:58:46 INFO PTY output poller exiting  # Shell died, poller exits
...
2026-01-09T19:01:27 INFO AttachSession request from Client(2)
2026-01-09T19:01:27 INFO Client attached to session (1 windows, 1 panes)
# No output poller started - client hangs
```

## Root Cause Analysis

The PTY output poller (`ccmux-server/src/pty/output.rs`) broadcast a `PaneClosed` message to clients when the PTY died, but did NOT clean up the pane/session from server state. This created zombie sessions that:

1. Appeared in the session list
2. Could be "attached" to
3. Had no active output poller
4. Left the client waiting for output that would never come

### Key Code Locations

| File | Line | Issue |
|------|------|-------|
| `ccmux-server/src/pty/output.rs` | 240-245 | Only broadcast PaneClosed, didn't clean up state |
| `ccmux-server/src/handlers/pane.rs` | 120-181 | ClosePane handler had proper cleanup, but only called on explicit request |

## Resolution

Implemented automatic cleanup when PTY processes exit:

### Changes Made

1. **`ccmux-server/src/pty/output.rs`**:
   - Added `PaneClosedNotification` struct
   - Added `pane_closed_tx` channel to `PtyOutputPoller`
   - New `spawn_with_cleanup()` method
   - Updated `PollerManager::with_cleanup_channel()`
   - Send notification when PTY exits

2. **`ccmux-server/src/main.rs`**:
   - Added `pane_closed_tx` to `SharedState`
   - Added `run_pane_cleanup_loop()` background task that:
     - Removes PTY from manager
     - Removes pane from window
     - Removes window if empty
     - Removes session if no windows remain

3. **`ccmux-server/src/handlers/mod.rs`**:
   - Added `pane_closed_tx` to `HandlerContext`
   - New sessions' panes now get cleanup channel

### Server Logs (After Fix)

```
2026-01-09T20:00:00 INFO PTY output poller exiting
2026-01-09T20:00:00 INFO Processing pane cleanup notification
2026-01-09T20:00:00 INFO Pane removed from window
2026-01-09T20:00:00 INFO Empty window removed from session
2026-01-09T20:00:00 INFO Empty session removed
```

## Testing

- All 135 existing tests pass
- Manual testing confirms:
  - Shell exit triggers automatic cleanup
  - Session disappears from session list after shell exits
  - No more zombie sessions

## Files Changed

- `ccmux-server/src/pty/output.rs`
- `ccmux-server/src/pty/mod.rs`
- `ccmux-server/src/main.rs`
- `ccmux-server/src/handlers/mod.rs`
- `ccmux-server/src/handlers/session.rs`
- `ccmux-server/src/handlers/pane.rs`
- `ccmux-server/src/handlers/connection.rs`
- `ccmux-server/src/handlers/input.rs`
- `ccmux-server/src/handlers/orchestration.rs`
