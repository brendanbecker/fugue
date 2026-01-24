# BUG-001: Client Input Not Captured After Connection

**Priority**: P0 (Blocker)
**Component**: fugue-client
**Status**: new
**Created**: 2026-01-09
**Discovered During**: HA-001 Manual Testing

## Summary

Client connects to server successfully but does not respond to any keyboard input, including Ctrl+C. Terminal appears stuck.

## Reproduction Steps

1. Start server: `cargo run --bin fugue-server`
2. Start client: `cargo run --bin fugue`
3. Client connects (server logs: "Client connected, active clients: 1")
4. Try any keyboard input - nothing happens
5. Ctrl+C does not work
6. Must kill client from another terminal

## Expected Behavior

- Client should enter raw mode and capture keyboard input
- Session selection UI should respond to up/down, j/k, n, r, Enter keys
- Ctrl+C or Ctrl+Q should allow exit

## Actual Behavior

- Client displays UI but does not respond to any input
- Terminal is stuck - even Ctrl+C doesn't work
- Server shows client connected and protocol handshake completed

## Server Logs

```
2026-01-09T16:06:58.477663Z  INFO fugue_server: Client connected, active clients: 1
2026-01-09T16:06:58.477853Z  INFO fugue_server: Client e0c65e44-1bf5-4d70-b2a4-64245c74322b connecting with protocol version 1
```

## Investigation Findings (2026-01-09)

### Pre-existing Bug
This bug existed before FEAT-021 but was masked because the server never ran. FEAT-021 only changed `fugue-utils/src/lib.rs` (added a re-export), not client code.

### Trace Logging Results
Running with `FUGUE_LOG=trace` shows:
```
2026-01-09T16:19:41.599785Z  INFO fugue: fugue client starting
2026-01-09T16:19:41.600218Z TRACE mio::poll: registering event source with poller...
2026-01-09T16:19:41.600225Z TRACE mio::poll: registering event source with poller...
2026-01-09T16:19:41.600371Z TRACE mio::poll: registering event source with poller...
```
- No Tick events logged (should occur every 100ms)
- No Connected message handling logged
- No input events logged

### Root Cause Analysis
The input polling thread (`fugue-client/src/ui/event.rs:69`) uses `std::thread::spawn` with `crossterm::event::poll()` and sends to a tokio mpsc channel. The thread is either:
1. Not starting properly
2. `event::poll(tick_rate)` blocking/failing instead of timing out
3. `tx.send()` failing silently
4. Main event loop not receiving from channel

### Key Code Locations
| File | Line | Function |
|------|------|----------|
| `fugue-client/src/ui/event.rs` | 65-115 | `start_input_polling()` - spawns input thread |
| `fugue-client/src/ui/event.rs` | 72 | `event::poll(tick_rate)` - may be blocking |
| `fugue-client/src/ui/app.rs` | 130 | `events.next().await` - blocks waiting for events |
| `fugue-client/src/ui/terminal.rs` | 27 | `enable_raw_mode()` - terminal setup |

### Crossterm Configuration
- Version: crossterm 0.28
- No special features enabled (no `event-stream`)
- Using synchronous `event::poll()` and `event::read()` from std::thread

## Affected Features

- HA-001: Manual testing blocked
- FEAT-024: Session Selection UI cannot be verified
- All client functionality blocked

## Workaround

Kill client from another terminal:
```bash
pkill -f fugue
```

## Suggested Fix Approaches

1. Add debug logging to `start_input_polling()` to verify thread starts
2. Check if `event::poll()` returns error (currently swallowed by `unwrap_or(false)`)
3. Consider using crossterm's async `EventStream` with tokio instead of std::thread
4. Verify channel connectivity between std::thread and tokio runtime
