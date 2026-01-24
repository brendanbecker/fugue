//! Human-Control Arbitration (FEAT-079)
//!
//! Arbitrates between human users and automated agents (MCP) to prevent
//! interference with active human work.
//!
//! This module centralizes logic for blocking automated mutations (input, focus,
//! layout, kill) when human activity is detected or an explicit lockout is active.

use std::time::{Duration, Instant};

use dashmap::DashMap;
use tracing::{debug, info};
use uuid::Uuid;

use crate::registry::ClientId;

/// Actors that can initiate actions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Actor {
    /// Human user via a specific client
    Human(ClientId),
    /// Automated agent (MCP)
    Agent,
}

/// Resources that can be acted upon
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Resource {
    /// A specific pane
    Pane(Uuid),
    /// A specific window
    Window(Uuid),
    /// A specific session
    Session(Uuid),
    /// System-wide
    Global,
}

/// Actions that can be performed
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Change focus/selection
    Focus,
    /// Send text input
    Input,
    /// Mutate layout (resize, split)
    Layout,
    /// Destructive actions (kill)
    Kill,
}

/// Result of an arbitration check
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArbitrationResult {
    /// Action is allowed
    Allowed,
    /// Action is blocked by a human control lock
    Blocked {
        /// Which client holds the lock (or ClientId(0) for system-level lockout)
        client_id: ClientId,
        /// How much longer the lock is active (ms)
        remaining_ms: u64,
        /// Reason for the lock
        reason: String,
    },
}

impl ArbitrationResult {
    pub fn is_allowed(&self) -> bool {
        matches!(self, ArbitrationResult::Allowed)
    }
}

/// State of human activity for a resource
#[derive(Debug)]
struct ActivityState {
    /// Last time a human interacted with this resource
    last_input_at: Option<Instant>,
    /// Last time a human performed a layout action on this resource
    last_layout_at: Option<Instant>,
}

/// State of a specific explicit lock
#[derive(Debug)]
struct LockState {
    /// When the lock was activated
    activated_at: Instant,
    /// How long the lock should last
    timeout: Duration,
    /// Reason for the lock (e.g. "Command Mode")
    reason: String,
}

impl LockState {
    fn is_active(&self) -> bool {
        self.activated_at.elapsed() < self.timeout
    }

    fn remaining_ms(&self) -> u64 {
        let elapsed = self.activated_at.elapsed();
        if elapsed >= self.timeout {
            0
        } else {
            (self.timeout - elapsed).as_millis() as u64
        }
    }
}

/// Human-Control Arbitrator
///
/// Central authority for deciding if an action from an actor is allowed
/// on a specific resource at this time.
pub struct Arbitrator {
    /// Explicit locks per client (e.g. while in prefix mode)
    locks: DashMap<ClientId, LockState>,
    /// Human activity timestamps per resource
    activity: DashMap<Resource, ActivityState>,
    /// Default lockout duration for input (ms)
    input_lockout_ms: u64,
    /// Default lockout duration for layout (ms)
    layout_lockout_ms: u64,
}

impl Default for Arbitrator {
    fn default() -> Self {
        Self::new()
    }
}

impl Arbitrator {
    /// Create a new arbitrator with default settings
    pub fn new() -> Self {
        Self {
            locks: DashMap::new(),
            activity: DashMap::new(),
            input_lockout_ms: 2000,  // 2 seconds human input lockout
            layout_lockout_ms: 5000, // 5 seconds human layout lockout
        }
    }

    /// Record human activity on a resource
    pub fn record_activity(&self, resource: Resource, action: Action) {
        let now = Instant::now();
        self.activity.entry(resource).and_modify(|s| {
            match action {
                Action::Input => s.last_input_at = Some(now),
                Action::Layout => s.last_layout_at = Some(now),
                _ => {}
            }
        }).or_insert_with(|| {
            let mut s = ActivityState { last_input_at: None, last_layout_at: None };
            match action {
                Action::Input => s.last_input_at = Some(now),
                Action::Layout => s.last_layout_at = Some(now),
                _ => {}
            }
            s
        });
    }

    /// Set an explicit lock for a client
    ///
    /// Called when client sends `UserCommandModeEntered`.
    pub fn set_lock(&self, client_id: ClientId, timeout_ms: u32, reason: String) {
        info!(
            client = %client_id,
            timeout_ms = timeout_ms,
            reason = %reason,
            "Explicit human control lock activated"
        );
        self.locks.insert(client_id, LockState {
            activated_at: Instant::now(),
            timeout: Duration::from_millis(timeout_ms as u64),
            reason,
        });
    }

    /// Release a client's explicit lock
    pub fn release_lock(&self, client_id: ClientId) {
        if self.locks.remove(&client_id).is_some() {
            debug!(client = %client_id, "Explicit human control lock released");
        }
    }

