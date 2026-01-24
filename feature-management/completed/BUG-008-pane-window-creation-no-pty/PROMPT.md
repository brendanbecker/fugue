# BUG-008: Pane/Window Creation Does Not Spawn PTY

**Priority**: P0 (Critical)
**Component**: fugue-server
**Type**: bug
**Status**: fixed

## Problem

When creating new panes (`Ctrl+b %`, `Ctrl+b "`) or windows (`Ctrl+b c`), the handlers created the data structures but did not spawn PTY processes. This resulted in:

- Panes appearing in the UI but being completely unresponsive
- No shell prompt or command input
- "Dead" panes that did nothing when typed into

## Root Cause

`handle_create_pane()` in `pane.rs` and `handle_create_window()` in `session.rs` were missing the PTY spawn and output poller startup code that exists in `handle_create_session()`.

The session creation code properly:
1. Creates session → window → pane
2. Spawns PTY with `pty_manager.spawn()`
3. Starts output poller with `PtyOutputPoller::spawn_with_sideband()`

But the pane/window creation handlers only did step 1.

## Solution

Added PTY spawning and output poller startup to both handlers:

### `handle_create_pane()` - pane.rs
- Now spawns PTY after creating pane struct
- Uses `default_command` from config or falls back to shell
- Inherits server's working directory
- Starts output poller with sideband parsing

### `handle_create_window()` - session.rs
- Now creates a default pane in new windows
- Spawns PTY for the default pane
- Same PTY config and output poller as pane creation

## Files Modified

- `fugue-server/src/handlers/pane.rs` - Added PTY spawn to `handle_create_pane()`
- `fugue-server/src/handlers/session.rs` - Added default pane + PTY to `handle_create_window()`

## Testing

- All 1,285 workspace tests pass
- Manual testing: `Ctrl+b %` now creates working split pane with shell
- Manual testing: `Ctrl+b c` now creates working window with shell

## Commits

- `8f02484` - fix(server): spawn PTY for new panes and windows (BUG-008)
