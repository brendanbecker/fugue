# Task Breakdown: FEAT-061

**Work Item**: [FEAT-061: Add screen redraw command to fix display corruption](PROMPT.md)
**Status**: Not Started
**Last Updated**: 2026-01-11

## Prerequisites

- [ ] Read and understand PROMPT.md
- [ ] Review PLAN.md and update if needed
- [ ] Identify current key handling code structure
- [ ] Review existing prefix key implementation

## Implementation Tasks

### Add Input Action

- [ ] Add `Redraw` variant to `InputAction` enum in `input/keys.rs` or equivalent
- [ ] Document the new action

### Add Keybinding

- [ ] Map `Ctrl+B, r` to `Redraw` action in command mode handler
- [ ] Ensure keybinding works with current prefix key mechanism
- [ ] Test keybinding is recognized

### Implement Redraw Function

- [ ] Create `force_redraw()` function
- [ ] Call `terminal.clear()` to reset Ratatui state
- [ ] Optionally call crossterm `Clear(ClearType::All)` for extra safety
- [ ] Ensure next render pass draws all elements

### Handle Redraw Action

- [ ] Add handler for `InputAction::Redraw` in main event loop
- [ ] Call `force_redraw()` when action received
- [ ] Log redraw action for debugging

### SIGWINCH Integration (Optional)

- [ ] Add function to send SIGWINCH to pane child process
- [ ] Call SIGWINCH function for active pane after redraw
- [ ] Consider making SIGWINCH configurable

## Testing Tasks

### Unit Tests

- [ ] Test `Redraw` action is recognized from key sequence
- [ ] Test `force_redraw()` function can be called without error

### Integration Tests

- [ ] Test full redraw cycle (key press -> action -> redraw)
- [ ] Verify terminal state is clean after redraw

### Manual Testing

- [ ] Reproduce display corruption scenarios:
  - [ ] Rapid status updates (run process with fast output)
  - [ ] Terminal resize during output
  - [ ] Session switching
  - [ ] Window switching
- [ ] Press `Ctrl+B, r` and verify display is restored
- [ ] Test in various terminal emulators (if available):
  - [ ] Standard terminal
  - [ ] tmux (nested)
  - [ ] Screen (nested)

## Documentation Tasks

- [ ] Add keybinding to help documentation
- [ ] Update keybinding configuration docs (if configurable)
- [ ] Add to README or user guide

## Verification Tasks

- [ ] All acceptance criteria from PROMPT.md met:
  - [ ] `Ctrl+B, r` triggers full screen redraw
  - [ ] After redraw, all UI elements are correctly positioned
  - [ ] Pane contents are re-rendered from buffer (not lost)
  - [ ] No visual artifacts remain after redraw
- [ ] Tests passing
- [ ] Update feature_request.json status

## Completion Checklist

- [ ] All implementation tasks complete
- [ ] All tests passing
- [ ] Manual testing completed
- [ ] Documentation updated
- [ ] PLAN.md updated with final notes
- [ ] Ready for review/merge

---
*Check off tasks as you complete them. Update status field above.*
