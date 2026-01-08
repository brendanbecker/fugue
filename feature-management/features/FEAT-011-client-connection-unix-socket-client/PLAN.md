# Implementation Plan: FEAT-011

**Work Item**: [FEAT-011: Client Connection - Unix Socket Client](PROMPT.md)
**Component**: ccmux-client
**Priority**: P1
**Created**: 2026-01-08
**Status**: Completed

## Overview

Unix socket client connecting to ccmux-server, async message framing, connection state management, and reconnection logic.

## Architecture Decisions

### Connection State Machine

The connection uses a four-state model:

```
Disconnected --> Connecting --> Connected --> Disconnected
                     |              |
                     v              v
               Disconnected    Reconnecting --> Connected
                                    |
                                    v
                              Disconnected
```

State transitions:
- `Disconnected -> Connecting`: On `connect()` call
- `Connecting -> Connected`: On successful socket connection
- `Connecting -> Disconnected`: On connection failure
- `Connected -> Disconnected`: On `disconnect()` call or fatal error
- `Connected -> Reconnecting`: On recoverable connection loss (future)
- `Reconnecting -> Connected`: On successful reconnection (future)
- `Reconnecting -> Disconnected`: On reconnection failure (future)

### Message Channel Architecture

```
                 +------------------+
                 |   Connection     |
                 +------------------+
                        |
          +-------------+-------------+
          |                           |
   outgoing_tx/rx              incoming_tx/rx
          |                           |
          v                           v
+------------------+       +------------------+
| connection_task  | <---> |   UnixStream     |
| (background)     |       |   + ClientCodec  |
+------------------+       +------------------+
```

- Outgoing: User sends to `tx`, task receives from `rx` and writes to socket
- Incoming: Task reads from socket, sends to `tx`, user receives from `rx`
- Channel buffer: 100 messages (prevents blocking on burst traffic)

### Framed Transport

Uses `tokio_util::codec::Framed` with `ClientCodec`:
- Handles length-prefixed message framing
- Serializes `ClientMessage` to bytes on send
- Deserializes bytes to `ServerMessage` on receive
- Automatic buffer management

### Error Handling Strategy

| Error Type | Cause | Recovery |
|------------|-------|----------|
| ServerNotRunning | Socket file doesn't exist | User must start server |
| Connection(String) | Socket connect failed | User retry or check config |
| ConnectionClosed | Channel dropped | Reconnect or exit |

## Affected Components

| Component | Type of Change | Risk Level |
|-----------|----------------|------------|
| ccmux-client/src/connection/client.rs | New file | Low |
| ccmux-client/src/connection/handler.rs | New file | Low |
| ccmux-client/src/connection/mod.rs | Module declaration | Low |

## Dependencies

- `ccmux-protocol`: Provides `ClientCodec`, `ClientMessage`, `ServerMessage`
- `ccmux-utils`: Provides `socket_path()`, `CcmuxError`, `Result`
- `tokio`: Async runtime, `UnixStream`, `mpsc` channels
- `tokio-util`: `Framed` transport, `codec` support
- `futures`: `SinkExt`, `StreamExt` for async stream operations

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Message loss on disconnect | Medium | Medium | Buffer messages during reconnect |
| Channel backpressure | Low | Medium | Configurable buffer size, backpressure handling |
| Task leak on error | Low | Low | Proper task abort on disconnect |
| Race conditions | Low | Medium | Clear state transitions, atomic operations |

## Implementation Phases

### Phase 1: Core Connection (Completed)
- ConnectionState enum
- Connection struct with channels
- connect() and disconnect() methods
- connection_task background worker

### Phase 2: Message Handling (Completed)
- MessageSender clonable wrapper
- MessageHandler trait
- CallbackHandler implementation
- send/recv/try_recv methods

### Phase 3: Error Handling (Completed)
- ServerNotRunning error
- ConnectionClosed error
- Graceful failure handling

### Phase 4: Testing (Completed)
- Unit tests for all components
- Integration tests with mock server
- State transition coverage

### Future: Automatic Reconnection
- Implement Reconnecting state behavior
- Add exponential backoff
- Message buffering during reconnect

## Rollback Strategy

If implementation causes issues:
1. Revert commits for ccmux-client/src/connection/
2. Remove connection module from ccmux-client/src/lib.rs
3. Client applications use direct socket communication
4. Document what went wrong in comments.md

## Testing Strategy

1. **Unit Tests**: State transitions, channel operations, error cases
2. **Integration Tests**: Mock UnixListener, full connect/send/receive cycle
3. **Edge Cases**: Double connect, send when disconnected, channel closure

## Implementation Notes

Implementation is complete with comprehensive test coverage. Key implementation details:

- `Connection::new()` creates disconnected connection with default socket path
- `Connection::with_socket_path()` allows custom socket path for testing
- `connect()` checks socket existence before attempting connection
- `connection_task` uses `tokio::select!` for concurrent send/receive
- `MessageSender` is clonable for multi-producer scenarios
- `send_nowait()` uses `try_send()` for non-blocking fire-and-forget

---
*Plan completed - implementation done.*
