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
    ClientCodec, ClientMessage, ClientType, ServerMessage, PROTOCOL_VERSION,
};
use ccmux_utils::socket_path;

use crate::mcp::error::McpError;
use super::health::{self, ConnectionState};

/// Reconnection delays in milliseconds (exponential backoff)
pub const RECONNECT_DELAYS_MS: &[u64] = &[100, 200, 400, 800, 1600];

/// Maximum number of reconnection attempts
pub const MAX_RECONNECT_ATTEMPTS: u8 = 5;

/// BUG-037 FIX: Timeout for waiting for a daemon response (in seconds)
pub const DAEMON_RESPONSE_TIMEOUT_SECS: u64 = 25;

/// Manages connection to the ccmux daemon
pub struct ConnectionManager {
    /// Channel for sending messages to daemon
    pub daemon_tx: Option<mpsc::Sender<ClientMessage>>,
    /// Channel for receiving messages from daemon
    /// BUG-037 FIX: Changed to unbounded to prevent I/O task blocking
    pub daemon_rx: Option<mpsc::UnboundedReceiver<ServerMessage>>,
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

        info!(
            socket_path = %socket.display(),
            client_id = %self.client_id,
            "Attempting to connect to ccmux daemon"
        );

        // Check if socket exists
        if !socket.exists() {
            error!(
                socket_path = %socket.display(),
                "Daemon socket does not exist - is the daemon running?"
            );
            return Err(McpError::DaemonNotRunning);
        }

        // Connect with retry logic
        let stream = self.connect_with_retry(&socket, 3, Duration::from_millis(500)).await?;
        debug!(socket_path = %socket.display(), "TCP connection established");

        // Create framed transport
        let framed = Framed::new(stream, ClientCodec::new());
        let (mut sink, mut stream) = framed.split();

        // Set up channels
        // BUG-037 FIX: Use unbounded channel for incoming messages to prevent I/O task blocking.
        // When the channel was bounded (32), heavy broadcast traffic (PTY output) could fill
        // the channel, causing `incoming_tx.send().await` to block. This blocked the entire
        // I/O task, preventing it from sending outgoing messages or receiving responses.
        // Tool calls would then timeout waiting for responses that could never arrive.
        let (daemon_tx, mut outgoing_rx) = mpsc::channel::<ClientMessage>(32);
        let (incoming_tx, daemon_rx) = mpsc::unbounded_channel::<ServerMessage>();

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
                                // FEAT-060: Filter out broadcast messages here to prevent
                                // channel flooding when attached to a session.
                                // The MCP bridge uses a request-response model and doesn't
                                // process unsolicited broadcasts.
                                if ConnectionManager::is_broadcast_message(&msg) {
                                    continue;
                                }