    /// Check if any client has an active explicit lock
    ///
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
            debug!(client = %client_id, "Cleaning up expired human control lock");
            self.locks.remove(&client_id);
        }

        active
    }

    /// Check if a specific client is currently subject to an arbitration block
    pub fn is_client_locked(&self, client_id: ClientId) -> bool {
        if let Some(entry) = self.locks.get(&client_id) {
            if entry.is_active() {
                return true;
            }
        }
        false
    }

    /// Check if an action is allowed
    ///
    /// Returns `ArbitrationResult::Allowed` if the actor can perform the action,
    /// otherwise returns `ArbitrationResult::Blocked` with details.
    pub fn check_access(&self, actor: Actor, resource: Resource, action: Action) -> ArbitrationResult {
        // Human actors always have access to everything
        if let Actor::Human(_) = actor {
            return ArbitrationResult::Allowed;
        }

        // Agent actors are subject to arbitration rules

        // 1. Check explicit locks (system-wide or session-wide)
        // Currently locks are per-client and apply system-wide
        if let Some((client_id, remaining_ms)) = self.is_any_lock_active() {
            let reason = self.locks.get(&client_id)
                .map(|l| l.reason.clone())
                .unwrap_or_else(|| "Human control mode".to_string());
            
            return ArbitrationResult::Blocked {
                client_id,
                remaining_ms,
                reason,
            };
        }

        // 2. Check implicit activity-based lockout per resource
        if let Some(state) = self.activity.get(&resource) {
            match action {
                Action::Input => {
                    // Block Agent input if there was recent Human input
                    if let Some(last) = state.last_input_at {
                        let elapsed = last.elapsed().as_millis() as u64;
                        if elapsed < self.input_lockout_ms {
                            return ArbitrationResult::Blocked {
                                client_id: ClientId::new(0), // System lock
                                remaining_ms: self.input_lockout_ms - elapsed,
                                reason: "Human input active".to_string(),
                            };
                        }
                    }
                }
                Action::Layout | Action::Focus | Action::Kill => {
                    // Layout changes blocked by recent Human layout changes
                    if let Some(last) = state.last_layout_at {
                        let elapsed = last.elapsed().as_millis() as u64;
                        if elapsed < self.layout_lockout_ms {
                            return ArbitrationResult::Blocked {
                                client_id: ClientId::new(0),
                                remaining_ms: self.layout_lockout_ms - elapsed,
                                reason: "Human layout active".to_string(),
                            };
                        }
                    }
                    // Layout changes also blocked by recent Human input (working in the pane)
                    if let Some(last) = state.last_input_at {
                        let elapsed = last.elapsed().as_millis() as u64;
                        if elapsed < self.input_lockout_ms {
                            return ArbitrationResult::Blocked {
                                client_id: ClientId::new(0),
                                remaining_ms: self.input_lockout_ms - elapsed,
                                reason: "Human input active".to_string(),
                            };
                        }
                    }
                }
            }
        }

        ArbitrationResult::Allowed
    }

    /// Remove locks when client disconnects
    pub fn on_client_disconnect(&self, client_id: ClientId) {
        self.release_lock(client_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    fn client_id(n: u64) -> ClientId {
        ClientId::new(n)
    }

    #[test]
    fn test_new_arbitrator() {
        let arb = Arbitrator::new();
        assert!(arb.is_any_lock_active().is_none());
    }

    #[test]
    fn test_explicit_lock() {
        let arb = Arbitrator::new();
        let id = client_id(1);

        arb.set_lock(id, 1000, "Command Mode".to_string());

        assert!(arb.is_client_locked(id));
        
        let result = arb.check_access(Actor::Agent, Resource::Global, Action::Focus);
        assert!(matches!(result, ArbitrationResult::Blocked { client_id, .. } if client_id == id));
    }

    #[test]
    fn test_human_always_allowed() {
        let arb = Arbitrator::new();
        let id = client_id(1);

        // Even with a lock, humans are allowed
        arb.set_lock(id, 1000, "Command Mode".to_string());
        
        let result = arb.check_access(Actor::Human(id), Resource::Global, Action::Focus);
        assert_eq!(result, ArbitrationResult::Allowed);
    }

    #[test]
    fn test_input_activity_lockout() {
        let arb = Arbitrator::new();
        let pane_id = Uuid::new_v4();
        let res = Resource::Pane(pane_id);

        arb.record_activity(res, Action::Input);

        // Agent should be blocked from input on this pane
        let result = arb.check_access(Actor::Agent, res, Action::Input);
        assert!(matches!(result, ArbitrationResult::Blocked { reason, .. } if reason == "Human input active"));

        // Agent should be allowed on OTHER panes
        let other_pane = Resource::Pane(Uuid::new_v4());
        let result = arb.check_access(Actor::Agent, other_pane, Action::Input);
        assert_eq!(result, ArbitrationResult::Allowed);

        // Wait for lockout to expire
        thread::sleep(Duration::from_millis(2100));
        let result = arb.check_access(Actor::Agent, res, Action::Input);
        assert_eq!(result, ArbitrationResult::Allowed);
    }

    #[test]
    fn test_layout_activity_lockout() {
        let arb = Arbitrator::new();
        let pane_id = Uuid::new_v4();
        let res = Resource::Pane(pane_id);

        arb.record_activity(res, Action::Layout);

        // Agent should be blocked from layout on this pane
        let result = arb.check_access(Actor::Agent, res, Action::Layout);
        assert!(matches!(result, ArbitrationResult::Blocked { reason, .. } if reason == "Human layout active"));

        // Layout also blocked by input
        arb.record_activity(res, Action::Input);
        let result = arb.check_access(Actor::Agent, res, Action::Layout);
        assert!(matches!(result, ArbitrationResult::Blocked { .. }));
    }

    #[test]
    fn test_client_disconnect_cleanup() {
        let arb = Arbitrator::new();
        let id = client_id(1);

        arb.set_lock(id, 1000, "Test".to_string());
        assert!(arb.is_client_locked(id));

        arb.on_client_disconnect(id);
        assert!(!arb.is_client_locked(id));
    }
}