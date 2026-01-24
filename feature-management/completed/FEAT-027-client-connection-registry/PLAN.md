# Implementation Plan: FEAT-027

**Work Item**: [FEAT-027: Client Connection Registry](PROMPT.md)
**Component**: fugue-server
**Priority**: P0 (Critical - FEAT-022 and FEAT-023 depend on it)
**Created**: 2026-01-09

## Overview

Implement a server-side registry that tracks connected clients and their session associations, enabling targeted message broadcasting.

## Architecture Decisions

### Concurrency Strategy

Use `DashMap` for the registry storage:
- Better concurrent read/write performance than `RwLock<HashMap>`
- No need to hold locks across async boundaries
- Built-in entry API for atomic operations

```rust
use dashmap::DashMap;

pub struct ClientRegistry {
    clients: DashMap<ClientId, ClientEntry>,
    session_clients: DashMap<SessionId, DashSet<ClientId>>,
    next_id: AtomicU64,
}
```

### Client ID Generation

Use atomic counter for client IDs:
- Simple and efficient
- Monotonically increasing (good for debugging)
- No external dependencies (vs UUID)

```rust
impl ClientRegistry {
    pub fn next_client_id(&self) -> ClientId {
        ClientId(self.next_id.fetch_add(1, Ordering::Relaxed))
    }
}
```

### Message Sender Type

Use bounded channels for backpressure:
```rust
pub type ClientSender = mpsc::Sender<ServerMessage>;

// In accept loop:
let (tx, rx) = mpsc::channel(32); // Buffer 32 messages
```

### Disconnected Client Handling

On send failure, mark client for removal:
```rust
pub async fn send_to_client(&self, client_id: ClientId, msg: ServerMessage) -> Result<(), SendError> {
    if let Some(entry) = self.clients.get(&client_id) {
        if entry.sender.send(msg).await.is_err() {
            // Channel closed, client disconnected
            drop(entry);
            self.unregister_client(client_id);
            return Err(SendError::ClientDisconnected);
        }
    }
    Ok(())
}
```

## Core Data Structures

```rust
use dashmap::{DashMap, DashSet};
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::mpsc;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ClientId(pub u64);

pub struct ClientEntry {
    pub sender: mpsc::Sender<ServerMessage>,
    pub attached_session: Option<SessionId>,
}

pub struct ClientRegistry {
    /// Map from client ID to client entry
    clients: DashMap<ClientId, ClientEntry>,
    /// Reverse index: session ID to set of attached client IDs
    session_clients: DashMap<SessionId, DashSet<ClientId>>,
    /// Counter for generating unique client IDs
    next_id: AtomicU64,
}
```

## API Design

```rust
impl ClientRegistry {
    /// Create a new empty registry
    pub fn new() -> Self;

    /// Generate a unique client ID
    pub fn next_client_id(&self) -> ClientId;

    /// Register a new client connection
    pub fn register_client(&self, client_id: ClientId, sender: mpsc::Sender<ServerMessage>);

    /// Unregister a client, cleaning up session associations
    pub fn unregister_client(&self, client_id: ClientId);

    /// Attach a client to a session
    pub fn attach_to_session(&self, client_id: ClientId, session_id: SessionId);

    /// Detach a client from its current session
    pub fn detach_from_session(&self, client_id: ClientId);

    /// Send a message to a specific client
    pub async fn send_to_client(&self, client_id: ClientId, msg: ServerMessage) -> Result<(), SendError>;

    /// Broadcast a message to all clients attached to a session
    pub async fn broadcast_to_session(&self, session_id: SessionId, msg: ServerMessage);

    /// Get the number of connected clients
    pub fn client_count(&self) -> usize;

    /// Get the session a client is attached to (if any)
    pub fn get_attached_session(&self, client_id: ClientId) -> Option<SessionId>;

    /// Get all client IDs attached to a session
    pub fn get_session_clients(&self, session_id: SessionId) -> Vec<ClientId>;
}
```

## Affected Components

| Component | Type of Change | Risk Level |
|-----------|----------------|------------|
| fugue-server/src/registry.rs | New file | Low |
| fugue-server/src/lib.rs | Add module export | Low |
| fugue-server/src/server.rs | Add registry field | Low |
| fugue-server/Cargo.toml | Add dashmap dependency | Low |

