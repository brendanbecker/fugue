//! JSON-RPC and MCP protocol types
//!
//! Implements the JSON-RPC 2.0 message types used by the MCP protocol.

use serde::{Deserialize, Serialize};

/// JSON-RPC 2.0 request
#[derive(Debug, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    /// JSON-RPC version (always "2.0")
    pub jsonrpc: String,
    /// Request ID (can be string, number, or null)
    pub id: serde_json::Value,
    /// Method name
    pub method: String,
    /// Method parameters (optional)
    #[serde(default)]
    pub params: serde_json::Value,
}

/// JSON-RPC 2.0 response
#[derive(Debug, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    /// JSON-RPC version (always "2.0")
    pub jsonrpc: String,
    /// Request ID that this response is for
    pub id: serde_json::Value,
    /// Result (mutually exclusive with error)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    /// Error (mutually exclusive with result)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

impl JsonRpcResponse {
    /// Create a success response
    pub fn success(id: serde_json::Value, result: serde_json::Value) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            id,
            result: Some(result),
            error: None,
        }
    }

    /// Create an error response
    pub fn error(id: serde_json::Value, error: JsonRpcError) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            id,
            result: None,
            error: Some(error),
        }
    }
}

/// JSON-RPC 2.0 error object
#[derive(Debug, Serialize, Deserialize)]
pub struct JsonRpcError {
    /// Error code
    pub code: i32,
    /// Error message
    pub message: String,
    /// Additional error data (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

impl JsonRpcError {
    /// Create a new JSON-RPC error
    pub fn new(code: i32, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            data: None,
        }
    }

    /// Create a new JSON-RPC error with additional data
    pub fn with_data(code: i32, message: impl Into<String>, data: serde_json::Value) -> Self {
        Self {
            code,
            message: message.into(),
            data: Some(data),
        }
    }

    // Standard JSON-RPC error codes
    /// Parse error: Invalid JSON was received
    pub const PARSE_ERROR: i32 = -32700;
    /// Invalid Request: The JSON sent is not a valid Request object
    pub const INVALID_REQUEST: i32 = -32600;
    /// Method not found: The method does not exist
    pub const METHOD_NOT_FOUND: i32 = -32601;
    /// Invalid params: Invalid method parameter(s)
    pub const INVALID_PARAMS: i32 = -32602;
    /// Internal error: Internal JSON-RPC error
    pub const INTERNAL_ERROR: i32 = -32603;
}

/// MCP Tool definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    /// Tool name (e.g., "ccmux_list_panes")
    pub name: String,
    /// Human-readable description
    pub description: String,
    /// JSON Schema for input parameters
    #[serde(rename = "inputSchema")]
    pub input_schema: serde_json::Value,
}

/// MCP Tool call result
#[derive(Debug, Serialize, Deserialize)]
pub struct ToolResult {
    /// Content blocks returned by the tool
    pub content: Vec<ToolContent>,
    /// Whether this result represents an error
    #[serde(rename = "isError", skip_serializing_if = "Option::is_none")]
    pub is_error: Option<bool>,
}

impl ToolResult {
    /// Create a text result
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            content: vec![ToolContent::Text { text: text.into() }],
            is_error: None,
        }
    }

    /// Create an error result
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            content: vec![ToolContent::Text {
                text: message.into(),
            }],
            is_error: Some(true),
        }
    }
}

/// MCP Tool content types
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ToolContent {
    /// Text content
    #[serde(rename = "text")]
    Text {
        /// The text content
        text: String,
    },
}

/// MCP Server capabilities
#[derive(Debug, Serialize, Deserialize)]
pub struct ServerCapabilities {
    /// Tool capabilities
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<ToolsCapability>,
}

impl Default for ServerCapabilities {
    fn default() -> Self {
        Self {
            tools: Some(ToolsCapability {}),
        }
    }
}

/// Tool capability marker (empty for now, but can be extended)
#[derive(Debug, Serialize, Deserialize)]
pub struct ToolsCapability {}

/// MCP Server information
#[derive(Debug, Serialize, Deserialize)]
pub struct ServerInfo {
    /// Server name
    pub name: String,
    /// Server version
    pub version: String,
}

impl Default for ServerInfo {
    fn default() -> Self {
        Self {
            name: "ccmux".into(),
            version: env!("CARGO_PKG_VERSION").into(),
        }
    }
}

/// Initialize response
#[derive(Debug, Serialize, Deserialize)]
pub struct InitializeResult {
    /// Protocol version
    #[serde(rename = "protocolVersion")]
    pub protocol_version: String,
    /// Server capabilities
    pub capabilities: ServerCapabilities,
    /// Server info
    #[serde(rename = "serverInfo")]
    pub server_info: ServerInfo,
}

impl Default for InitializeResult {
    fn default() -> Self {
        Self {
            protocol_version: "2024-11-05".into(),
            capabilities: ServerCapabilities::default(),
            server_info: ServerInfo::default(),
        }
    }
}

/// Tools list response
#[derive(Debug, Serialize, Deserialize)]
pub struct ToolsListResult {
    /// Available tools
    pub tools: Vec<Tool>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_rpc_request_deserialize() {
        let json = r#"{
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {}
        }"#;

        let request: JsonRpcRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.jsonrpc, "2.0");
        assert_eq!(request.id, serde_json::json!(1));
        assert_eq!(request.method, "initialize");
    }

    #[test]
    fn test_json_rpc_request_without_params() {
        let json = r#"{
            "jsonrpc": "2.0",
            "id": "abc",
            "method": "tools/list"
        }"#;

        let request: JsonRpcRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.method, "tools/list");
        assert!(request.params.is_null());
    }

    #[test]
    fn test_json_rpc_response_success() {
        let response = JsonRpcResponse::success(
            serde_json::json!(1),
            serde_json::json!({"status": "ok"}),
        );

        assert_eq!(response.jsonrpc, "2.0");
        assert!(response.result.is_some());
        assert!(response.error.is_none());
    }

    #[test]
    fn test_json_rpc_response_error() {
        let error = JsonRpcError::new(JsonRpcError::METHOD_NOT_FOUND, "Method not found");
        let response = JsonRpcResponse::error(serde_json::json!(1), error);

        assert_eq!(response.jsonrpc, "2.0");
        assert!(response.result.is_none());
        assert!(response.error.is_some());
    }

    #[test]
    fn test_tool_result_text() {
        let result = ToolResult::text("Hello, world!");
        assert!(result.is_error.is_none());
        assert_eq!(result.content.len(), 1);
    }

    #[test]
    fn test_tool_result_error() {
        let result = ToolResult::error("Something went wrong");
        assert_eq!(result.is_error, Some(true));
    }

    #[test]
    fn test_initialize_result_serialization() {
        let result = InitializeResult::default();
        let json = serde_json::to_string(&result).unwrap();

        assert!(json.contains("protocolVersion"));
        assert!(json.contains("2024-11-05"));
        assert!(json.contains("ccmux"));
    }

    #[test]
    fn test_tool_definition() {
        let tool = Tool {
            name: "test_tool".into(),
            description: "A test tool".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {}
            }),
        };

        let json = serde_json::to_string(&tool).unwrap();
        assert!(json.contains("test_tool"));
        assert!(json.contains("inputSchema"));
    }
}
