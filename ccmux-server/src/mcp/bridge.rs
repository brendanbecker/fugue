//! MCP Bridge - Connects MCP protocol to the ccmux daemon
//!
//! This module implements the MCP bridge that translates between MCP JSON-RPC
//! (over stdio) and the ccmux IPC protocol (over Unix socket).
//!
//! Instead of running a standalone MCP server with its own session state,
//! the bridge connects to the existing ccmux daemon so Claude can control
//! the same sessions the user sees in the TUI.

use std::io::{BufRead, Write};
use std::time::Duration;

use futures::{SinkExt, StreamExt};
use tokio::net::UnixStream;
use tokio::sync::mpsc;
use tokio_util::codec::Framed;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use ccmux_protocol::{
    ClientCodec, ClientMessage, PaneListEntry, ServerMessage, SplitDirection, PROTOCOL_VERSION,
};
use ccmux_utils::socket_path;

use super::error::McpError;
use super::protocol::{
    InitializeResult, JsonRpcError, JsonRpcRequest, JsonRpcResponse, ToolResult, ToolsListResult,
};
use super::tools::get_tool_definitions;

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
}

impl McpBridge {
    /// Create a new MCP bridge
    pub fn new() -> Self {
        Self {
            daemon_tx: None,
            daemon_rx: None,
            initialized: false,
            client_id: Uuid::new_v4(),
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

        // Spawn task to handle socket I/O
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    // Send outgoing messages
                    Some(msg) = outgoing_rx.recv() => {
                        if let Err(e) = sink.send(msg).await {
                            error!("Failed to send to daemon: {}", e);
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
                                break;
                            }
                            None => {
                                info!("Daemon connection closed");
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
                Ok(())
            }
            ServerMessage::Error { code, message } => {
                Err(McpError::DaemonError(format!("{:?}: {}", code, message)))
            }
            msg => Err(McpError::UnexpectedResponse(format!("{:?}", msg))),
        }
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

    /// Receive a message from the daemon
    async fn recv_from_daemon(&mut self) -> Result<ServerMessage, McpError> {
        let rx = self
            .daemon_rx
            .as_mut()
            .ok_or(McpError::NotConnected)?;

        rx.recv()
            .await
            .ok_or(McpError::DaemonDisconnected)
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
    async fn handle_tools_call(
        &mut self,
        params: &serde_json::Value,
    ) -> Result<serde_json::Value, McpError> {
        let name = params["name"]
            .as_str()
            .ok_or_else(|| McpError::InvalidParams("Missing 'name' parameter".into()))?;

        let arguments = &params["arguments"];

        debug!("Tool call: {} with args: {}", name, arguments);

        let result = self.dispatch_tool(name, arguments).await?;

        serde_json::to_value(result).map_err(|e| McpError::Internal(e.to_string()))
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
                let direction = arguments["direction"].as_str().map(String::from);
                let command = arguments["command"].as_str().map(String::from);
                let cwd = arguments["cwd"].as_str().map(String::from);
                let select = arguments["select"].as_bool().unwrap_or(false);
                self.tool_create_pane(session, window, direction, command, cwd, select)
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
                let layout = arguments["layout"].clone();
                self.tool_create_layout(session, window, layout).await
            }
            _ => Err(McpError::UnknownTool(name.into())),
        }
    }

    // ==================== Tool Implementations ====================

    async fn tool_list_sessions(&mut self) -> Result<ToolResult, McpError> {
        self.send_to_daemon(ClientMessage::ListSessions).await?;

        match self.recv_from_daemon().await? {
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

        match self.recv_from_daemon().await? {
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

        match self.recv_from_daemon().await? {
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

        match self.recv_from_daemon().await? {
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

        match self.recv_from_daemon().await? {
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

        match self.recv_from_daemon().await? {
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

        match self.recv_from_daemon().await? {
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

        self.send_to_daemon(ClientMessage::CreatePaneWithOptions {
            session_filter: session,
            window_filter: window,
            direction: split_direction,
            command,
            cwd,
            select,
        })
        .await?;

        match self.recv_from_daemon().await? {
            ServerMessage::PaneCreatedWithDetails {
                pane_id,
                session_id,
                session_name,
                window_id,
                direction,
            } => {
                let result = serde_json::json!({
                    "pane_id": pane_id.to_string(),
                    "session_id": session_id.to_string(),
                    "session": session_name,
                    "window_id": window_id.to_string(),
                    "direction": direction,
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

        match self.recv_from_daemon().await? {
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

        // SelectPane doesn't have a dedicated response in current protocol
        // Wait briefly and return success
        let result = serde_json::json!({
            "pane_id": pane_id.to_string(),
            "status": "focused"
        });

        let json = serde_json::to_string_pretty(&result)
            .map_err(|e| McpError::Internal(e.to_string()))?;
        Ok(ToolResult::text(json))
    }

    async fn tool_select_window(&mut self, window_id: Uuid) -> Result<ToolResult, McpError> {
        self.send_to_daemon(ClientMessage::SelectWindow { window_id })
            .await?;

        // SelectWindow doesn't have a dedicated response in current protocol
        let result = serde_json::json!({
            "window_id": window_id.to_string(),
            "status": "selected"
        });

        let json = serde_json::to_string_pretty(&result)
            .map_err(|e| McpError::Internal(e.to_string()))?;
        Ok(ToolResult::text(json))
    }

    async fn tool_select_session(&mut self, session_id: Uuid) -> Result<ToolResult, McpError> {
        self.send_to_daemon(ClientMessage::SelectSession { session_id })
            .await?;

        // SelectSession doesn't have a dedicated response in current protocol
        let result = serde_json::json!({
            "session_id": session_id.to_string(),
            "status": "selected"
        });

        let json = serde_json::to_string_pretty(&result)
            .map_err(|e| McpError::Internal(e.to_string()))?;
        Ok(ToolResult::text(json))
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

        match self.recv_from_daemon().await? {
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

        self.send_to_daemon(ClientMessage::SplitPane {
            pane_id,
            direction: split_direction,
            ratio,
            command,
            cwd,
            select,
        })
        .await?;

        match self.recv_from_daemon().await? {
            ServerMessage::PaneSplit {
                new_pane_id,
                original_pane_id,
                session_id,
                session_name,
                window_id,
                direction,
            } => {
                let result = serde_json::json!({
                    "new_pane_id": new_pane_id.to_string(),
                    "original_pane_id": original_pane_id.to_string(),
                    "session_id": session_id.to_string(),
                    "session": session_name,
                    "window_id": window_id.to_string(),
                    "direction": direction,
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

        match self.recv_from_daemon().await? {
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
            layout,
        })
        .await?;

        match self.recv_from_daemon().await? {
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
}
