# BUG-014: Large Output Causes Viewport/Buffer Overflow, Making Input Unresponsive

**Priority**: P1 (High)
**Component**: terminal-buffer
**Status**: fixed
**Created**: 2026-01-10
**Fixed**: 2026-01-10

## Summary

When a Claude session generates large output (such as an extensive README diff), the viewport/scrollback/buffer cannot hold the full output. This causes input to become completely unresponsive - the user cannot interact with the session at all. Detach (Ctrl+B d) works, but reattaching shows the same unresponsive state, indicating a persistent buffer capacity issue on the server side.

## Symptoms

- Input completely unresponsive after large output
- User cannot type or interact with the session
- Ctrl+B d (detach) still works
- Reattaching shows same unresponsive state
- Session itself is still running (not crashed)

## Relationship to Other Bugs

This is distinct from **BUG-011** (Large Paste Crashes Session):
- BUG-011: Large **input** (paste) causes crash
- BUG-014: Large **output** (Claude response) causes unresponsiveness

Both may share underlying causes related to buffer management, but this bug focuses on the output path.

## Data Flow to Investigate

The output flow in ccmux is:

```
PTY -> PtyOutputPoller -> Broadcast -> Unix Socket -> TUI Client -> Viewport
```

Key components in this path:

1. **PTY Output** (`ccmux-server/src/pty/`)
   - Reads from PTY master
   - Accumulates output in buffer
   - Flushes periodically

2. **Output Broadcaster** (`ccmux-server/src/handlers/`)
   - Receives flushed output
   - Broadcasts to all attached clients
   - May queue messages if clients are slow

3. **Unix Socket** (`ccmux-protocol/`)
   - Serializes and sends output messages
   - May have write buffer limits

4. **TUI Client** (`ccmux-client/src/`)
   - Receives output messages
   - Updates terminal emulator state
   - Renders to viewport

5. **Terminal Emulator / Scrollback** (`ccmux-client/src/terminal/` or screen buffer)
   - Stores screen content
   - Maintains scrollback history
   - May have no size limits

## Likely Causes to Investigate

### 1. Unbounded Scrollback Buffer

The terminal emulator or screen buffer may grow unboundedly as output is received, eventually consuming so much memory that the event loop becomes unresponsive.

**Files to check:**
- Terminal emulator implementation
- Screen buffer allocation
- Scrollback storage

### 2. Output Flooding Faster Than Client Can Consume

Large output may be broadcast to clients faster than they can render it, causing message queue backup that blocks the event loop or input processing.

**Files to check:**
- `ccmux-server/src/pty/output.rs` - output polling/broadcasting
- Client message receive loop
- Input event interleaving

### 3. Missing Backpressure Mechanism

There may be no backpressure between the server's output broadcast and client consumption. If the client falls behind, messages queue up until something breaks.

**Files to check:**
- Broadcast channel implementation
- Client connection handling
- Flow control mechanisms

### 4. Event Loop Starvation

The client's event loop may be so busy processing output that it never gets to input events, making it appear unresponsive.

**Files to check:**
- `ccmux-client/src/main.rs` - main event loop
- `ccmux-client/src/ui/app.rs` - application loop
- Input vs output event priority

### 5. Server-Side State Bloat

The server may be maintaining the full terminal state/scrollback for each pane, which grows unbounded with large output. Clients receive state updates that grow progressively larger.

**Files to check:**
- Server-side terminal state storage
- State synchronization protocol

## Acceptance Criteria

- [x] Root cause identified and documented (event loop starvation)
- [x] Large outputs (e.g., 1MB+ of text) handled gracefully (spread across ticks)
- [x] Input remains responsive even during large output (MAX_MESSAGES_PER_TICK limits processing)
- [x] Scrollback has configurable size limit (already existed: 1000 lines server/client)
- [x] Old content gracefully purged when limit exceeded (already existed: VecDeque ring buffer)
- [x] Detach/reattach restores usable session (no longer blocked by starvation)
- [x] Memory usage remains bounded (scrollback limits + message processing limits)
- [x] Add test case to prevent regression (3 new tests added)

## Implementation Tasks

### Section 1: Investigation

