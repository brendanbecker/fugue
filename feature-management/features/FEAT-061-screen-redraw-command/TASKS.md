# Task Breakdown: FEAT-061

**Work Item**: [FEAT-061: Add screen redraw command to fix display corruption](PROMPT.md)
**Status**: Completed
**Last Updated**: 2026-01-14

## Prerequisites

- [x] Read and understand PROMPT.md
- [x] Review PLAN.md and update if needed
- [x] Verify existing keybinding system in ccmux-client

## Implementation Tasks

### Phase 1: Update Protocol

- [x] Add `ClientMessage::Redraw` to `ccmux-protocol/src/messages.rs`
- [x] Update codec to handle the new message variant

### Phase 2: Implement Server Handler

- [x] Add `handle_redraw` to `HandlerContext` in `ccmux-server/src/handlers/pane.rs`
- [x] Logic: for each pane, call `pty_handle.resize()` with current dimensions to trigger `SIGWINCH`
- [x] Update `route_message` in `ccmux-server/src/handlers/mod.rs`

### Phase 3: Update Client Input Handling

- [x] Add `Redraw` to `ClientCommand` in `ccmux-client/src/input/commands.rs`
- [x] Add `redraw` to `CommandHandler::parse_command`
- [x] Add `Ctrl+B, r` keybinding in `ccmux-client/src/input/mod.rs`
- [x] Add `Ctrl+L` keybinding in `ccmux-client/src/input/mod.rs` (Normal mode)

### Phase 4: Implement Client Redraw Logic

- [x] Add `needs_redraw` flag to `App` struct in `ccmux-client/src/ui/app.rs`
- [x] Handle `ClientCommand::Redraw` in `handle_client_command`:
  - Set `needs_redraw = true`
  - Send `ClientMessage::Redraw` to server
- [x] Update `run` loop in `app.rs` to call `terminal.clear()?` when `needs_redraw` is true

## Testing Tasks

- [x] Build and run tests: `cargo test -p ccmux-client -p ccmux-server -p ccmux-protocol`
- [x] Verify keybindings trigger the redraw logic (via tests)
- [ ] Manual test: `Ctrl+L` clears visual artifacts
- [ ] Manual test: `Ctrl+B, r` clears visual artifacts

## Documentation Tasks

- [x] Update help text in `CommandHandler::help_text()`

## Verification Tasks

- [x] All acceptance criteria from PROMPT.md met
- [x] No regressions in other commands
- [x] Update feature_request.json status when complete

## Completion Checklist

- [x] Core implementation complete
- [x] All tests passing
- [x] Keybindings verified
- [x] Server-side PTY signaling implemented
- [x] Ready for review/merge

---
*Check off tasks as you complete them. Update status field above.*