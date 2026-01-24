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
    ClientMessage, ErrorCode, OrchestrationMessage, OrchestrationTarget, PaneListEntry,
    ServerMessage,
};
pub use types::{
    AgentActivity, AgentState, ClaudeActivity, ClaudeState, ClientType, Dimensions, JsonValue,
    MailPriority, PaneInfo, PaneState, PaneStuckStatus, PaneTarget, ReplyMessage, ReplyResult,
    SessionInfo, SplitDirection, ViewportState, Widget, WidgetConversionError, WidgetUpdate,
    WindowInfo, WorktreeInfo,
};

/// Current protocol version
pub const PROTOCOL_VERSION: u32 = 1;
