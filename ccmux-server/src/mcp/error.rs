//! MCP error types

use std::io;

use super::protocol::JsonRpcError;

/// MCP server errors
#[derive(Debug, thiserror::Error)]
pub enum McpError {
    /// IO error (stdin/stdout)
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    /// JSON serialization/deserialization error
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// Method not found
    #[error("Method not found: {0}")]
    MethodNotFound(String),

    /// Invalid parameters
    #[error("Invalid parameters: {0}")]
    InvalidParams(String),

    /// Unknown tool
    #[error("Unknown tool: {0}")]
    UnknownTool(String),

    /// Pane not found
    #[error("Pane not found: {0}")]
    PaneNotFound(String),

    /// Window not found
    #[error("Window not found: {0}")]
    WindowNotFound(String),

    /// Session not found
    #[error("Session not found: {0}")]
    SessionNotFound(String),

    /// Internal error
    #[error("Internal error: {0}")]
    Internal(String),

    /// PTY error
    #[error("PTY error: {0}")]
    Pty(String),

    // ==================== Bridge-specific errors ====================

    /// Daemon not running
    #[error("ccmux daemon is not running")]
    DaemonNotRunning,

    /// Connection to daemon failed
    #[error("Failed to connect to daemon: {0}")]
    ConnectionFailed(String),

    /// Not connected to daemon
    #[error("Not connected to daemon")]
    NotConnected,

    /// Daemon disconnected
    #[error("Daemon connection lost")]
    DaemonDisconnected,

    /// Daemon returned an error
    #[error("Daemon error: {0}")]
    DaemonError(String),

    /// Unexpected response from daemon
    #[error("Unexpected response from daemon: {0}")]
    UnexpectedResponse(String),

    // ==================== FEAT-060: Recovery Errors ====================

    /// Connection is being recovered
    #[error("Daemon connection lost, recovery in progress (attempt {attempt}/{max})")]
    RecoveringConnection { attempt: u8, max: u8 },

    /// Recovery failed permanently
    #[error("Daemon connection lost and recovery failed after {attempts} attempts")]
    RecoveryFailed { attempts: u8 },
}

impl From<McpError> for JsonRpcError {
    fn from(err: McpError) -> Self {
        match err {
            McpError::MethodNotFound(method) => {
                JsonRpcError::new(JsonRpcError::METHOD_NOT_FOUND, format!("Method not found: {}", method))
            }
            McpError::InvalidParams(msg) => {
                JsonRpcError::new(JsonRpcError::INVALID_PARAMS, msg)
            }
            McpError::UnknownTool(name) => {
                JsonRpcError::new(JsonRpcError::METHOD_NOT_FOUND, format!("Unknown tool: {}", name))
            }
            McpError::PaneNotFound(id) => {
                JsonRpcError::new(JsonRpcError::INVALID_PARAMS, format!("Pane not found: {}", id))
            }
            McpError::WindowNotFound(id) => {
                JsonRpcError::new(JsonRpcError::INVALID_PARAMS, format!("Window not found: {}", id))
            }
            McpError::SessionNotFound(name) => {
                JsonRpcError::new(JsonRpcError::INVALID_PARAMS, format!("Session not found: {}", name))
            }
            McpError::Io(err) => {
                JsonRpcError::new(JsonRpcError::INTERNAL_ERROR, format!("IO error: {}", err))
            }
            McpError::Json(err) => {
                JsonRpcError::new(JsonRpcError::PARSE_ERROR, format!("JSON error: {}", err))
            }
            McpError::Internal(msg) => {
                JsonRpcError::new(JsonRpcError::INTERNAL_ERROR, msg)
            }
            McpError::Pty(msg) => {
                JsonRpcError::new(JsonRpcError::INTERNAL_ERROR, format!("PTY error: {}", msg))
            }
            McpError::DaemonNotRunning => {
                JsonRpcError::new(JsonRpcError::INTERNAL_ERROR, "ccmux daemon is not running".to_string())
            }
            McpError::ConnectionFailed(msg) => {
                JsonRpcError::new(JsonRpcError::INTERNAL_ERROR, format!("Connection failed: {}", msg))
            }
            McpError::NotConnected => {
                JsonRpcError::new(JsonRpcError::INTERNAL_ERROR, "Not connected to daemon".to_string())
            }
            McpError::DaemonDisconnected => {
                JsonRpcError::new(JsonRpcError::INTERNAL_ERROR, "Daemon connection lost".to_string())
            }
            McpError::DaemonError(msg) => {
                JsonRpcError::new(JsonRpcError::INTERNAL_ERROR, format!("Daemon error: {}", msg))
            }
            McpError::UnexpectedResponse(msg) => {
                JsonRpcError::new(JsonRpcError::INTERNAL_ERROR, format!("Unexpected response: {}", msg))
            }
            // FEAT-060: Recovery error conversions with structured data
            McpError::RecoveringConnection { attempt, max } => {
                JsonRpcError::with_data(
                    JsonRpcError::INTERNAL_ERROR,
                    format!("Daemon connection lost, recovery in progress ({}/{})", attempt, max),
                    serde_json::json!({
                        "error": "daemon_connection_lost",
                        "recoverable": true,
                        "reconnect_status": "attempting",
                        "reconnect_attempt": attempt,
                        "max_attempts": max
                    })
                )
            }
            McpError::RecoveryFailed { attempts } => {
                JsonRpcError::with_data(
                    JsonRpcError::INTERNAL_ERROR,
                    format!("Recovery failed after {} attempts", attempts),
                    serde_json::json!({
                        "error": "daemon_connection_lost",
                        "recoverable": false,
                        "reconnect_attempts": attempts,
                        "action_required": "Please restart the ccmux daemon"
                    })
                )
            }
        }
    }
}