## Dependencies

- **FEAT-021** (Server Socket Listen Loop) - provides client connections

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Race between send and unregister | Low | Low | DashMap provides atomic operations |
| Memory leak from orphaned entries | Low | Medium | Clean up session_clients on unregister |
| Channel buffer exhaustion | Medium | Low | Use bounded channels with try_send fallback |

## Implementation Phases

### Phase 1: Core Structure (30 min)
- Add dashmap dependency
- Create registry.rs module
- Implement ClientId, ClientEntry, ClientRegistry structs
- Implement new(), next_client_id()

### Phase 2: Client Management (30 min)
- Implement register_client()
- Implement unregister_client() with session cleanup
- Implement client_count()
- Add unit tests for registration

### Phase 3: Session Association (30 min)
- Implement attach_to_session() with reverse index update
- Implement detach_from_session()
- Handle re-attachment case
- Add unit tests for session operations

### Phase 4: Message Delivery (30 min)
- Implement send_to_client() with disconnect handling
- Implement broadcast_to_session()
- Add unit tests for message delivery
- Test concurrent access

## Test Strategy

### Unit Tests

```rust
#[tokio::test]
async fn test_register_and_unregister() {
    let registry = ClientRegistry::new();
    let (tx, _rx) = mpsc::channel(1);
    let client_id = registry.next_client_id();

    registry.register_client(client_id, tx);
    assert_eq!(registry.client_count(), 1);

    registry.unregister_client(client_id);
    assert_eq!(registry.client_count(), 0);
}

#[tokio::test]
async fn test_session_attach_detach() {
    let registry = ClientRegistry::new();
    let (tx, _rx) = mpsc::channel(1);
    let client_id = registry.next_client_id();
    let session_id = SessionId::new();

    registry.register_client(client_id, tx);
    registry.attach_to_session(client_id, session_id);

    assert_eq!(registry.get_attached_session(client_id), Some(session_id));
    assert!(registry.get_session_clients(session_id).contains(&client_id));

    registry.detach_from_session(client_id);
    assert_eq!(registry.get_attached_session(client_id), None);
}

#[tokio::test]
async fn test_broadcast_to_session() {
    let registry = ClientRegistry::new();
    let session_id = SessionId::new();

    // Create 3 clients attached to same session
    let mut receivers = Vec::new();
    for _ in 0..3 {
        let (tx, rx) = mpsc::channel(1);
        let client_id = registry.next_client_id();
        registry.register_client(client_id, tx);
        registry.attach_to_session(client_id, session_id);
        receivers.push(rx);
    }

    // Broadcast
    let msg = ServerMessage::Ping;
    registry.broadcast_to_session(session_id, msg.clone()).await;

    // All receivers should get the message
    for mut rx in receivers {
        assert!(rx.try_recv().is_ok());
    }
}

#[tokio::test]
async fn test_disconnected_client_cleanup() {
    let registry = ClientRegistry::new();
    let (tx, rx) = mpsc::channel(1);
    let client_id = registry.next_client_id();

    registry.register_client(client_id, tx);
    drop(rx); // Simulate disconnect

    // Send should fail and trigger cleanup
    let result = registry.send_to_client(client_id, ServerMessage::Ping).await;
    assert!(result.is_err());
    assert_eq!(registry.client_count(), 0);
}
```

### Concurrency Tests

```rust
#[tokio::test]
async fn test_concurrent_registration() {
    let registry = Arc::new(ClientRegistry::new());
    let mut handles = Vec::new();

    for _ in 0..100 {
        let reg = Arc::clone(&registry);
        handles.push(tokio::spawn(async move {
            let (tx, _rx) = mpsc::channel(1);
            let client_id = reg.next_client_id();
            reg.register_client(client_id, tx);
        }));
    }

    for h in handles {
        h.await.unwrap();
    }

    assert_eq!(registry.client_count(), 100);
}
```

## Rollback Strategy

If implementation causes issues:
1. Revert commits associated with this work item
2. FEAT-022 and FEAT-023 will be blocked (expected)
3. No runtime impact on existing functionality

## Implementation Notes

<!-- Add notes during implementation -->

---
*This plan should be updated as implementation progresses.*
