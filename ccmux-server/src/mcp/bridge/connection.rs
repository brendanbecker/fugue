use std::sync::Arc;
use std::time::{Duration, Instant};

use futures::{SinkExt, StreamExt};
use tokio::net::UnixStream;
use tokio::sync::{mpsc, watch, RwLock};
use tokio::task::JoinHandle;
use tokio_util::codec::Framed;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use ccmux_protocol::{
    ClientCodec, ClientMessage, ServerMessage, PROTOCOL_VERSION,
};
use ccmux_utils::socket_path;

use crate::mcp::error::McpError;
use super::types::{
    ConnectionState, DAEMON_RESPONSE_TIMEOUT_SECS, HEARTBEAT_INTERVAL_MS, HEARTBEAT_TIMEOUT_MS,
    MAX_RECONNECT_ATTEMPTS, RECONNECT_DELAYS_MS,
};

/// Manages connection to the ccmux daemon
pub struct ConnectionManager {
    /// Channel for sending messages to daemon
    pub daemon_tx: Option<mpsc::Sender<ClientMessage>>,
    /// Channel for receiving messages from daemon
    pub daemon_rx: Option<mpsc::Receiver<ServerMessage>>,
    /// Client ID for daemon connection
    client_id: Uuid,
    /// Current connection state (shared with health monitor)
    pub connection_state: Arc<RwLock<ConnectionState>>,
    /// Watch channel sender for state updates
    pub state_tx: watch::Sender<ConnectionState>,
    /// Watch channel receiver for state updates
    #[allow(dead_code)]
    state_rx: watch::Receiver<ConnectionState>,
    /// Handle to health monitor task (for cleanup)
    #[allow(dead_code)]
    health_monitor_handle: Option<JoinHandle<()>>,
}

impl ConnectionManager {
    pub fn new() -> Self {
        let (state_tx, state_rx) = watch::channel(ConnectionState::Disconnected);

        Self {
            daemon_tx: None,
            daemon_rx: None,
            client_id: Uuid::new_v4(),
            connection_state: Arc::new(RwLock::new(ConnectionState::Disconnected)),
            state_tx,
            state_rx,
            health_monitor_handle: None,
        }
    }

