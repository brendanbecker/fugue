# FEAT-027: Client Connection Registry

**Priority**: P0 (Critical - FEAT-022 and FEAT-023 depend on it)
**Component**: fugue-server
**Type**: new_feature
**Estimated Effort**: small (1-2 hours)
**Business Value**: high
**Status**: new

## Overview

Implement a server-side registry that tracks connected clients and their session associations, enabling targeted message broadcasting. This is a critical component that bridges FEAT-021 (Server Socket) with FEAT-022 (Message Routing) and FEAT-023 (PTY Output Broadcasting).

## Requirements

1. Track all connected clients with unique client IDs
2. Track which session each client is attached to (if any)
3. Support multiple clients attached to the same session
4. Provide methods:
   - `register_client(client_id, sender)` - add new connection
   - `unregister_client(client_id)` - remove on disconnect
   - `attach_to_session(client_id, session_id)` - associate client with session
   - `detach_from_session(client_id)` - remove association
   - `broadcast_to_session(session_id, message)` - send to all clients in session
   - `send_to_client(client_id, message)` - send to specific client
5. Thread-safe for concurrent access from multiple client handler tasks

## Location

Primary implementation target: `/home/becker/projects/tools/fugue/fugue-server/src/registry.rs`

Alternative: `/home/becker/projects/tools/fugue/fugue-server/src/connection/registry.rs` (if using a connection module)

## Technical Notes

- Use `tokio::sync::mpsc::Sender<ServerMessage>` for per-client channels
- Consider `DashMap` or `RwLock<HashMap>` for concurrent access
- Client ID could be UUID or connection-assigned integer (recommend u64 counter with AtomicU64)
- Must handle client disconnect cleanup gracefully
- Sender channel errors indicate disconnected clients - remove from registry

## Data Structures

```rust
pub struct ClientEntry {
    pub sender: mpsc::Sender<ServerMessage>,
    pub attached_session: Option<SessionId>,
}

pub struct ClientRegistry {
    clients: DashMap<ClientId, ClientEntry>,
    // Reverse index for efficient session broadcast
    session_clients: DashMap<SessionId, HashSet<ClientId>>,
    next_client_id: AtomicU64,
}
```

## Affected Files

- `fugue-server/src/registry.rs` - New file with registry implementation
- `fugue-server/src/lib.rs` or `main.rs` - Export registry module
- `fugue-server/src/server.rs` - Integrate registry with Server struct

## Implementation Tasks

### Section 1: Core Registry Structure
- [ ] Create `ClientId` newtype (wrapping u64)
- [ ] Create `ClientEntry` struct with sender and session
- [ ] Create `ClientRegistry` struct with DashMap storage
- [ ] Implement atomic client ID generation

### Section 2: Client Management
- [ ] Implement `register_client()` - add client to registry
- [ ] Implement `unregister_client()` - remove client and clean up session associations
- [ ] Implement `get_client()` - get client entry by ID
- [ ] Implement `client_count()` - return number of connected clients

### Section 3: Session Association
- [ ] Implement `attach_to_session()` - associate client with session
- [ ] Implement `detach_from_session()` - remove session association
- [ ] Maintain reverse index (session -> clients) for efficient broadcast
- [ ] Handle re-attachment (detach from old session first)

### Section 4: Message Delivery
- [ ] Implement `send_to_client()` - send message to specific client
- [ ] Implement `broadcast_to_session()` - send to all clients in a session
- [ ] Handle sender errors (channel closed = client disconnected)
- [ ] Auto-remove disconnected clients on send failure

### Section 5: Server Integration
- [ ] Add `ClientRegistry` to `Server` struct
- [ ] Wire up client registration in accept loop
- [ ] Wire up client unregistration on disconnect
- [ ] Expose registry methods through Server

### Section 6: Testing
- [ ] Unit test: Client registration and unregistration
- [ ] Unit test: Session attach and detach
- [ ] Unit test: Send to specific client
- [ ] Unit test: Broadcast to session
- [ ] Unit test: Concurrent access from multiple tasks
- [ ] Unit test: Disconnected client cleanup

## Acceptance Criteria

- [ ] Clients can be registered and unregistered
- [ ] Clients can attach/detach from sessions
- [ ] Broadcasting reaches all clients in a session
- [ ] No memory leaks on client disconnect
- [ ] Thread-safe under concurrent access
- [ ] Unit tests for all registry operations
- [ ] Sender errors trigger automatic client cleanup

## Dependencies

- **FEAT-021** (Server Socket Listen Loop) - provides the client connections to register

## Depended On By

- **FEAT-022** (Message Routing) - needs registry to send responses to correct client
- **FEAT-023** (PTY Output Broadcasting) - needs registry to broadcast output to session clients

## Notes

- DashMap provides better concurrency than RwLock<HashMap> for this use case
- Consider using `try_send()` vs `send()` for non-blocking message delivery
- The registry is the central coordination point for all client communication
- Session ID should match the `SessionId` type from fugue-server session management
- Consider adding metrics (client count, session counts) for observability
