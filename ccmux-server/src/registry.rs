//! Client Connection Registry
//!
//! Tracks connected clients and their session associations, enabling
//! targeted message broadcasting. This bridges the server socket (FEAT-021)
//! with message routing (FEAT-022) and PTY output broadcasting (FEAT-023).

use std::collections::HashSet;
use std::sync::atomic::{AtomicU64, Ordering};

use dashmap::DashMap;
use tokio::sync::mpsc;
use tracing::{debug, info, warn};
use uuid::Uuid;

use ccmux_protocol::ServerMessage;

/// Type alias for session IDs (matches the Uuid type used in session module)
pub type SessionId = Uuid;

/// Unique client identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ClientId(u64);

impl ClientId {
    /// Create a new ClientId from a raw value (mainly for testing)
    #[cfg(test)]
    pub fn new(value: u64) -> Self {
        Self(value)
    }

    /// Get the raw value
    pub fn value(&self) -> u64 {
        self.0
    }
}

impl std::fmt::Display for ClientId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Client({})", self.0)
    }
}

/// Entry for a connected client
pub struct ClientEntry {
    /// Channel for sending messages to this client
    pub sender: mpsc::Sender<ServerMessage>,
    /// Session this client is attached to (if any)
    pub attached_session: Option<SessionId>,
}

impl std::fmt::Debug for ClientEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClientEntry")
            .field("attached_session", &self.attached_session)
            .field("sender_closed", &self.sender.is_closed())
            .finish()
    }
}

/// Registry tracking all connected clients
///
/// Thread-safe for concurrent access from multiple client handler tasks.
pub struct ClientRegistry {
    /// Client ID -> Client entry
    clients: DashMap<ClientId, ClientEntry>,
    /// Session ID -> Set of client IDs (reverse index for efficient broadcast)
    session_clients: DashMap<SessionId, HashSet<ClientId>>,
    /// Counter for generating unique client IDs
    next_client_id: AtomicU64,
}

impl Default for ClientRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ClientRegistry {
    /// Create a new empty client registry
    pub fn new() -> Self {
        Self {
            clients: DashMap::new(),
            session_clients: DashMap::new(),
            next_client_id: AtomicU64::new(1),
        }
    }

    // ==================== Client Management ====================

    /// Register a new client connection
    ///
    /// Returns the assigned ClientId for this connection.
    pub fn register_client(&self, sender: mpsc::Sender<ServerMessage>) -> ClientId {
        let id = ClientId(self.next_client_id.fetch_add(1, Ordering::SeqCst));

        let entry = ClientEntry {
            sender,
            attached_session: None,
        };

        self.clients.insert(id, entry);
        debug!("Registered client {}", id);

        id
    }

    /// Unregister a client connection
    ///
    /// Removes the client from the registry and cleans up session associations.
    pub fn unregister_client(&self, client_id: ClientId) {
        // First, remove from any session it's attached to
        if let Some((_, entry)) = self.clients.remove(&client_id) {
            if let Some(session_id) = entry.attached_session {
                self.remove_client_from_session_index(client_id, session_id);
            }
            debug!("Unregistered client {}", client_id);
        }
    }

