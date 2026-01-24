//! FEAT-104: Watchdog Timer for Orchestration
//! FEAT-114: Named/Multiple Watchdogs
//!
//! Provides native timers that send periodic messages to panes, typically used
//! for watchdog agents that monitor worker agents. Supports multiple named
//! watchdogs running simultaneously.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use uuid::Uuid;

use crate::pty::PtyManager;

/// Default watchdog name when none is specified
pub const DEFAULT_WATCHDOG_NAME: &str = "default";

/// State of a single watchdog timer
#[derive(Debug, Clone)]
pub struct WatchdogState {
    /// Name identifier for this watchdog
    pub name: String,
    /// Target pane receiving the messages
    pub pane_id: Uuid,
    /// Interval between messages in seconds
    pub interval_secs: u64,
    /// Message to send
    pub message: String,
}

/// Watchdog timer manager
///
/// Manages multiple named watchdog timers that send periodic messages to panes.
/// Each watchdog is identified by a unique name.
pub struct WatchdogManager {
    /// Named watchdog states (name -> state)
    states: Mutex<HashMap<String, WatchdogState>>,
    /// Handles to cancel running timers (name -> cancel sender)
    cancel_txs: Mutex<HashMap<String, tokio::sync::oneshot::Sender<()>>>,
}

impl WatchdogManager {
    /// Create a new watchdog manager
    pub fn new() -> Self {
        Self {
            states: Mutex::new(HashMap::new()),
            cancel_txs: Mutex::new(HashMap::new()),
        }
    }

    /// Start a watchdog timer with the given name
    ///
    /// Returns the state of the started timer.
    /// If a timer with the same name is already running, it is stopped first.
    pub async fn start(
        &self,
        pane_id: Uuid,
        interval_secs: u64,
        message: Option<String>,
        name: Option<String>,
        pty_manager: Arc<RwLock<PtyManager>>,
    ) -> WatchdogState {
        let name = name.unwrap_or_else(|| DEFAULT_WATCHDOG_NAME.to_string());

        // Stop existing timer with this name if running
        self.stop_by_name(&name).await;

        let message = message.unwrap_or_else(|| "check".to_string());
        let state = WatchdogState {
            name: name.clone(),
            pane_id,
            interval_secs,
            message: message.clone(),
        };

        // Create cancellation channel
        let (cancel_tx, cancel_rx) = tokio::sync::oneshot::channel();

        // Store state and cancel handle
        self.states.lock().await.insert(name.clone(), state.clone());
        self.cancel_txs.lock().await.insert(name.clone(), cancel_tx);

        // Spawn the timer task
        let watchdog_name = name.clone();
        tokio::spawn(watchdog_timer_task(
            watchdog_name,
            pane_id,
            interval_secs,
            message.clone(),
            pty_manager,
            cancel_rx,
        ));

        tracing::info!(
            name = %name,
            pane_id = %pane_id,
            interval_secs = interval_secs,
            message = %message,
            "Watchdog timer started"
        );

        state
    }

    /// Stop a specific watchdog timer by name
    ///
    /// Returns true if a timer was running and was stopped.
    async fn stop_by_name(&self, name: &str) -> bool {
        let cancel_tx = self.cancel_txs.lock().await.remove(name);
        let was_running = cancel_tx.is_some();

        if let Some(tx) = cancel_tx {
            // Send cancel signal (ignore error if receiver already dropped)
            let _ = tx.send(());
        }

        self.states.lock().await.remove(name);

        if was_running {
            tracing::info!(name = %name, "Watchdog timer stopped");
        }

        was_running
    }

    /// Stop the watchdog timer
    ///
    /// If name is Some, stops only that specific watchdog.
    /// If name is None, stops all watchdogs.
    /// Returns true if at least one timer was stopped.
    pub async fn stop(&self, name: Option<String>) -> bool {
        match name {
            Some(n) => self.stop_by_name(&n).await,
            None => self.stop_all().await,
        }
    }

    /// Stop all running watchdog timers
    ///
    /// Returns true if at least one timer was stopped.
    pub async fn stop_all(&self) -> bool {
        let names: Vec<String> = self.states.lock().await.keys().cloned().collect();
        let mut any_stopped = false;

        for name in names {
            if self.stop_by_name(&name).await {
                any_stopped = true;
            }
        }

        if any_stopped {
            tracing::info!("All watchdog timers stopped");
        }

        any_stopped
    }

