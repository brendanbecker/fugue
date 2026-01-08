//! ccmux-protocol: Shared IPC definitions for client-server communication
//!
//! This crate defines all message types and data structures used for
//! communication between the ccmux client and server over Unix sockets.

pub mod codec;
pub mod messages;
pub mod types;

// Re-export main types at crate root
pub use codec::{ClientCodec, CodecError, ServerCodec};
pub use messages::{ClientMessage, ErrorCode, ServerMessage};
pub use types::{
    ClaudeActivity, ClaudeState, Dimensions, PaneInfo, PaneState, SessionInfo, SplitDirection,
    WindowInfo,
};

/// Current protocol version
pub const PROTOCOL_VERSION: u32 = 1;
