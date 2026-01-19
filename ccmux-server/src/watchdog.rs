//! FEAT-104: Watchdog Timer for Orchestration
//!
//! Provides a native timer that sends periodic messages to a pane, typically used
//! for watchdog agents that monitor worker agents.

use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use uuid::Uuid;

use crate::pty::PtyManager;

/// State of the watchdog timer
#[derive(Debug, Clone)]
pub struct WatchdogState {
    /// Target pane receiving the messages
    pub pane_id: Uuid,
    /// Interval between messages in seconds
    pub interval_secs: u64,
    /// Message to send
    pub message: String,
}

/// Watchdog timer manager
///
/// Manages a single watchdog timer that sends periodic messages to a pane.
/// Only one watchdog timer can be active at a time.
pub struct WatchdogManager {
    /// Current watchdog state (None if not running)
    state: Mutex<Option<WatchdogState>>,
    /// Handle to cancel the running timer
    cancel_tx: Mutex<Option<tokio::sync::oneshot::Sender<()>>>,
}

impl WatchdogManager {
    /// Create a new watchdog manager
    pub fn new() -> Self {
        Self {
            state: Mutex::new(None),
            cancel_tx: Mutex::new(None),
        }
    }

    /// Start the watchdog timer
    ///
    /// Returns the state of the started timer.
    /// If a timer is already running, it is stopped first.
    pub async fn start(
        &self,
        pane_id: Uuid,
        interval_secs: u64,
        message: Option<String>,
        pty_manager: Arc<RwLock<PtyManager>>,
    ) -> WatchdogState {
        // Stop existing timer if running
        self.stop().await;

        let message = message.unwrap_or_else(|| "check".to_string());
        let state = WatchdogState {
            pane_id,
            interval_secs,
            message: message.clone(),
        };

        // Create cancellation channel
        let (cancel_tx, cancel_rx) = tokio::sync::oneshot::channel();

        // Store state
        *self.state.lock().await = Some(state.clone());
        *self.cancel_tx.lock().await = Some(cancel_tx);

        // Spawn the timer task
        let message_to_send = format!("{}\n", message); // Add newline like Enter key
        tokio::spawn(watchdog_timer_task(
            pane_id,
            interval_secs,
            message_to_send,
            pty_manager,
            cancel_rx,
        ));

        tracing::info!(
            pane_id = %pane_id,
            interval_secs = interval_secs,
            message = %message,
            "Watchdog timer started"
        );

        state
    }

    /// Stop the watchdog timer
    ///
    /// Returns true if a timer was running and was stopped.
    pub async fn stop(&self) -> bool {
        let cancel_tx = self.cancel_tx.lock().await.take();
        let was_running = cancel_tx.is_some();

        if let Some(tx) = cancel_tx {
            // Send cancel signal (ignore error if receiver already dropped)
            let _ = tx.send(());
        }

        *self.state.lock().await = None;

        if was_running {
            tracing::info!("Watchdog timer stopped");
        }

        was_running
    }

    /// Get the current watchdog status
    pub async fn status(&self) -> Option<WatchdogState> {
        self.state.lock().await.clone()
    }

    /// Check if the watchdog is running
    pub async fn is_running(&self) -> bool {
        self.state.lock().await.is_some()
    }
}

impl Default for WatchdogManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Background task that sends periodic messages to a pane
async fn watchdog_timer_task(
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
                // Send the message to the pane
                let pty_mgr = pty_manager.read().await;
                if let Some(handle) = pty_mgr.get(pane_id) {
                    if let Err(e) = handle.write_all(message.as_bytes()) {
                        tracing::warn!(
                            pane_id = %pane_id,
                            error = %e,
                            "Failed to send watchdog message to pane"
                        );
                        // Continue trying - the pane might recover
                    } else {
                        tracing::debug!(
                            pane_id = %pane_id,
                            "Sent watchdog message"
                        );
                    }
                } else {
                    tracing::warn!(
                        pane_id = %pane_id,
                        "Watchdog target pane not found, timer will continue"
                    );
                    // Continue - the pane might be created later or we'll be stopped
                }
            }

            // Check for cancellation
            _ = &mut cancel_rx => {
                tracing::debug!("Watchdog timer task cancelled");
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
        assert!(manager.status().await.is_none());

        // Start the timer
        let pane_id = Uuid::new_v4();
        let state = manager
            .start(pane_id, 60, Some("test".to_string()), pty_manager.clone())
            .await;

        assert_eq!(state.pane_id, pane_id);
        assert_eq!(state.interval_secs, 60);
        assert_eq!(state.message, "test");

        // Should be running now
        assert!(manager.is_running().await);
        assert!(manager.status().await.is_some());

        // Stop the timer
        let was_running = manager.stop().await;
        assert!(was_running);

        // Should not be running anymore
        assert!(!manager.is_running().await);
        assert!(manager.status().await.is_none());

        // Stopping again should return false
        let was_running = manager.stop().await;
        assert!(!was_running);
    }

    #[tokio::test]
    async fn test_watchdog_manager_default_message() {
        let manager = WatchdogManager::new();
        let pty_manager = Arc::new(RwLock::new(PtyManager::new()));

        let pane_id = Uuid::new_v4();
        let state = manager
            .start(pane_id, 90, None, pty_manager.clone())
            .await;

        assert_eq!(state.message, "check");

        manager.stop().await;
    }

    #[tokio::test]
    async fn test_watchdog_manager_restart() {
        let manager = WatchdogManager::new();
        let pty_manager = Arc::new(RwLock::new(PtyManager::new()));

        // Start first timer
        let pane_id_1 = Uuid::new_v4();
        manager
            .start(pane_id_1, 60, None, pty_manager.clone())
            .await;

        // Start second timer (should stop first)
        let pane_id_2 = Uuid::new_v4();
        let state = manager
            .start(pane_id_2, 30, Some("ping".to_string()), pty_manager.clone())
            .await;

        // Should have second timer's state
        assert_eq!(state.pane_id, pane_id_2);
        assert_eq!(state.interval_secs, 30);
        assert_eq!(state.message, "ping");

        manager.stop().await;
    }
}
