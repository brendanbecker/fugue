# FEAT-011: Client Connection - Unix Socket Client

**Priority**: P1
**Component**: ccmux-client
**Type**: new_feature
**Estimated Effort**: medium
**Business Value**: high
**Status**: completed

## Overview

Unix socket client connecting to ccmux-server, async message framing, connection state management, and reconnection logic.

## Requirements

### Unix Socket Client Using Tokio
- Connect to ccmux-server via Unix domain socket
- Use tokio's async runtime for non-blocking I/O
- Support custom socket paths for testing and configuration
- Check socket existence before connection attempts

### Async Message Framing with Protocol Codec
- Use tokio-util's Framed transport with ClientCodec
- Handle message serialization/deserialization via ccmux-protocol
- Support bidirectional message flow (ClientMessage/ServerMessage)
- Maintain message boundaries in the stream

### Connection State Management
- Track connection state: Disconnected, Connecting, Connected, Reconnecting
- Expose state for UI and monitoring
- State transitions on connect/disconnect events
- Prevent duplicate connection attempts when already connected

### Automatic Reconnection Logic with Backoff
- Detect connection loss (stream end, errors)
- Transition to Reconnecting state on connection loss
- Support configurable backoff strategy
- Resume message flow after reconnection

### Message Send/Receive Queuing
- Channel-based message queuing (mpsc)
- Non-blocking send with configurable buffer size
- Blocking and non-blocking receive options
- Fire-and-forget send option (send_nowait)

### Error Handling for Connection Failures
- Clear error types for different failure modes
- ServerNotRunning error when socket doesn't exist
- ConnectionClosed error when channel drops
- Graceful handling of send failures

## Current State

This feature is **completed**. The implementation includes:

### Connection Client (`ccmux-client/src/connection/client.rs`)
- `ConnectionState` enum with all four states
- `Connection` struct with socket path, state, channels, and task handle
- `connect()` async method with state transitions
- `disconnect()` async method with task cleanup
- `send()` and `recv()` methods for message flow
- `try_recv()` for non-blocking receive
- Background `connection_task` for socket I/O
- Comprehensive test coverage

### Message Handler (`ccmux-client/src/connection/handler.rs`)
- `MessageSender` clonable wrapper for outgoing channel
- `MessageHandler` trait for incoming message handling
- `CallbackHandler` implementation for simple callback-based handling
- Connection lifecycle callbacks (`on_connected`, `on_disconnected`)
- Comprehensive test coverage

## Affected Files

- `ccmux-client/src/connection/client.rs` - Core connection implementation
- `ccmux-client/src/connection/handler.rs` - Message handler trait and utilities

## Implementation Tasks

### Section 1: Design
- [x] Design connection state machine
- [x] Design message channel architecture
- [x] Plan error handling strategy
- [x] Document connection lifecycle

### Section 2: Core Implementation
- [x] Implement ConnectionState enum
- [x] Implement Connection struct
- [x] Implement connect() with state transitions
- [x] Implement disconnect() with cleanup
- [x] Implement connection_task for async I/O

### Section 3: Message Handling
- [x] Implement MessageSender clonable wrapper
- [x] Implement MessageHandler trait
- [x] Implement CallbackHandler
- [x] Add send/recv methods to Connection
- [x] Add try_recv for non-blocking receive

### Section 4: Error Handling
- [x] Define ServerNotRunning error
- [x] Define ConnectionClosed error
- [x] Handle connection failures gracefully
- [x] Handle channel send failures

### Section 5: Testing
- [x] Unit tests for connection states
- [x] Unit tests for MessageSender
- [x] Unit tests for CallbackHandler
- [x] Integration tests with mock server
- [x] Test connection lifecycle

### Section 6: Documentation
- [x] Document public API
- [x] Add code comments
- [x] Create feature documentation

## Acceptance Criteria

- [x] Client connects to Unix socket successfully
- [x] Messages are framed correctly with protocol codec
- [x] Connection state is tracked accurately
- [x] Connection errors are handled gracefully
- [x] Messages can be sent and received asynchronously
- [x] Multiple message senders can share the connection
- [x] Tests cover all connection states and transitions

## Dependencies

- FEAT-007: Protocol codec (provides ClientCodec, ClientMessage, ServerMessage)

## Notes

- Uses tokio-util's Framed for async message framing
- Channel buffer size is 100 messages (configurable in future)
- Reconnection logic framework is in place (Reconnecting state) but automatic reconnection is not yet implemented
- Fire-and-forget send (send_nowait) silently drops messages if channel is full
