use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::{mpsc, watch, RwLock};
use tokio::task::JoinHandle;
use tracing::{debug, info, warn};

use ccmux_protocol::ClientMessage;

/// Heartbeat interval in milliseconds
pub const HEARTBEAT_INTERVAL_MS: u64 = 1000;

/// Heartbeat timeout in milliseconds (detect loss within 2-3 seconds)
pub const HEARTBEAT_TIMEOUT_MS: u64 = 2000;

/// Connection state for daemon communication
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    /// Connected and healthy
    Connected,
    /// Connection lost, attempting recovery
    Reconnecting { attempt: u8 },
    /// Disconnected, recovery failed or not yet attempted
    Disconnected,
}

/// Spawn a background health monitoring task
pub fn spawn_health_monitor(
    daemon_tx: Option<mpsc::Sender<ClientMessage>>,
    state_tx: watch::Sender<ConnectionState>,
    connection_state: Arc<RwLock<ConnectionState>>,
) -> JoinHandle<()> {
    let mut state_rx = state_tx.subscribe();

    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_millis(HEARTBEAT_INTERVAL_MS));
        // Track when we last successfully communicated with daemon
        #[allow(unused_assignments)]
        let mut last_healthy = Instant::now();

        loop {
            tokio::select! {
                // Periodic heartbeat
                _ = interval.tick() => {
                    // Check if we've been signaled as disconnected
                    if *state_rx.borrow() == ConnectionState::Disconnected {
                        info!("Health monitor: detected disconnection signal");
                        break;
                    }

                    // Try to send a Ping
                    if let Some(ref tx) = daemon_tx {
                        match tx.send(ClientMessage::Ping).await {
                            Ok(()) => {
                                // Ping sent successfully, daemon is reachable
                                last_healthy = Instant::now();
                                debug!("Health monitor: ping sent successfully");
                            }
                            Err(_) => {
                                // Channel closed - daemon disconnected
                                warn!("Health monitor: failed to send ping, daemon disconnected");
                                {
                                    let mut state = connection_state.write().await;
                                    *state = ConnectionState::Disconnected;
                                }
                                let _ = state_tx.send(ConnectionState::Disconnected);
                                break;
                            }
                        }
                    } else {
                        // No daemon_tx - not connected
                        break;
                    }

                    // Check if we've exceeded the heartbeat timeout
                    if last_healthy.elapsed() > Duration::from_millis(HEARTBEAT_TIMEOUT_MS * 2) {
                        warn!("Health monitor: heartbeat timeout exceeded");
                        {
                            let mut state = connection_state.write().await;
                            *state = ConnectionState::Disconnected;
                        }
                        let _ = state_tx.send(ConnectionState::Disconnected);
                        break;
                    }
                }

                // Watch for state changes (disconnection signal from I/O task)
                result = state_rx.changed() => {
                    if result.is_err() {
                        break;
                    }
                    if *state_rx.borrow() == ConnectionState::Disconnected {
                        info!("Health monitor: received disconnection signal");
                        break;
                    }
                }
            }
        }

        info!("Health monitor task exiting");
    })
}
