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

use uuid::Uuid;

/// Default lockout duration for input (2 seconds)
const DEFAULT_INPUT_LOCKOUT: Duration = Duration::from_millis(2000);
/// Default lockout duration for layout changes (5 seconds)
const DEFAULT_LAYOUT_LOCKOUT: Duration = Duration::from_millis(5000);

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

/// Arbitrator for Human-Control Mode (FEAT-079)
///
/// Centralizes arbitration logic to prevent MCP agents from interfering with
/// active human usage. Tracks:
/// 1. Focus Locks (explicit command mode)
/// 2. Input Activity (recent typing)
/// 3. Layout Activity (recent resizing/splitting)
#[derive(Debug)]
pub struct Arbitrator {
    /// Client ID -> Focus Lock state
    focus_locks: DashMap<ClientId, LockState>,
    /// Pane ID -> Last User Input Time
    input_activity: DashMap<Uuid, Instant>,
    /// Window ID -> Last User Layout Activity Time
    layout_activity: DashMap<Uuid, Instant>,
}

impl Default for Arbitrator {
    fn default() -> Self {
        Self::new()
    }
}

impl Arbitrator {
    /// Create a new arbitrator
    pub fn new() -> Self {
        Self {
            focus_locks: DashMap::new(),
            input_activity: DashMap::new(),
            layout_activity: DashMap::new(),
        }
    }

    // ==================== Focus Locking (FEAT-056) ====================

    /// Set a focus lock for a client
    ///
    /// Called when client sends `UserCommandModeEntered`.
    pub fn set_focus_lock(&self, client_id: ClientId, timeout_ms: u32) {
        info!(
            client = %client_id,
            timeout_ms = timeout_ms,
            "User priority focus lock activated"
        );
        self.focus_locks.insert(client_id, LockState::new(timeout_ms));
    }

    /// Release a client's focus lock
    ///
    /// Called when client sends `UserCommandModeExited`.
    pub fn release_focus_lock(&self, client_id: ClientId) {
        if self.focus_locks.remove(&client_id).is_some() {
            debug!(client = %client_id, "User priority focus lock released");
        }
    }

    /// Check if any client has an active focus lock
    ///
    /// Returns `Some((client_id, remaining_ms))` if locked.
    pub fn check_focus_lock(&self) -> Option<(ClientId, u64)> {
        let mut expired = Vec::new();
        let mut active = None;

        for entry in self.focus_locks.iter() {
            let client_id = *entry.key();
            let state = entry.value();

            if state.is_active() {
                active = Some((client_id, state.remaining_ms()));
            } else {
                expired.push(client_id);
            }
        }

        for client_id in expired {
            self.focus_locks.remove(&client_id);
        }

        active
    }

    /// Check if a specific client is locked
    pub fn is_client_locked(&self, client_id: ClientId) -> bool {
        if let Some(entry) = self.focus_locks.get(&client_id) {
            if entry.is_active() {
                return true;
            }
        }
        if self.focus_locks.get(&client_id).map(|e| !e.is_active()).unwrap_or(false) {
            self.focus_locks.remove(&client_id);
        }
        false
    }

    // ==================== Input/Layout Locking (FEAT-079) ====================

    /// Record human input activity on a pane
    pub fn record_human_input(&self, pane_id: Uuid) {
        self.input_activity.insert(pane_id, Instant::now());
    }

    /// Check if MCP can send input to a pane
    ///
    /// Returns `Err(remaining_ms)` if blocked by recent human activity.
    pub fn check_input_access(&self, pane_id: Uuid) -> Result<(), u64> {
        if let Some(last) = self.input_activity.get(&pane_id) {
            let elapsed = last.elapsed();
            if elapsed < DEFAULT_INPUT_LOCKOUT {
                let remaining = (DEFAULT_INPUT_LOCKOUT - elapsed).as_millis() as u64;
                return Err(remaining);
            }
        }
        Ok(())
    }

    /// Record human layout activity (resize/split/close) on a window
    ///
    /// Can also be used with pane_id if we map it to window, but currently
    /// we track by ID provided (caller should ensure it's the relevant scope).
    /// For simplicity, we'll track by "Resource ID" (Pane or Window).
    pub fn record_human_layout(&self, resource_id: Uuid) {
        self.layout_activity.insert(resource_id, Instant::now());
    }

    /// Check if MCP can modify layout for a resource
    pub fn check_layout_access(&self, resource_id: Uuid) -> Result<(), u64> {
        if let Some(last) = self.layout_activity.get(&resource_id) {
            let elapsed = last.elapsed();
            if elapsed < DEFAULT_LAYOUT_LOCKOUT {
                let remaining = (DEFAULT_LAYOUT_LOCKOUT - elapsed).as_millis() as u64;
                return Err(remaining);
            }
        }
        Ok(())
    }

    // ==================== Lifecycle ====================

    /// Remove locks when client disconnects
    pub fn on_client_disconnect(&self, client_id: ClientId) {
        self.release_focus_lock(client_id);
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
        let manager = Arbitrator::new();
        assert!(manager.check_focus_lock().is_none());
    }

