# Task Breakdown: FEAT-027

**Work Item**: [FEAT-027: Client Connection Registry](PROMPT.md)
**Status**: Not Started
**Last Updated**: 2026-01-09

## Prerequisites

- [ ] Read and understand PROMPT.md
- [ ] Review PLAN.md and update if needed
- [ ] Verify FEAT-021 provides client connection context
- [ ] Review existing fugue-server structure
- [ ] Check for existing SessionId type definition
- [ ] Review fugue-protocol ServerMessage type

## Design Tasks

- [ ] Confirm ClientId type (u64 counter vs UUID)
- [ ] Confirm channel buffer size for client senders
- [ ] Review DashMap vs RwLock tradeoffs
- [ ] Design error types for send failures
- [ ] Plan integration with FEAT-021 accept loop

## Implementation Tasks

### Module Setup (fugue-server/Cargo.toml, src/lib.rs)

- [ ] Add dashmap dependency to Cargo.toml
- [ ] Create registry.rs module file
- [ ] Export registry module from lib.rs or main.rs
- [ ] Add necessary imports (tokio, dashmap, etc.)

### Core Types (fugue-server/src/registry.rs)

- [ ] Define `ClientId` newtype with derive macros
- [ ] Define `ClientEntry` struct
- [ ] Define `SendError` enum for send failures
- [ ] Implement Display/Debug for ClientId

### ClientRegistry Struct (fugue-server/src/registry.rs)

- [ ] Create `ClientRegistry` struct with DashMap fields
- [ ] Add AtomicU64 for client ID generation
- [ ] Implement `new()` constructor
- [ ] Implement `next_client_id()` method

### Client Management Methods

- [ ] Implement `register_client(client_id, sender)`
- [ ] Implement `unregister_client(client_id)` with session cleanup
- [ ] Implement `get_client(client_id)` -> Option<ClientEntry>
- [ ] Implement `client_count()` -> usize

### Session Association Methods

- [ ] Implement `attach_to_session(client_id, session_id)`
- [ ] Update session_clients reverse index on attach
- [ ] Implement `detach_from_session(client_id)`
- [ ] Update session_clients reverse index on detach
- [ ] Handle re-attachment (detach old, attach new)

### Message Delivery Methods

- [ ] Implement `send_to_client(client_id, msg)` -> Result
- [ ] Handle channel closed error (cleanup client)
- [ ] Implement `broadcast_to_session(session_id, msg)`
- [ ] Handle mixed success/failure in broadcast
- [ ] Consider try_send for non-blocking variant

### Query Methods

- [ ] Implement `get_attached_session(client_id)` -> Option<SessionId>
- [ ] Implement `get_session_clients(session_id)` -> Vec<ClientId>
- [ ] Implement `is_client_attached(client_id)` -> bool

### Server Integration (fugue-server/src/server.rs)

- [ ] Add `registry: ClientRegistry` field to Server struct
- [ ] Initialize registry in Server::new()
- [ ] Add accessor method for registry
- [ ] Integrate with FEAT-021 accept loop (when ready)

## Testing Tasks

### Unit Tests

- [ ] Test client registration
- [ ] Test client unregistration
- [ ] Test duplicate registration handling
- [ ] Test unregister non-existent client
- [ ] Test session attach
- [ ] Test session detach
- [ ] Test re-attachment to different session
- [ ] Test attach non-existent client
- [ ] Test send to client
- [ ] Test send to non-existent client
- [ ] Test send to disconnected client (cleanup)
- [ ] Test broadcast to session
- [ ] Test broadcast to empty session
- [ ] Test client_count accuracy

### Concurrency Tests

- [ ] Test concurrent client registration
- [ ] Test concurrent client unregistration
- [ ] Test concurrent attach/detach
- [ ] Test concurrent send/unregister race
- [ ] Test broadcast during client disconnect

### Edge Case Tests

- [ ] Test with 0 clients
- [ ] Test with 1000+ clients
- [ ] Test session with 100+ clients
- [ ] Test rapid attach/detach cycles

## Documentation Tasks

- [ ] Add doc comments to ClientRegistry
- [ ] Add doc comments to all public methods
- [ ] Document thread-safety guarantees
- [ ] Add usage examples in doc comments

## Verification Tasks

- [ ] All acceptance criteria from PROMPT.md met
- [ ] All tests passing
- [ ] No clippy warnings
- [ ] cargo fmt clean
- [ ] Update feature_request.json status
- [ ] Document any deviations in PLAN.md

## Completion Checklist

- [ ] All implementation tasks complete
- [ ] All tests passing
- [ ] Documentation complete
- [ ] PLAN.md reflects final implementation
- [ ] Ready for review/merge

---
*Check off tasks as you complete them. Update status field above.*
