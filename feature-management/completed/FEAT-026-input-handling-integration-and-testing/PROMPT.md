# FEAT-026: Input Handling Integration and Testing

**Priority**: P1
**Component**: fugue-client
**Type**: enhancement
**Estimated Effort**: small (1-2 hours)
**Business Value**: high

## Overview

Verify and test keyboard input forwarding from client to server PTY. Input handling code already exists from FEAT-010, including keyboard parsing in `input/keys.rs` and byte conversion in `input_handler.rs`. The main gap is integration testing to ensure end-to-end input flow works correctly.

## Background

The input handling infrastructure was implemented in FEAT-010:
- Keyboard parsing exists in `fugue-client/src/input/keys.rs`
- Byte conversion exists in `fugue-client/src/input/input_handler.rs`
- Sending to server already works via `handle_input_action()` and `ClientMessage::Input`

This feature focuses on verification and testing rather than new implementation.

## Requirements

### 1. Verify Keyboard Event Parsing
- Confirm `input/keys.rs` correctly parses all key types
- Test letter keys, numbers, punctuation
- Test modifier keys (Ctrl, Alt, Shift)
- Test function keys (F1-F12)
- Test navigation keys (arrows, Home, End, PageUp, PageDown)

### 2. Test Byte Conversion
- Verify `input_handler.rs` converts key events to correct byte sequences
- Test standard ASCII characters
- Test control characters (Ctrl-C = 0x03, Ctrl-D = 0x04, etc.)
- Test escape sequences for special keys

### 3. Confirm ClientMessage::Input Sending
- Verify `handle_input_action()` sends input correctly
- Test message framing over Unix socket
- Confirm server receives and processes input messages

### 4. Implement Pane Navigation
- Prefix key (Ctrl-B by default) + arrow keys for pane switching
- Confirm command mode activation/deactivation works
- Test timeout behavior

### 5. Implement Command Mode
- Prefix key + : for command mode entry
- Basic command parsing and execution
- Visual feedback for command mode

### 6. Test Special Keys
- Ctrl combinations (Ctrl-C interrupt, Ctrl-D EOF, Ctrl-Z suspend)
- Alt combinations
- Function keys in terminal applications
- Bracketed paste mode

## Location

Primary files to verify/test:
- `fugue-client/src/input/keys.rs` - Keyboard event parsing
- `fugue-client/src/input/input_handler.rs` - Byte conversion
- `fugue-client/src/input/mod.rs` - Input module organization
- `fugue-client/src/main.rs` - `handle_input_action()` integration

## Technical Notes

- Input handling code already exists from FEAT-010
- Keyboard parsing exists in `input/keys.rs`
- Conversion to bytes in `input_handler.rs`
- Sending to server already works in `handle_input_action()`
- Main gap is integration testing to verify end-to-end flow

## Dependencies

- **FEAT-022** (Message Routing) - Server must handle Input messages
- **FEAT-023** (PTY Output) - To see results of input (feedback loop)

## Acceptance Criteria

- [ ] Can type in terminal pane and see characters echoed
- [ ] Special keys work correctly:
  - [ ] Ctrl-C sends interrupt signal
  - [ ] Ctrl-D sends EOF
  - [ ] Ctrl-Z sends suspend signal
  - [ ] Arrow keys navigate in editors/shells
- [ ] Pane switching with prefix key + arrows works
- [ ] Command mode accessible with prefix key + :
- [ ] Mouse events work (if supported by terminal)
- [ ] No input lag or dropped keystrokes
- [ ] Function keys work in terminal applications (vim, htop, etc.)

## Testing Approach

### Unit Tests
- Key parsing for all key types
- Byte conversion for control characters
- Command mode state machine

### Integration Tests
- End-to-end input flow from client to PTY
- Multi-pane input routing
- Command mode workflow

### Manual Testing
- Type in shell and verify responsiveness
- Run vim and test all key combinations
- Run htop and verify function keys
- Test Ctrl-C in running process

## Notes

- This is primarily a verification/testing task since code exists
- Focus on finding and fixing edge cases
- Document any discovered issues as bugs
- Consider performance testing for high-speed typing
