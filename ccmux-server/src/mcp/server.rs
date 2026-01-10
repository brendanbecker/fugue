//! MCP Server implementation
//!
//! Implements the MCP server using stdio transport (stdin/stdout).

use std::io::{BufRead, Write};

use tracing::{debug, info};
use uuid::Uuid;

use crate::pty::PtyManager;
use crate::session::SessionManager;

use super::error::McpError;
use super::handlers::ToolContext;
use super::protocol::{
    InitializeResult, JsonRpcError, JsonRpcRequest, JsonRpcResponse, ToolResult, ToolsListResult,
};
use super::tools::get_tool_definitions;

/// MCP Server
///
/// Handles MCP protocol communication over stdio.
pub struct McpServer {
    session_manager: SessionManager,
    pty_manager: PtyManager,
    initialized: bool,
}

impl McpServer {
    /// Create a new MCP server
    pub fn new() -> Self {
        Self {
            session_manager: SessionManager::new(),
            pty_manager: PtyManager::new(),
            initialized: false,
        }
    }

    /// Create an MCP server with existing managers
    pub fn with_managers(session_manager: SessionManager, pty_manager: PtyManager) -> Self {
        Self {
            session_manager,
            pty_manager,
            initialized: false,
        }
    }

    /// Run the MCP server, reading from stdin and writing to stdout
    pub fn run(&mut self) -> Result<(), McpError> {
        let stdin = std::io::stdin();
        let stdout = std::io::stdout();
        let mut stdout = stdout.lock();

        info!("MCP server starting");

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
            let response = self.handle_request(request);

            // Write response
            let json = serde_json::to_string(&response)?;
            debug!("Sending: {}", json);
            writeln!(stdout, "{}", json)?;
            stdout.flush()?;
        }

