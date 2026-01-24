# BUG-071: Watchdog Timer Submit Not Working

**Priority**: P2
**Component**: watchdog
**Severity**: medium
**Status**: completed

## Problem

The native watchdog timer (FEAT-104) sends periodic messages to a pane, but the carriage return (`\r`) does not trigger Enter/submit in TUI applications like Claude Code or Gemini CLI. The message appears in the input field but remains unsubmitted.

## Current Implementation

In `fugue-server/src/watchdog.rs`, the timer task:

1. Sends the message text via `handle.write_all(message.as_bytes())`
2. Waits 50ms
3. Sends carriage return via `handle.write_all(b"\r")`

This mirrors the BUG-054 workaround pattern, but still doesn't work.

## Related Issues

- **BUG-054**: `send_input` with `submit:true` has the same problem
- **INQ-005**: Investigation into submit/enter behavior

## Root Cause Hypothesis

Both BUG-054 and BUG-071 share the same underlying issue:
- 50ms delay may not be sufficient for TUI event processing
- `\r` alone may not be recognized (might need `\r\n`)
- The PTY write may need flushing between sends
- TUI frameworks may require different handling

## Key Files

- `fugue-server/src/watchdog.rs` - Watchdog timer implementation (lines 141-174)
- `fugue-server/src/mcp/bridge/handlers.rs` - send_input handler (compare implementations)
- `fugue-server/src/pty/mod.rs` - PTY handle write methods

## Investigation Steps

### Section 1: Analyze Write Behavior
- [ ] Compare how watchdog writes to PTY vs how send_input does
- [ ] Check if there's a flush or sync difference
- [ ] Add debug logging to trace exact byte sequence timing

### Section 2: Test Different Approaches
- [ ] Try increasing delay (100ms, 200ms)
- [ ] Try `\r\n` instead of `\r`
- [ ] Try explicit flush after each write

### Section 3: Implement Fix
- [ ] Choose best approach based on testing
- [ ] Apply fix to watchdog timer task
- [ ] Ensure consistency with send_input fix (if BUG-054 is fixed first)

## Acceptance Criteria

- [ ] Watchdog timer messages are submitted (Enter triggered) in Claude Code
- [ ] Watchdog timer messages work with Gemini CLI
- [ ] No regression in watchdog functionality
- [ ] Consistent behavior with `send_input submit:true`

## Coordination Note

This bug is likely related to BUG-054. If fixing BUG-054 reveals the root cause, apply the same fix here. The two fixes should be consistent.
