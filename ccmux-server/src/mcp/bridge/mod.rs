//! MCP Bridge - Connects MCP protocol to the ccmux daemon
//!
//! This module implements the MCP bridge that translates between MCP JSON-RPC
//! (over stdio) and the ccmux IPC protocol (over Unix socket).

pub mod connection;
pub mod handlers;
pub mod health;
pub mod orchestration;

#[cfg(test)]
mod tests;

use std::io::Write;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::io::{AsyncBufReadExt, BufReader};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Global request counter for generating unique request IDs within this bridge instance
static REQUEST_COUNTER: AtomicU64 = AtomicU64::new(1);

use crate::mcp::error::McpError;
use crate::mcp::protocol::{
    InitializeResult, JsonRpcError, JsonRpcRequest, JsonRpcResponse, ToolResult, ToolsListResult,
};
use crate::mcp::tools::get_tool_definitions;

use self::connection::{ConnectionManager, MAX_RECONNECT_ATTEMPTS};
use self::handlers::{ToolHandlers, parse_uuid};
use self::health::ConnectionState;

/// MCP Bridge
///
/// Connects to the ccmux daemon and handles MCP protocol communication over stdio.
pub struct McpBridge {
    connection: ConnectionManager,
    initialized: bool,
}

impl McpBridge {
    /// Create a new MCP bridge
    pub fn new() -> Self {
        Self {
            connection: ConnectionManager::new(),
            initialized: false,
        }
    }

