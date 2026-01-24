# BUG-007: Shift+Tab Not Passed Through to PTY

**Priority**: P1 (High)
**Component**: fugue-client
**Status**: resolved
**Resolved**: 2026-01-09
**Created**: 2026-01-09
**Discovered During**: Manual Testing with Claude Code

## Summary

Shift+Tab keystrokes are not being passed through to programs running in the PTY (e.g., Claude Code). Something in the fugue client input handling is intercepting or dropping the Shift+Tab combination.

## Reproduction Steps

1. Start fugue and create/attach to a session
2. Run Claude Code or another program that uses Shift+Tab
3. Press Shift+Tab
4. Observe that the keystroke is not received by the program

## Expected Behavior

- Shift+Tab should be translated to the appropriate escape sequence (typically `\x1b[Z` or CSI Z)
- The escape sequence should be sent to the PTY
- The program should receive and respond to the keystroke

## Actual Behavior

- Shift+Tab appears to be intercepted or dropped
- The program does not receive the keystroke

## Investigation Areas

### 1. Key Translation (`fugue-client/src/input/keys.rs`)

Check if Shift+Tab is being translated correctly:
```rust
// Expected: KeyCode::BackTab or KeyCode::Tab with SHIFT modifier
// Should produce: \x1b[Z (CSI Z)
```

### 2. Input Handler (`fugue-client/src/input/mod.rs`)

Check if Shift+Tab is being consumed by:
- Quick bindings check
- Prefix key handling
- Any special case handling

### 3. Mouse Handler (`fugue-client/src/input/mouse.rs`)

Unlikely but check if there's any Tab-related handling.

### 4. App Event Handling (`fugue-client/src/ui/app.rs`)

Check `handle_input_action()` for any Shift+Tab specific handling.

## Root Cause (CONFIRMED)

In `fugue-client/src/input/keys.rs:111`, the catch-all `_ => None` drops any unhandled key codes.

The code at line 21-28 handles `KeyCode::Tab` with SHIFT modifier:
```rust
KeyCode::Tab => {
    if modifiers.contains(KeyModifiers::SHIFT) {
        Some(b"\x1b[Z".to_vec())  // This path is never hit
    } else {
        Some(vec![b'\t'])
    }
}
```

**But crossterm sends `KeyCode::BackTab` for Shift+Tab, not `KeyCode::Tab` with SHIFT modifier.**

There is no `KeyCode::BackTab` match arm, so it falls through to `_ => None` and the keystroke is silently dropped.

## Fix

Add `KeyCode::BackTab` case in `keys.rs`:

```rust
KeyCode::BackTab => Some(b"\x1b[Z".to_vec()),
```

This should be added near the `KeyCode::Tab` handling (around line 28).

## Related Code

| File | Purpose |
|------|---------|
| `fugue-client/src/input/keys.rs` | Key-to-escape-sequence translation |
| `fugue-client/src/input/mod.rs` | Input state machine and routing |
| `fugue-client/src/ui/app.rs` | High-level input action handling |

## Notes

- Shift+Tab is commonly used in Claude Code for cycling through suggestions
- This may affect other modifier+Tab combinations
- Check if regular Tab works (likely yes, since this wasn't reported)