                                // BUG-037 FIX: Non-blocking send with unbounded channel
                                if incoming_tx.send(msg).is_err() {
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
            client_type: ClientType::Mcp,
        })
        .await?;

        // Wait for Connected response
        match self.recv_from_daemon().await? {
            ServerMessage::Connected { .. } => {
                info!(
                    client_id = %self.client_id,
                    "Successfully connected to ccmux daemon"
                );

                // FEAT-060: Update connection state to Connected
                {
                    let mut state = self.connection_state.write().await;
                    *state = ConnectionState::Connected;
                }
                let _ = self.state_tx.send(ConnectionState::Connected);
                info!(state = "Connected", "Connection state changed");

                // FEAT-060: Spawn health monitor task
                self.health_monitor_handle = Some(health::spawn_health_monitor(
                    self.daemon_tx.clone(),
                    self.state_tx.clone(),
                    self.connection_state.clone(),
                ));
                debug!("Health monitor task spawned");

                Ok(())
            }
            ServerMessage::Error { code, message, .. } => {
                error!(
                    error_code = ?code,
                    error_message = %message,
                    "Daemon rejected connection"
                );
                Err(McpError::DaemonError(format!("{:?}: {}", code, message)))
            }
            msg => {
                error!(
                    response = ?msg,
                    "Unexpected response from daemon during connection"
                );
                Err(McpError::UnexpectedResponse(format!("{:?}", msg)))
            }
        }
    }

    /// Attempt to reconnect to the daemon with exponential backoff
    pub async fn attempt_reconnection(&mut self) -> Result<(), McpError> {
        info!(
            client_id = %self.client_id,
            max_attempts = MAX_RECONNECT_ATTEMPTS,
            "Starting reconnection attempts to daemon"
        );

        for (attempt, delay_ms) in RECONNECT_DELAYS_MS.iter().enumerate() {
            let attempt_num = (attempt + 1) as u8;

            // Update state to show reconnection progress
            {
                let mut state = self.connection_state.write().await;
                *state = ConnectionState::Reconnecting { attempt: attempt_num };
            }
            let _ = self.state_tx.send(ConnectionState::Reconnecting { attempt: attempt_num });
            info!(
                state = "Reconnecting",
                attempt = attempt_num,
                max_attempts = MAX_RECONNECT_ATTEMPTS,
                "Connection state changed"
            );

            info!(
                attempt = attempt_num,
                max_attempts = MAX_RECONNECT_ATTEMPTS,
                delay_ms = delay_ms,
                "Waiting before reconnection attempt"
            );

            // Wait before attempting
            tokio::time::sleep(Duration::from_millis(*delay_ms)).await;

            // Clean up old resources
            self.daemon_tx = None;
            self.daemon_rx = None;
            self.health_monitor_handle = None;
            debug!("Cleaned up old connection resources");

            // Try to reconnect
            match self.connect_to_daemon().await {
                Ok(()) => {
                    info!(
                        attempt = attempt_num,
                        "Reconnection successful"
                    );
                    // connect_to_daemon already sets state to Connected
                    return Ok(());
                }
                Err(e) => {
                    warn!(
                        attempt = attempt_num,
                        max_attempts = MAX_RECONNECT_ATTEMPTS,
                        error = %e,
                        "Reconnection attempt failed"
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
        info!(state = "Disconnected", "Connection state changed");

        error!(
            attempts_made = MAX_RECONNECT_ATTEMPTS,
            "All reconnection attempts exhausted - daemon connection failed"
        );
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

            // BUG-043: Unwrap Sequenced messages before predicate check
            // The daemon wraps responses in Sequenced { seq, inner } for persistence tracking,
            // but predicates expect the unwrapped message types
            let msg = match self.recv_from_daemon_with_timeout(remaining).await? {
                ServerMessage::Sequenced { inner, .. } => *inner,
                other => other,
            };

            if predicate(&msg) {
                return Ok(msg);
            }
            if let ServerMessage::Error { code, message, .. } = msg {
                // Always return errors immediately unless the predicate specifically wanted them
                return Err(McpError::DaemonError(format!("{:?}: {}", code, message)));
            }
            // Not the message we're looking for, continue waiting
        }
    }

    /// Receive a response from the daemon, filtering out broadcast messages
    pub async fn recv_response_from_daemon(&mut self) -> Result<ServerMessage, McpError> {
        let msg = self.recv_filtered(|msg| !Self::is_broadcast_message(msg)).await?;
        // BUG-043: Unwrap Sequenced messages to get the actual response
        // The daemon wraps responses in Sequenced { seq, inner } for persistence tracking,
        // but tool handlers expect the unwrapped message types
        match msg {
            ServerMessage::Sequenced { inner, .. } => Ok(*inner),
            other => Ok(other),
        }
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
            // NOTE: PaneClosed is NOT filtered here because it's used as a direct response
            // to ClosePane requests. The tool_close_pane handler uses recv_filtered with a
            // predicate that checks for the specific pane_id, so spurious broadcasts are
            // ignored. Filtering PaneClosed here causes tool_close_pane to timeout (BUG-062).
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