    /// Get the status of a specific watchdog or all watchdogs
    ///
    /// If name is Some, returns status of that specific watchdog.
    /// If name is None, returns status of all watchdogs.
    pub async fn status(&self, name: Option<String>) -> Vec<WatchdogState> {
        let states = self.states.lock().await;
        match name {
            Some(n) => states.get(&n).cloned().into_iter().collect(),
            None => states.values().cloned().collect(),
        }
    }

    /// Check if any watchdog is running
    pub async fn is_running(&self) -> bool {
        !self.states.lock().await.is_empty()
    }

    /// Check if a specific watchdog is running
    pub async fn is_running_by_name(&self, name: &str) -> bool {
        self.states.lock().await.contains_key(name)
    }

    /// Get count of running watchdogs
    pub async fn count(&self) -> usize {
        self.states.lock().await.len()
    }
}

impl Default for WatchdogManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Background task that sends periodic messages to a pane
async fn watchdog_timer_task(
    name: String,
    pane_id: Uuid,
    interval_secs: u64,
    message: String,
    pty_manager: Arc<RwLock<PtyManager>>,
    mut cancel_rx: tokio::sync::oneshot::Receiver<()>,
) {
    let interval = std::time::Duration::from_secs(interval_secs);

    loop {
        tokio::select! {
            // Wait for the interval
            _ = tokio::time::sleep(interval) => {
                // Send the message to the pane, then carriage return separately
                // (TUI apps like Claude Code expect Enter as a separate event - BUG-054)
                let pty_mgr = pty_manager.read().await;
                if let Some(handle) = pty_mgr.get(pane_id) {
                    // Send the message text
                    if let Err(e) = handle.write_all(message.as_bytes()) {
                        tracing::warn!(
                            name = %name,
                            pane_id = %pane_id,
                            error = %e,
                            "Failed to send watchdog message to pane"
                        );
                        // Continue trying - the pane might recover
                    } else {
                        // Flush to ensure text is sent
                        if let Err(e) = handle.flush() {
                            tracing::warn!(name = %name, pane_id = %pane_id, error = %e, "Failed to flush watchdog message");
                        }

                        // Small delay so TUI sees Enter as separate event
                        drop(pty_mgr); // Release lock during sleep
                        // BUG-071: Increased delay to 200ms to match send_input (BUG-054)
                        // 100ms was not sufficient for some TUI apps like Claude Code and Gemini CLI
                        tokio::time::sleep(std::time::Duration::from_millis(200)).await;

                        // Send carriage return to submit
                        let pty_mgr = pty_manager.read().await;
                        if let Some(handle) = pty_mgr.get(pane_id) {
                            if let Err(e) = handle.write_all(b"\r") {
                                tracing::warn!(
                                    name = %name,
                                    pane_id = %pane_id,
                                    error = %e,
                                    "Failed to send watchdog submit to pane"
                                );
                            } else {
                                // Flush again for the submit
                                if let Err(e) = handle.flush() {
                                    tracing::warn!(name = %name, pane_id = %pane_id, error = %e, "Failed to flush watchdog submit");
                                }

                                tracing::debug!(
                                    name = %name,
                                    pane_id = %pane_id,
                                    "Sent watchdog message"
                                );
                            }
                        }
                    }
                } else {
                    tracing::warn!(
                        name = %name,
                        pane_id = %pane_id,
                        "Watchdog target pane not found, timer will continue"
                    );
                    // Continue - the pane might be created later or we'll be stopped
                }
            }

            // Check for cancellation
            _ = &mut cancel_rx => {
                tracing::debug!(name = %name, "Watchdog timer task cancelled");
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_watchdog_manager_start_stop() {
        let manager = WatchdogManager::new();
        let pty_manager = Arc::new(RwLock::new(PtyManager::new()));

        // Initially not running
        assert!(!manager.is_running().await);
        assert!(manager.status(None).await.is_empty());

        // Start the timer with default name
        let pane_id = Uuid::new_v4();
        let state = manager
            .start(pane_id, 60, Some("test".to_string()), None, pty_manager.clone())
            .await;

        assert_eq!(state.name, DEFAULT_WATCHDOG_NAME);
        assert_eq!(state.pane_id, pane_id);
        assert_eq!(state.interval_secs, 60);
        assert_eq!(state.message, "test");

        // Should be running now
        assert!(manager.is_running().await);
        assert_eq!(manager.count().await, 1);

        // Stop the timer
        let was_running = manager.stop(None).await;
        assert!(was_running);

        // Should not be running anymore
        assert!(!manager.is_running().await);
        assert!(manager.status(None).await.is_empty());

        // Stopping again should return false
        let was_running = manager.stop(None).await;
        assert!(!was_running);
    }

    #[tokio::test]
    async fn test_watchdog_manager_default_message() {
        let manager = WatchdogManager::new();
        let pty_manager = Arc::new(RwLock::new(PtyManager::new()));

        let pane_id = Uuid::new_v4();
        let state = manager
            .start(pane_id, 90, None, None, pty_manager.clone())
            .await;

        assert_eq!(state.message, "check");

        manager.stop(None).await;
    }

    #[tokio::test]
    async fn test_watchdog_manager_restart_same_name() {
        let manager = WatchdogManager::new();
        let pty_manager = Arc::new(RwLock::new(PtyManager::new()));

        // Start first timer
        let pane_id_1 = Uuid::new_v4();
        manager
            .start(pane_id_1, 60, None, None, pty_manager.clone())
            .await;

        // Start second timer with same name (should stop first)
        let pane_id_2 = Uuid::new_v4();
        let state = manager
            .start(pane_id_2, 30, Some("ping".to_string()), None, pty_manager.clone())
            .await;

        // Should have second timer's state, still only one running
        assert_eq!(state.pane_id, pane_id_2);
        assert_eq!(state.interval_secs, 30);
        assert_eq!(state.message, "ping");
        assert_eq!(manager.count().await, 1);

        manager.stop(None).await;
    }

    #[tokio::test]
    async fn test_multiple_named_watchdogs() {
        let manager = WatchdogManager::new();
        let pty_manager = Arc::new(RwLock::new(PtyManager::new()));

        // Start watchdog "alpha"
        let pane_id_1 = Uuid::new_v4();
        let state1 = manager
            .start(pane_id_1, 60, Some("check-alpha".to_string()), Some("alpha".to_string()), pty_manager.clone())
            .await;
        assert_eq!(state1.name, "alpha");

        // Start watchdog "beta"
        let pane_id_2 = Uuid::new_v4();
        let state2 = manager
            .start(pane_id_2, 30, Some("check-beta".to_string()), Some("beta".to_string()), pty_manager.clone())
            .await;
        assert_eq!(state2.name, "beta");

        // Both should be running
        assert_eq!(manager.count().await, 2);
        assert!(manager.is_running_by_name("alpha").await);
        assert!(manager.is_running_by_name("beta").await);
        assert!(!manager.is_running_by_name("gamma").await);

        // Get status of all
        let all_status = manager.status(None).await;
        assert_eq!(all_status.len(), 2);

        // Get status of specific
        let alpha_status = manager.status(Some("alpha".to_string())).await;
        assert_eq!(alpha_status.len(), 1);
        assert_eq!(alpha_status[0].name, "alpha");
        assert_eq!(alpha_status[0].pane_id, pane_id_1);

        let beta_status = manager.status(Some("beta".to_string())).await;
        assert_eq!(beta_status.len(), 1);
        assert_eq!(beta_status[0].name, "beta");
        assert_eq!(beta_status[0].pane_id, pane_id_2);

        // Non-existent watchdog status
        let gamma_status = manager.status(Some("gamma".to_string())).await;
        assert!(gamma_status.is_empty());

        // Stop only "alpha"
        let stopped = manager.stop(Some("alpha".to_string())).await;
        assert!(stopped);
        assert_eq!(manager.count().await, 1);
        assert!(!manager.is_running_by_name("alpha").await);
        assert!(manager.is_running_by_name("beta").await);

        // Stop all remaining
        let stopped = manager.stop(None).await;
        assert!(stopped);
        assert_eq!(manager.count().await, 0);
    }

    #[tokio::test]
    async fn test_stop_nonexistent_watchdog() {
        let manager = WatchdogManager::new();

        // Stopping non-existent watchdog should return false
        let stopped = manager.stop(Some("nonexistent".to_string())).await;
        assert!(!stopped);
    }

    #[tokio::test]
    async fn test_backward_compatibility_no_name() {
        let manager = WatchdogManager::new();
        let pty_manager = Arc::new(RwLock::new(PtyManager::new()));

        // Start without name (uses "default")
        let pane_id = Uuid::new_v4();
        let state = manager
            .start(pane_id, 60, None, None, pty_manager.clone())
            .await;

        assert_eq!(state.name, DEFAULT_WATCHDOG_NAME);

        // Check status without name
        let status = manager.status(None).await;
        assert_eq!(status.len(), 1);
        assert_eq!(status[0].name, DEFAULT_WATCHDOG_NAME);

        // Stop without name
        let stopped = manager.stop(None).await;
        assert!(stopped);
        assert!(!manager.is_running().await);
    }
}
