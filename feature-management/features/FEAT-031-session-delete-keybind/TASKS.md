# Task Breakdown: FEAT-031

**Work Item**: [FEAT-031: Session Delete/Kill Keybind in Session Select UI](PROMPT.md)
**Status**: Complete
**Last Updated**: 2026-01-09

## Prerequisites

- [x] Read and understand PROMPT.md
- [x] Review PLAN.md and update if needed
- [x] Verify FEAT-024 (Session Selection UI) is available
- [x] Verify FEAT-012 (Session Management) is available

## Protocol Tasks

- [x] Review existing ClientMessage enum in ccmux-protocol/src/lib.rs
- [x] Add `DestroySession { session_id: Uuid }` variant to ClientMessage
- [x] Verify serde serialization works for new variant
- [x] Run `cargo check` on ccmux-protocol

## Server Tasks

- [x] Add match arm for `DestroySession` in server message handler
- [x] Implement session destruction in SessionManager
  - [x] Remove session from sessions map
  - [x] Kill all PTY processes in session
  - [x] Clean up any associated resources
- [x] Broadcast updated session list after destruction
- [x] Add appropriate logging for session destruction
- [x] Handle "session not found" case gracefully

## Client Tasks

- [x] Add Ctrl+D keybind in `handle_session_select_input()`
- [x] Guard against empty session list
- [x] Guard against no session selected
- [x] Send `DestroySession` message to server
- [x] Update help text to show Ctrl+D keybind
- [x] Adjust selection index after deletion (if needed)

## Testing Tasks

- [ ] Manual test: Delete a session with Ctrl+D
- [ ] Manual test: Verify PTY processes are killed
- [ ] Manual test: Delete session with multiple panes
- [ ] Manual test: Delete last session (empty state)
- [ ] Manual test: Try Ctrl+D with empty session list
- [ ] Manual test: Verify session list refreshes after delete
- [ ] Manual test: Multiple clients - verify all see updated list

## Documentation Tasks

- [x] Verify help text shows Ctrl+D keybind
- [ ] Update any relevant documentation

## Verification Tasks

- [x] All acceptance criteria from PROMPT.md met
- [x] No regressions in existing functionality (all tests pass)
- [ ] Update feature_request.json status
- [ ] Document completion in comments.md

## Completion Checklist

- [x] All implementation tasks complete
- [x] All tests passing
- [x] `cargo check` passes for all crates
- [ ] PLAN.md updated with final notes
- [x] Ready for review/merge

---
*Check off tasks as you complete them. Update status field above.*
