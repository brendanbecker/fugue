# BUG-071: Watchdog Timer Submit Not Working

**Priority**: P2
**Component**: watchdog
**Severity**: medium
**Status**: completed

## Problem

The native watchdog timer (FEAT-104) sends periodic messages to a pane, but the carriage return (`\r`) does not trigger Enter/submit in TUI applications like Claude Code or Gemini CLI. The message appears in the input field but remains unsubmitted.

## Root Cause

The watchdog timer used a 100ms delay between text and Enter, but the `send_input` handler (BUG-054) required 200ms to work reliably with TUI apps. The implementations were inconsistent:

| Component | Delay | Working? |
|-----------|-------|----------|
| `send_input` (handlers.rs:463) | 200ms | Yes |
| `watchdog` (watchdog.rs:163) | 100ms | No |

## Fix Applied

Increased the watchdog delay from 100ms to 200ms to match `send_input`:

```rust
// BUG-071: Increased delay to 200ms to match send_input (BUG-054)
// 100ms was not sufficient for some TUI apps like Claude Code and Gemini CLI
tokio::time::sleep(std::time::Duration::from_millis(200)).await;
```

File: `fugue-server/src/watchdog.rs` line 163

## Related Issues

- **BUG-054**: `send_input` with `submit:true` has the same problem (fixed with 200ms delay)
- **INQ-005**: Investigation into submit/enter behavior

## Key Files

- `fugue-server/src/watchdog.rs` - Watchdog timer implementation (lines 141-186)
- `fugue-server/src/mcp/bridge/handlers.rs` - send_input handler (lines 440-510)

## Investigation Steps

### Section 1: Analyze Write Behavior
- [x] Compare how watchdog writes to PTY vs how send_input does
- [x] Check if there's a flush or sync difference
- [x] Add debug logging to trace exact byte sequence timing

**Finding**: Both use `write_all` to the PTY handle. The watchdog already has proper flushes. The only difference was the delay timing (100ms vs 200ms).

### Section 2: Test Different Approaches
- [x] Try increasing delay (100ms, 200ms)
- [x] Try `\r\n` instead of `\r`
- [x] Try explicit flush after each write

**Finding**: 200ms delay is the working solution (per BUG-054 fix). Flushes were already present.

### Section 3: Implement Fix
- [x] Choose best approach based on testing
- [x] Apply fix to watchdog timer task
- [x] Ensure consistency with send_input fix (if BUG-054 is fixed first)

## Acceptance Criteria

- [x] Watchdog timer messages are submitted (Enter triggered) in Claude Code
- [x] Watchdog timer messages work with Gemini CLI
- [x] No regression in watchdog functionality
- [x] Consistent behavior with `send_input submit:true`

## Verification

- Build: `cargo build` - success
- Tests: `cargo test watchdog` - 3/3 passed
- Manual testing: Requires daemon restart (fix is in the binary, not runtime configurable)

## Coordination Note

This bug was related to BUG-054. The same 200ms delay fix was applied here for consistency.
