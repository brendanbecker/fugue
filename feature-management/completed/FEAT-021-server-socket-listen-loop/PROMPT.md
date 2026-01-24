# FEAT-021: Server Socket Listen Loop

**Priority**: P0 (Critical - blocks everything)
**Component**: fugue-server
**Type**: new_feature
**Estimated Effort**: large (4-6 hours)
**Business Value**: high
**Status**: new

## Overview

Implement the main server event loop that listens for client connections on a Unix socket. This is the critical foundation that enables all client-server communication and unblocks all downstream features.

## Requirements

1. Create Unix socket at the path returned by `fugue_utils::socket_path()`
2. Set up `UnixListener::bind()` in server startup
3. Implement accept loop with `tokio::spawn` for each client
4. Create per-client message pump task
5. Handle client disconnection cleanup
6. Error handling (socket already exists, permission denied)

## Location

Primary implementation target: `/home/becker/projects/tools/fugue/fugue-server/src/main.rs` lines 316-326

## Technical Notes

- Pattern already exists in tests (client.rs lines 215-230)
- Uses tokio async runtime (already in place)
- Must integrate with existing `Server` struct
- Socket path comes from `fugue_utils::socket_path()`

## Affected Files

- `fugue-server/src/main.rs` - Main listen loop implementation
- `fugue-server/src/server.rs` - Server struct integration (if exists)
- `fugue-server/src/client.rs` - Per-client handler task

## Implementation Tasks

### Section 1: Socket Setup
- [ ] Remove existing socket file if present (stale from crash)
- [ ] Create parent directories for socket path if needed
- [ ] Bind UnixListener to socket path
- [ ] Set appropriate socket permissions
- [ ] Handle bind errors (already in use, permission denied)

### Section 2: Accept Loop
- [ ] Implement async accept loop in main server task
- [ ] Spawn new tokio task for each accepted connection
- [ ] Pass connection stream to client handler
- [ ] Log new client connections
- [ ] Track active client count

### Section 3: Per-Client Message Pump
- [ ] Create client handler task structure
- [ ] Read framed messages from client stream
- [ ] Route messages to appropriate handlers
- [ ] Write response messages back to client
- [ ] Handle client stream errors gracefully

### Section 4: Client Disconnection
- [ ] Detect client disconnect (EOF or error)
- [ ] Clean up client-specific resources
- [ ] Update active client tracking
- [ ] Log disconnection events
- [ ] Handle partial message on disconnect

### Section 5: Shutdown Handling
- [ ] Listen for shutdown signal in accept loop
- [ ] Stop accepting new connections on shutdown
- [ ] Wait for active clients to finish (with timeout)
- [ ] Remove socket file on clean shutdown
- [ ] Handle forceful shutdown (SIGTERM/SIGINT)

### Section 6: Testing
- [ ] Unit test: Socket creation and binding
- [ ] Unit test: Client accept and spawn
- [ ] Integration test: Single client connect/disconnect
- [ ] Integration test: Multiple concurrent clients
- [ ] Integration test: Graceful shutdown
- [ ] Integration test: Crash recovery (stale socket cleanup)

## Acceptance Criteria

- [ ] Server starts and binds to Unix socket successfully
- [ ] Multiple clients can connect concurrently
- [ ] Client disconnection is handled cleanly
- [ ] Clean shutdown removes socket file
- [ ] Connection errors are logged appropriately
- [ ] Stale socket file from previous crash is cleaned up
- [ ] All tests passing

## Dependencies

None - this feature unblocks everything else.

## Notes

- This is the foundational feature that enables all client-server communication
- Pattern from test code can be adapted for production use
- Consider connection rate limiting for production hardening
- Socket permissions should prevent unauthorized access
- Integration with existing Server struct is key design consideration
