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

    /// Session not found
    #[error("Session not found: {0}")]
    SessionNotFound(String),

    /// Internal error
    #[error("Internal error: {0}")]
    Internal(String),

    /// PTY error
    #[error("PTY error: {0}")]
    Pty(String),
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
        }
    }
}
