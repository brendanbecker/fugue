# Session: Stream C - Buffer Overflow Fix

## Work Item
- **BUG-014**: Large output causes viewport/buffer overflow, making input unresponsive

## Priority: P1 (High) - fixed

## Status: COMPLETED

## Problem Summary

When a Claude session generates large output (such as an extensive README diff), the viewport/scrollback/buffer cannot hold the full output. This causes input to become completely unresponsive - the user cannot interact with the session at all.

## Root Cause Identified

**Event loop starvation** in the client. The `poll_server_messages()` function in `ccmux-client/src/ui/app.rs` used a `while let` loop that processed ALL pending server messages before returning. During large output bursts, this blocked the event loop from processing input events or redrawing the UI.

### What Was NOT the Problem
- Server ScrollbackBuffer: Already uses VecDeque with 1000 line limit
- Client vt100 Parser: Already has 1000 line scrollback limit
- Connection channel: 100 message buffer

## Fix Applied

Added `MAX_MESSAGES_PER_TICK = 50` constant to limit message processing per tick in `poll_server_messages()`. This ensures:
- Input events are processed between message batches
- UI redraws happen regularly
- Large outputs are spread across multiple ticks (~200ms for 1MB)
- The event loop remains responsive

## Files Changed

- `ccmux-client/src/ui/app.rs`: Added `MAX_MESSAGES_PER_TICK` constant and modified `poll_server_messages()`

## Tests Added

- `test_max_messages_per_tick_is_reasonable`
- `test_max_messages_per_tick_allows_responsive_input`
- `test_large_output_message_count`

## Acceptance Criteria

- [x] Large outputs (1MB+ of text) handled gracefully
- [x] Input remains responsive even during large output
- [x] Scrollback has configurable size limit (already existed)
- [x] Old content gracefully purged when limit exceeded (already existed)
- [x] Memory usage remains bounded

## Related Work Items

- See `feature-management/bugs/BUG-014-large-output-buffer-overflow/PROMPT.md`
- Related to BUG-011 (large paste) - may share buffer management root cause

## Commands

```bash
# Build
cargo build --release

# Run tests
cargo test --workspace

# Test with large output
./target/release/ccmux
# Then generate large output in Claude
```
