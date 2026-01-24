# Implementation Plan: FEAT-021

**Work Item**: [FEAT-021: Server Socket Listen Loop](PROMPT.md)
**Component**: fugue-server
**Priority**: P0 (Critical - blocks everything)
**Created**: 2026-01-09

## Overview

Implement the main server event loop that listens for client connections on a Unix socket. This is the critical foundation that enables all client-server communication.

## Architecture Decisions

### Socket Location

Use `fugue_utils::socket_path()` for consistent socket location:
- Default: `$XDG_RUNTIME_DIR/fugue/fugue.sock` or `~/.fugue/fugue.sock`
- Ensures client and server use the same path

### Accept Loop Pattern

```rust
async fn run_accept_loop(listener: UnixListener, server: Arc<Server>) {
    loop {
        tokio::select! {
            result = listener.accept() => {
                match result {
                    Ok((stream, _addr)) => {
                        let server = Arc::clone(&server);
                        tokio::spawn(async move {
                            handle_client(stream, server).await;
                        });
                    }
                    Err(e) => {
                        tracing::error!("Accept error: {}", e);
                    }
                }
            }
            _ = server.shutdown_signal() => {
                tracing::info!("Shutdown signal received, stopping accept loop");
                break;
            }
        }
    }
}
```

### Per-Client Handler Pattern

```rust
async fn handle_client(stream: UnixStream, server: Arc<Server>) {
    let (reader, writer) = stream.into_split();
    let mut framed_reader = FramedRead::new(reader, MessageCodec::new());
    let mut framed_writer = FramedWrite::new(writer, MessageCodec::new());

    // Register client with server
    let client_id = server.register_client().await;

    loop {
        tokio::select! {
            result = framed_reader.next() => {
                match result {
                    Some(Ok(msg)) => {
                        // Handle message, send response
                    }
                    Some(Err(e)) => {
                        tracing::error!("Client {} read error: {}", client_id, e);
                        break;
                    }
                    None => {
                        tracing::info!("Client {} disconnected", client_id);
                        break;
                    }
                }
            }
            // Handle outbound messages from server to client
        }
    }

    // Cleanup
    server.unregister_client(client_id).await;
}
```

### Socket Cleanup Strategy

1. **On startup**: Check if socket file exists
   - If exists, try to connect to verify if server is running
   - If connection fails, remove stale socket (previous crash)
   - If connection succeeds, exit with "server already running" error

2. **On shutdown**: Remove socket file
   - Handle SIGTERM/SIGINT signals
   - Use tokio::signal for async signal handling
   - Ensure cleanup runs even on error paths

## Affected Components

| Component | Type of Change | Risk Level |
|-----------|----------------|------------|
| fugue-server/src/main.rs | Major - Add listen loop | Medium |
| fugue-server/src/server.rs | Modify - Add client tracking | Low |
| fugue-server/src/client.rs | New - Client handler task | Low |

## Dependencies

None - this feature has no dependencies and unblocks all other features.

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Stale socket prevents startup | Medium | High | Auto-cleanup of stale sockets |
| Client panic crashes server | Low | High | Catch panics in spawned tasks |
| Resource leak on disconnect | Low | Medium | Explicit cleanup in drop/finally |
| Permission errors on socket | Low | Medium | Clear error messages, socket_path() validation |

## Implementation Phases

### Phase 1: Socket Setup (1 hour)
- Implement socket path validation
- Add stale socket cleanup logic
- Bind UnixListener with error handling
- Add socket file permission setting

### Phase 2: Accept Loop (1.5 hours)
- Implement main accept loop
- Add shutdown signal handling
- Spawn client handler tasks
- Add client connection logging

### Phase 3: Client Handler (1.5 hours)
- Implement per-client message pump
- Integrate with MessageCodec from fugue-protocol
- Handle message routing (stub for now)
- Implement disconnect detection and cleanup

### Phase 4: Testing (1-2 hours)
- Unit tests for socket operations
- Integration tests for full flow
- Concurrent client testing
- Shutdown and crash recovery testing

## Rollback Strategy

If implementation causes issues:
1. Revert commits associated with this work item
2. Server will return to non-functional state (expected)
3. No other features depend on this yet

## Test Strategy

### Unit Tests
```rust
#[tokio::test]
async fn test_socket_creation() {
    let socket_path = temp_socket_path();
    let listener = create_listener(&socket_path).await.unwrap();
    assert!(socket_path.exists());
}

#[tokio::test]
async fn test_stale_socket_cleanup() {
    let socket_path = temp_socket_path();
    std::fs::write(&socket_path, "stale").unwrap();
    let listener = create_listener(&socket_path).await.unwrap();
    // Should succeed after cleaning stale socket
}
```

### Integration Tests
```rust
#[tokio::test]
async fn test_client_connect_disconnect() {
    let server = start_test_server().await;
    let client = connect_client().await.unwrap();
    drop(client);
    // Verify server handled disconnect cleanly
}

#[tokio::test]
async fn test_multiple_concurrent_clients() {
    let server = start_test_server().await;
    let clients: Vec<_> = (0..10)
        .map(|_| connect_client())
        .collect();
    // All should connect successfully
}
```

## Implementation Notes

<!-- Add notes during implementation -->

---
*This plan should be updated as implementation progresses.*