    /// Run the MCP bridge, reading from stdin and writing to stdout
    pub async fn run(&mut self) -> Result<(), McpError> {
        // Connect to daemon first
        self.connection.connect_to_daemon().await?;

        // Use async stdin to avoid blocking the Tokio runtime (BUG-044 fix)
        let stdin = tokio::io::stdin();
        let reader = BufReader::new(stdin);
        let mut lines = reader.lines();

        let stdout = std::io::stdout();
        let mut stdout = stdout.lock();

        info!("MCP bridge starting");

        while let Some(line) = lines.next_line().await? {
            if line.is_empty() {
                continue;
            }

            // Generate a unique request ID for logging correlation
            let log_req_id = REQUEST_COUNTER.fetch_add(1, Ordering::Relaxed);

            // Log raw incoming request at debug level (may contain sensitive data)
            debug!(req_id = log_req_id, raw = %line, "Received raw JSON-RPC request");

            // Parse request
            let request: JsonRpcRequest = match serde_json::from_str(&line) {
                Ok(req) => req,
                Err(e) => {
                    error!(
                        req_id = log_req_id,
                        error = %e,
                        raw_input = %line,
                        "Failed to parse JSON-RPC request"
                    );
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

            // Log parsed request at info level
            info!(
                req_id = log_req_id,
                method = %request.method,
                jsonrpc_id = ?request.id,
                "Incoming JSON-RPC request"
            );

            // Validate JSON-RPC version
            if request.jsonrpc != "2.0" {
                error!(
                    req_id = log_req_id,
                    method = %request.method,
                    got_version = %request.jsonrpc,
                    "Invalid JSON-RPC version"
                );
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

            // Handle request with timing
            let start = std::time::Instant::now();
            let response = self.handle_request(request.clone()).await;
            let elapsed_ms = start.elapsed().as_millis();

            if let Some(response) = response {
                // Log response at info level
                let is_error = matches!(response, JsonRpcResponse { error: Some(_), .. });
                if is_error {
                    warn!(
                        req_id = log_req_id,
                        method = %request.method,
                        elapsed_ms = %elapsed_ms,
                        error = ?response.error,
                        "JSON-RPC request completed with error"
                    );
                } else {
                    info!(
                        req_id = log_req_id,
                        method = %request.method,
                        elapsed_ms = %elapsed_ms,
                        "JSON-RPC request completed successfully"
                    );
                }

                // Write response
                let json = serde_json::to_string(&response)?;
                debug!(req_id = log_req_id, raw = %json, "Sending raw JSON-RPC response");
                writeln!(stdout, "{}", json)?;
                stdout.flush()?;
            } else {
                info!(
                    req_id = log_req_id,
                    method = %request.method,
                    elapsed_ms = %elapsed_ms,
                    "Notification handled (no response)"
                );
            }
        }

        info!("MCP bridge shutting down");
        Ok(())
    }

    /// Handle a JSON-RPC request
    async fn handle_request(&mut self, request: JsonRpcRequest) -> Option<JsonRpcResponse> {
        let is_notification = request.id.is_null();
        let result = match request.method.as_str() {
            "initialize" => self.handle_initialize(&request.params),
            "initialized" | "notifications/initialized" => Ok(serde_json::json!({})),
            "ping" => Ok(serde_json::json!({})),
            "tools/list" => self.handle_tools_list(),
            "tools/call" => self.handle_tools_call(&request.params).await,
            _ => Err(McpError::MethodNotFound(request.method.clone())),
        };

        if is_notification {
            if let Err(e) = result {
                warn!(method = %request.method, error = %e, "Notification handling failed");
            }
            return None;
        }

        Some(match result {
            Ok(value) => JsonRpcResponse::success(request.id, value),
            Err(e) => JsonRpcResponse::error(request.id, e.into()),
        })
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

        info!(tool = %name, "Dispatching tool call");
        debug!(tool = %name, arguments = %arguments, "Tool call arguments");

        // FEAT-060: Check connection state and handle recovery
        let result = match self.dispatch_tool_with_recovery(name, arguments).await {
            Ok(r) => {
                info!(tool = %name, "Tool call completed successfully");
                Ok(r)
            }
            Err(e) => {
                error!(
                    tool = %name,
                    error = %e,
                    error_debug = ?e,
                    arguments = %arguments,
                    "Tool call failed"
                );
                Err(e)
            }
        }?;

        serde_json::to_value(result).map_err(|e| McpError::Internal(e.to_string()))
    }

    /// Dispatch tool call with automatic connection recovery
    async fn dispatch_tool_with_recovery(
        &mut self,
        name: &str,
        arguments: &serde_json::Value,
    ) -> Result<ToolResult, McpError> {
        // Check current connection state
        let state = *self.connection.connection_state.read().await;

        match state {
            ConnectionState::Connected => {
                // Try to execute the tool
                match self.dispatch_tool(name, arguments).await {
                    Ok(result) => Ok(result),
                    Err(McpError::DaemonDisconnected) 
                    | Err(McpError::NotConnected)
                    | Err(McpError::ResponseTimeout { .. }) => {
                        // Connection lost or timed out during tool execution - attempt recovery
                        // For ResponseTimeout, reconnecting is essential to clear the stale response 
                        // from the head of the queue (BUG-035).
                        warn!("Connection lost or timed out during tool execution, attempting recovery");

                        // Update state
                        {
                            let mut s = self.connection.connection_state.write().await;
                            *s = ConnectionState::Disconnected;
                        }
                        let _ = self.connection.state_tx.send(ConnectionState::Disconnected);

                        // Attempt reconnection
                        self.connection.attempt_reconnection().await?;

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
                self.connection.attempt_reconnection().await?;

                // Execute the tool after successful reconnection
                self.dispatch_tool(name, arguments).await
            }
        }
    }

    /// Dispatch tool call to handler
    async fn dispatch_tool(
        &mut self,
        name: &str,
        arguments: &serde_json::Value,
    ) -> Result<ToolResult, McpError> {
        let mut handlers = ToolHandlers::new(&mut self.connection);

        match name {
            "ccmux_list_sessions" => handlers.tool_list_sessions().await,
            "ccmux_list_windows" => {
                let session = arguments["session"].as_str().map(String::from);
                handlers.tool_list_windows(session).await
            }
            "ccmux_list_panes" => {
                let session = arguments["session"].as_str().map(String::from);
                handlers.tool_list_panes(session).await
            }
            "ccmux_read_pane" => {
                let pane_id = parse_uuid(arguments, "pane_id")?;
                let lines = arguments["lines"].as_u64().unwrap_or(100) as usize;
                handlers.tool_read_pane(pane_id, lines).await
            }
            "ccmux_get_status" => {
                let pane_id = parse_uuid(arguments, "pane_id")?;
                handlers.tool_get_status(pane_id).await
            }
            "ccmux_create_session" => {
                let name = arguments["name"].as_str().map(String::from);
                let command = arguments["command"].as_str().map(String::from);
                let cwd = arguments["cwd"].as_str().map(String::from);
                let model = arguments["model"].as_str().map(String::from);
                let config = arguments["config"].as_object().map(|o| serde_json::Value::Object(o.clone()));
                let preset = arguments["preset"].as_str().map(String::from);
                handlers.tool_create_session(name, command, cwd, model, config, preset).await
            }
            "ccmux_attach_session" => {
                let session_id = parse_uuid(arguments, "session_id")?;
                handlers.tool_attach_session(session_id).await
            }
            "ccmux_create_window" => {
                let session = arguments["session"].as_str().map(String::from);
                let name = arguments["name"].as_str().map(String::from);
                let command = arguments["command"].as_str().map(String::from);
                let cwd = arguments["cwd"].as_str().map(String::from);
                handlers.tool_create_window(session, name, command, cwd).await
            }
            "ccmux_create_pane" => {
                let session = arguments["session"].as_str().map(String::from);
                let window = arguments["window"].as_str().map(String::from);
                let name = arguments["name"].as_str().map(String::from);
                let direction = arguments["direction"].as_str().map(String::from);
                let command = arguments["command"].as_str().map(String::from);
                let cwd = arguments["cwd"].as_str().map(String::from);
                let select = arguments["select"].as_bool().unwrap_or(false);
                let model = arguments["model"].as_str().map(String::from);
                let config = arguments["config"].as_object().map(|o| serde_json::Value::Object(o.clone()));
                let preset = arguments["preset"].as_str().map(String::from);

                handlers.tool_create_pane(
                    session, window, name, direction, command, cwd, select, model, config, preset,
                )
                .await
            }
            "ccmux_send_input" => {
                let pane_id = parse_uuid(arguments, "pane_id")?;
                let input = arguments["input"].as_str().map(String::from);
                let key = arguments["key"].as_str().map(String::from);
                let submit = arguments["submit"].as_bool().unwrap_or(false);
                handlers.tool_send_input(pane_id, input, key, submit).await
            }
            "ccmux_close_pane" => {
                let pane_id = parse_uuid(arguments, "pane_id")?;
                handlers.tool_close_pane(pane_id).await
            }
            "ccmux_focus_pane" => {
                let pane_id = parse_uuid(arguments, "pane_id")?;
                handlers.tool_focus_pane(pane_id).await
            }
            "ccmux_select_window" => {
                let window_id = parse_uuid(arguments, "window_id")?;
                handlers.tool_select_window(window_id).await
            }
            "ccmux_select_session" => {
                let session_id = parse_uuid(arguments, "session_id")?;
                handlers.tool_select_session(session_id).await
            }
            "ccmux_rename_session" => {
                let session = arguments["session"]
                    .as_str()
                    .ok_or_else(|| McpError::InvalidParams("Missing 'session' parameter".into()))?;
                let name = arguments["name"]
                    .as_str()
                    .ok_or_else(|| McpError::InvalidParams("Missing 'name' parameter".into()))?;
                handlers.tool_rename_session(session, name).await
            }
            "ccmux_rename_pane" => {
                let pane_id = parse_uuid(arguments, "pane_id")?;
                let name = arguments["name"]
                    .as_str()
                    .ok_or_else(|| McpError::InvalidParams("Missing 'name' parameter".into()))?;
                handlers.tool_rename_pane(pane_id, name).await
            }
            "ccmux_rename_window" => {
                let window_id = parse_uuid(arguments, "window_id")?;
                let name = arguments["name"]
                    .as_str()
                    .ok_or_else(|| McpError::InvalidParams("Missing 'name' parameter".into()))?;
                handlers.tool_rename_window(window_id, name).await
            }
            "ccmux_split_pane" => {
                let pane_id = parse_uuid(arguments, "pane_id")?;
                let direction = arguments["direction"].as_str().map(String::from);
                let ratio = arguments["ratio"].as_f64().unwrap_or(0.5) as f32;
                let command = arguments["command"].as_str().map(String::from);
                let cwd = arguments["cwd"].as_str().map(String::from);
                let select = arguments["select"].as_bool().unwrap_or(false);
                handlers.tool_split_pane(pane_id, direction, ratio, command, cwd, select)
                    .await
            }
            "ccmux_resize_pane" => {
                let pane_id = parse_uuid(arguments, "pane_id")?;
                let delta = arguments["delta"]
                    .as_f64()
                    .ok_or_else(|| McpError::InvalidParams("Missing 'delta' parameter".into()))?
                    as f32;
                handlers.tool_resize_pane(pane_id, delta).await
            }
            "ccmux_create_layout" => {
                let session = arguments["session"].as_str().map(String::from);
                let window = arguments["window"].as_str().map(String::from);
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
                handlers.tool_create_layout(session, window, layout).await
            }
            "ccmux_kill_session" => {
                let session = arguments["session"]
                    .as_str()
                    .ok_or_else(|| McpError::InvalidParams("Missing 'session' parameter".into()))?;
                handlers.tool_kill_session(session).await
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
                handlers.tool_set_environment(session, key, value).await
            }
            "ccmux_get_environment" => {
                let session = arguments["session"]
                    .as_str()
                    .ok_or_else(|| McpError::InvalidParams("Missing 'session' parameter".into()))?;
                let key = arguments["key"].as_str().map(String::from);
                handlers.tool_get_environment(session, key).await
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
                handlers.tool_set_metadata(session, key, value).await
            }
            "ccmux_get_metadata" => {
                let session = arguments["session"]
                    .as_str()
                    .ok_or_else(|| McpError::InvalidParams("Missing 'session' parameter".into()))?;
                let key = arguments["key"].as_str().map(String::from);
                handlers.tool_get_metadata(session, key).await
            }
            "ccmux_send_orchestration" => {
                let raw_target = arguments["target"].clone();
                // BUG-061: Handle target as JSON string (some MCP clients serialize nested objects)
                let target = match &raw_target {
                    serde_json::Value::String(s) => {
                        debug!("Parsing target from JSON string");
                        serde_json::from_str(s).map_err(|e| {
                            McpError::InvalidParams(format!("Invalid target JSON string: {}", e))
                        })?
                    }
                    other => other.clone(),
                };
                let msg_type = arguments["msg_type"]
                    .as_str()
                    .ok_or_else(|| McpError::InvalidParams("Missing 'msg_type' parameter".into()))?;
                let payload = arguments["payload"].clone();
                handlers.tool_send_orchestration(&target, msg_type, payload).await
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
                handlers.tool_set_tags(session, add, remove).await
            }
            "ccmux_get_tags" => {
                let session = arguments["session"].as_str().map(String::from);
                handlers.tool_get_tags(session).await
            }
            "ccmux_report_status" => {
                let status = arguments["status"]
                    .as_str()
                    .ok_or_else(|| McpError::InvalidParams("Missing 'status' parameter".into()))?;
                let message = arguments["message"].as_str().map(String::from);
                handlers.tool_report_status(status, message).await
            }
            "ccmux_request_help" => {
                let context = arguments["context"]
                    .as_str()
                    .ok_or_else(|| McpError::InvalidParams("Missing 'context' parameter".into()))?;
                handlers.tool_request_help(context).await
            }
            "ccmux_broadcast" => {
                let msg_type = arguments["msg_type"]
                    .as_str()
                    .ok_or_else(|| McpError::InvalidParams("Missing 'msg_type' parameter".into()))?;
                let payload = arguments["payload"].clone();
                handlers.tool_broadcast(msg_type, payload).await
            }
            "ccmux_connection_status" => handlers.tool_connection_status().await,
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
                handlers.tool_beads_assign(issue_id, pane_id).await
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
                handlers.tool_beads_release(pane_id, outcome).await
            }
            "ccmux_beads_find_pane" => {
                let issue_id = arguments["issue_id"]
                    .as_str()
                    .ok_or_else(|| McpError::InvalidParams("Missing 'issue_id' parameter".into()))?;
                handlers.tool_beads_find_pane(issue_id).await
            }
            "ccmux_beads_pane_history" => {
                let pane_id = arguments["pane_id"]
                    .as_str()
                    .map(|s| {
                        Uuid::parse_str(s)
                            .map_err(|e| McpError::InvalidParams(format!("Invalid pane_id: {}", e)))
                    })
                    .transpose()?;
                handlers.tool_beads_pane_history(pane_id).await
            }
            "ccmux_mirror_pane" => {
                let source_pane_id = parse_uuid(arguments, "source_pane_id")?;
                let direction = arguments["direction"].as_str();
                handlers.tool_mirror_pane(source_pane_id, direction).await
            }
            "ccmux_expect" => {
                let pane_id = parse_uuid(arguments, "pane_id")?;
                let pattern = arguments["pattern"]
                    .as_str()
                    .ok_or_else(|| McpError::InvalidParams("Missing 'pattern' parameter".into()))?;
                let timeout_ms = arguments["timeout_ms"].as_u64().unwrap_or(60000);
                let action = arguments["action"].as_str().unwrap_or("notify");
                let poll_interval_ms = arguments["poll_interval_ms"].as_u64().unwrap_or(200);
                let lines = arguments["lines"].as_u64().unwrap_or(100) as usize;

                handlers.tool_expect(pane_id, pattern, timeout_ms, action, poll_interval_ms, lines).await
            }
            "ccmux_run_parallel" => {
                let request: orchestration::RunParallelRequest = serde_json::from_value(arguments.clone())
                    .map_err(|e| McpError::InvalidParams(format!("Invalid run_parallel parameters: {}", e)))?;
                orchestration::run_parallel(handlers.connection, request).await
            }
            "ccmux_run_pipeline" => {
                let request: orchestration::RunPipelineRequest = serde_json::from_value(arguments.clone())
                    .map_err(|e| McpError::InvalidParams(format!("Invalid run_pipeline parameters: {}", e)))?;
                handlers.tool_run_pipeline(request).await
            }
            "ccmux_get_worker_status" => {
                let worker_id = arguments["worker_id"].as_str().map(String::from);
                handlers.tool_get_worker_status(worker_id).await
            }
            "ccmux_poll_messages" => {
                let worker_id = arguments["worker_id"]
                    .as_str()
                    .ok_or_else(|| McpError::InvalidParams("Missing 'worker_id' parameter".into()))?
                    .to_string();
                handlers.tool_poll_messages(worker_id).await
            }
            _ => Err(McpError::UnknownTool(name.into())),
        }
    }
}

impl Default for McpBridge {
    fn default() -> Self {
        Self::new()
    }
}pub mod orchestration_context;
