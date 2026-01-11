//! User Priority Lock State (FEAT-056)
//!
//! Prevents MCP agents from interfering with user focus-changing operations
//! when the user is in command mode (prefix key pressed).
//!
//! When a user presses the prefix key (e.g., Ctrl+B in tmux-style bindings),
//! the client sends a `UserCommandModeEntered` message. This module tracks
//! that lock state and allows MCP handlers to check if focus operations
//! should be blocked.

use std::time::{Duration, Instant};

use dashmap::DashMap;
use tracing::{debug, info};

use crate::registry::ClientId;

/// User priority lock state for a single client
#[derive(Debug)]
struct LockState {
    /// When the lock was activated
    activated_at: Instant,
    /// How long the lock should last before auto-expiring
    timeout: Duration,
}

impl LockState {
    /// Create a new lock state with the given timeout
    fn new(timeout_ms: u32) -> Self {
        Self {
            activated_at: Instant::now(),
            timeout: Duration::from_millis(timeout_ms as u64),
        }
    }

    /// Check if the lock is still active (hasn't timed out)
    fn is_active(&self) -> bool {
        self.activated_at.elapsed() < self.timeout
    }

    /// Get remaining time before expiration in milliseconds
    fn remaining_ms(&self) -> u64 {
        let elapsed = self.activated_at.elapsed();
        if elapsed >= self.timeout {
            0
        } else {
            (self.timeout - elapsed).as_millis() as u64
        }
    }
}

/// User priority state manager
///
/// Thread-safe tracking of which clients have active user priority locks.
/// This is used by MCP handlers to determine if focus-changing operations
/// should be blocked.
#[derive(Debug)]
pub struct UserPriorityManager {
    /// Client ID -> Lock state
    locks: DashMap<ClientId, LockState>,
}

impl Default for UserPriorityManager {
    fn default() -> Self {
        Self::new()
    }
}

impl UserPriorityManager {
    /// Create a new user priority manager
    pub fn new() -> Self {
        Self {
            locks: DashMap::new(),
        }
    }

    /// Set a lock for a client with the given timeout
    ///
    /// Called when client sends `UserCommandModeEntered`.
    pub fn set_lock(&self, client_id: ClientId, timeout_ms: u32) {
        info!(
            client = %client_id,
            timeout_ms = timeout_ms,
            "User priority lock activated"
        );
        self.locks.insert(client_id, LockState::new(timeout_ms));
    }

    /// Release a client's lock
    ///
    /// Called when client sends `UserCommandModeExited` or when cleaning up.
    pub fn release_lock(&self, client_id: ClientId) {
        if self.locks.remove(&client_id).is_some() {
            debug!(client = %client_id, "User priority lock released");
        }
    }

    /// Check if any client has an active user priority lock
    ///
    /// Used by MCP handlers to determine if focus operations should be blocked.
    /// Returns `Some((client_id, remaining_ms))` if a lock is active, None otherwise.
    pub fn is_any_lock_active(&self) -> Option<(ClientId, u64)> {
        // Clean up expired locks and find any active one
        let mut expired = Vec::new();
        let mut active = None;

        for entry in self.locks.iter() {
            let client_id = *entry.key();
            let state = entry.value();

            if state.is_active() {
                active = Some((client_id, state.remaining_ms()));
            } else {
                expired.push(client_id);
            }
        }

        // Clean up expired entries
        for client_id in expired {
            debug!(client = %client_id, "Cleaning up expired user priority lock");
            self.locks.remove(&client_id);
        }

        active
    }

    /// Check if a specific client has an active lock
    pub fn is_client_locked(&self, client_id: ClientId) -> bool {
        if let Some(entry) = self.locks.get(&client_id) {
            if entry.is_active() {
                return true;
            }
        }
        // Clean up expired lock if any
        if self.locks.get(&client_id).map(|e| !e.is_active()).unwrap_or(false) {
            self.locks.remove(&client_id);
        }
        false
    }

    /// Get the number of active locks (for debugging/metrics)
    pub fn active_lock_count(&self) -> usize {
        self.locks.iter().filter(|e| e.value().is_active()).count()
    }