    /// Connect to the ccmux daemon
    pub async fn connect_to_daemon(&mut self) -> Result<(), McpError> {
        let socket = socket_path();

        // Check if socket exists
        if !socket.exists() {
            return Err(McpError::DaemonNotRunning);
        }

        // Connect with retry logic
        let stream = self.connect_with_retry(&socket, 3, Duration::from_millis(500)).await?;

        // Create framed transport
        let framed = Framed::new(stream, ClientCodec::new());
        let (mut sink, mut stream) = framed.split();

        // Set up channels
        let (daemon_tx, mut outgoing_rx) = mpsc::channel::<ClientMessage>(32);
        let (incoming_tx, daemon_rx) = mpsc::channel::<ServerMessage>(32);

        self.daemon_tx = Some(daemon_tx);
        self.daemon_rx = Some(daemon_rx);

        // FEAT-060: Clone state_tx for the I/O task to signal disconnection
        let io_state_tx = self.state_tx.clone();

        // Spawn task to handle socket I/O
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    // Send outgoing messages
                    Some(msg) = outgoing_rx.recv() => {
                        if let Err(e) = sink.send(msg).await {
                            error!("Failed to send to daemon: {}", e);
                            // FEAT-060: Signal disconnection when send fails
                            let _ = io_state_tx.send(ConnectionState::Disconnected);
                            break;
                        }
                    }
                    // Receive incoming messages
                    result = stream.next() => {
                        match result {
                            Some(Ok(msg)) => {
                                if incoming_tx.send(msg).await.is_err() {
                                    break; // Receiver dropped
                                }
                            }
                            Some(Err(e)) => {
                                error!("Failed to receive from daemon: {}", e);
                                // FEAT-060: Signal disconnection on receive error
                                let _ = io_state_tx.send(ConnectionState::Disconnected);
                                break;
                            }
                            None => {
                                info!("Daemon connection closed");
                                // FEAT-060: Signal disconnection when connection closes
                                let _ = io_state_tx.send(ConnectionState::Disconnected);
                                break;
                            }
                        }
                    }
                }
            }
        });

        // Send Connect message to establish session with daemon
        self.send_to_daemon(ClientMessage::Connect {
            client_id: self.client_id,
            protocol_version: PROTOCOL_VERSION,
        })
        .await?;

        // Wait for Connected response
        match self.recv_from_daemon().await? {
            ServerMessage::Connected { .. } => {
                info!("Connected to ccmux daemon");

                // FEAT-060: Update connection state to Connected
                {
                    let mut state = self.connection_state.write().await;
                    *state = ConnectionState::Connected;
                }
                let _ = self.state_tx.send(ConnectionState::Connected);

                // FEAT-060: Spawn health monitor task
                self.health_monitor_handle = Some(self.spawn_health_monitor());

                Ok(())
            }
            ServerMessage::Error { code, message } => {
                Err(McpError::DaemonError(format!("{:?}: {}", code, message)))
            }
            msg => Err(McpError::UnexpectedResponse(format!("{:?}", msg))),
        }
    }

    /// Spawn a background health monitoring task
    fn spawn_health_monitor(&self) -> JoinHandle<()> {
        let daemon_tx = self.daemon_tx.clone();
        let state_tx = self.state_tx.clone();
        let connection_state = self.connection_state.clone();
        let mut state_rx = self.state_tx.subscribe();

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

    /// Attempt to reconnect to the daemon with exponential backoff
    pub async fn attempt_reconnection(&mut self) -> Result<(), McpError> {
        info!("Starting reconnection attempts");

        for (attempt, delay_ms) in RECONNECT_DELAYS_MS.iter().enumerate() {
            let attempt_num = (attempt + 1) as u8;

            // Update state to show reconnection progress
            {
                let mut state = self.connection_state.write().await;
                *state = ConnectionState::Reconnecting { attempt: attempt_num };
            }
            let _ = self.state_tx.send(ConnectionState::Reconnecting { attempt: attempt_num });

            info!(
                "Reconnection attempt {}/{}: waiting {}ms before trying",
                attempt_num,
                MAX_RECONNECT_ATTEMPTS,
                delay_ms
            );

            // Wait before attempting
            tokio::time::sleep(Duration::from_millis(*delay_ms)).await;

            // Clean up old resources
            self.daemon_tx = None;
            self.daemon_rx = None;
            self.health_monitor_handle = None;

            // Try to reconnect
            match self.connect_to_daemon().await {
                Ok(()) => {
                    info!("Reconnection successful on attempt {}", attempt_num);
                    // connect_to_daemon already sets state to Connected
                    return Ok(());
                }
                Err(e) => {
                    warn!(
                        "Reconnection attempt {} failed: {}",
                        attempt_num, e
                    );
                }
            }
        }

        // All attempts failed
        {
            let mut state = self.connection_state.write().await;
            *state = ConnectionState::Disconnected;
        }
        let _ = self.state_tx.send(ConnectionState::Disconnected);

        error!("All {} reconnection attempts exhausted", MAX_RECONNECT_ATTEMPTS);
        Err(McpError::RecoveryFailed { attempts: MAX_RECONNECT_ATTEMPTS })
    }

    async fn connect_with_retry(
        &self,
        socket: &std::path::Path,
        retries: u32,
        delay: Duration,
    ) -> Result<UnixStream, McpError> {
        let mut last_error = None;

        for attempt in 0..retries {
            match UnixStream::connect(socket).await {
                Ok(stream) => return Ok(stream),
                Err(e) => {
                    warn!(
                        "Connection attempt {} failed: {} (retrying in {:?})",
                        attempt + 1,
                        e,
                        delay
                    );
                    last_error = Some(e);
                    tokio::time::sleep(delay).await;
                }
            }
        }

        Err(McpError::ConnectionFailed(
            last_error.map(|e| e.to_string()).unwrap_or_default(),
        ))
    }

    /// Send a message to the daemon
    pub async fn send_to_daemon(&self, msg: ClientMessage) -> Result<(), McpError> {
        let tx = self
            .daemon_tx
            .as_ref()
            .ok_or(McpError::NotConnected)?;

        tx.send(msg)
            .await
            .map_err(|_| McpError::DaemonDisconnected)
    }

    /// Receive a message from the daemon (raw, includes broadcasts)
    pub async fn recv_from_daemon(&mut self) -> Result<ServerMessage, McpError> {
        let rx = self
            .daemon_rx
            .as_mut()
            .ok_or(McpError::NotConnected)?;

        rx.recv()
            .await
            .ok_or(McpError::DaemonDisconnected)
    }

    /// Receive a message from the daemon with a timeout
    pub async fn recv_from_daemon_with_timeout(
        &mut self,
        timeout: Duration,
    ) -> Result<ServerMessage, McpError> {
        match tokio::time::timeout(timeout, self.recv_from_daemon()).await {
            Ok(result) => result,
            Err(_) => Err(McpError::ResponseTimeout {
                seconds: timeout.as_secs(),
            }),
        }
    }

    /// Receive a response from the daemon, filtering based on a predicate
    pub async fn recv_filtered<F>(&mut self, mut predicate: F) -> Result<ServerMessage, McpError>
    where
        F: FnMut(&ServerMessage) -> bool,
    {
        let timeout_duration = Duration::from_secs(DAEMON_RESPONSE_TIMEOUT_SECS);
        let deadline = Instant::now() + timeout_duration;

        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                warn!(
                    "Timeout waiting for daemon response after {}s",
                    DAEMON_RESPONSE_TIMEOUT_SECS
                );
                return Err(McpError::ResponseTimeout {
                    seconds: DAEMON_RESPONSE_TIMEOUT_SECS,
                });
            }

            match self.recv_from_daemon_with_timeout(remaining).await? {
                msg if predicate(&msg) => return Ok(msg),
                ServerMessage::Error { code, message } => {
                    // Always return errors immediately unless the predicate specifically wanted them
                    return Err(McpError::DaemonError(format!("{:?}: {}", code, message)));
                }
                _ => continue,
            }
        }
    }

    /// Receive a response from the daemon, filtering out broadcast messages
    pub async fn recv_response_from_daemon(&mut self) -> Result<ServerMessage, McpError> {
        self.recv_filtered(|msg| !Self::is_broadcast_message(msg)).await
    }

    /// Check if a message is a broadcast
    pub fn is_broadcast_message(msg: &ServerMessage) -> bool {
        matches!(
            msg,
            // Terminal output from panes
            ServerMessage::Output { .. }
            // Pane state changes (normal, claude, exited)
            | ServerMessage::PaneStateChanged { .. }
            // Claude activity updates
            | ServerMessage::ClaudeStateChanged { .. }
            // Simple session created (broadcast)
            | ServerMessage::SessionCreated { .. }
            // Simple pane created (broadcast from other clients, not the WithDetails response)
            | ServerMessage::PaneCreated { .. }
            // Pane closed notifications (broadcast)
            | ServerMessage::PaneClosed { .. }
            // Simple window created (broadcast from other clients)
            | ServerMessage::WindowCreated { .. }
            // Window closed notifications
            | ServerMessage::WindowClosed { .. }
            // Session ended notifications
            | ServerMessage::SessionEnded { .. }
            // BUG-038 FIX: Session list change broadcasts
            | ServerMessage::SessionsChanged { .. }
            // Viewport updates
            | ServerMessage::ViewportUpdated { .. }
            // Orchestration messages from other sessions
            | ServerMessage::OrchestrationReceived { .. }
            // FEAT-060: Pong responses from health monitor Pings
            | ServerMessage::Pong
            // BUG-029 FIX: Focus change broadcasts
            | ServerMessage::SessionFocused { .. }
            | ServerMessage::WindowFocused { .. }
            | ServerMessage::PaneFocused { .. }
            // FEAT-058: Beads query integration broadcasts
            | ServerMessage::BeadsStatusUpdate { .. }
            | ServerMessage::BeadsReadyList { .. }
        )
    }
}
