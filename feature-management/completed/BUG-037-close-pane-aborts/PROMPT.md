# BUG-037: close_pane Returns AbortError

**Priority**: P2
**Component**: ccmux-server
**Severity**: medium
**Status**: fixed

## Problem Statement

`ccmux_close_pane` fails with "AbortError: The operation was aborted" instead of closing the pane.

## Steps to Reproduce

1. Have a pane open (e.g., the stray "logs" window pane from BUG-034)
2. Call `ccmux_close_pane(pane_id: "062d7f57-87d4-40c6-9238-01df075c3cee")`
3. **Observe**: Returns error instead of closing

## Expected Behavior
- Pane should be closed
- Returns success confirmation

## Actual Behavior
```
MCP error -32001: AbortError: The operation was aborted.
```

Pane remains open. User had to close it manually.

## Environment
- ccmux version: current main branch
- Platform: Linux (WSL2)
- Triggered during: QA demo cleanup

## Impact
- **Severity**: P2 - Cannot programmatically close panes
- **Workaround**: User manually closes pane via keyboard

## Root Cause Analysis

The MCP bridge uses an I/O task to handle bidirectional communication with the daemon:
```rust
tokio::spawn(async move {
    loop {
        tokio::select! {
            Some(msg) = outgoing_rx.recv() => { /* send to daemon */ }
            result = stream.next() => {
                incoming_tx.send(msg).await;  // BLOCKS when channel full!
            }
        }
    }
});
```

The incoming channel had a bounded capacity of 32 messages. When heavy broadcast traffic
(PTY output) filled the channel, `incoming_tx.send(msg).await` would **block the entire
I/O task**, preventing it from:
1. Sending outgoing messages (tool requests like ClosePane)
2. Receiving new messages (including tool responses like PaneClosed)

Tool calls would timeout waiting for responses that could never arrive because the
I/O task was blocked.

## Solution

Changed the incoming message channel from bounded (`mpsc::channel(32)`) to unbounded
(`mpsc::unbounded_channel()`). This ensures the I/O task never blocks due to channel
pressure:

```rust
// Before:
let (incoming_tx, daemon_rx) = mpsc::channel::<ServerMessage>(32);
if incoming_tx.send(msg).await.is_err() { ... }  // Could block!

// After (BUG-037 FIX):
let (incoming_tx, daemon_rx) = mpsc::unbounded_channel::<ServerMessage>();
if incoming_tx.send(msg).is_err() { ... }  // Never blocks
```

The fix is safe because:
- `recv_filtered` actively consumes messages, providing natural backpressure
- The daemon rate-limits output inherently
- Blocking the I/O task is worse than buffering messages temporarily

## Files Changed
- `ccmux-server/src/mcp/bridge/connection.rs`: Changed to unbounded channel