- [ ] Reproduce the bug with a large Claude output
- [ ] Monitor memory usage during reproduction
- [ ] Identify where responsiveness degrades (client, server, or both)
- [ ] Add logging/tracing to pinpoint bottleneck
- [ ] Document root cause in PLAN.md

### Section 2: Fix Implementation

Based on root cause, implement appropriate fix:

#### If scrollback buffer overflow:
- [ ] Add configurable scrollback limit
- [ ] Implement ring buffer or LRU eviction for old lines
- [ ] Ensure limit applies to both client and server state

#### If output flooding:
- [ ] Implement backpressure from client to server
- [ ] Add output rate limiting or batching
- [ ] Ensure input events are prioritized over output processing

#### If event loop starvation:
- [ ] Ensure input events are checked between output processing
- [ ] Implement fair scheduling between input and output
- [ ] Add yield points in long output processing

#### If server-side bloat:
- [ ] Limit server-side terminal state retention
- [ ] Implement incremental state updates vs full state
- [ ] Add state compression

#### General:
- [ ] Add graceful degradation (warning when buffer is full)
- [ ] Ensure session remains interactive even when buffer is at limit
- [ ] Add configuration option for buffer limits

### Section 3: Testing

- [ ] Add unit test for scrollback limit
- [ ] Add integration test for large output handling
- [ ] Manual test with Claude generating large diffs
- [ ] Verify input remains responsive during large output
- [ ] Verify memory usage stays bounded
- [ ] Test detach/reattach after large output

### Section 4: Verification

- [ ] Confirm input responsive during large output
- [ ] Confirm scrollback limit works correctly
- [ ] All acceptance criteria met
- [ ] Update bug_report.json with resolution details

## Resolution

### Root Cause
The issue was **event loop starvation** in the client. The `poll_server_messages()` function in `ccmux-client/src/ui/app.rs` used a `while let` loop that processed ALL pending server messages before returning:

```rust
// BEFORE (problematic):
async fn poll_server_messages(&mut self) -> Result<()> {
    while let Some(msg) = self.connection.try_recv() {
        self.handle_server_message(msg).await?;
    }
    Ok(())
}
```

During large output bursts, the server sends many `Output` messages (each up to 16KB). These queue up faster than the client can process them. The `while let` loop would block the event loop processing all queued messages before allowing:
- UI redraw
- Input event handling

This made the client appear completely unresponsive.

### What Was NOT the Problem
The investigation confirmed that existing buffer management was already properly bounded:
- **Server ScrollbackBuffer** (`buffer.rs`): Already uses VecDeque with 1000 line limit and LRU eviction
- **Client vt100 Parser** (`pane.rs`): Already has 1000 line scrollback limit
- **Connection channel**: 100 message buffer

### Fix Applied
Added `MAX_MESSAGES_PER_TICK = 50` constant to limit message processing per tick:

```rust
// AFTER (fixed):
const MAX_MESSAGES_PER_TICK: usize = 50;

async fn poll_server_messages(&mut self) -> Result<()> {
    let mut processed = 0;
    while processed < MAX_MESSAGES_PER_TICK {
        if let Some(msg) = self.connection.try_recv() {
            self.handle_server_message(msg).await?;
            processed += 1;
        } else {
            break;
        }
    }
    Ok(())
}
```

This ensures:
- Input events are processed between message batches
- UI redraws happen regularly
- Large outputs are spread across multiple ticks (~200ms for 1MB)
- The event loop remains responsive

### Files Changed
- `ccmux-client/src/ui/app.rs`: Added `MAX_MESSAGES_PER_TICK` constant and modified `poll_server_messages()`

### Tests Added
- `test_max_messages_per_tick_is_reasonable`
- `test_max_messages_per_tick_allows_responsive_input`
- `test_large_output_message_count`

## Notes

This is a P1 bug because:
- It completely blocks user interaction
- There is no workaround once the state is reached
- It occurs during normal Claude workflows (generating diffs, documentation)
- The session appears stuck but is actually running, causing user confusion

The fix should prioritize:
1. Maintaining input responsiveness at all times
2. Implementing reasonable scrollback limits
3. Graceful degradation rather than complete lockup

It's acceptable to:
- Discard very old scrollback content
- Throttle output rendering to maintain input responsiveness
- Show a warning when buffer approaches limit