        info!("MCP server shutting down");
        Ok(())
    }

    /// Handle a JSON-RPC request
    fn handle_request(&mut self, request: JsonRpcRequest) -> JsonRpcResponse {
        let result = match request.method.as_str() {
            "initialize" => self.handle_initialize(&request.params),
            "initialized" => {
                // Notification, no response needed but we'll acknowledge
                Ok(serde_json::json!({}))
            }
            "tools/list" => self.handle_tools_list(),
            "tools/call" => self.handle_tools_call(&request.params),
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
        info!("MCP server initialized");

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
    fn handle_tools_call(
        &mut self,
        params: &serde_json::Value,
    ) -> Result<serde_json::Value, McpError> {
        let name = params["name"]
            .as_str()
            .ok_or_else(|| McpError::InvalidParams("Missing 'name' parameter".into()))?;

        let arguments = &params["arguments"];

        debug!("Tool call: {} with args: {}", name, arguments);

        let result = self.dispatch_tool(name, arguments)?;

        serde_json::to_value(result).map_err(|e| McpError::Internal(e.to_string()))
    }

    /// Dispatch tool call to appropriate handler
    ///
    /// Returns `Err(McpError)` for protocol-level errors (unknown tool, invalid params).
    /// Returns `Ok(ToolResult::error(...))` for tool execution errors (pane not found, etc.).
    /// Returns `Ok(ToolResult::text(...))` for successful tool execution.
    fn dispatch_tool(
        &mut self,
        name: &str,
        arguments: &serde_json::Value,
    ) -> Result<ToolResult, McpError> {
        // Validate tool name first (protocol-level error if unknown)
        if !is_known_tool(name) {
            return Err(McpError::UnknownTool(name.into()));
        }

        // Parse and validate required parameters (protocol-level errors)
        let params = match name {
            "ccmux_list_panes" => ToolParams::ListPanes {
                session: arguments["session"].as_str().map(String::from),
            },
            "ccmux_read_pane" => ToolParams::ReadPane {
                pane_id: parse_uuid(arguments, "pane_id")?,
                lines: arguments["lines"].as_u64().unwrap_or(100) as usize,
            },
            "ccmux_create_pane" => ToolParams::CreatePane {
                session: arguments["session"].as_str().map(String::from),
                window: arguments["window"].as_str().map(String::from),
                direction: arguments["direction"].as_str().map(String::from),
                command: arguments["command"].as_str().map(String::from),
                cwd: arguments["cwd"].as_str().map(String::from),
            },
            "ccmux_send_input" => ToolParams::SendInput {
                pane_id: parse_uuid(arguments, "pane_id")?,
                input: arguments["input"]
                    .as_str()
                    .ok_or_else(|| McpError::InvalidParams("Missing 'input' parameter".into()))?
                    .to_string(),
            },
            "ccmux_get_status" => ToolParams::GetStatus {
                pane_id: parse_uuid(arguments, "pane_id")?,
            },
            "ccmux_close_pane" => ToolParams::ClosePane {
                pane_id: parse_uuid(arguments, "pane_id")?,
            },
            "ccmux_focus_pane" => ToolParams::FocusPane {
                pane_id: parse_uuid(arguments, "pane_id")?,
            },
            "ccmux_list_sessions" => ToolParams::ListSessions,
            "ccmux_list_windows" => ToolParams::ListWindows {
                session: arguments["session"].as_str().map(String::from),
            },
            "ccmux_create_session" => ToolParams::CreateSession {
                name: arguments["name"].as_str().map(String::from),
            },
            "ccmux_create_window" => ToolParams::CreateWindow {
                session: arguments["session"].as_str().map(String::from),
                name: arguments["name"].as_str().map(String::from),
                command: arguments["command"].as_str().map(String::from),
            },
            _ => unreachable!(), // Already validated above
        };

        // Execute tool - convert execution errors to ToolResult::error()
        let mut ctx = ToolContext::new(&mut self.session_manager, &mut self.pty_manager);

        let result = match params {
            ToolParams::ListPanes { session } => ctx.list_panes(session.as_deref()),
            ToolParams::ReadPane { pane_id, lines } => ctx.read_pane(pane_id, lines),
            ToolParams::CreatePane { session, window, direction, command, cwd } => {
                ctx.create_pane(
                    session.as_deref(),
                    window.as_deref(),
                    direction.as_deref(),
                    command.as_deref(),
                    cwd.as_deref(),
                )
            }
            ToolParams::SendInput { pane_id, input } => ctx.send_input(pane_id, &input),
            ToolParams::GetStatus { pane_id } => ctx.get_status(pane_id),
            ToolParams::ClosePane { pane_id } => ctx.close_pane(pane_id),
            ToolParams::FocusPane { pane_id } => ctx.focus_pane(pane_id),
            ToolParams::ListSessions => ctx.list_sessions(),
            ToolParams::ListWindows { session } => ctx.list_windows(session.as_deref()),
            ToolParams::CreateSession { name } => ctx.create_session(name.as_deref()),
            ToolParams::CreateWindow { session, name, command } => {
                ctx.create_window(session.as_deref(), name.as_deref(), command.as_deref())
            }
        };

        // Convert Result to ToolResult
        Ok(match result {
            Ok(text) => ToolResult::text(text),
            Err(e) => ToolResult::error(e.to_string()),
        })
    }
}

/// Check if a tool name is known
fn is_known_tool(name: &str) -> bool {
    matches!(
        name,
        "ccmux_list_panes"
            | "ccmux_read_pane"
            | "ccmux_create_pane"
            | "ccmux_send_input"
            | "ccmux_get_status"
            | "ccmux_close_pane"
            | "ccmux_focus_pane"
            | "ccmux_list_sessions"
            | "ccmux_list_windows"
            | "ccmux_create_session"
            | "ccmux_create_window"
    )
}

/// Parsed and validated tool parameters
enum ToolParams {
    ListPanes { session: Option<String> },
    ReadPane { pane_id: Uuid, lines: usize },
    CreatePane {
        session: Option<String>,
        window: Option<String>,
        direction: Option<String>,
        command: Option<String>,
        cwd: Option<String>,
    },
    SendInput { pane_id: Uuid, input: String },
    GetStatus { pane_id: Uuid },
    ClosePane { pane_id: Uuid },
    FocusPane { pane_id: Uuid },
    ListSessions,
    ListWindows { session: Option<String> },
    CreateSession { name: Option<String> },
    CreateWindow { session: Option<String>, name: Option<String>, command: Option<String> },
}

impl Default for McpServer {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_creation() {
        let server = McpServer::new();
        assert!(!server.initialized);
    }

    #[test]
    fn test_handle_initialize() {
        let mut server = McpServer::new();
        let params = serde_json::json!({});

        let result = server.handle_initialize(&params).unwrap();
        assert!(server.initialized);
        assert!(result["protocolVersion"].is_string());
        assert!(result["capabilities"]["tools"].is_object());
    }

    #[test]
    fn test_handle_tools_list() {
        let server = McpServer::new();
        let result = server.handle_tools_list().unwrap();

        let tools = result["tools"].as_array().unwrap();
        assert!(!tools.is_empty());
        assert!(tools.iter().any(|t| t["name"] == "ccmux_list_panes"));
    }

    #[test]
    fn test_handle_unknown_method() {
        let mut server = McpServer::new();
        let request = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: serde_json::json!(1),
            method: "unknown/method".into(),
            params: serde_json::json!({}),
        };

        let response = server.handle_request(request);
        assert!(response.error.is_some());
    }

    #[test]
    fn test_dispatch_list_panes() {
        let mut server = McpServer::new();
        let result = server
            .dispatch_tool("ccmux_list_panes", &serde_json::json!({}))
            .unwrap();

        assert!(result.is_error.is_none());
    }

    #[test]
    fn test_dispatch_unknown_tool() {
        let mut server = McpServer::new();
        let result = server.dispatch_tool("unknown_tool", &serde_json::json!({}));

        assert!(matches!(result, Err(McpError::UnknownTool(_))));
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
    fn test_tool_execution_error_returns_tool_result_error() {
        let mut server = McpServer::new();
        let nonexistent_pane = Uuid::new_v4();

        // Call get_status on a non-existent pane
        let result = server
            .dispatch_tool(
                "ccmux_get_status",
                &serde_json::json!({"pane_id": nonexistent_pane.to_string()}),
            )
            .unwrap(); // Should NOT return Err - tool errors are ToolResult::error()

        // Should be a ToolResult with is_error=true
        assert_eq!(result.is_error, Some(true));
    }

    #[test]
    fn test_full_request_cycle() {
        let mut server = McpServer::new();

        // Initialize
        let init_request = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: serde_json::json!(1),
            method: "initialize".into(),
            params: serde_json::json!({}),
        };
        let response = server.handle_request(init_request);
        assert!(response.error.is_none());

        // List tools
        let tools_request = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: serde_json::json!(2),
            method: "tools/list".into(),
            params: serde_json::json!({}),
        };
        let response = server.handle_request(tools_request);
        assert!(response.error.is_none());

        // Call list_panes tool
        let call_request = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: serde_json::json!(3),
            method: "tools/call".into(),
            params: serde_json::json!({
                "name": "ccmux_list_panes",
                "arguments": {}
            }),
        };
        let response = server.handle_request(call_request);
        assert!(response.error.is_none());
    }
}