    #[test]
    fn test_set_focus_lock() {
        let manager = Arbitrator::new();
        let id = client_id(1);

        manager.set_focus_lock(id, 1000);

        assert!(manager.is_client_locked(id));
        assert!(manager.check_focus_lock().is_some());
    }

    #[test]
    fn test_release_focus_lock() {
        let manager = Arbitrator::new();
        let id = client_id(1);

        manager.set_focus_lock(id, 1000);
        assert!(manager.is_client_locked(id));

        manager.release_focus_lock(id);
        assert!(!manager.is_client_locked(id));
        assert!(manager.check_focus_lock().is_none());
    }

    #[test]
    fn test_lock_expiration() {
        let manager = Arbitrator::new();
        let id = client_id(1);

        // Set a very short lock (10ms)
        manager.set_focus_lock(id, 10);
        assert!(manager.is_client_locked(id));

        // Wait for expiration
        thread::sleep(Duration::from_millis(20));

        // Lock should have expired
        assert!(!manager.is_client_locked(id));
        assert!(manager.check_focus_lock().is_none());
    }

    #[test]
    fn test_multiple_clients() {
        let manager = Arbitrator::new();
        let id1 = client_id(1);
        let id2 = client_id(2);

        manager.set_focus_lock(id1, 1000);
        manager.set_focus_lock(id2, 1000);

        assert!(manager.is_client_locked(id1));
        assert!(manager.is_client_locked(id2));

        manager.release_focus_lock(id1);

        assert!(!manager.is_client_locked(id1));
        assert!(manager.is_client_locked(id2));
    }

    #[test]
    fn test_check_focus_lock_returns_info() {
        let manager = Arbitrator::new();
        let id = client_id(42);

        manager.set_focus_lock(id, 1000);

        let result = manager.check_focus_lock();
        assert!(result.is_some());

        let (active_client, remaining) = result.unwrap();
        assert_eq!(active_client, id);
        assert!(remaining > 0);
        assert!(remaining <= 1000);
    }

    #[test]
    fn test_on_client_disconnect() {
        let manager = Arbitrator::new();
        let id = client_id(1);

        manager.set_focus_lock(id, 1000);
        assert!(manager.is_client_locked(id));

        manager.on_client_disconnect(id);
        assert!(!manager.is_client_locked(id));
    }

    #[test]
    fn test_remaining_time() {
        let manager = Arbitrator::new();
        let id = client_id(1);

        manager.set_focus_lock(id, 500);

        // Check remaining time is reasonable
        if let Some((_, remaining)) = manager.check_focus_lock() {
            assert!(remaining <= 500);
            assert!(remaining > 400); // Should be close to 500
        } else {
            panic!("Expected active lock");
        }
    }

    #[test]
    fn test_release_nonexistent_lock() {
        let manager = Arbitrator::new();
        let id = client_id(999);

        // Should not panic
        manager.release_focus_lock(id);
        assert!(!manager.is_client_locked(id));
    }

    #[test]
    fn test_overwrite_existing_lock() {
        let manager = Arbitrator::new();
        let id = client_id(1);

        manager.set_focus_lock(id, 100);
        manager.set_focus_lock(id, 2000); // Overwrite with longer timeout

        // Wait 150ms - first lock would have expired, second should still be active
        thread::sleep(Duration::from_millis(150));

        assert!(manager.is_client_locked(id));
    }

    #[test]
    fn test_expired_cleanup_during_check() {
        let manager = Arbitrator::new();
        let id1 = client_id(1);
        let id2 = client_id(2);

        // Set short lock for client 1
        manager.set_focus_lock(id1, 10);
        // Set longer lock for client 2
        manager.set_focus_lock(id2, 1000);

        // Wait for client 1's lock to expire
        thread::sleep(Duration::from_millis(20));

        // Check should clean up client 1's expired lock
        let result = manager.check_focus_lock();
        assert!(result.is_some());

        // Client 1 should no longer be locked
        assert!(!manager.is_client_locked(id1));
        // Client 2 should still be locked
        assert!(manager.is_client_locked(id2));
    }

    #[test]
    fn test_input_locking() {
        let manager = Arbitrator::new();
        let pane_id = Uuid::new_v4();

        // Initially allowed
        assert!(manager.check_input_access(pane_id).is_ok());

        // Record human input
        manager.record_human_input(pane_id);

        // Should be blocked now
        match manager.check_input_access(pane_id) {
            Err(remaining) => assert!(remaining > 0),
            Ok(_) => panic!("Should be blocked"),
        }

        // Wait for lockout (simulated by checking smaller threshold manually if we could, 
        // but here we just check logic. Real sleep is slow)
        // thread::sleep(Duration::from_millis(2100));
        // assert!(manager.check_input_access(pane_id).is_ok());
    }

    #[test]
    fn test_layout_locking() {
        let manager = Arbitrator::new();
        let resource_id = Uuid::new_v4();

        // Initially allowed
        assert!(manager.check_layout_access(resource_id).is_ok());

        // Record human layout change
        manager.record_human_layout(resource_id);

        // Should be blocked
        assert!(manager.check_layout_access(resource_id).is_err());
    }
}