    /// Remove lock when client disconnects
    ///
    /// Called by the registry or connection handler when a client disconnects.
    pub fn on_client_disconnect(&self, client_id: ClientId) {
        self.release_lock(client_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    fn client_id(n: u64) -> ClientId {
        ClientId::new(n)
    }

    #[test]
    fn test_new_manager() {
        let manager = UserPriorityManager::new();
        assert_eq!(manager.active_lock_count(), 0);
        assert!(manager.is_any_lock_active().is_none());
    }

    #[test]
    fn test_set_lock() {
        let manager = UserPriorityManager::new();
        let id = client_id(1);

        manager.set_lock(id, 1000);

        assert!(manager.is_client_locked(id));
        assert!(manager.is_any_lock_active().is_some());
        assert_eq!(manager.active_lock_count(), 1);
    }

    #[test]
    fn test_release_lock() {
        let manager = UserPriorityManager::new();
        let id = client_id(1);

        manager.set_lock(id, 1000);
        assert!(manager.is_client_locked(id));

        manager.release_lock(id);
        assert!(!manager.is_client_locked(id));
        assert!(manager.is_any_lock_active().is_none());
    }

    #[test]
    fn test_lock_expiration() {
        let manager = UserPriorityManager::new();
        let id = client_id(1);

        // Set a very short lock (10ms)
        manager.set_lock(id, 10);
        assert!(manager.is_client_locked(id));

        // Wait for expiration
        thread::sleep(Duration::from_millis(20));

        // Lock should have expired
        assert!(!manager.is_client_locked(id));
        assert!(manager.is_any_lock_active().is_none());
    }

    #[test]
    fn test_multiple_clients() {
        let manager = UserPriorityManager::new();
        let id1 = client_id(1);
        let id2 = client_id(2);

        manager.set_lock(id1, 1000);
        manager.set_lock(id2, 1000);

        assert!(manager.is_client_locked(id1));
        assert!(manager.is_client_locked(id2));
        assert_eq!(manager.active_lock_count(), 2);

        manager.release_lock(id1);

        assert!(!manager.is_client_locked(id1));
        assert!(manager.is_client_locked(id2));
        assert_eq!(manager.active_lock_count(), 1);
    }

    #[test]
    fn test_is_any_lock_active_returns_info() {
        let manager = UserPriorityManager::new();
        let id = client_id(42);

        manager.set_lock(id, 1000);

        let result = manager.is_any_lock_active();
        assert!(result.is_some());

        let (active_client, remaining) = result.unwrap();
        assert_eq!(active_client, id);
        assert!(remaining > 0);
        assert!(remaining <= 1000);
    }

    #[test]
    fn test_on_client_disconnect() {
        let manager = UserPriorityManager::new();
        let id = client_id(1);

        manager.set_lock(id, 1000);
        assert!(manager.is_client_locked(id));

        manager.on_client_disconnect(id);
        assert!(!manager.is_client_locked(id));
    }

    #[test]
    fn test_remaining_time() {
        let manager = UserPriorityManager::new();
        let id = client_id(1);

        manager.set_lock(id, 500);

        // Check remaining time is reasonable
        if let Some((_, remaining)) = manager.is_any_lock_active() {
            assert!(remaining <= 500);
            assert!(remaining > 400); // Should be close to 500
        } else {
            panic!("Expected active lock");
        }
    }

    #[test]
    fn test_release_nonexistent_lock() {
        let manager = UserPriorityManager::new();
        let id = client_id(999);

        // Should not panic
        manager.release_lock(id);
        assert!(!manager.is_client_locked(id));
    }

    #[test]
    fn test_overwrite_existing_lock() {
        let manager = UserPriorityManager::new();
        let id = client_id(1);

        manager.set_lock(id, 100);
        manager.set_lock(id, 2000); // Overwrite with longer timeout

        // Should still have only 1 lock
        assert_eq!(manager.active_lock_count(), 1);

        // Wait 150ms - first lock would have expired, second should still be active
        thread::sleep(Duration::from_millis(150));

        assert!(manager.is_client_locked(id));
    }

    #[test]
    fn test_expired_cleanup_during_check() {
        let manager = UserPriorityManager::new();
        let id1 = client_id(1);
        let id2 = client_id(2);

        // Set short lock for client 1
        manager.set_lock(id1, 10);
        // Set longer lock for client 2
        manager.set_lock(id2, 1000);

        // Wait for client 1's lock to expire
        thread::sleep(Duration::from_millis(20));

        // Check should clean up client 1's expired lock
        let result = manager.is_any_lock_active();
        assert!(result.is_some());

        // Client 1 should no longer be locked
        assert!(!manager.is_client_locked(id1));
        // Client 2 should still be locked
        assert!(manager.is_client_locked(id2));
    }
}
