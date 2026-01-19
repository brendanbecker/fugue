//! MCP Bridge message handlers
//!
//! Handlers for messages used by the MCP bridge to query and manipulate
//! sessions, windows, and panes without being attached to a specific session.

pub mod environment;
pub mod io;
pub mod layout;
pub mod metadata;
pub mod pane;
pub mod session;
pub mod window;

#[cfg(test)]
mod tests;

// Note: Submodules extend HandlerContext