    /// Get a reference to a client entry
    ///
    /// Returns None if the client doesn't exist.
    pub fn get_client(&self, client_id: ClientId) -> Option<dashmap::mapref::one::Ref<'_, ClientId, ClientEntry>> {
        self.clients.get(&client_id)
    }

    /// Get the number of connected clients
    pub fn client_count(&self) -> usize {
        self.clients.len()
    }

    // ==================== Session Association ====================

    /// Attach a client to a session
    ///
    /// If the client is already attached to another session, it will be
    /// detached from that session first.
    ///
    /// Returns `true` if the client exists and was attached, `false` if
    /// the client doesn't exist.
    pub fn attach_to_session(&self, client_id: ClientId, session_id: SessionId) -> bool {
        // Get mutable access to the client entry
        let mut entry = match self.clients.get_mut(&client_id) {
            Some(entry) => entry,
            None => return false,
        };

        // Detach from old session if attached
        if let Some(old_session_id) = entry.attached_session {
            if old_session_id != session_id {
                self.remove_client_from_session_index(client_id, old_session_id);
            }
        }

        // Update client's attached session
        entry.attached_session = Some(session_id);

        // Add to session's client set
        self.session_clients
            .entry(session_id)
            .or_insert_with(HashSet::new)
            .insert(client_id);

        debug!("Client {} attached to session {}", client_id, session_id);
        true
    }

    /// Detach a client from its current session
    ///
    /// Returns `true` if the client was attached and is now detached,
    /// `false` if the client doesn't exist or wasn't attached.
    pub fn detach_from_session(&self, client_id: ClientId) -> bool {
        // Get mutable access to the client entry
        let mut entry = match self.clients.get_mut(&client_id) {
            Some(entry) => entry,
            None => return false,
        };

        // Get the session ID and clear it
        let session_id = match entry.attached_session.take() {
            Some(id) => id,
            None => return false,
        };

        // Remove from session's client set
        self.remove_client_from_session_index(client_id, session_id);

        debug!("Client {} detached from session {}", client_id, session_id);
        true
    }

    /// Get the session a client is attached to
    pub fn get_client_session(&self, client_id: ClientId) -> Option<SessionId> {
        self.clients.get(&client_id)?.attached_session
    }

    /// Get the number of clients attached to a session
    pub fn session_client_count(&self, session_id: SessionId) -> usize {
        self.session_clients
            .get(&session_id)
            .map(|set| set.len())
            .unwrap_or(0)
    }

    /// Helper to remove a client from the session index
    fn remove_client_from_session_index(&self, client_id: ClientId, session_id: SessionId) {
        // Remove from session's client set
        if let Some(mut clients) = self.session_clients.get_mut(&session_id) {
            clients.remove(&client_id);
            // Clean up empty sets
            if clients.is_empty() {
                drop(clients); // Release the lock before removing
                self.session_clients.remove(&session_id);
            }
        }
    }

    // ==================== Message Delivery ====================

    /// Send a message to a specific client
    ///
    /// Returns `true` if the message was sent successfully, `false` if
    /// the client doesn't exist or the channel is closed.
    ///
    /// If the channel is closed (client disconnected), the client will
    /// be automatically unregistered.
    pub async fn send_to_client(&self, client_id: ClientId, message: ServerMessage) -> bool {
        // Get the sender - we need to clone it to avoid holding the lock during send
        let sender = match self.clients.get(&client_id) {
            Some(entry) => entry.sender.clone(),
            None => return false,
        };

        // Try to send the message
        match sender.send(message).await {
            Ok(()) => true,
            Err(_) => {
                // Channel closed - client disconnected
                warn!("Client {} channel closed, removing from registry", client_id);
                self.unregister_client(client_id);
                false
            }
        }
    }

    /// Send a message to a specific client (non-blocking)
    ///
    /// Uses `try_send` for non-blocking delivery. Returns `true` if the
    /// message was queued successfully, `false` if the client doesn't
    /// exist, the channel is closed, or the channel is full.
    ///
    /// If the channel is closed (client disconnected), the client will
    /// be automatically unregistered.
    pub fn try_send_to_client(&self, client_id: ClientId, message: ServerMessage) -> bool {
        // Get the sender
        let sender = match self.clients.get(&client_id) {
            Some(entry) => entry.sender.clone(),
            None => return false,
        };

        // Try to send the message
        match sender.try_send(message) {
            Ok(()) => true,
            Err(mpsc::error::TrySendError::Closed(_)) => {
                // Channel closed - client disconnected
                warn!("Client {} channel closed, removing from registry", client_id);
                self.unregister_client(client_id);
                false
            }
            Err(mpsc::error::TrySendError::Full(_)) => {
                // Channel full - client slow to consume
                warn!("Client {} channel full, message dropped", client_id);
                false
            }
        }
    }

    /// Broadcast a message to all clients attached to a session
    ///
    /// Returns the number of clients that successfully received the message.
    ///
    /// Clients with closed channels (disconnected) will be automatically
    /// unregistered.
    pub async fn broadcast_to_session(&self, session_id: SessionId, message: ServerMessage) -> usize {
        // Get the list of client IDs for this session
        let client_ids: Vec<ClientId> = match self.session_clients.get(&session_id) {
            Some(clients) => clients.iter().copied().collect(),
            None => return 0,
        };

        if client_ids.is_empty() {
            return 0;
        }

        debug!(
            "Broadcasting to {} clients in session {}",
            client_ids.len(),
            session_id
        );

        let mut success_count = 0;

        for client_id in client_ids {
            if self.send_to_client(client_id, message.clone()).await {
                success_count += 1;
            }
        }

        success_count
    }

    /// Broadcast a message to all clients attached to a session (non-blocking)
    ///
    /// Uses `try_send` for non-blocking delivery. Returns the number of
    /// clients that successfully received the message.
    ///
    /// Clients with closed channels (disconnected) will be automatically
    /// unregistered.
    pub fn try_broadcast_to_session(&self, session_id: SessionId, message: ServerMessage) -> usize {
        // Get the list of client IDs for this session
        let client_ids: Vec<ClientId> = match self.session_clients.get(&session_id) {
            Some(clients) => clients.iter().copied().collect(),
            None => return 0,
        };

        if client_ids.is_empty() {
            return 0;
        }

        debug!(
            "Broadcasting (try_send) to {} clients in session {}",
            client_ids.len(),
            session_id
        );

        let mut success_count = 0;

        for client_id in client_ids {
            if self.try_send_to_client(client_id, message.clone()) {
                success_count += 1;
            }
        }

        success_count
    }

    /// Broadcast a message to all clients attached to a session except one
    ///
    /// Useful for broadcasting state changes to other clients without
    /// sending back to the originator.
    ///
    /// Returns the number of clients that successfully received the message.
    pub async fn broadcast_to_session_except(
        &self,
        session_id: SessionId,
        except_client: ClientId,
        message: ServerMessage,
    ) -> usize {
        // Log all clients attached to this session for debugging
        let all_session_clients: Vec<ClientId> = self.session_clients
            .get(&session_id)
            .map(|clients| clients.iter().copied().collect())
            .unwrap_or_default();

        info!(
            "broadcast_to_session_except: session={}, except={}, all_clients_in_session={:?}",
            session_id, except_client, all_session_clients
        );

        // Get the list of client IDs for this session
        let client_ids: Vec<ClientId> = match self.session_clients.get(&session_id) {
            Some(clients) => clients
                .iter()
                .copied()
                .filter(|&id| id != except_client)
                .collect(),
            None => {
                warn!(
                    "broadcast_to_session_except: No clients found for session {}",
                    session_id
                );
                return 0;
            }
        };

        if client_ids.is_empty() {
            warn!(
                "broadcast_to_session_except: All clients filtered out for session {} (except={})",
                session_id, except_client
            );
            return 0;
        }

        info!(
            "Broadcasting {:?} to {} clients in session {} (except {}): {:?}",
            std::mem::discriminant(&message),
            client_ids.len(),
            session_id,
            except_client,
            client_ids
        );

        let mut success_count = 0;

        for client_id in client_ids {
            if self.send_to_client(client_id, message.clone()).await {
                info!("Broadcast to client {} succeeded", client_id);
                success_count += 1;
            } else {
                warn!("Broadcast to client {} failed", client_id);
            }
        }

        info!("Broadcast complete: {}/{} succeeded", success_count, success_count);
        success_count
    }

    /// Get all client IDs attached to a session
    pub fn get_session_clients(&self, session_id: SessionId) -> Vec<ClientId> {
        self.session_clients
            .get(&session_id)
            .map(|clients| clients.iter().copied().collect())
            .unwrap_or_default()
    }

    /// Get all registered client IDs
    pub fn get_all_clients(&self) -> Vec<ClientId> {
        self.clients.iter().map(|entry| *entry.key()).collect()
    }

    /// Detach all clients from a session
    ///
    /// Used when destroying a session to cleanly detach all connected clients.
    /// Returns the number of clients that were detached.
    pub fn detach_session_clients(&self, session_id: SessionId) -> usize {
        let client_ids = self.get_session_clients(session_id);
        let mut count = 0;

        for client_id in client_ids {
            if self.detach_from_session(client_id) {
                count += 1;
            }
        }

        debug!(
            "Detached {} clients from session {}",
            count, session_id
        );
        count
    }

    /// Broadcast a message to all connected clients (non-blocking)
    ///
    /// Returns the number of clients that successfully received the message.
    pub fn broadcast_to_all(&self, message: ServerMessage) -> usize {
        let client_ids = self.get_all_clients();

        if client_ids.is_empty() {
            return 0;
        }

        debug!("Broadcasting to all {} clients", client_ids.len());

        let mut success_count = 0;

        for client_id in client_ids {
            if self.try_send_to_client(client_id, message.clone()) {
                success_count += 1;
            }
        }

        success_count
    }
    /// Broadcast a message to all connected clients except one (non-blocking)
    ///
    /// Returns the number of clients that successfully received the message.
    pub fn broadcast_to_all_except(&self, except_client: ClientId, message: ServerMessage) -> usize {
        let client_ids = self.get_all_clients();

        if client_ids.is_empty() {
            return 0;
        }

        let mut success_count = 0;

        for client_id in client_ids {
            if client_id != except_client {
                if self.try_send_to_client(client_id, message.clone()) {
                    success_count += 1;
                }
            }
        }

        success_count
    }
}

