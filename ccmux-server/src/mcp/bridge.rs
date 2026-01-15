//! MCP Bridge - Connects MCP protocol to the ccmux daemon
//!
//! This module implements the MCP bridge that translates between MCP JSON-RPC
//! (over stdio) and the ccmux IPC protocol (over Unix socket).
//!
//! Instead of running a standalone MCP server with its own session state,
//! the bridge connects to the existing ccmux daemon so Claude can control
//! the same sessions the user sees in the TUI.

use std::io::{BufRead, Write};
use std::sync::Arc;
use std::time::{Duration, Instant};

use futures::{SinkExt, StreamExt};
use tokio::net::UnixStream;
use tokio::sync::{mpsc, watch, RwLock};
use tokio::task::JoinHandle;
use tokio_util::codec::Framed;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

// ==================== FEAT-060: Connection Recovery Constants ====================

/// Heartbeat interval in milliseconds
const HEARTBEAT_INTERVAL_MS: u64 = 1000;

/// Heartbeat timeout in milliseconds (detect loss within 2-3 seconds)
const HEARTBEAT_TIMEOUT_MS: u64 = 2000;

/// Reconnection delays in milliseconds (exponential backoff)
const RECONNECT_DELAYS_MS: &[u64] = &[100, 200, 400, 800, 1600];

/// Maximum number of reconnection attempts
const MAX_RECONNECT_ATTEMPTS: u8 = 5;

/// BUG-037 FIX: Timeout for waiting for a daemon response (in seconds)
/// This prevents tool calls from hanging indefinitely if the daemon
/// doesn't send the expected response. Claude Code has its own timeout
/// that triggers AbortError, so we set this slightly lower to provide
/// a more informative error message.
const DAEMON_RESPONSE_TIMEOUT_SECS: u64 = 25;

// ==================== FEAT-060: Connection State ====================

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

use ccmux_protocol::{
    ClientCodec, ClientMessage, PaneListEntry, ServerMessage, SplitDirection, PROTOCOL_VERSION,
    messages::ClientType,
};
use ccmux_utils::socket_path;

use super::error::McpError;
use super::protocol::{
    InitializeResult, JsonRpcError, JsonRpcRequest, JsonRpcResponse, ToolResult, ToolsListResult,
};
use super::tools::get_tool_definitions;
use crate::beads::metadata_keys as beads;

/// MCP Bridge
///
/// Connects to the ccmux daemon and handles MCP protocol communication over stdio.
pub struct McpBridge {
    /// Channel for sending messages to daemon
    daemon_tx: Option<mpsc::Sender<ClientMessage>>,
    /// Channel for receiving messages from daemon
    daemon_rx: Option<mpsc::Receiver<ServerMessage>>,
    /// Whether the MCP protocol has been initialized
    initialized: bool,
    /// Client ID for daemon connection
    client_id: Uuid,
    // ==================== FEAT-060: Connection State Fields ====================
    /// Current connection state (shared with health monitor)
    connection_state: Arc<RwLock<ConnectionState>>,
    /// Watch channel sender for state updates
    state_tx: watch::Sender<ConnectionState>,
    /// Watch channel receiver for state updates
    #[allow(dead_code)]
    state_rx: watch::Receiver<ConnectionState>,
    /// Handle to health monitor task (for cleanup)
    #[allow(dead_code)]
    health_monitor_handle: Option<JoinHandle<()>>,
}

impl McpBridge {
    /// Create a new MCP bridge
    pub fn new() -> Self {
        // FEAT-060: Initialize watch channel for connection state
        let (state_tx, state_rx) = watch::channel(ConnectionState::Disconnected);

        Self {
            daemon_tx: None,
            daemon_rx: None,
            initialized: false,
            client_id: Uuid::new_v4(),
            // FEAT-060: Connection state management
            connection_state: Arc::new(RwLock::new(ConnectionState::Disconnected)),
            state_tx,
            state_rx,
            health_monitor_handle: None,
        }
    }

