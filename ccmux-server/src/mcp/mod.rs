//! MCP (Model Context Protocol) server implementation
//!
//! Enables Claude Code to programmatically interact with ccmux through
//! the MCP protocol, providing tools to list panes, read output, create
//! panes, and send input.
//!
//! MCP Protocol: <https://modelcontextprotocol.io/>
//!
//! ## Two Modes
//!
//! - **`mcp-server`**: Standalone mode with its own session state (legacy)
//! - **`mcp-bridge`**: Connects to the ccmux daemon, sharing sessions with TUI

pub mod bridge;
mod error;
mod handlers;
pub mod keys;
mod protocol;
mod server;
mod tools;

pub use bridge::McpBridge;
pub use server::McpServer;
