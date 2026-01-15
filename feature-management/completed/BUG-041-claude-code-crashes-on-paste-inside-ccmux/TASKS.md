# Tasks: BUG-041 - Claude Code Crashes on Paste

## Section 1: PTY Layer Analysis
- [x] Read `ccmux-server/src/pty/mod.rs` to understand initialization.
- [x] Read `ccmux-client/src/input/mod.rs` to see how paste events are sent.
- [x] Check if `ccmux` handles bracketed paste sequences in input forwarding.
- [x] Check termios settings in PTY creation.

## Section 2: Implementation (Fix)
- [x] Implement bracketed paste support if missing.
- [x] Adjust termios settings if needed.

## Section 3: Verification
- [x] Add unit/integration test for bracketed paste simulation.