impl std::fmt::Debug for ClientRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClientRegistry")
            .field("client_count", &self.clients.len())
            .field("session_count", &self.session_clients.len())
            .field("next_client_id", &self.next_client_id.load(Ordering::SeqCst))
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc;

    /// Create a test registry with a client
    fn setup_client() -> (ClientRegistry, ClientId, mpsc::Receiver<ServerMessage>) {
        let registry = ClientRegistry::new();
        let (tx, rx) = mpsc::channel(10);
        let client_id = registry.register_client(tx);
        (registry, client_id, rx)
    }

    // ==================== Core Registry Structure Tests ====================

    #[test]
    fn test_registry_new() {
        let registry = ClientRegistry::new();
        assert_eq!(registry.client_count(), 0);
    }

    #[test]
    fn test_registry_default() {
        let registry = ClientRegistry::default();
        assert_eq!(registry.client_count(), 0);
    }

    #[test]
    fn test_client_id_display() {
        let id = ClientId::new(42);
        assert_eq!(format!("{}", id), "Client(42)");
    }

    #[test]
    fn test_client_id_equality() {
        let id1 = ClientId::new(1);
        let id2 = ClientId::new(1);
        let id3 = ClientId::new(2);

        assert_eq!(id1, id2);
        assert_ne!(id1, id3);
    }

    #[test]
    fn test_client_id_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();

        set.insert(ClientId::new(1));
        set.insert(ClientId::new(2));
        set.insert(ClientId::new(1)); // Duplicate

        assert_eq!(set.len(), 2);
    }

    #[test]
    fn test_client_id_value() {
        let id = ClientId::new(123);
        assert_eq!(id.value(), 123);
    }

    // ==================== Client Registration Tests ====================

    #[tokio::test]
    async fn test_register_client() {
        let registry = ClientRegistry::new();
        let (tx, _rx) = mpsc::channel(10);

        let client_id = registry.register_client(tx);

        assert_eq!(client_id.value(), 1);
        assert_eq!(registry.client_count(), 1);
    }

    #[tokio::test]
    async fn test_register_multiple_clients() {
        let registry = ClientRegistry::new();

        let (tx1, _rx1) = mpsc::channel(10);
        let (tx2, _rx2) = mpsc::channel(10);
        let (tx3, _rx3) = mpsc::channel(10);

        let id1 = registry.register_client(tx1);
        let id2 = registry.register_client(tx2);
        let id3 = registry.register_client(tx3);

        assert_eq!(id1.value(), 1);
        assert_eq!(id2.value(), 2);
        assert_eq!(id3.value(), 3);
        assert_eq!(registry.client_count(), 3);
    }

    #[tokio::test]
    async fn test_unregister_client() {
        let (registry, client_id, _rx) = setup_client();

        assert_eq!(registry.client_count(), 1);
        registry.unregister_client(client_id);
        assert_eq!(registry.client_count(), 0);
    }

    #[tokio::test]
    async fn test_unregister_nonexistent_client() {
        let registry = ClientRegistry::new();
        let fake_id = ClientId::new(999);

        // Should not panic
        registry.unregister_client(fake_id);
        assert_eq!(registry.client_count(), 0);
    }

    #[tokio::test]
    async fn test_get_client() {
        let (registry, client_id, _rx) = setup_client();

        let client = registry.get_client(client_id);
        assert!(client.is_some());
        assert!(client.unwrap().attached_session.is_none());
    }

    #[tokio::test]
    async fn test_get_nonexistent_client() {
        let registry = ClientRegistry::new();
        let fake_id = ClientId::new(999);

        assert!(registry.get_client(fake_id).is_none());
    }

    // ==================== Session Association Tests ====================

    #[tokio::test]
    async fn test_attach_to_session() {
        let (registry, client_id, _rx) = setup_client();
        let session_id = Uuid::new_v4();

        let result = registry.attach_to_session(client_id, session_id);

        assert!(result);
        assert_eq!(registry.get_client_session(client_id), Some(session_id));
        assert_eq!(registry.session_client_count(session_id), 1);
    }

    #[tokio::test]
    async fn test_attach_nonexistent_client() {
        let registry = ClientRegistry::new();
        let fake_id = ClientId::new(999);
        let session_id = Uuid::new_v4();

        let result = registry.attach_to_session(fake_id, session_id);
        assert!(!result);
    }

    #[tokio::test]
    async fn test_detach_from_session() {
        let (registry, client_id, _rx) = setup_client();
        let session_id = Uuid::new_v4();

        registry.attach_to_session(client_id, session_id);
        let result = registry.detach_from_session(client_id);

        assert!(result);
        assert!(registry.get_client_session(client_id).is_none());
        assert_eq!(registry.session_client_count(session_id), 0);
    }

    #[tokio::test]
    async fn test_detach_not_attached() {
        let (registry, client_id, _rx) = setup_client();

        let result = registry.detach_from_session(client_id);
        assert!(!result);
    }

    #[tokio::test]
    async fn test_detach_nonexistent_client() {
        let registry = ClientRegistry::new();
        let fake_id = ClientId::new(999);

        let result = registry.detach_from_session(fake_id);
        assert!(!result);
    }

    #[tokio::test]
    async fn test_reattach_to_different_session() {
        let (registry, client_id, _rx) = setup_client();
        let session1 = Uuid::new_v4();
        let session2 = Uuid::new_v4();

        registry.attach_to_session(client_id, session1);
        assert_eq!(registry.session_client_count(session1), 1);

        registry.attach_to_session(client_id, session2);

        // Should be in new session, not old
        assert_eq!(registry.get_client_session(client_id), Some(session2));
        assert_eq!(registry.session_client_count(session1), 0);
        assert_eq!(registry.session_client_count(session2), 1);
    }

    #[tokio::test]
    async fn test_reattach_to_same_session() {
        let (registry, client_id, _rx) = setup_client();
        let session_id = Uuid::new_v4();

        registry.attach_to_session(client_id, session_id);
        registry.attach_to_session(client_id, session_id);

        assert_eq!(registry.get_client_session(client_id), Some(session_id));
        assert_eq!(registry.session_client_count(session_id), 1);
    }

    #[tokio::test]
    async fn test_multiple_clients_same_session() {
        let registry = ClientRegistry::new();
        let session_id = Uuid::new_v4();

        let (tx1, _rx1) = mpsc::channel(10);
        let (tx2, _rx2) = mpsc::channel(10);
        let (tx3, _rx3) = mpsc::channel(10);

        let id1 = registry.register_client(tx1);
        let id2 = registry.register_client(tx2);
        let id3 = registry.register_client(tx3);

        registry.attach_to_session(id1, session_id);
        registry.attach_to_session(id2, session_id);
        registry.attach_to_session(id3, session_id);

        assert_eq!(registry.session_client_count(session_id), 3);
    }

    #[tokio::test]
    async fn test_unregister_cleans_up_session() {
        let (registry, client_id, _rx) = setup_client();
        let session_id = Uuid::new_v4();

        registry.attach_to_session(client_id, session_id);
        assert_eq!(registry.session_client_count(session_id), 1);

        registry.unregister_client(client_id);
        assert_eq!(registry.session_client_count(session_id), 0);
    }

    #[tokio::test]
    async fn test_get_session_clients() {
        let registry = ClientRegistry::new();
        let session_id = Uuid::new_v4();

        let (tx1, _rx1) = mpsc::channel(10);
        let (tx2, _rx2) = mpsc::channel(10);

        let id1 = registry.register_client(tx1);
        let id2 = registry.register_client(tx2);

        registry.attach_to_session(id1, session_id);
        registry.attach_to_session(id2, session_id);

        let clients = registry.get_session_clients(session_id);
        assert_eq!(clients.len(), 2);
        assert!(clients.contains(&id1));
        assert!(clients.contains(&id2));
    }

    #[tokio::test]
    async fn test_get_session_clients_empty() {
        let registry = ClientRegistry::new();
        let session_id = Uuid::new_v4();

        let clients = registry.get_session_clients(session_id);
        assert!(clients.is_empty());
    }

    // ==================== Send to Client Tests ====================

    #[tokio::test]
    async fn test_send_to_client() {
        let (registry, client_id, mut rx) = setup_client();

        let result = registry.send_to_client(client_id, ServerMessage::Pong).await;

        assert!(result);
        let msg = rx.recv().await.unwrap();
        assert_eq!(msg, ServerMessage::Pong);
    }

    #[tokio::test]
    async fn test_send_to_nonexistent_client() {
        let registry = ClientRegistry::new();
        let fake_id = ClientId::new(999);

        let result = registry.send_to_client(fake_id, ServerMessage::Pong).await;
        assert!(!result);
    }

    #[tokio::test]
    async fn test_send_to_disconnected_client() {
        let (registry, client_id, rx) = setup_client();

        // Close the receiver
        drop(rx);

        // Send should fail and client should be unregistered
        let result = registry.send_to_client(client_id, ServerMessage::Pong).await;
        assert!(!result);
        assert_eq!(registry.client_count(), 0);
    }

    #[tokio::test]
    async fn test_try_send_to_client() {
        let (registry, client_id, mut rx) = setup_client();

        let result = registry.try_send_to_client(client_id, ServerMessage::Pong);

        assert!(result);
        let msg = rx.recv().await.unwrap();
        assert_eq!(msg, ServerMessage::Pong);
    }

    #[tokio::test]
    async fn test_try_send_to_nonexistent_client() {
        let registry = ClientRegistry::new();
        let fake_id = ClientId::new(999);

        let result = registry.try_send_to_client(fake_id, ServerMessage::Pong);
        assert!(!result);
    }

    #[tokio::test]
    async fn test_try_send_to_disconnected_client() {
        let (registry, client_id, rx) = setup_client();

        // Close the receiver
        drop(rx);

        // Send should fail and client should be unregistered
        let result = registry.try_send_to_client(client_id, ServerMessage::Pong);
        assert!(!result);
        assert_eq!(registry.client_count(), 0);
    }

    #[tokio::test]
    async fn test_try_send_channel_full() {
        let registry = ClientRegistry::new();
        let (tx, _rx) = mpsc::channel(1); // Small buffer
        let client_id = registry.register_client(tx);

        // Fill the buffer
        assert!(registry.try_send_to_client(client_id, ServerMessage::Pong));

        // Next send should fail (channel full)
        let result = registry.try_send_to_client(client_id, ServerMessage::Pong);
        assert!(!result);

        // Client should still be registered (not disconnected, just slow)
        assert_eq!(registry.client_count(), 1);
    }

    // ==================== Broadcast Tests ====================

    #[tokio::test]
    async fn test_broadcast_to_session() {
        let registry = ClientRegistry::new();
        let session_id = Uuid::new_v4();

        let (tx1, mut rx1) = mpsc::channel(10);
        let (tx2, mut rx2) = mpsc::channel(10);

        let id1 = registry.register_client(tx1);
        let id2 = registry.register_client(tx2);

        registry.attach_to_session(id1, session_id);
        registry.attach_to_session(id2, session_id);

        let count = registry.broadcast_to_session(session_id, ServerMessage::Pong).await;

        assert_eq!(count, 2);
        assert_eq!(rx1.recv().await.unwrap(), ServerMessage::Pong);
        assert_eq!(rx2.recv().await.unwrap(), ServerMessage::Pong);
    }

    #[tokio::test]
    async fn test_broadcast_to_empty_session() {
        let registry = ClientRegistry::new();
        let session_id = Uuid::new_v4();

        let count = registry.broadcast_to_session(session_id, ServerMessage::Pong).await;
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn test_broadcast_with_disconnected_client() {
        let registry = ClientRegistry::new();
        let session_id = Uuid::new_v4();

        let (tx1, mut rx1) = mpsc::channel(10);
        let (tx2, rx2) = mpsc::channel(10);

        let id1 = registry.register_client(tx1);
        let id2 = registry.register_client(tx2);

        registry.attach_to_session(id1, session_id);
        registry.attach_to_session(id2, session_id);

        // Disconnect client 2
        drop(rx2);

        let count = registry.broadcast_to_session(session_id, ServerMessage::Pong).await;

        // Only client 1 should receive the message
        assert_eq!(count, 1);
        assert_eq!(rx1.recv().await.unwrap(), ServerMessage::Pong);

        // Client 2 should be unregistered
        assert_eq!(registry.client_count(), 1);
    }

    #[tokio::test]
    async fn test_try_broadcast_to_session() {
        let registry = ClientRegistry::new();
        let session_id = Uuid::new_v4();

        let (tx1, mut rx1) = mpsc::channel(10);
        let (tx2, mut rx2) = mpsc::channel(10);

        let id1 = registry.register_client(tx1);
        let id2 = registry.register_client(tx2);

        registry.attach_to_session(id1, session_id);
        registry.attach_to_session(id2, session_id);

        let count = registry.try_broadcast_to_session(session_id, ServerMessage::Pong);

        assert_eq!(count, 2);
        assert_eq!(rx1.recv().await.unwrap(), ServerMessage::Pong);
        assert_eq!(rx2.recv().await.unwrap(), ServerMessage::Pong);
    }

    #[tokio::test]
    async fn test_try_broadcast_to_empty_session() {
        let registry = ClientRegistry::new();
        let session_id = Uuid::new_v4();

        let count = registry.try_broadcast_to_session(session_id, ServerMessage::Pong);
        assert_eq!(count, 0);
    }

    // ==================== Get All Clients Tests ====================

    #[tokio::test]
    async fn test_get_all_clients() {
        let registry = ClientRegistry::new();

        let (tx1, _rx1) = mpsc::channel(10);
        let (tx2, _rx2) = mpsc::channel(10);

        let id1 = registry.register_client(tx1);
        let id2 = registry.register_client(tx2);

        let all_clients = registry.get_all_clients();
        assert_eq!(all_clients.len(), 2);
        assert!(all_clients.contains(&id1));
        assert!(all_clients.contains(&id2));
    }

    #[tokio::test]
    async fn test_get_all_clients_empty() {
        let registry = ClientRegistry::new();
        assert!(registry.get_all_clients().is_empty());
    }

    // ==================== Concurrent Access Tests ====================

    #[tokio::test]
    async fn test_concurrent_registration() {
        use std::sync::Arc;

        let registry = Arc::new(ClientRegistry::new());
        let mut handles = vec![];

        // Spawn 100 tasks that each register a client
        for _ in 0..100 {
            let registry = Arc::clone(&registry);
            handles.push(tokio::spawn(async move {
                let (tx, _rx) = mpsc::channel(10);
                registry.register_client(tx)
            }));
        }

        // Wait for all to complete
        for handle in handles {
            handle.await.unwrap();
        }

        assert_eq!(registry.client_count(), 100);
    }

    #[tokio::test]
    async fn test_concurrent_attach_detach() {
        use std::sync::Arc;

        let registry = Arc::new(ClientRegistry::new());
        let session_id = Uuid::new_v4();

        // Register some clients
        let mut client_ids = vec![];
        for _ in 0..10 {
            let (tx, _rx) = mpsc::channel(10);
            client_ids.push(registry.register_client(tx));
        }

        let mut handles = vec![];

        // Spawn tasks that attach and detach clients
        for client_id in client_ids.clone() {
            let registry = Arc::clone(&registry);
            handles.push(tokio::spawn(async move {
                for _ in 0..10 {
                    registry.attach_to_session(client_id, session_id);
                    tokio::task::yield_now().await;
                    registry.detach_from_session(client_id);
                    tokio::task::yield_now().await;
                }
            }));
        }

        // Wait for all to complete
        for handle in handles {
            handle.await.unwrap();
        }

        // All clients should be detached
        for client_id in client_ids {
            assert!(registry.get_client_session(client_id).is_none());
        }
    }

    #[tokio::test]
    async fn test_concurrent_broadcast() {
        use std::sync::Arc;

        let registry = Arc::new(ClientRegistry::new());
        let session_id = Uuid::new_v4();

        // Register and attach 10 clients
        let mut receivers = vec![];
        for _ in 0..10 {
            let (tx, rx) = mpsc::channel(100);
            let client_id = registry.register_client(tx);
            registry.attach_to_session(client_id, session_id);
            receivers.push(rx);
        }

        let mut handles = vec![];

        // Spawn 10 tasks that each broadcast 10 messages
        for i in 0..10 {
            let registry = Arc::clone(&registry);
            handles.push(tokio::spawn(async move {
                for j in 0..10 {
                    let msg = ServerMessage::Output {
                        pane_id: Uuid::nil(),
                        data: format!("task {} msg {}", i, j).into_bytes(),
                    };
                    registry.broadcast_to_session(session_id, msg).await;
                }
            }));
        }

        // Wait for all broadcasts to complete
        for handle in handles {
            handle.await.unwrap();
        }

        // Each receiver should have 100 messages
        for mut rx in receivers {
            let mut count = 0;
            while rx.try_recv().is_ok() {
                count += 1;
            }
            assert_eq!(count, 100);
        }
    }

    // ==================== Debug Format Tests ====================

    #[tokio::test]
    async fn test_registry_debug() {
        let (registry, _client_id, _rx) = setup_client();

        let debug = format!("{:?}", registry);
        assert!(debug.contains("ClientRegistry"));
        assert!(debug.contains("client_count"));
    }

    #[tokio::test]
    async fn test_client_entry_debug() {
        let (tx, _rx) = mpsc::channel::<ServerMessage>(10);
        let entry = ClientEntry {
            sender: tx,
            attached_session: Some(Uuid::new_v4()),
        };

        let debug = format!("{:?}", entry);
        assert!(debug.contains("ClientEntry"));
        assert!(debug.contains("attached_session"));
    }

    // ==================== Disconnected Client Cleanup Tests ====================

    #[tokio::test]
    async fn test_disconnected_client_cleanup_on_send() {
        let registry = ClientRegistry::new();
        let session_id = Uuid::new_v4();

        let (tx, rx) = mpsc::channel(10);
        let client_id = registry.register_client(tx);
        registry.attach_to_session(client_id, session_id);

        assert_eq!(registry.client_count(), 1);
        assert_eq!(registry.session_client_count(session_id), 1);

        // Disconnect the client
        drop(rx);

        // Send should fail and cleanup should happen
        let result = registry.send_to_client(client_id, ServerMessage::Pong).await;
        assert!(!result);

        // Client should be removed
        assert_eq!(registry.client_count(), 0);
        // Session should be cleaned up
        assert_eq!(registry.session_client_count(session_id), 0);
    }

    #[tokio::test]
    async fn test_disconnected_client_cleanup_on_broadcast() {
        let registry = ClientRegistry::new();
        let session_id = Uuid::new_v4();

        let (tx1, mut rx1) = mpsc::channel(10);
        let (tx2, rx2) = mpsc::channel(10);

        let id1 = registry.register_client(tx1);
        let id2 = registry.register_client(tx2);

        registry.attach_to_session(id1, session_id);
        registry.attach_to_session(id2, session_id);

        // Disconnect client 2
        drop(rx2);

        // Broadcast should cleanup disconnected client
        let count = registry.broadcast_to_session(session_id, ServerMessage::Pong).await;
        assert_eq!(count, 1);

        // Client 1 should receive the message
        assert_eq!(rx1.recv().await.unwrap(), ServerMessage::Pong);

        // Client 2 should be cleaned up
        assert_eq!(registry.client_count(), 1);
        assert_eq!(registry.session_client_count(session_id), 1);
    }

    // ==================== MCP-to-TUI Broadcast Tests (BUG-010) ====================

    /// Test that simulates MCP pane creation broadcast to TUI
    ///
    /// This test validates the fix for BUG-010: When MCP creates a pane,
    /// the TUI client should receive the PaneCreated broadcast even though
    /// the MCP client is not attached to any session.
    #[tokio::test]
    async fn test_mcp_to_tui_broadcast_except() {
        let registry = ClientRegistry::new();
        let session_id = Uuid::new_v4();

        // Simulate TUI client: registered AND attached to session
        let (tui_tx, mut tui_rx) = mpsc::channel(10);
        let tui_client_id = registry.register_client(tui_tx);
        registry.attach_to_session(tui_client_id, session_id);

        // Simulate MCP client: registered but NOT attached to any session
        let (mcp_tx, mut mcp_rx) = mpsc::channel(10);
        let mcp_client_id = registry.register_client(mcp_tx);
        // Note: MCP client is NOT attached to any session

        // Verify initial state
        assert_eq!(registry.client_count(), 2);
        assert_eq!(registry.session_client_count(session_id), 1); // Only TUI is attached

        // Simulate broadcasting PaneCreated to session, excluding MCP client
        // This is exactly what happens in main.rs when processing ResponseWithBroadcast
        let pane_info = ccmux_protocol::PaneInfo {
            id: Uuid::new_v4(),
            window_id: Uuid::new_v4(),
            index: 1,
            cols: 80,
            rows: 24,
            state: ccmux_protocol::PaneState::Normal,
            name: None,
            title: Some("test".to_string()),
            cwd: None,
        };
        let broadcast_msg = ServerMessage::PaneCreated {
            pane: pane_info,
            direction: ccmux_protocol::SplitDirection::Vertical,
        };

        let count = registry
            .broadcast_to_session_except(session_id, mcp_client_id, broadcast_msg.clone())
            .await;

        // Should have sent to 1 client (TUI)
        assert_eq!(count, 1, "Should broadcast to 1 client (TUI)");

        // TUI should receive the message
        let received = tui_rx.try_recv();
        assert!(received.is_ok(), "TUI should receive the broadcast");
        match received.unwrap() {
            ServerMessage::PaneCreated { pane, direction: _ } => {
                assert_eq!(pane.index, 1);
            }
            _ => panic!("Expected PaneCreated message"),
        }

        // MCP should NOT receive the message (it's the except_client AND not attached)
        let mcp_received = mcp_rx.try_recv();
        assert!(mcp_received.is_err(), "MCP should NOT receive the broadcast");
    }

    /// Test broadcast when except_client is not in the session
    ///
    /// This verifies that when the except_client (MCP) is not attached to
    /// the session, filtering still works correctly and TUI receives message.
    #[tokio::test]
    async fn test_broadcast_except_unattached_client() {
        let registry = ClientRegistry::new();
        let session_id = Uuid::new_v4();

        // Multiple TUI clients attached to the session
        let (tui1_tx, mut tui1_rx) = mpsc::channel(10);
        let (tui2_tx, mut tui2_rx) = mpsc::channel(10);
        let tui1_id = registry.register_client(tui1_tx);
        let tui2_id = registry.register_client(tui2_tx);
        registry.attach_to_session(tui1_id, session_id);
        registry.attach_to_session(tui2_id, session_id);

        // MCP client NOT attached to any session
        let (mcp_tx, mut mcp_rx) = mpsc::channel(10);
        let mcp_client_id = registry.register_client(mcp_tx);

        // Verify TUI clients are attached, MCP is not
        assert_eq!(registry.session_client_count(session_id), 2);
        assert!(registry.get_client_session(mcp_client_id).is_none());

        // Broadcast excluding MCP client (which isn't in the session anyway)
        let count = registry
            .broadcast_to_session_except(session_id, mcp_client_id, ServerMessage::Pong)
            .await;

        // Both TUI clients should receive the message
        assert_eq!(count, 2, "Should broadcast to 2 TUI clients");

        assert!(tui1_rx.try_recv().is_ok(), "TUI 1 should receive broadcast");
        assert!(tui2_rx.try_recv().is_ok(), "TUI 2 should receive broadcast");
        assert!(mcp_rx.try_recv().is_err(), "MCP should NOT receive broadcast");
    }

    /// Test broadcast_to_session_except when TUI is attached to different session
    ///
    /// This tests the scenario where MCP broadcasts to session A but TUI is
    /// attached to session B - TUI should NOT receive the message.
    #[tokio::test]
    async fn test_broadcast_to_different_session() {
        let registry = ClientRegistry::new();
        let session_a = Uuid::new_v4();
        let session_b = Uuid::new_v4();

        // TUI client attached to session B
        let (tui_tx, mut tui_rx) = mpsc::channel(10);
        let tui_client_id = registry.register_client(tui_tx);
        registry.attach_to_session(tui_client_id, session_b);

        // MCP client not attached
        let (mcp_tx, _) = mpsc::channel(10);
        let mcp_client_id = registry.register_client(mcp_tx);

        // Broadcast to session A (where TUI is NOT attached)
        let count = registry
            .broadcast_to_session_except(session_a, mcp_client_id, ServerMessage::Pong)
            .await;

        // No clients should receive (session A has no clients)
        assert_eq!(count, 0, "No clients attached to session A");
        assert!(tui_rx.try_recv().is_err(), "TUI (in session B) should NOT receive");
    }
}
