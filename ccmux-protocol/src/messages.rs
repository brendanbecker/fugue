//! Client-server message types

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::types::*;

/// Messages sent from client to server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClientMessage {
    /// Initial connection handshake
    Connect {
        client_id: Uuid,
        protocol_version: u32,
    },

    /// Request list of sessions
    ListSessions,

    /// Create a new session
    CreateSession { name: String },

    /// Attach to existing session
    AttachSession { session_id: Uuid },

    /// Create new window in session
    CreateWindow {
        session_id: Uuid,
        name: Option<String>,
    },

    /// Create new pane (split)
    CreatePane {
        window_id: Uuid,
        direction: SplitDirection,
    },

    /// Send input to pane
    Input { pane_id: Uuid, data: Vec<u8> },

    /// Resize pane
    Resize { pane_id: Uuid, cols: u16, rows: u16 },

    /// Close pane
    ClosePane { pane_id: Uuid },

    /// Select/focus pane
    SelectPane { pane_id: Uuid },

    /// Detach from session (keep session running)
    Detach,

    /// Request full state sync
    Sync,

    /// Ping for keepalive
    Ping,
}

/// Messages sent from server to client
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServerMessage {
    /// Connection accepted
    Connected {
        server_version: String,
        protocol_version: u32,
    },

    /// List of available sessions
    SessionList { sessions: Vec<SessionInfo> },

    /// Session created
    SessionCreated { session: SessionInfo },

    /// Attached to session - full state
    Attached {
        session: SessionInfo,
        windows: Vec<WindowInfo>,
        panes: Vec<PaneInfo>,
    },

    /// Window created
    WindowCreated { window: WindowInfo },

    /// Pane created
    PaneCreated { pane: PaneInfo },

    /// Pane output data
    Output { pane_id: Uuid, data: Vec<u8> },

    /// Pane state changed
    PaneStateChanged { pane_id: Uuid, state: PaneState },

    /// Claude state update (for Claude-detected panes)
    ClaudeStateChanged { pane_id: Uuid, state: ClaudeState },

    /// Pane closed
    PaneClosed {
        pane_id: Uuid,
        exit_code: Option<i32>,
    },

    /// Window closed
    WindowClosed { window_id: Uuid },

    /// Session ended
    SessionEnded { session_id: Uuid },

    /// Error response
    Error { code: ErrorCode, message: String },

    /// Pong response to ping
    Pong,
}

/// Error codes for protocol errors
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ErrorCode {
    SessionNotFound,
    WindowNotFound,
    PaneNotFound,
    InvalidOperation,
    ProtocolMismatch,
    InternalError,
}