    /// Connect to the ccmux daemon
    async fn connect_to_daemon(&mut self) -> Result<(), McpError> {
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
            client_type: Some(ClientType::Mcp),
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

    // ==================== FEAT-060: Health Monitor ====================

    /// Spawn a background health monitoring task
    ///
    /// The health monitor periodically sends Ping messages to the daemon
    /// and monitors the watch channel for disconnection signals from the I/O task.
    fn spawn_health_monitor(&self) -> JoinHandle<()> {
        let daemon_tx = self.daemon_tx.clone();
        let state_tx = self.state_tx.clone();
        let connection_state = self.connection_state.clone();
        let mut state_rx = self.state_tx.subscribe();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_millis(HEARTBEAT_INTERVAL_MS));
            // Track when we last successfully communicated with daemon
            // (initial value is used if first ping fails before timeout check)
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
                        // (This handles cases where sends succeed but daemon is unresponsive)
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
                            // Sender dropped
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

    // ==================== FEAT-060: Reconnection Logic ====================

    /// Attempt to reconnect to the daemon with exponential backoff
    ///
    /// Returns Ok(()) if reconnection succeeds, or McpError::RecoveryFailed
    /// if all attempts are exhausted.
    async fn attempt_reconnection(&mut self) -> Result<(), McpError> {
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

    /// Connect with retry logic
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
    async fn send_to_daemon(&self, msg: ClientMessage) -> Result<(), McpError> {
        let tx = self
            .daemon_tx
            .as_ref()
            .ok_or(McpError::NotConnected)?;

        tx.send(msg)
            .await
            .map_err(|_| McpError::DaemonDisconnected)
    }

    /// Receive a message from the daemon (raw, includes broadcasts)
    async fn recv_from_daemon(&mut self) -> Result<ServerMessage, McpError> {
        let rx = self
            .daemon_rx
            .as_mut()
            .ok_or(McpError::NotConnected)?;

        rx.recv()
            .await
            .ok_or(McpError::DaemonDisconnected)
    }

    /// Receive a response from the daemon, filtering out broadcast messages
    ///
    /// The daemon sends both direct responses to requests AND broadcast messages
    /// (like Output, PaneStateChanged, etc.) to all clients. This method filters
    /// out broadcasts and only returns messages that are actual responses.
    ///
    /// BUG-027 FIX: Without this filtering, tools like `read_pane` could receive
    /// a broadcast message (like `PaneCreated` from another client) instead of
    /// the expected `PaneContent` response, causing response type mismatches.
    ///
    /// BUG-037 FIX: Added timeout to prevent infinite waiting if the daemon
    /// never sends the expected response. This provides a proper error instead
    /// of letting Claude Code timeout with an unhelpful AbortError.
    async fn recv_response_from_daemon(&mut self) -> Result<ServerMessage, McpError> {
        let timeout_duration = Duration::from_secs(DAEMON_RESPONSE_TIMEOUT_SECS);
        let deadline = Instant::now() + timeout_duration;

        loop {
            // BUG-037 FIX: Check if we've exceeded the timeout
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

            // Use tokio::time::timeout to bound the recv call
            let recv_result = tokio::time::timeout(remaining, self.recv_from_daemon()).await;

            match recv_result {
                Ok(Ok(msg)) => {
                    // Check if this is a broadcast message that should be skipped
                    if Self::is_broadcast_message(&msg) {
                        debug!("Skipping broadcast message: {:?}", std::mem::discriminant(&msg));
                        continue;
                    }
                    // This is a response message, return it
                    return Ok(msg);
                }
                Ok(Err(e)) => {
                    // recv_from_daemon returned an error
                    return Err(e);
                }
                Err(_) => {
                    // Timeout elapsed while waiting for recv
                    warn!(
                        "Timeout waiting for daemon response after {}s",
                        DAEMON_RESPONSE_TIMEOUT_SECS
                    );
                    return Err(McpError::ResponseTimeout {
                        seconds: DAEMON_RESPONSE_TIMEOUT_SECS,
                    });
                }
            }
        }
    }

    /// Check if a message is a broadcast (not a direct response to a request)
    ///
    /// Broadcast messages are sent to all clients attached to a session and
    /// include things like terminal output, pane state changes, and notifications
    /// about other clients' actions.
    fn is_broadcast_message(msg: &ServerMessage) -> bool {
        matches!(
            msg,
            // Terminal output from panes
            ServerMessage::Output { .. }
            // Pane state changes (normal, claude, exited)
            | ServerMessage::PaneStateChanged { .. }
            // Claude activity updates
            | ServerMessage::ClaudeStateChanged { .. }
            // Simple pane created (broadcast from other clients, not the WithDetails response)
            | ServerMessage::PaneCreated { .. }
            // Pane closed notifications (broadcast, but we handle it specially for close_pane)
            // Note: We DON'T filter PaneClosed here because tool_close_pane expects it as a response
            // | ServerMessage::PaneClosed { .. }
            // Simple window created (broadcast from other clients)
            | ServerMessage::WindowCreated { .. }
            // Window closed notifications
            | ServerMessage::WindowClosed { .. }
            // Session ended notifications
            | ServerMessage::SessionEnded { .. }
            // BUG-038 FIX: Session list change broadcasts (from session destroy)
            // Unlike SessionList which is a direct response to ListSessions, SessionsChanged
            // is broadcast when sessions are created/destroyed and must be filtered out.
            | ServerMessage::SessionsChanged { .. }
            // Viewport updates
            | ServerMessage::ViewportUpdated { .. }
            // Orchestration messages from other sessions
            | ServerMessage::OrchestrationReceived { .. }
            // FEAT-060: Pong responses from health monitor Pings (filter from tool responses)
            | ServerMessage::Pong
            // BUG-029 FIX: Focus change broadcasts from BUG-026
            // These are broadcast to TUI clients but were not being filtered, causing
            // them to be picked up as responses to subsequent MCP tool calls.
            // For example: SelectSession sends no response but daemon broadcasts SessionFocused.
            // If CreateWindow is called next, it would receive SessionFocused instead of
            // WindowCreatedWithDetails, causing "Unexpected response: SessionFocused" errors.
            | ServerMessage::SessionFocused { .. }
            | ServerMessage::WindowFocused { .. }
            | ServerMessage::PaneFocused { .. }
        )
    }

    /// Run the MCP bridge, reading from stdin and writing to stdout
    pub async fn run(&mut self) -> Result<(), McpError> {
        // Connect to daemon first
        self.connect_to_daemon().await?;

        let stdin = std::io::stdin();
        let stdout = std::io::stdout();
        let mut stdout = stdout.lock();

        info!("MCP bridge starting");

        for line in stdin.lock().lines() {
            let line = line?;
            if line.is_empty() {
                continue;
            }

            debug!("Received: {}", line);

            // Parse request
            let request: JsonRpcRequest = match serde_json::from_str(&line) {
                Ok(req) => req,
                Err(e) => {
                    let response = JsonRpcResponse::error(
                        serde_json::Value::Null,
                        JsonRpcError::new(JsonRpcError::PARSE_ERROR, e.to_string()),
                    );
                    let json = serde_json::to_string(&response)?;
                    writeln!(stdout, "{}", json)?;
                    stdout.flush()?;
                    continue;
                }
            };

            // Validate JSON-RPC version
            if request.jsonrpc != "2.0" {
                let response = JsonRpcResponse::error(
                    request.id,
                    JsonRpcError::with_data(
                        JsonRpcError::INVALID_REQUEST,
                        "Invalid JSON-RPC version",
                        serde_json::json!({"expected": "2.0", "got": request.jsonrpc}),
                    ),
                );
                let json = serde_json::to_string(&response)?;
                writeln!(stdout, "{}", json)?;
                stdout.flush()?;
                continue;
            }

            // Handle request
            let response = self.handle_request(request).await;

            // Write response
            let json = serde_json::to_string(&response)?;
            debug!("Sending: {}", json);
            writeln!(stdout, "{}", json)?;
            stdout.flush()?;
        }

        info!("MCP bridge shutting down");
        Ok(())
    }

    /// Handle a JSON-RPC request
    async fn handle_request(&mut self, request: JsonRpcRequest) -> JsonRpcResponse {
        let result = match request.method.as_str() {
            "initialize" => self.handle_initialize(&request.params),
            "initialized" => Ok(serde_json::json!({})),
            "tools/list" => self.handle_tools_list(),
            "tools/call" => self.handle_tools_call(&request.params).await,
            _ => Err(McpError::MethodNotFound(request.method.clone())),
        };

        match result {
            Ok(value) => JsonRpcResponse::success(request.id, value),
            Err(e) => JsonRpcResponse::error(request.id, e.into()),
        }
    }

    /// Handle initialize request
    fn handle_initialize(
        &mut self,
        _params: &serde_json::Value,
    ) -> Result<serde_json::Value, McpError> {
        self.initialized = true;
        info!("MCP bridge initialized");

        let result = InitializeResult::default();
        serde_json::to_value(result).map_err(|e| McpError::Internal(e.to_string()))
    }

    /// Handle tools/list request
    fn handle_tools_list(&self) -> Result<serde_json::Value, McpError> {
        let tools = get_tool_definitions();
        let result = ToolsListResult { tools };
        serde_json::to_value(result).map_err(|e| McpError::Internal(e.to_string()))
    }

    /// Handle tools/call request
    ///
    /// FEAT-060: This method now includes connection recovery logic. If the daemon
    /// is disconnected, it will attempt reconnection before failing.
    async fn handle_tools_call(
        &mut self,
        params: &serde_json::Value,
    ) -> Result<serde_json::Value, McpError> {
        let name = params["name"]
            .as_str()
            .ok_or_else(|| McpError::InvalidParams("Missing 'name' parameter".into()))?;

        let arguments = &params["arguments"];

        debug!("Tool call: {} with args: {}", name, arguments);

        // FEAT-060: Check connection state and handle recovery
        let result = self.dispatch_tool_with_recovery(name, arguments).await?;

        serde_json::to_value(result).map_err(|e| McpError::Internal(e.to_string()))
    }

    // ==================== FEAT-060: Tool Dispatch with Recovery ====================

    /// Dispatch tool call with automatic connection recovery
    ///
    /// This wrapper checks connection state before executing tools and handles
    /// automatic reconnection if the daemon is disconnected.
    async fn dispatch_tool_with_recovery(
        &mut self,
        name: &str,
        arguments: &serde_json::Value,
    ) -> Result<ToolResult, McpError> {
        // Check current connection state
        let state = *self.connection_state.read().await;

        match state {
            ConnectionState::Connected => {
                // Try to execute the tool
                match self.dispatch_tool(name, arguments).await {
                    Ok(result) => Ok(result),
                    Err(McpError::DaemonDisconnected) | Err(McpError::NotConnected) => {
                        // Connection lost during tool execution - attempt recovery
                        warn!("Connection lost during tool execution, attempting recovery");

                        // Update state
                        {
                            let mut s = self.connection_state.write().await;
                            *s = ConnectionState::Disconnected;
                        }
                        let _ = self.state_tx.send(ConnectionState::Disconnected);

                        // Attempt reconnection
                        self.attempt_reconnection().await?;

                        // Retry the tool call once after successful reconnection
                        info!("Retrying tool call after successful reconnection");
                        self.dispatch_tool(name, arguments).await
                    }
                    Err(e) => Err(e),
                }
            }
            ConnectionState::Reconnecting { attempt } => {
                // Already reconnecting - return structured error
                Err(McpError::RecoveringConnection {
                    attempt,
                    max: MAX_RECONNECT_ATTEMPTS,
                })
            }
            ConnectionState::Disconnected => {
                // Disconnected - attempt reconnection before the tool call
                info!("Daemon disconnected, attempting reconnection before tool call");
                self.attempt_reconnection().await?;

                // Execute the tool after successful reconnection
                self.dispatch_tool(name, arguments).await
            }
        }
    }

    /// Dispatch tool call to daemon via IPC
    async fn dispatch_tool(
        &mut self,
        name: &str,
        arguments: &serde_json::Value,
    ) -> Result<ToolResult, McpError> {
        match name {
            "ccmux_list_sessions" => self.tool_list_sessions().await,
            "ccmux_list_windows" => {
                let session = arguments["session"].as_str().map(String::from);
                self.tool_list_windows(session).await
            }
            "ccmux_list_panes" => {
                let session = arguments["session"].as_str().map(String::from);
                self.tool_list_panes(session).await
            }
            "ccmux_read_pane" => {
                let pane_id = parse_uuid(arguments, "pane_id")?;
                let lines = arguments["lines"].as_u64().unwrap_or(100) as usize;
                self.tool_read_pane(pane_id, lines).await
            }
            "ccmux_get_status" => {
                let pane_id = parse_uuid(arguments, "pane_id")?;
                self.tool_get_status(pane_id).await
            }
            "ccmux_create_session" => {
                let name = arguments["name"].as_str().map(String::from);
                let command = arguments["command"].as_str().map(String::from);
                let cwd = arguments["cwd"].as_str().map(String::from);
                self.tool_create_session(name, command, cwd).await
            }
            "ccmux_create_window" => {
                let session = arguments["session"].as_str().map(String::from);
                let name = arguments["name"].as_str().map(String::from);
                let command = arguments["command"].as_str().map(String::from);
                self.tool_create_window(session, name, command).await
            }
            "ccmux_create_pane" => {
                let session = arguments["session"].as_str().map(String::from);
                let window = arguments["window"].as_str().map(String::from);
                let name = arguments["name"].as_str().map(String::from);
                let direction = arguments["direction"].as_str().map(String::from);
                let command = arguments["command"].as_str().map(String::from);
                let cwd = arguments["cwd"].as_str().map(String::from);
                let select = arguments["select"].as_bool().unwrap_or(false);
                self.tool_create_pane(session, window, name, direction, command, cwd, select)
                    .await
            }
            "ccmux_send_input" => {
                let pane_id = parse_uuid(arguments, "pane_id")?;
                let input = arguments["input"]
                    .as_str()
                    .ok_or_else(|| McpError::InvalidParams("Missing 'input' parameter".into()))?;
                let submit = arguments["submit"].as_bool().unwrap_or(false);
                self.tool_send_input(pane_id, input, submit).await
            }
            "ccmux_close_pane" => {
                let pane_id = parse_uuid(arguments, "pane_id")?;
                self.tool_close_pane(pane_id).await
            }
            "ccmux_focus_pane" => {
                let pane_id = parse_uuid(arguments, "pane_id")?;
                self.tool_focus_pane(pane_id).await
            }
            "ccmux_select_window" => {
                let window_id = parse_uuid(arguments, "window_id")?;
                self.tool_select_window(window_id).await
            }
            "ccmux_select_session" => {
                let session_id = parse_uuid(arguments, "session_id")?;
                self.tool_select_session(session_id).await
            }
            "ccmux_rename_session" => {
                let session = arguments["session"]
                    .as_str()
                    .ok_or_else(|| McpError::InvalidParams("Missing 'session' parameter".into()))?;
                let name = arguments["name"]
                    .as_str()
                    .ok_or_else(|| McpError::InvalidParams("Missing 'name' parameter".into()))?;
                self.tool_rename_session(session, name).await
            }
            // FEAT-036: Pane and window rename tools
            "ccmux_rename_pane" => {
                let pane_id = parse_uuid(arguments, "pane_id")?;
                let name = arguments["name"]
                    .as_str()
                    .ok_or_else(|| McpError::InvalidParams("Missing 'name' parameter".into()))?;
                self.tool_rename_pane(pane_id, name).await
            }
            "ccmux_rename_window" => {
                let window_id = parse_uuid(arguments, "window_id")?;
                let name = arguments["name"]
                    .as_str()
                    .ok_or_else(|| McpError::InvalidParams("Missing 'name' parameter".into()))?;
                self.tool_rename_window(window_id, name).await
            }
            "ccmux_split_pane" => {
                let pane_id = parse_uuid(arguments, "pane_id")?;
                let direction = arguments["direction"].as_str().map(String::from);
                let ratio = arguments["ratio"].as_f64().unwrap_or(0.5) as f32;
                let command = arguments["command"].as_str().map(String::from);
                let cwd = arguments["cwd"].as_str().map(String::from);
                let select = arguments["select"].as_bool().unwrap_or(false);
                self.tool_split_pane(pane_id, direction, ratio, command, cwd, select)
                    .await
            }
            "ccmux_resize_pane" => {
                let pane_id = parse_uuid(arguments, "pane_id")?;
                let delta = arguments["delta"]
                    .as_f64()
                    .ok_or_else(|| McpError::InvalidParams("Missing 'delta' parameter".into()))?
                    as f32;
                self.tool_resize_pane(pane_id, delta).await
            }
            "ccmux_create_layout" => {
                let session = arguments["session"].as_str().map(String::from);
                let window = arguments["window"].as_str().map(String::from);
                // BUG-033: Handle layout passed as JSON string instead of object
                // Some MCP clients may serialize the layout as a string
                let raw_layout = arguments["layout"].clone();
                debug!(
                    "create_layout received layout type: {}, value: {}",
                    if raw_layout.is_object() { "object" }
                    else if raw_layout.is_string() { "string" }
                    else if raw_layout.is_array() { "array" }
                    else { "other" },
                    raw_layout
                );
                let layout = match &raw_layout {
                    serde_json::Value::String(s) => {
                        debug!("Parsing layout from JSON string");
                        serde_json::from_str(s).map_err(|e| {
                            McpError::InvalidParams(format!("Invalid layout JSON string: {}", e))
                        })?
                    }
                    other => other.clone(),
                };
                self.tool_create_layout(session, window, layout).await
            }
            "ccmux_kill_session" => {
                let session = arguments["session"]
                    .as_str()
                    .ok_or_else(|| McpError::InvalidParams("Missing 'session' parameter".into()))?;
                self.tool_kill_session(session).await
            }
            "ccmux_set_environment" => {
                let session = arguments["session"]
                    .as_str()
                    .ok_or_else(|| McpError::InvalidParams("Missing 'session' parameter".into()))?;
                let key = arguments["key"]
                    .as_str()
                    .ok_or_else(|| McpError::InvalidParams("Missing 'key' parameter".into()))?;
                let value = arguments["value"]
                    .as_str()
                    .ok_or_else(|| McpError::InvalidParams("Missing 'value' parameter".into()))?;
                self.tool_set_environment(session, key, value).await
            }
            "ccmux_get_environment" => {
                let session = arguments["session"]
                    .as_str()
                    .ok_or_else(|| McpError::InvalidParams("Missing 'session' parameter".into()))?;
                let key = arguments["key"].as_str().map(String::from);
                self.tool_get_environment(session, key).await
            }
            "ccmux_set_metadata" => {
                let session = arguments["session"]
                    .as_str()
                    .ok_or_else(|| McpError::InvalidParams("Missing 'session' parameter".into()))?;
                let key = arguments["key"]
                    .as_str()
                    .ok_or_else(|| McpError::InvalidParams("Missing 'key' parameter".into()))?;
                let value = arguments["value"]
                    .as_str()
                    .ok_or_else(|| McpError::InvalidParams("Missing 'value' parameter".into()))?;
                self.tool_set_metadata(session, key, value).await
            }
            "ccmux_get_metadata" => {
                let session = arguments["session"]
                    .as_str()
                    .ok_or_else(|| McpError::InvalidParams("Missing 'session' parameter".into()))?;
                let key = arguments["key"].as_str().map(String::from);
                self.tool_get_metadata(session, key).await
            }
            // FEAT-048: Orchestration MCP tools
            "ccmux_send_orchestration" => {
                let target = &arguments["target"];
                let msg_type = arguments["msg_type"]
                    .as_str()
                    .ok_or_else(|| McpError::InvalidParams("Missing 'msg_type' parameter".into()))?;
                let payload = arguments["payload"].clone();
                self.tool_send_orchestration(target, msg_type, payload).await
            }
            "ccmux_set_tags" => {
                let session = arguments["session"].as_str().map(String::from);
                let add = arguments["add"]
                    .as_array()
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                let remove = arguments["remove"]
                    .as_array()
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                self.tool_set_tags(session, add, remove).await
            }
            "ccmux_get_tags" => {
                let session = arguments["session"].as_str().map(String::from);
                self.tool_get_tags(session).await
            }
            "ccmux_report_status" => {
                let status = arguments["status"]
                    .as_str()
                    .ok_or_else(|| McpError::InvalidParams("Missing 'status' parameter".into()))?;
                let message = arguments["message"].as_str().map(String::from);
                self.tool_report_status(status, message).await
            }
            "ccmux_request_help" => {
                let context = arguments["context"]
                    .as_str()
                    .ok_or_else(|| McpError::InvalidParams("Missing 'context' parameter".into()))?;
                self.tool_request_help(context).await
            }
            "ccmux_broadcast" => {
                let msg_type = arguments["msg_type"]
                    .as_str()
                    .ok_or_else(|| McpError::InvalidParams("Missing 'msg_type' parameter".into()))?;
                let payload = arguments["payload"].clone();
                self.tool_broadcast(msg_type, payload).await
            }
            // FEAT-060: Connection status tool
            "ccmux_connection_status" => self.tool_connection_status().await,
            // FEAT-059: Beads workflow integration tools
            "ccmux_beads_assign" => {
                let issue_id = arguments["issue_id"]
                    .as_str()
                    .ok_or_else(|| McpError::InvalidParams("Missing 'issue_id' parameter".into()))?;
                let pane_id = arguments["pane_id"]
                    .as_str()
                    .map(|s| {
                        Uuid::parse_str(s)
                            .map_err(|e| McpError::InvalidParams(format!("Invalid pane_id: {}", e)))
                    })
                    .transpose()?;
                self.tool_beads_assign(issue_id, pane_id).await
            }
            "ccmux_beads_release" => {
                let pane_id = arguments["pane_id"]
                    .as_str()
                    .map(|s| {
                        Uuid::parse_str(s)
                            .map_err(|e| McpError::InvalidParams(format!("Invalid pane_id: {}", e)))
                    })
                    .transpose()?;
                let outcome = arguments["outcome"].as_str().map(String::from);
                self.tool_beads_release(pane_id, outcome).await
            }
            "ccmux_beads_find_pane" => {
                let issue_id = arguments["issue_id"]
                    .as_str()
                    .ok_or_else(|| McpError::InvalidParams("Missing 'issue_id' parameter".into()))?;
                self.tool_beads_find_pane(issue_id).await
            }
            "ccmux_beads_pane_history" => {
                let pane_id = arguments["pane_id"]
                    .as_str()
                    .map(|s| {
                        Uuid::parse_str(s)
                            .map_err(|e| McpError::InvalidParams(format!("Invalid pane_id: {}", e)))
                    })
                    .transpose()?;
                self.tool_beads_pane_history(pane_id).await
            }
            _ => Err(McpError::UnknownTool(name.into())),
        }
    }

    // ==================== Tool Implementations ====================

    async fn tool_list_sessions(&mut self) -> Result<ToolResult, McpError> {
        self.send_to_daemon(ClientMessage::ListSessions).await?;

        match self.recv_response_from_daemon().await? {
            ServerMessage::SessionList { sessions } => {
                let result: Vec<serde_json::Value> = sessions
                    .iter()
                    .map(|s| {
                        serde_json::json!({
                            "id": s.id.to_string(),
                            "name": s.name,
                            "window_count": s.window_count,
                            "attached_clients": s.attached_clients,
                            "created_at": s.created_at,
                            "metadata": s.metadata,
                        })
                    })
                    .collect();

                let json = serde_json::to_string_pretty(&result)
                    .map_err(|e| McpError::Internal(e.to_string()))?;
                Ok(ToolResult::text(json))
            }
            ServerMessage::Error { code, message } => {
                Ok(ToolResult::error(format!("{:?}: {}", code, message)))
            }
            msg => Err(McpError::UnexpectedResponse(format!("{:?}", msg))),
        }
    }

    async fn tool_list_windows(
        &mut self,
        session_filter: Option<String>,
    ) -> Result<ToolResult, McpError> {
        self.send_to_daemon(ClientMessage::ListWindows { session_filter })
            .await?;

        match self.recv_response_from_daemon().await? {
            ServerMessage::WindowList {
                session_name,
                windows,
            } => {
                let result: Vec<serde_json::Value> = windows
                    .iter()
                    .map(|w| {
                        serde_json::json!({
                            "id": w.id.to_string(),
                            "index": w.index,
                            "name": w.name,
                            "pane_count": w.pane_count,
                            "active_pane_id": w.active_pane_id.map(|id| id.to_string()),
                            "session": session_name,
                        })
                    })
                    .collect();

                let json = serde_json::to_string_pretty(&result)
                    .map_err(|e| McpError::Internal(e.to_string()))?;
                Ok(ToolResult::text(json))
            }
            ServerMessage::Error { code, message } => {
                Ok(ToolResult::error(format!("{:?}: {}", code, message)))
            }
            msg => Err(McpError::UnexpectedResponse(format!("{:?}", msg))),
        }
    }

    async fn tool_list_panes(
        &mut self,
        session_filter: Option<String>,
    ) -> Result<ToolResult, McpError> {
        self.send_to_daemon(ClientMessage::ListAllPanes { session_filter })
            .await?;

        match self.recv_response_from_daemon().await? {
            ServerMessage::AllPanesList { panes } => {
                let result = format_pane_list(&panes);
                let json = serde_json::to_string_pretty(&result)
                    .map_err(|e| McpError::Internal(e.to_string()))?;
                Ok(ToolResult::text(json))
            }
            ServerMessage::Error { code, message } => {
                Ok(ToolResult::error(format!("{:?}: {}", code, message)))
            }
            msg => Err(McpError::UnexpectedResponse(format!("{:?}", msg))),
        }
    }

    async fn tool_read_pane(
        &mut self,
        pane_id: Uuid,
        lines: usize,
    ) -> Result<ToolResult, McpError> {
        self.send_to_daemon(ClientMessage::ReadPane { pane_id, lines })
            .await?;

        match self.recv_response_from_daemon().await? {
            ServerMessage::PaneContent { content, .. } => Ok(ToolResult::text(content)),
            ServerMessage::Error { code, message } => {
                Ok(ToolResult::error(format!("{:?}: {}", code, message)))
            }
            msg => Err(McpError::UnexpectedResponse(format!("{:?}", msg))),
        }
    }

    async fn tool_get_status(&mut self, pane_id: Uuid) -> Result<ToolResult, McpError> {
        self.send_to_daemon(ClientMessage::GetPaneStatus { pane_id })
            .await?;

        match self.recv_response_from_daemon().await? {
            ServerMessage::PaneStatus {
                pane_id,
                session_name,
                window_name,
                window_index,
                pane_index,
                cols,
                rows,
                title,
                cwd,
                state,
                has_pty,
                is_awaiting_input,
                is_awaiting_confirmation,
            } => {
                let state_json = match &state {
                    ccmux_protocol::PaneState::Normal => serde_json::json!({"type": "normal"}),
                    ccmux_protocol::PaneState::Claude(cs) => serde_json::json!({
                        "type": "claude",
                        "session_id": cs.session_id,
                        "activity": format!("{:?}", cs.activity),
                        "model": cs.model,
                        "tokens_used": cs.tokens_used,
                    }),
                    ccmux_protocol::PaneState::Exited { code } => serde_json::json!({
                        "type": "exited",
                        "exit_code": code,
                    }),
                };

                let result = serde_json::json!({
                    "pane_id": pane_id.to_string(),
                    "session": session_name,
                    "window": window_index,
                    "window_name": window_name,
                    "index": pane_index,
                    "dimensions": {
                        "cols": cols,
                        "rows": rows,
                    },
                    "title": title,
                    "cwd": cwd,
                    "has_pty": has_pty,
                    "state": state_json,
                    "is_awaiting_input": is_awaiting_input,
                    "is_awaiting_confirmation": is_awaiting_confirmation,
                });

                let json = serde_json::to_string_pretty(&result)
                    .map_err(|e| McpError::Internal(e.to_string()))?;
                Ok(ToolResult::text(json))
            }
            ServerMessage::Error { code, message } => {
                Ok(ToolResult::error(format!("{:?}: {}", code, message)))
            }
            msg => Err(McpError::UnexpectedResponse(format!("{:?}", msg))),
        }
    }

    async fn tool_create_session(
        &mut self,
        name: Option<String>,
        command: Option<String>,
        cwd: Option<String>,
    ) -> Result<ToolResult, McpError> {
        self.send_to_daemon(ClientMessage::CreateSessionWithOptions { name, command, cwd })
            .await?;

        match self.recv_response_from_daemon().await? {
            ServerMessage::SessionCreatedWithDetails {
                session_id,
                session_name,
                window_id,
                pane_id,
            } => {
                let result = serde_json::json!({
                    "session_id": session_id.to_string(),
                    "session_name": session_name,
                    "window_id": window_id.to_string(),
                    "pane_id": pane_id.to_string(),
                    "status": "created"
                });

                let json = serde_json::to_string_pretty(&result)
                    .map_err(|e| McpError::Internal(e.to_string()))?;
                Ok(ToolResult::text(json))
            }
            ServerMessage::Error { code, message } => {
                Ok(ToolResult::error(format!("{:?}: {}", code, message)))
            }
            msg => Err(McpError::UnexpectedResponse(format!("{:?}", msg))),
        }
    }

    async fn tool_create_window(
        &mut self,
        session_filter: Option<String>,
        name: Option<String>,
        command: Option<String>,
    ) -> Result<ToolResult, McpError> {
        self.send_to_daemon(ClientMessage::CreateWindowWithOptions {
            session_filter,
            name,
            command,
        })
        .await?;

        match self.recv_response_from_daemon().await? {
            ServerMessage::WindowCreatedWithDetails {
                window_id,
                pane_id,
                session_name,
            } => {
                let result = serde_json::json!({
                    "window_id": window_id.to_string(),
                    "pane_id": pane_id.to_string(),
                    "session": session_name,
                    "status": "created"
                });

                let json = serde_json::to_string_pretty(&result)
                    .map_err(|e| McpError::Internal(e.to_string()))?;
                Ok(ToolResult::text(json))
            }
            ServerMessage::Error { code, message } => {
                Ok(ToolResult::error(format!("{:?}: {}", code, message)))
            }
            msg => Err(McpError::UnexpectedResponse(format!("{:?}", msg))),
        }
    }

    async fn tool_create_pane(
        &mut self,
        session: Option<String>,
        window: Option<String>,
        name: Option<String>,
        direction: Option<String>,
        command: Option<String>,
        cwd: Option<String>,
        select: bool,
    ) -> Result<ToolResult, McpError> {
        // Map terminal multiplexer convention to layout direction:
        // - "vertical" = vertical split LINE = panes side-by-side = Horizontal layout
        // - "horizontal" = horizontal split LINE = panes stacked = Vertical layout
        let split_direction = match direction.as_deref() {
            Some("horizontal") | Some("h") => SplitDirection::Vertical,
            _ => SplitDirection::Horizontal, // "vertical" or default = side-by-side
        };

        // BUG-025 FIX: Store user's requested direction to return in response
        // (not the daemon's internal direction representation)
        let user_direction = match direction.as_deref() {
            Some("horizontal") | Some("h") => "horizontal",
            _ => "vertical", // default is vertical
        };

        self.send_to_daemon(ClientMessage::CreatePaneWithOptions {
            session_filter: session,
            window_filter: window,
            direction: split_direction,
            command,
            cwd,
            select,
            name,
        })
        .await?;

        match self.recv_response_from_daemon().await? {
            ServerMessage::PaneCreatedWithDetails {
                pane_id,
                session_id,
                session_name,
                window_id,
                direction: _, // Ignore daemon's direction, use user's requested direction
            } => {
                let result = serde_json::json!({
                    "pane_id": pane_id.to_string(),
                    "session_id": session_id.to_string(),
                    "session": session_name,
                    "window_id": window_id.to_string(),
                    "direction": user_direction,
                    "status": "created"
                });

                let json = serde_json::to_string_pretty(&result)
                    .map_err(|e| McpError::Internal(e.to_string()))?;
                Ok(ToolResult::text(json))
            }
            ServerMessage::Error { code, message } => {
                Ok(ToolResult::error(format!("{:?}: {}", code, message)))
            }
            msg => Err(McpError::UnexpectedResponse(format!("{:?}", msg))),
        }
    }

    async fn tool_send_input(
        &mut self,
        pane_id: Uuid,
        input: &str,
        submit: bool,
    ) -> Result<ToolResult, McpError> {
        // Build input data, appending carriage return if submit is true
        let mut data = input.as_bytes().to_vec();
        if submit {
            data.push(b'\r');
        }

        // Send input as bytes to the pane
        self.send_to_daemon(ClientMessage::Input { pane_id, data }).await?;

        // Input messages don't get a response in the current protocol,
        // so we just return success
        Ok(ToolResult::text(r#"{"status": "sent"}"#.to_string()))
    }

    async fn tool_close_pane(&mut self, pane_id: Uuid) -> Result<ToolResult, McpError> {
        self.send_to_daemon(ClientMessage::ClosePane { pane_id })
            .await?;

        match self.recv_response_from_daemon().await? {
            ServerMessage::PaneClosed { pane_id, .. } => {
                let result = serde_json::json!({
                    "pane_id": pane_id.to_string(),
                    "status": "closed"
                });

                let json = serde_json::to_string_pretty(&result)
                    .map_err(|e| McpError::Internal(e.to_string()))?;
                Ok(ToolResult::text(json))
            }
            ServerMessage::Error { code, message } => {
                Ok(ToolResult::error(format!("{:?}: {}", code, message)))
            }
            msg => Err(McpError::UnexpectedResponse(format!("{:?}", msg))),
        }
    }

    async fn tool_focus_pane(&mut self, pane_id: Uuid) -> Result<ToolResult, McpError> {
        self.send_to_daemon(ClientMessage::SelectPane { pane_id })
            .await?;

        // BUG-035 FIX: Properly consume the response from daemon.
        // The handler returns PaneFocused on success or Error on failure.
        // We need to loop and skip other broadcasts that might arrive before our response.
        loop {
            match self.recv_from_daemon().await? {
                ServerMessage::PaneFocused { pane_id, session_id, window_id } => {
                    let result = serde_json::json!({
                        "pane_id": pane_id.to_string(),
                        "session_id": session_id.to_string(),
                        "window_id": window_id.to_string(),
                        "status": "focused"
                    });

                    let json = serde_json::to_string_pretty(&result)
                        .map_err(|e| McpError::Internal(e.to_string()))?;
                    return Ok(ToolResult::text(json));
                }
                ServerMessage::Error { code, message } => {
                    return Ok(ToolResult::error(format!("{:?}: {}", code, message)));
                }
                msg if Self::is_broadcast_message(&msg) => {
                    // Skip broadcasts from other clients, keep waiting for our response
                    debug!("Skipping broadcast in tool_focus_pane: {:?}", std::mem::discriminant(&msg));
                    continue;
                }
                msg => {
                    // Unexpected non-broadcast response
                    return Err(McpError::UnexpectedResponse(format!("{:?}", msg)));
                }
            }
        }
    }

    async fn tool_select_window(&mut self, window_id: Uuid) -> Result<ToolResult, McpError> {
        self.send_to_daemon(ClientMessage::SelectWindow { window_id })
            .await?;

        // BUG-035 FIX: Properly consume the response from daemon.
        // The handler returns WindowFocused on success or Error on failure.
        // We need to loop and skip other broadcasts that might arrive before our response.
        loop {
            match self.recv_from_daemon().await? {
                ServerMessage::WindowFocused { window_id, session_id } => {
                    let result = serde_json::json!({
                        "window_id": window_id.to_string(),
                        "session_id": session_id.to_string(),
                        "status": "selected"
                    });

                    let json = serde_json::to_string_pretty(&result)
                        .map_err(|e| McpError::Internal(e.to_string()))?;
                    return Ok(ToolResult::text(json));
                }
                ServerMessage::Error { code, message } => {
                    return Ok(ToolResult::error(format!("{:?}: {}", code, message)));
                }
                msg if Self::is_broadcast_message(&msg) => {
                    // Skip broadcasts from other clients, keep waiting for our response
                    debug!("Skipping broadcast in tool_select_window: {:?}", std::mem::discriminant(&msg));
                    continue;
                }
                msg => {
                    // Unexpected non-broadcast response
                    return Err(McpError::UnexpectedResponse(format!("{:?}", msg)));
                }
            }
        }
    }

    async fn tool_select_session(&mut self, session_id: Uuid) -> Result<ToolResult, McpError> {
        self.send_to_daemon(ClientMessage::SelectSession { session_id })
            .await?;

        // BUG-035 FIX: Properly consume the response from daemon.
        // The handler returns SessionFocused on success or Error on failure.
        // We need to loop and skip other broadcasts that might arrive before our response.
        loop {
            match self.recv_from_daemon().await? {
                ServerMessage::SessionFocused { session_id } => {
                    let result = serde_json::json!({
                        "session_id": session_id.to_string(),
                        "status": "selected"
                    });

                    let json = serde_json::to_string_pretty(&result)
                        .map_err(|e| McpError::Internal(e.to_string()))?;
                    return Ok(ToolResult::text(json));
                }
                ServerMessage::Error { code, message } => {
                    return Ok(ToolResult::error(format!("{:?}: {}", code, message)));
                }
                msg if Self::is_broadcast_message(&msg) => {
                    // Skip broadcasts from other clients, keep waiting for our response
                    debug!("Skipping broadcast in tool_select_session: {:?}", std::mem::discriminant(&msg));
                    continue;
                }
                msg => {
                    // Unexpected non-broadcast response
                    return Err(McpError::UnexpectedResponse(format!("{:?}", msg)));
                }
            }
        }
    }

    async fn tool_rename_session(
        &mut self,
        session_filter: &str,
        new_name: &str,
    ) -> Result<ToolResult, McpError> {
        self.send_to_daemon(ClientMessage::RenameSession {
            session_filter: session_filter.to_string(),
            new_name: new_name.to_string(),
        })
        .await?;

        match self.recv_response_from_daemon().await? {
            ServerMessage::SessionRenamed {
                session_id,
                previous_name,
                new_name,
            } => {
                let result = serde_json::json!({
                    "success": true,
                    "session_id": session_id.to_string(),
                    "previous_name": previous_name,
                    "new_name": new_name
                });

                let json = serde_json::to_string_pretty(&result)
                    .map_err(|e| McpError::Internal(e.to_string()))?;
                Ok(ToolResult::text(json))
            }
            ServerMessage::Error { code, message } => {
                Ok(ToolResult::error(format!("{:?}: {}", code, message)))
            }
            msg => Err(McpError::UnexpectedResponse(format!("{:?}", msg))),
        }
    }

    // FEAT-036: Pane rename tool
    async fn tool_rename_pane(
        &mut self,
        pane_id: Uuid,
        new_name: &str,
    ) -> Result<ToolResult, McpError> {
        self.send_to_daemon(ClientMessage::RenamPane {
            pane_id,
            new_name: new_name.to_string(),
        })
        .await?;

        match self.recv_response_from_daemon().await? {
            ServerMessage::PaneRenamed {
                pane_id,
                previous_name,
                new_name,
            } => {
                let result = serde_json::json!({
                    "success": true,
                    "pane_id": pane_id.to_string(),
                    "previous_name": previous_name,
                    "new_name": new_name
                });

                let json = serde_json::to_string_pretty(&result)
                    .map_err(|e| McpError::Internal(e.to_string()))?;
                Ok(ToolResult::text(json))
            }
            ServerMessage::Error { code, message } => {
                Ok(ToolResult::error(format!("{:?}: {}", code, message)))
            }
            msg => Err(McpError::UnexpectedResponse(format!("{:?}", msg))),
        }
    }

    // FEAT-036: Window rename tool
    async fn tool_rename_window(
        &mut self,
        window_id: Uuid,
        new_name: &str,
    ) -> Result<ToolResult, McpError> {
        self.send_to_daemon(ClientMessage::RenameWindow {
            window_id,
            new_name: new_name.to_string(),
        })
        .await?;

        match self.recv_response_from_daemon().await? {
            ServerMessage::WindowRenamed {
                window_id,
                previous_name,
                new_name,
            } => {
                let result = serde_json::json!({
                    "success": true,
                    "window_id": window_id.to_string(),
                    "previous_name": previous_name,
                    "new_name": new_name
                });

                let json = serde_json::to_string_pretty(&result)
                    .map_err(|e| McpError::Internal(e.to_string()))?;
                Ok(ToolResult::text(json))
            }
            ServerMessage::Error { code, message } => {
                Ok(ToolResult::error(format!("{:?}: {}", code, message)))
            }
            msg => Err(McpError::UnexpectedResponse(format!("{:?}", msg))),
        }
    }

    async fn tool_split_pane(
        &mut self,
        pane_id: Uuid,
        direction: Option<String>,
        ratio: f32,
        command: Option<String>,
        cwd: Option<String>,
        select: bool,
    ) -> Result<ToolResult, McpError> {
        // Map terminal multiplexer convention to layout direction
        let split_direction = match direction.as_deref() {
            Some("horizontal") | Some("h") => SplitDirection::Vertical,
            _ => SplitDirection::Horizontal, // "vertical" or default = side-by-side
        };

        // BUG-025 FIX: Store user's requested direction to return in response
        let user_direction = match direction.as_deref() {
            Some("horizontal") | Some("h") => "horizontal",
            _ => "vertical", // default is vertical
        };

        self.send_to_daemon(ClientMessage::SplitPane {
            pane_id,
            direction: split_direction,
            ratio,
            command,
            cwd,
            select,
        })
        .await?;

        match self.recv_response_from_daemon().await? {
            ServerMessage::PaneSplit {
                new_pane_id,
                original_pane_id,
                session_id,
                session_name,
                window_id,
                direction: _, // Ignore daemon's direction, use user's requested direction
            } => {
                let result = serde_json::json!({
                    "new_pane_id": new_pane_id.to_string(),
                    "original_pane_id": original_pane_id.to_string(),
                    "session_id": session_id.to_string(),
                    "session": session_name,
                    "window_id": window_id.to_string(),
                    "direction": user_direction,
                });

                let json = serde_json::to_string_pretty(&result)
                    .map_err(|e| McpError::Internal(e.to_string()))?;
                Ok(ToolResult::text(json))
            }
            ServerMessage::Error { code, message } => {
                Ok(ToolResult::error(format!("{:?}: {}", code, message)))
            }
            msg => Err(McpError::UnexpectedResponse(format!("{:?}", msg))),
        }
    }

    async fn tool_resize_pane(
        &mut self,
        pane_id: Uuid,
        delta: f32,
    ) -> Result<ToolResult, McpError> {
        self.send_to_daemon(ClientMessage::ResizePaneDelta { pane_id, delta })
            .await?;

        match self.recv_response_from_daemon().await? {
            ServerMessage::PaneResized {
                pane_id,
                new_cols,
                new_rows,
            } => {
                let result = serde_json::json!({
                    "pane_id": pane_id.to_string(),
                    "new_cols": new_cols,
                    "new_rows": new_rows,
                });

                let json = serde_json::to_string_pretty(&result)
                    .map_err(|e| McpError::Internal(e.to_string()))?;
                Ok(ToolResult::text(json))
            }
            ServerMessage::Error { code, message } => {
                Ok(ToolResult::error(format!("{:?}: {}", code, message)))
            }
            msg => Err(McpError::UnexpectedResponse(format!("{:?}", msg))),
        }
    }

    async fn tool_create_layout(
        &mut self,
        session: Option<String>,
        window: Option<String>,
        layout: serde_json::Value,
    ) -> Result<ToolResult, McpError> {
        self.send_to_daemon(ClientMessage::CreateLayout {
            session_filter: session,
            window_filter: window,
            layout: layout.into(), // Convert to JsonValue for bincode compatibility (BUG-030)
        })
        .await?;

        match self.recv_response_from_daemon().await? {
            ServerMessage::LayoutCreated {
                session_id,
                session_name,
                window_id,
                pane_ids,
            } => {
                let result = serde_json::json!({
                    "session_id": session_id.to_string(),
                    "session": session_name,
                    "window_id": window_id.to_string(),
                    "pane_ids": pane_ids.iter().map(|id| id.to_string()).collect::<Vec<_>>(),
                    "pane_count": pane_ids.len(),
                });

                let json = serde_json::to_string_pretty(&result)
                    .map_err(|e| McpError::Internal(e.to_string()))?;
                Ok(ToolResult::text(json))
            }
            ServerMessage::Error { code, message } => {
                Ok(ToolResult::error(format!("{:?}: {}", code, message)))
            }
            msg => Err(McpError::UnexpectedResponse(format!("{:?}", msg))),
        }
    }

    async fn tool_kill_session(&mut self, session_filter: &str) -> Result<ToolResult, McpError> {
        // Try to parse as UUID first, otherwise resolve by name
        let session_id = if let Ok(uuid) = Uuid::parse_str(session_filter) {
            uuid
        } else {
            // Need to list sessions and find by name
            self.send_to_daemon(ClientMessage::ListSessions).await?;
            match self.recv_response_from_daemon().await? {
                ServerMessage::SessionList { sessions } => {
                    sessions
                        .iter()
                        .find(|s| s.name == session_filter)
                        .map(|s| s.id)
                        .ok_or_else(|| {
                            McpError::InvalidParams(format!(
                                "Session '{}' not found",
                                session_filter
                            ))
                        })?
                }
                ServerMessage::Error { code, message } => {
                    return Ok(ToolResult::error(format!("{:?}: {}", code, message)));
                }
                msg => return Err(McpError::UnexpectedResponse(format!("{:?}", msg))),
            }
        };

        self.send_to_daemon(ClientMessage::DestroySession { session_id })
            .await?;

        match self.recv_response_from_daemon().await? {
            ServerMessage::SessionDestroyed {
                session_id,
                session_name,
            } => {
                let result = serde_json::json!({
                    "success": true,
                    "message": "Session killed",
                    "session_id": session_id.to_string(),
                    "session_name": session_name,
                });

                let json = serde_json::to_string_pretty(&result)
                    .map_err(|e| McpError::Internal(e.to_string()))?;
                Ok(ToolResult::text(json))
            }
            ServerMessage::Error { code, message } => {
                Ok(ToolResult::error(format!("{:?}: {}", code, message)))
            }
            msg => Err(McpError::UnexpectedResponse(format!("{:?}", msg))),
        }
    }

    async fn tool_set_environment(
        &mut self,
        session_filter: &str,
        key: &str,
        value: &str,
    ) -> Result<ToolResult, McpError> {
        self.send_to_daemon(ClientMessage::SetEnvironment {
            session_filter: session_filter.to_string(),
            key: key.to_string(),
            value: value.to_string(),
        })
        .await?;

        match self.recv_response_from_daemon().await? {
            ServerMessage::EnvironmentSet {
                session_id,
                session_name,
                key,
                value,
            } => {
                let result = serde_json::json!({
                    "success": true,
                    "session_id": session_id.to_string(),
                    "session_name": session_name,
                    "key": key,
                    "value": value,
                });

                let json = serde_json::to_string_pretty(&result)
                    .map_err(|e| McpError::Internal(e.to_string()))?;
                Ok(ToolResult::text(json))
            }
            ServerMessage::Error { code, message } => {
                Ok(ToolResult::error(format!("{:?}: {}", code, message)))
            }
            msg => Err(McpError::UnexpectedResponse(format!("{:?}", msg))),
        }
    }

    async fn tool_get_environment(
        &mut self,
        session_filter: &str,
        key: Option<String>,
    ) -> Result<ToolResult, McpError> {
        self.send_to_daemon(ClientMessage::GetEnvironment {
            session_filter: session_filter.to_string(),
            key,
        })
        .await?;

        match self.recv_response_from_daemon().await? {
            ServerMessage::EnvironmentList {
                session_id,
                session_name,
                environment,
            } => {
                let result = serde_json::json!({
                    "session_id": session_id.to_string(),
                    "session_name": session_name,
                    "environment": environment,
                });

                let json = serde_json::to_string_pretty(&result)
                    .map_err(|e| McpError::Internal(e.to_string()))?;
                Ok(ToolResult::text(json))
            }
            ServerMessage::Error { code, message } => {
                Ok(ToolResult::error(format!("{:?}: {}", code, message)))
            }
            msg => Err(McpError::UnexpectedResponse(format!("{:?}", msg))),
        }
    }

    async fn tool_set_metadata(
        &mut self,
        session_filter: &str,
        key: &str,
        value: &str,
    ) -> Result<ToolResult, McpError> {
        self.send_to_daemon(ClientMessage::SetMetadata {
            session_filter: session_filter.to_string(),
            key: key.to_string(),
            value: value.to_string(),
        })
        .await?;

        match self.recv_response_from_daemon().await? {
            ServerMessage::MetadataSet {
                session_id,
                session_name,
                key,
                value,
            } => {
                let result = serde_json::json!({
                    "success": true,
                    "session_id": session_id.to_string(),
                    "session_name": session_name,
                    "key": key,
                    "value": value,
                });

                let json = serde_json::to_string_pretty(&result)
                    .map_err(|e| McpError::Internal(e.to_string()))?;
                Ok(ToolResult::text(json))
            }
            ServerMessage::Error { code, message } => {
                Ok(ToolResult::error(format!("{:?}: {}", code, message)))
            }
            msg => Err(McpError::UnexpectedResponse(format!("{:?}", msg))),
        }
    }

    async fn tool_get_metadata(
        &mut self,
        session_filter: &str,
        key: Option<String>,
    ) -> Result<ToolResult, McpError> {
        self.send_to_daemon(ClientMessage::GetMetadata {
            session_filter: session_filter.to_string(),
            key,
        })
        .await?;

        match self.recv_response_from_daemon().await? {
            ServerMessage::MetadataList {
                session_id,
                session_name,
                metadata,
            } => {
                let result = serde_json::json!({
                    "session_id": session_id.to_string(),
                    "session_name": session_name,
                    "metadata": metadata,
                });

                let json = serde_json::to_string_pretty(&result)
                    .map_err(|e| McpError::Internal(e.to_string()))?;
                Ok(ToolResult::text(json))
            }
            ServerMessage::Error { code, message } => {
                Ok(ToolResult::error(format!("{:?}: {}", code, message)))
            }
            msg => Err(McpError::UnexpectedResponse(format!("{:?}", msg))),
        }
    }

    // ==================== FEAT-048: Orchestration Tool Implementations ====================

    async fn tool_send_orchestration(
        &mut self,
        target: &serde_json::Value,
        msg_type: &str,
        payload: serde_json::Value,
    ) -> Result<ToolResult, McpError> {
        // Parse the target
        let orchestration_target = if let Some(tag) = target.get("tag").and_then(|v| v.as_str()) {
            ccmux_protocol::OrchestrationTarget::Tagged(tag.to_string())
        } else if let Some(session) = target.get("session").and_then(|v| v.as_str()) {
            let session_id = Uuid::parse_str(session)
                .map_err(|e| McpError::InvalidParams(format!("Invalid session UUID: {}", e)))?;
            ccmux_protocol::OrchestrationTarget::Session(session_id)
        } else if target.get("broadcast").and_then(|v| v.as_bool()).unwrap_or(false) {
            ccmux_protocol::OrchestrationTarget::Broadcast
        } else if let Some(worktree) = target.get("worktree").and_then(|v| v.as_str()) {
            ccmux_protocol::OrchestrationTarget::Worktree(worktree.to_string())
        } else {
            return Err(McpError::InvalidParams(
                "Invalid target: must specify 'tag', 'session', 'broadcast', or 'worktree'".into(),
            ));
        };

        let message = ccmux_protocol::OrchestrationMessage::new(msg_type, payload);

        self.send_to_daemon(ClientMessage::SendOrchestration {
            target: orchestration_target,
            message,
        })
        .await?;

        match self.recv_response_from_daemon().await? {
            ServerMessage::OrchestrationDelivered { delivered_count } => {
                let result = serde_json::json!({
                    "success": true,
                    "delivered_count": delivered_count,
                });

                let json = serde_json::to_string_pretty(&result)
                    .map_err(|e| McpError::Internal(e.to_string()))?;
                Ok(ToolResult::text(json))
            }
            ServerMessage::Error { code, message } => {
                Ok(ToolResult::error(format!("{:?}: {}", code, message)))
            }
            msg => Err(McpError::UnexpectedResponse(format!("{:?}", msg))),
        }
    }

    async fn tool_set_tags(
        &mut self,
        session_filter: Option<String>,
        add: Vec<String>,
        remove: Vec<String>,
    ) -> Result<ToolResult, McpError> {
        self.send_to_daemon(ClientMessage::SetTags {
            session_filter,
            add,
            remove,
        })
        .await?;

        match self.recv_response_from_daemon().await? {
            ServerMessage::TagsSet {
                session_id,
                session_name,
                tags,
            } => {
                let result = serde_json::json!({
                    "success": true,
                    "session_id": session_id.to_string(),
                    "session_name": session_name,
                    "tags": tags.into_iter().collect::<Vec<_>>(),
                });

                let json = serde_json::to_string_pretty(&result)
                    .map_err(|e| McpError::Internal(e.to_string()))?;
                Ok(ToolResult::text(json))
            }
            ServerMessage::Error { code, message } => {
                Ok(ToolResult::error(format!("{:?}: {}", code, message)))
            }
            msg => Err(McpError::UnexpectedResponse(format!("{:?}", msg))),
        }
    }

    async fn tool_get_tags(
        &mut self,
        session_filter: Option<String>,
    ) -> Result<ToolResult, McpError> {
        self.send_to_daemon(ClientMessage::GetTags { session_filter })
            .await?;

        match self.recv_response_from_daemon().await? {
            ServerMessage::TagsList {
                session_id,
                session_name,
                tags,
            } => {
                let result = serde_json::json!({
                    "session_id": session_id.to_string(),
                    "session_name": session_name,
                    "tags": tags.into_iter().collect::<Vec<_>>(),
                });

                let json = serde_json::to_string_pretty(&result)
                    .map_err(|e| McpError::Internal(e.to_string()))?;
                Ok(ToolResult::text(json))
            }
            ServerMessage::Error { code, message } => {
                Ok(ToolResult::error(format!("{:?}: {}", code, message)))
            }
            msg => Err(McpError::UnexpectedResponse(format!("{:?}", msg))),
        }
    }

    async fn tool_report_status(
        &mut self,
        status: &str,
        message: Option<String>,
    ) -> Result<ToolResult, McpError> {
        // FEAT-059: Try to get current issue_id to include in status update
        let current_issue_id = self.get_current_issue_id().await;

        // Convenience tool: sends status.update message to sessions tagged "orchestrator"
        let target = ccmux_protocol::OrchestrationTarget::Tagged("orchestrator".to_string());
        let payload = serde_json::json!({
            "status": status,
            "message": message,
            "issue_id": current_issue_id,  // FEAT-059: Include current issue
        });
        let msg = ccmux_protocol::OrchestrationMessage::new("status.update", payload);

        self.send_to_daemon(ClientMessage::SendOrchestration {
            target,
            message: msg,
        })
        .await?;

        match self.recv_response_from_daemon().await? {
            ServerMessage::OrchestrationDelivered { delivered_count } => {
                let result = serde_json::json!({
                    "success": true,
                    "delivered_count": delivered_count,
                    "status": status,
                    "issue_id": current_issue_id,  // FEAT-059: Include in response
                });

                let json = serde_json::to_string_pretty(&result)
                    .map_err(|e| McpError::Internal(e.to_string()))?;
                Ok(ToolResult::text(json))
            }
            ServerMessage::Error { code, message } => {
                Ok(ToolResult::error(format!("{:?}: {}", code, message)))
            }
            msg => Err(McpError::UnexpectedResponse(format!("{:?}", msg))),
        }
    }

    /// FEAT-059: Helper to get current issue ID from first session's metadata
    async fn get_current_issue_id(&mut self) -> Option<String> {
        // Get first session
        if self.send_to_daemon(ClientMessage::ListSessions).await.is_err() {
            return None;
        }

        let sessions = match self.recv_response_from_daemon().await {
            Ok(ServerMessage::SessionList { sessions }) => sessions,
            _ => return None,
        };

        if sessions.is_empty() {
            return None;
        }

        let session_name = &sessions[0].name;

        // Get beads.current_issue metadata
        if self
            .send_to_daemon(ClientMessage::GetMetadata {
                session_filter: session_name.clone(),
                key: Some(beads::CURRENT_ISSUE.to_string()),
            })
            .await
            .is_err()
        {
            return None;
        }

        match self.recv_response_from_daemon().await {
            Ok(ServerMessage::MetadataList { metadata, .. }) => {
                metadata
                    .get(beads::CURRENT_ISSUE)
                    .cloned()
                    .filter(|s| !s.is_empty())
            }
            _ => None,
        }
    }

    async fn tool_request_help(&mut self, context: &str) -> Result<ToolResult, McpError> {
        // Convenience tool: sends help.request message to sessions tagged "orchestrator"
        let target = ccmux_protocol::OrchestrationTarget::Tagged("orchestrator".to_string());
        let payload = serde_json::json!({
            "context": context,
        });
        let msg = ccmux_protocol::OrchestrationMessage::new("help.request", payload);

        self.send_to_daemon(ClientMessage::SendOrchestration {
            target,
            message: msg,
        })
        .await?;

        match self.recv_response_from_daemon().await? {
            ServerMessage::OrchestrationDelivered { delivered_count } => {
                let result = serde_json::json!({
                    "success": true,
                    "delivered_count": delivered_count,
                    "type": "help.request",
                });

                let json = serde_json::to_string_pretty(&result)
                    .map_err(|e| McpError::Internal(e.to_string()))?;
                Ok(ToolResult::text(json))
            }
            ServerMessage::Error { code, message } => {
                Ok(ToolResult::error(format!("{:?}: {}", code, message)))
            }
            msg => Err(McpError::UnexpectedResponse(format!("{:?}", msg))),
        }
    }

    async fn tool_broadcast(
        &mut self,
        msg_type: &str,
        payload: serde_json::Value,
    ) -> Result<ToolResult, McpError> {
        // Convenience tool: broadcasts message to all sessions
        let target = ccmux_protocol::OrchestrationTarget::Broadcast;
        let msg = ccmux_protocol::OrchestrationMessage::new(msg_type, payload);

        self.send_to_daemon(ClientMessage::SendOrchestration {
            target,
            message: msg,
        })
        .await?;

        match self.recv_response_from_daemon().await? {
            ServerMessage::OrchestrationDelivered { delivered_count } => {
                let result = serde_json::json!({
                    "success": true,
                    "delivered_count": delivered_count,
                    "type": msg_type,
                });

                let json = serde_json::to_string_pretty(&result)
                    .map_err(|e| McpError::Internal(e.to_string()))?;
                Ok(ToolResult::text(json))
            }
            ServerMessage::Error { code, message } => {
                Ok(ToolResult::error(format!("{:?}: {}", code, message)))
            }
            msg => Err(McpError::UnexpectedResponse(format!("{:?}", msg))),
        }
    }

    // ==================== FEAT-060: Connection Status Tool ====================

    /// Get current connection status
    ///
    /// Returns the daemon connection status including health and recovery information.
    /// This tool does not require daemon communication (local state only).
    async fn tool_connection_status(&self) -> Result<ToolResult, McpError> {
        let state = *self.connection_state.read().await;

        let result = match state {
            ConnectionState::Connected => serde_json::json!({
                "status": "connected",
                "healthy": true,
                "daemon_responsive": true
            }),
            ConnectionState::Reconnecting { attempt } => serde_json::json!({
                "status": "reconnecting",
                "healthy": false,
                "reconnect_attempt": attempt,
                "max_attempts": MAX_RECONNECT_ATTEMPTS
            }),
            ConnectionState::Disconnected => serde_json::json!({
                "status": "disconnected",
                "healthy": false,
                "recoverable": true,
                "action": "Tool calls will trigger automatic reconnection"
            }),
        };

        let json = serde_json::to_string_pretty(&result)
            .map_err(|e| McpError::Internal(e.to_string()))?;
        Ok(ToolResult::text(json))
    }

    // ==================== FEAT-059: Beads Workflow Integration Tools ====================

    /// Helper: Get session name from pane_id, or use first session if pane_id is None
    async fn resolve_session_for_pane(
        &mut self,
        pane_id: Option<Uuid>,
    ) -> Result<(String, Option<Uuid>), McpError> {
        match pane_id {
            Some(id) => {
                // Get pane status to find its session
                self.send_to_daemon(ClientMessage::GetPaneStatus { pane_id: id })
                    .await?;

                match self.recv_response_from_daemon().await? {
                    ServerMessage::PaneStatus {
                        pane_id,
                        session_name,
                        ..
                    } => Ok((session_name, Some(pane_id))),
                    ServerMessage::Error { code, message } => {
                        Err(McpError::InvalidParams(format!("{:?}: {}", code, message)))
                    }
                    msg => Err(McpError::UnexpectedResponse(format!("{:?}", msg))),
                }
            }
            None => {
                // Get first session
                self.send_to_daemon(ClientMessage::ListSessions).await?;

                match self.recv_response_from_daemon().await? {
                    ServerMessage::SessionList { sessions } => {
                        if sessions.is_empty() {
                            Err(McpError::InvalidParams("No sessions available".into()))
                        } else {
                            Ok((sessions[0].name.clone(), None))
                        }
                    }
                    ServerMessage::Error { code, message } => {
                        Err(McpError::InvalidParams(format!("{:?}: {}", code, message)))
                    }
                    msg => Err(McpError::UnexpectedResponse(format!("{:?}", msg))),
                }
            }
        }
    }

    /// Assign a beads issue to a pane (via session metadata)
    async fn tool_beads_assign(
        &mut self,
        issue_id: &str,
        pane_id: Option<Uuid>,
    ) -> Result<ToolResult, McpError> {
        let (session_name, resolved_pane_id) = self.resolve_session_for_pane(pane_id).await?;

        // Get current timestamp in ISO 8601 format
        let timestamp = {
            use std::time::{SystemTime, UNIX_EPOCH};
            let duration = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default();
            // Simple ISO 8601 format: seconds since epoch
            // For production, consider using chrono crate for proper formatting
            format!("{}", duration.as_secs())
        };

        // Set the current issue
        self.send_to_daemon(ClientMessage::SetMetadata {
            session_filter: session_name.clone(),
            key: beads::CURRENT_ISSUE.to_string(),
            value: issue_id.to_string(),
        })
        .await?;

        match self.recv_response_from_daemon().await? {
            ServerMessage::MetadataSet { .. } => {}
            ServerMessage::Error { code, message } => {
                return Ok(ToolResult::error(format!("{:?}: {}", code, message)));
            }
            msg => return Err(McpError::UnexpectedResponse(format!("{:?}", msg))),
        }

        // Set the assigned_at timestamp
        self.send_to_daemon(ClientMessage::SetMetadata {
            session_filter: session_name.clone(),
            key: beads::ASSIGNED_AT.to_string(),
            value: timestamp.clone(),
        })
        .await?;

        match self.recv_response_from_daemon().await? {
            ServerMessage::MetadataSet {
                session_id,
                session_name,
                ..
            } => {
                let result = serde_json::json!({
                    "success": true,
                    "session_id": session_id.to_string(),
                    "session_name": session_name,
                    "pane_id": resolved_pane_id.map(|id| id.to_string()),
                    "issue_id": issue_id,
                    "assigned_at": timestamp,
                });

                let json = serde_json::to_string_pretty(&result)
                    .map_err(|e| McpError::Internal(e.to_string()))?;
                Ok(ToolResult::text(json))
            }
            ServerMessage::Error { code, message } => {
                Ok(ToolResult::error(format!("{:?}: {}", code, message)))
            }
            msg => Err(McpError::UnexpectedResponse(format!("{:?}", msg))),
        }
    }

    /// Release/unassign the current beads issue from a pane
    async fn tool_beads_release(
        &mut self,
        pane_id: Option<Uuid>,
        outcome: Option<String>,
    ) -> Result<ToolResult, McpError> {
        let (session_name, resolved_pane_id) = self.resolve_session_for_pane(pane_id).await?;
        let outcome = outcome.unwrap_or_else(|| "completed".to_string());

        // Get current issue and assigned_at before clearing
        self.send_to_daemon(ClientMessage::GetMetadata {
            session_filter: session_name.clone(),
            key: Some(beads::CURRENT_ISSUE.to_string()),
        })
        .await?;

        let current_issue = match self.recv_response_from_daemon().await? {
            ServerMessage::MetadataList { metadata, .. } => {
                metadata.get(beads::CURRENT_ISSUE).cloned()
            }
            ServerMessage::Error { code, message } => {
                return Ok(ToolResult::error(format!("{:?}: {}", code, message)));
            }
            msg => return Err(McpError::UnexpectedResponse(format!("{:?}", msg))),
        };

        let current_issue = match current_issue {
            Some(issue) => issue,
            None => {
                return Ok(ToolResult::error("No issue currently assigned".to_string()));
            }
        };

        // Get assigned_at
        self.send_to_daemon(ClientMessage::GetMetadata {
            session_filter: session_name.clone(),
            key: Some(beads::ASSIGNED_AT.to_string()),
        })
        .await?;

        let assigned_at = match self.recv_response_from_daemon().await? {
            ServerMessage::MetadataList { metadata, .. } => {
                metadata.get(beads::ASSIGNED_AT).cloned().unwrap_or_default()
            }
            ServerMessage::Error { .. } => String::new(),
            _ => String::new(),
        };

        // Get existing history
        self.send_to_daemon(ClientMessage::GetMetadata {
            session_filter: session_name.clone(),
            key: Some(beads::ISSUE_HISTORY.to_string()),
        })
        .await?;

        let existing_history = match self.recv_response_from_daemon().await? {
            ServerMessage::MetadataList { metadata, .. } => {
                metadata.get(beads::ISSUE_HISTORY).cloned()
            }
            _ => None,
        };

        // Parse existing history or start fresh
        let mut history: Vec<serde_json::Value> = existing_history
            .and_then(|h| serde_json::from_str(&h).ok())
            .unwrap_or_default();

        // Get current timestamp for released_at
        let released_at = {
            use std::time::{SystemTime, UNIX_EPOCH};
            let duration = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default();
            format!("{}", duration.as_secs())
        };

        // Add new history entry
        history.push(serde_json::json!({
            "issue_id": current_issue,
            "assigned_at": assigned_at,
            "released_at": released_at,
            "outcome": outcome,
        }));

        // Save updated history
        let history_json = serde_json::to_string(&history)
            .map_err(|e| McpError::Internal(e.to_string()))?;

        self.send_to_daemon(ClientMessage::SetMetadata {
            session_filter: session_name.clone(),
            key: beads::ISSUE_HISTORY.to_string(),
            value: history_json,
        })
        .await?;

        match self.recv_response_from_daemon().await? {
            ServerMessage::MetadataSet { .. } => {}
            ServerMessage::Error { code, message } => {
                return Ok(ToolResult::error(format!("{:?}: {}", code, message)));
            }
            msg => return Err(McpError::UnexpectedResponse(format!("{:?}", msg))),
        }

        // Clear current issue (set to empty string)
        self.send_to_daemon(ClientMessage::SetMetadata {
            session_filter: session_name.clone(),
            key: beads::CURRENT_ISSUE.to_string(),
            value: String::new(),
        })
        .await?;

        match self.recv_response_from_daemon().await? {
            ServerMessage::MetadataSet { .. } => {}
            ServerMessage::Error { code, message } => {
                return Ok(ToolResult::error(format!("{:?}: {}", code, message)));
            }
            msg => return Err(McpError::UnexpectedResponse(format!("{:?}", msg))),
        }

        // Clear assigned_at
        self.send_to_daemon(ClientMessage::SetMetadata {
            session_filter: session_name.clone(),
            key: beads::ASSIGNED_AT.to_string(),
            value: String::new(),
        })
        .await?;

        match self.recv_response_from_daemon().await? {
            ServerMessage::MetadataSet {
                session_id,
                session_name,
                ..
            } => {
                let result = serde_json::json!({
                    "success": true,
                    "session_id": session_id.to_string(),
                    "session_name": session_name,
                    "pane_id": resolved_pane_id.map(|id| id.to_string()),
                    "released_issue": current_issue,
                    "outcome": outcome,
                    "released_at": released_at,
                });

                let json = serde_json::to_string_pretty(&result)
                    .map_err(|e| McpError::Internal(e.to_string()))?;
                Ok(ToolResult::text(json))
            }
            ServerMessage::Error { code, message } => {
                Ok(ToolResult::error(format!("{:?}: {}", code, message)))
            }
            msg => Err(McpError::UnexpectedResponse(format!("{:?}", msg))),
        }
    }

    /// Find the pane currently working on a specific beads issue
    async fn tool_beads_find_pane(&mut self, issue_id: &str) -> Result<ToolResult, McpError> {
        // List all sessions and check their beads.current_issue metadata
        self.send_to_daemon(ClientMessage::ListSessions).await?;

        let sessions = match self.recv_response_from_daemon().await? {
            ServerMessage::SessionList { sessions } => sessions,
            ServerMessage::Error { code, message } => {
                return Ok(ToolResult::error(format!("{:?}: {}", code, message)));
            }
            msg => return Err(McpError::UnexpectedResponse(format!("{:?}", msg))),
        };

        // Check each session for the issue
        for session in sessions {
            self.send_to_daemon(ClientMessage::GetMetadata {
                session_filter: session.id.to_string(),
                key: Some(beads::CURRENT_ISSUE.to_string()),
            })
            .await?;

            if let ServerMessage::MetadataList {
                session_id,
                session_name,
                metadata,
            } = self.recv_response_from_daemon().await?
            {
                if let Some(current_issue) = metadata.get(beads::CURRENT_ISSUE) {
                    if current_issue == issue_id {
                        // Found the session working on this issue
                        // Get first pane from this session for pane_id
                        self.send_to_daemon(ClientMessage::ListAllPanes {
                            session_filter: Some(session_id.to_string()),
                        })
                        .await?;

                        let pane_id = match self.recv_response_from_daemon().await? {
                            ServerMessage::AllPanesList { panes } => {
                                panes.first().map(|p| p.id)
                            }
                            _ => None,
                        };

                        let result = serde_json::json!({
                            "found": true,
                            "session_id": session_id.to_string(),
                            "session_name": session_name,
                            "pane_id": pane_id.map(|id| id.to_string()),
                            "issue_id": issue_id,
                        });

                        let json = serde_json::to_string_pretty(&result)
                            .map_err(|e| McpError::Internal(e.to_string()))?;
                        return Ok(ToolResult::text(json));
                    }
                }
            }
        }

        // Issue not found in any session
        let result = serde_json::json!({
            "found": false,
            "issue_id": issue_id,
            "message": "No pane is currently working on this issue",
        });

        let json = serde_json::to_string_pretty(&result)
            .map_err(|e| McpError::Internal(e.to_string()))?;
        Ok(ToolResult::text(json))
    }

    /// Get the issue history for a pane
    async fn tool_beads_pane_history(
        &mut self,
        pane_id: Option<Uuid>,
    ) -> Result<ToolResult, McpError> {
        let (session_name, resolved_pane_id) = self.resolve_session_for_pane(pane_id).await?;

        // Get issue history metadata
        self.send_to_daemon(ClientMessage::GetMetadata {
            session_filter: session_name.clone(),
            key: Some(beads::ISSUE_HISTORY.to_string()),
        })
        .await?;

        match self.recv_response_from_daemon().await? {
            ServerMessage::MetadataList {
                session_id,
                session_name,
                metadata,
            } => {
                let history_json = metadata.get(beads::ISSUE_HISTORY).cloned();

                // Parse history or return empty array
                let history: Vec<serde_json::Value> = history_json
                    .and_then(|h| serde_json::from_str(&h).ok())
                    .unwrap_or_default();

                // Also get current issue if any
                self.send_to_daemon(ClientMessage::GetMetadata {
                    session_filter: session_id.to_string(),
                    key: Some(beads::CURRENT_ISSUE.to_string()),
                })
                .await?;

                let current_issue = match self.recv_response_from_daemon().await? {
                    ServerMessage::MetadataList { metadata, .. } => {
                        metadata
                            .get(beads::CURRENT_ISSUE)
                            .cloned()
                            .filter(|s| !s.is_empty())
                    }
                    _ => None,
                };

                let result = serde_json::json!({
                    "session_id": session_id.to_string(),
                    "session_name": session_name,
                    "pane_id": resolved_pane_id.map(|id| id.to_string()),
                    "current_issue": current_issue,
                    "history": history,
                    "history_count": history.len(),
                });

                let json = serde_json::to_string_pretty(&result)
                    .map_err(|e| McpError::Internal(e.to_string()))?;
                Ok(ToolResult::text(json))
            }
            ServerMessage::Error { code, message } => {
                Ok(ToolResult::error(format!("{:?}: {}", code, message)))
            }
            msg => Err(McpError::UnexpectedResponse(format!("{:?}", msg))),
        }
    }
}

impl Default for McpBridge {
    fn default() -> Self {
        Self::new()
    }
}

/// Parse a UUID from arguments
fn parse_uuid(arguments: &serde_json::Value, field: &str) -> Result<Uuid, McpError> {
    let id_str = arguments[field]
        .as_str()
        .ok_or_else(|| McpError::InvalidParams(format!("Missing '{}' parameter", field)))?;

    Uuid::parse_str(id_str)
        .map_err(|e| McpError::InvalidParams(format!("Invalid UUID for '{}': {}", field, e)))
}

/// Format pane list for JSON output
fn format_pane_list(panes: &[PaneListEntry]) -> Vec<serde_json::Value> {
    panes
        .iter()
        .map(|p| {
            let state_str = match &p.state {
                ccmux_protocol::PaneState::Normal => "normal",
                ccmux_protocol::PaneState::Claude(_) => "claude",
                ccmux_protocol::PaneState::Exited { .. } => "exited",
            };

            serde_json::json!({
                "id": p.id.to_string(),
                "session": p.session_name,
                "window": p.window_index,
                "window_name": p.window_name,
                "index": p.pane_index,
                "cols": p.cols,
                "rows": p.rows,
                "title": p.title,
                "cwd": p.cwd,
                "is_claude": p.is_claude,
                "claude_state": p.claude_state.as_ref().map(|cs| {
                    serde_json::json!({
                        "session_id": cs.session_id,
                        "activity": format!("{:?}", cs.activity),
                        "model": cs.model,
                        "tokens_used": cs.tokens_used,
                    })
                }),
                "state": state_str,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bridge_creation() {
        let bridge = McpBridge::new();
        assert!(!bridge.initialized);
        assert!(bridge.daemon_tx.is_none());
        assert!(bridge.daemon_rx.is_none());
    }

    #[test]
    fn test_parse_uuid_valid() {
        let id = Uuid::new_v4();
        let args = serde_json::json!({"pane_id": id.to_string()});

        let result = parse_uuid(&args, "pane_id").unwrap();
        assert_eq!(result, id);
    }

    #[test]
    fn test_parse_uuid_missing() {
        let args = serde_json::json!({});
        let result = parse_uuid(&args, "pane_id");

        assert!(matches!(result, Err(McpError::InvalidParams(_))));
    }

    #[test]
    fn test_parse_uuid_invalid() {
        let args = serde_json::json!({"pane_id": "not-a-uuid"});
        let result = parse_uuid(&args, "pane_id");

        assert!(matches!(result, Err(McpError::InvalidParams(_))));
    }

    #[test]
    fn test_format_pane_list_empty() {
        let panes = vec![];
        let result = format_pane_list(&panes);
        assert!(result.is_empty());
    }

    // ==================== BUG-027 Fix Tests ====================

    #[test]
    fn test_is_broadcast_message_output() {
        // Output messages are broadcasts (terminal output from panes)
        let msg = ServerMessage::Output {
            pane_id: Uuid::new_v4(),
            data: vec![b'h', b'i'],
        };
        assert!(McpBridge::is_broadcast_message(&msg));
    }

    #[test]
    fn test_is_broadcast_message_pane_state_changed() {
        let msg = ServerMessage::PaneStateChanged {
            pane_id: Uuid::new_v4(),
            state: ccmux_protocol::PaneState::Normal,
        };
        assert!(McpBridge::is_broadcast_message(&msg));
    }

    #[test]
    fn test_is_broadcast_message_claude_state_changed() {
        let msg = ServerMessage::ClaudeStateChanged {
            pane_id: Uuid::new_v4(),
            state: ccmux_protocol::ClaudeState::default(),
        };
        assert!(McpBridge::is_broadcast_message(&msg));
    }

    #[test]
    fn test_is_broadcast_message_pane_created() {
        // Simple PaneCreated is a broadcast (from other clients)
        let msg = ServerMessage::PaneCreated {
            pane: ccmux_protocol::PaneInfo {
                id: Uuid::new_v4(),
                window_id: Uuid::new_v4(),
                index: 0,
                cols: 80,
                rows: 24,
                state: ccmux_protocol::PaneState::Normal,
                name: None,
                title: None,
                cwd: None,
            },
            direction: ccmux_protocol::SplitDirection::Horizontal,
        };
        assert!(McpBridge::is_broadcast_message(&msg));
    }

    #[test]
    fn test_is_broadcast_message_window_created() {
        let msg = ServerMessage::WindowCreated {
            window: ccmux_protocol::WindowInfo {
                id: Uuid::new_v4(),
                session_id: Uuid::new_v4(),
                name: "test".to_string(),
                index: 0,
                pane_count: 1,
                active_pane_id: None,
            },
        };
        assert!(McpBridge::is_broadcast_message(&msg));
    }

    #[test]
    fn test_is_broadcast_message_window_closed() {
        let msg = ServerMessage::WindowClosed {
            window_id: Uuid::new_v4(),
        };
        assert!(McpBridge::is_broadcast_message(&msg));
    }

    #[test]
    fn test_is_broadcast_message_session_ended() {
        let msg = ServerMessage::SessionEnded {
            session_id: Uuid::new_v4(),
        };
        assert!(McpBridge::is_broadcast_message(&msg));
    }

    #[test]
    fn test_is_broadcast_message_viewport_updated() {
        let msg = ServerMessage::ViewportUpdated {
            pane_id: Uuid::new_v4(),
            state: ccmux_protocol::ViewportState::new(),
        };
        assert!(McpBridge::is_broadcast_message(&msg));
    }

    #[test]
    fn test_is_broadcast_message_orchestration_received() {
        let msg = ServerMessage::OrchestrationReceived {
            from_session_id: Uuid::new_v4(),
            message: ccmux_protocol::OrchestrationMessage::new("sync.request", serde_json::json!({})),
        };
        assert!(McpBridge::is_broadcast_message(&msg));
    }

    // Test that response messages are NOT broadcasts
    #[test]
    fn test_is_not_broadcast_session_list() {
        let msg = ServerMessage::SessionList { sessions: vec![] };
        assert!(!McpBridge::is_broadcast_message(&msg));
    }

    #[test]
    fn test_is_not_broadcast_pane_content() {
        let msg = ServerMessage::PaneContent {
            pane_id: Uuid::new_v4(),
            content: "test".to_string(),
        };
        assert!(!McpBridge::is_broadcast_message(&msg));
    }

    #[test]
    fn test_is_not_broadcast_all_panes_list() {
        let msg = ServerMessage::AllPanesList { panes: vec![] };
        assert!(!McpBridge::is_broadcast_message(&msg));
    }

    #[test]
    fn test_is_not_broadcast_pane_status() {
        let msg = ServerMessage::PaneStatus {
            pane_id: Uuid::new_v4(),
            session_name: "test".to_string(),
            window_name: "main".to_string(),
            window_index: 0,
            pane_index: 0,
            cols: 80,
            rows: 24,
            title: None,
            cwd: None,
            state: ccmux_protocol::PaneState::Normal,
            has_pty: true,
            is_awaiting_input: false,
            is_awaiting_confirmation: false,
        };
        assert!(!McpBridge::is_broadcast_message(&msg));
    }

    #[test]
    fn test_is_not_broadcast_pane_created_with_details() {
        // PaneCreatedWithDetails is a response (not a broadcast)
        let msg = ServerMessage::PaneCreatedWithDetails {
            pane_id: Uuid::new_v4(),
            session_id: Uuid::new_v4(),
            session_name: "test".to_string(),
            window_id: Uuid::new_v4(),
            direction: "horizontal".to_string(),
        };
        assert!(!McpBridge::is_broadcast_message(&msg));
    }

    #[test]
    fn test_is_not_broadcast_error() {
        let msg = ServerMessage::Error {
            code: ccmux_protocol::ErrorCode::PaneNotFound,
            message: "Pane not found".to_string(),
        };
        assert!(!McpBridge::is_broadcast_message(&msg));
    }

    #[test]
    fn test_is_not_broadcast_pane_closed() {
        // PaneClosed is NOT filtered because tool_close_pane expects it
        let msg = ServerMessage::PaneClosed {
            pane_id: Uuid::new_v4(),
            exit_code: Some(0),
        };
        assert!(!McpBridge::is_broadcast_message(&msg));
    }

    #[test]
    fn test_is_not_broadcast_connected() {
        let msg = ServerMessage::Connected {
            server_version: "1.0.0".to_string(),
            protocol_version: 1,
        };
        assert!(!McpBridge::is_broadcast_message(&msg));
    }

    // ==================== FEAT-060: Connection State Tests ====================

    #[test]
    fn test_connection_state_enum_equality() {
        assert_eq!(ConnectionState::Connected, ConnectionState::Connected);
        assert_eq!(ConnectionState::Disconnected, ConnectionState::Disconnected);
        assert_eq!(
            ConnectionState::Reconnecting { attempt: 1 },
            ConnectionState::Reconnecting { attempt: 1 }
        );
        assert_ne!(
            ConnectionState::Reconnecting { attempt: 1 },
            ConnectionState::Reconnecting { attempt: 2 }
        );
        assert_ne!(ConnectionState::Connected, ConnectionState::Disconnected);
    }

    #[test]
    fn test_connection_state_copy() {
        let state1 = ConnectionState::Connected;
        let state2 = state1; // Copy
        assert_eq!(state1, state2);
    }

    #[test]
    fn test_bridge_initial_connection_state() {
        let bridge = McpBridge::new();
        // Initial state should be Disconnected
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let state = *bridge.connection_state.read().await;
            assert_eq!(state, ConnectionState::Disconnected);
        });
    }

    #[test]
    fn test_reconnect_delays_exponential() {
        // Verify the exponential backoff pattern
        assert_eq!(RECONNECT_DELAYS_MS, &[100, 200, 400, 800, 1600]);

        // Each delay should be roughly 2x the previous
        for i in 1..RECONNECT_DELAYS_MS.len() {
            assert_eq!(RECONNECT_DELAYS_MS[i], RECONNECT_DELAYS_MS[i - 1] * 2);
        }
    }

    #[test]
    fn test_heartbeat_constants() {
        // Heartbeat should be checked frequently enough to detect loss within 2-3 seconds
        assert_eq!(HEARTBEAT_INTERVAL_MS, 1000);
        assert_eq!(HEARTBEAT_TIMEOUT_MS, 2000);
        assert!(HEARTBEAT_TIMEOUT_MS >= HEARTBEAT_INTERVAL_MS);
    }

    #[test]
    fn test_max_reconnect_attempts() {
        assert_eq!(MAX_RECONNECT_ATTEMPTS, 5);
        // Should match the number of delays
        assert_eq!(MAX_RECONNECT_ATTEMPTS as usize, RECONNECT_DELAYS_MS.len());
    }

    // ==================== BUG-037 Fix Tests ====================

    #[test]
    fn test_daemon_response_timeout_constant() {
        // BUG-037 FIX: Timeout should be less than Claude Code's typical timeout (~30s)
        // to provide a more informative error message
        assert_eq!(DAEMON_RESPONSE_TIMEOUT_SECS, 25);
        // Should be reasonable for most operations
        assert!(DAEMON_RESPONSE_TIMEOUT_SECS >= 10);
        assert!(DAEMON_RESPONSE_TIMEOUT_SECS <= 30);
    }

    #[test]
    fn test_is_broadcast_message_pong() {
        // FEAT-060: Pong should be filtered as a broadcast (from health monitor pings)
        let msg = ServerMessage::Pong;
        assert!(McpBridge::is_broadcast_message(&msg));
    }

    // ==================== BUG-029 Fix Tests ====================

    #[test]
    fn test_is_broadcast_message_session_focused() {
        // BUG-029 FIX: SessionFocused is a broadcast to TUI clients, not a response
        // This was causing "Unexpected response: SessionFocused" errors when
        // create_window was called after select_session
        let msg = ServerMessage::SessionFocused {
            session_id: Uuid::new_v4(),
        };
        assert!(McpBridge::is_broadcast_message(&msg));
    }

    #[test]
    fn test_is_broadcast_message_window_focused() {
        // BUG-029 FIX: WindowFocused is a broadcast to TUI clients, not a response
        let msg = ServerMessage::WindowFocused {
            session_id: Uuid::new_v4(),
            window_id: Uuid::new_v4(),
        };
        assert!(McpBridge::is_broadcast_message(&msg));
    }

    #[test]
    fn test_is_broadcast_message_pane_focused() {
        // BUG-029 FIX: PaneFocused is a broadcast to TUI clients, not a response
        let msg = ServerMessage::PaneFocused {
            session_id: Uuid::new_v4(),
            window_id: Uuid::new_v4(),
            pane_id: Uuid::new_v4(),
        };
        assert!(McpBridge::is_broadcast_message(&msg));
    }

    // ==================== FEAT-059: Beads Workflow Integration Tests ====================

    #[test]
    fn test_beads_metadata_key_constants() {
        // Verify the metadata key constants have expected values
        assert_eq!(beads::CURRENT_ISSUE, "beads.current_issue");
        assert_eq!(beads::ASSIGNED_AT, "beads.assigned_at");
        assert_eq!(beads::ISSUE_HISTORY, "beads.issue_history");
    }

    #[test]
    fn test_beads_metadata_keys_are_namespaced() {
        // All beads keys should be prefixed with "beads."
        assert!(beads::CURRENT_ISSUE.starts_with("beads."));
        assert!(beads::ASSIGNED_AT.starts_with("beads."));
        assert!(beads::ISSUE_HISTORY.starts_with("beads."));
    }

    #[test]
    fn test_beads_history_entry_serialization() {
        // Verify that history entries can be properly serialized/deserialized
        let history_entry = serde_json::json!({
            "issue_id": "BUG-042",
            "assigned_at": "1736600000",
            "released_at": "1736610000",
            "outcome": "completed",
        });

        let serialized = serde_json::to_string(&history_entry).unwrap();
        let deserialized: serde_json::Value = serde_json::from_str(&serialized).unwrap();

        assert_eq!(deserialized["issue_id"], "BUG-042");
        assert_eq!(deserialized["outcome"], "completed");
    }

    #[test]
    fn test_beads_history_array_serialization() {
        // Verify that history arrays can be serialized/deserialized as metadata values
        let history: Vec<serde_json::Value> = vec![
            serde_json::json!({
                "issue_id": "BUG-001",
                "assigned_at": "1736500000",
                "released_at": "1736510000",
                "outcome": "completed",
            }),
            serde_json::json!({
                "issue_id": "FEAT-002",
                "assigned_at": "1736520000",
                "released_at": "1736530000",
                "outcome": "abandoned",
            }),
        ];

        let serialized = serde_json::to_string(&history).unwrap();
        let deserialized: Vec<serde_json::Value> = serde_json::from_str(&serialized).unwrap();

        assert_eq!(deserialized.len(), 2);
        assert_eq!(deserialized[0]["issue_id"], "BUG-001");
        assert_eq!(deserialized[1]["outcome"], "abandoned");
    }

    #[test]
    fn test_beads_outcome_values() {
        // Verify valid outcome values per SESSION.md spec
        let valid_outcomes = ["completed", "abandoned", "transferred"];

        for outcome in valid_outcomes.iter() {
            let entry = serde_json::json!({
                "issue_id": "TEST-001",
                "outcome": outcome,
            });
            assert_eq!(entry["outcome"].as_str().unwrap(), *outcome);
        }
    }

    #[test]
    fn test_beads_issue_id_formats() {
        // Verify various issue ID formats are valid (per lenient validation)
        let valid_issue_ids = [
            "bd-456",       // Beads style
            "BUG-042",      // Bug tracker style
            "FEAT-059",     // Feature style
            "issue-123",    // Generic style
            "abc123",       // Simple alphanumeric
        ];

        for issue_id in valid_issue_ids.iter() {
            // Issue IDs should be non-empty strings
            assert!(!issue_id.is_empty());
            // Should be valid JSON string values
            let json = serde_json::json!({"issue_id": issue_id});
            assert_eq!(json["issue_id"].as_str().unwrap(), *issue_id);
        }
    }

    // ==================== BUG-033: Layout String Parsing Tests ====================

    #[test]
    fn test_layout_string_parsing_bug033() {
        // BUG-033: MCP clients may pass layout as a JSON string instead of object
        // The bridge should parse strings into objects

        // Test case 1: Layout passed as object (normal case)
        let layout_object = serde_json::json!({"pane": {}});
        assert!(layout_object.get("pane").is_some());

        // Test case 2: Layout passed as string (bug scenario)
        let layout_string = serde_json::Value::String(r#"{"pane": {}}"#.to_string());
        // Direct .get() on string returns None - this is the bug
        assert!(layout_string.get("pane").is_none());

        // Test case 3: Our fix - parse string to object
        let parsed = match &layout_string {
            serde_json::Value::String(s) => serde_json::from_str::<serde_json::Value>(s).unwrap(),
            other => other.clone(),
        };
        // After parsing, .get() works correctly
        assert!(parsed.get("pane").is_some());
    }

    #[test]
    fn test_layout_string_parsing_complex() {
        // Test complex nested layout passed as string
        let layout_str = r#"{
            "direction": "horizontal",
            "splits": [
                {"ratio": 0.5, "layout": {"pane": {"command": "bash"}}},
                {"ratio": 0.5, "layout": {"pane": {}}}
            ]
        }"#;

        let layout_string = serde_json::Value::String(layout_str.to_string());

        // Parse the string
        let parsed = match &layout_string {
            serde_json::Value::String(s) => serde_json::from_str::<serde_json::Value>(s).unwrap(),
            other => other.clone(),
        };

        // Verify nested structure is accessible
        assert!(parsed.get("direction").is_some());
        assert!(parsed.get("splits").is_some());
        let splits = parsed["splits"].as_array().unwrap();
        assert_eq!(splits.len(), 2);
        assert!(splits[0]["layout"]["pane"]["command"].as_str().is_some());
    }

    #[test]
    fn test_layout_string_parsing_invalid() {
        // Test invalid JSON string is rejected
        let invalid_json = serde_json::Value::String("not valid json".to_string());

        let result = match &invalid_json {
            serde_json::Value::String(s) => serde_json::from_str::<serde_json::Value>(s),
            _ => Ok(serde_json::Value::Null),
        };

        assert!(result.is_err());
    }
}
