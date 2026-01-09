//! ccmux-protocol: Shared IPC definitions for client-server communication
//!
//! This crate defines all message types and data structures used for
//! communication between the ccmux client and server over Unix sockets.

pub mod codec;
pub mod messages;
pub mod types;

// Re-export main types at crate root
pub use codec::{ClientCodec, CodecError, ServerCodec};
pub use messages::{
    ClientMessage, ErrorCode, OrchestrationMessage, OrchestrationTarget, ServerMessage,
    WorkerStatus,
};
pub use types::{
    ClaudeActivity, ClaudeState, Dimensions, PaneInfo, PaneState, PaneTarget, ReplyMessage,
    ReplyResult, SessionInfo, SplitDirection, ViewportState, WindowInfo, WorktreeInfo,
};

/// Current protocol version
pub const PROTOCOL_VERSION: u32 = 1;
