//! Client-server message types

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::types::*;

// ==================== Orchestration Types ====================

/// Messages for cross-session orchestration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum OrchestrationMessage {
    /// Status update from a worker session
    StatusUpdate {
        session_id: Uuid,
        status: WorkerStatus,
        message: Option<String>,
    },
    /// Task assignment from orchestrator
    TaskAssignment {
        task_id: Uuid,
        description: String,
        files: Vec<String>,
    },
    /// Task completion notification
    TaskComplete {
        task_id: Uuid,
        success: bool,
        summary: String,
    },
    /// Request for help/escalation
    HelpRequest {
        session_id: Uuid,
        context: String,
    },
    /// Broadcast message to all sessions
    Broadcast {
        from_session_id: Uuid,
        message: String,
    },
    /// Sync request (ask all sessions to report status)
    SyncRequest,
}

/// Worker session status
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum WorkerStatus {
    Idle,
    Working,
    WaitingForInput,
    Blocked,
    Complete,
    Error,
}

/// Target for orchestration messages
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum OrchestrationTarget {
    /// Send to orchestrator session
    Orchestrator,
    /// Send to specific session
    Session(Uuid),
    /// Broadcast to all sessions in same repo
    Broadcast,
    /// Send to sessions in specific worktree
    Worktree(String),
}

/// Messages sent from client to server
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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

    /// Set viewport scroll offset for a pane
    SetViewportOffset {
        pane_id: Uuid,
        /// Lines from bottom (0 = at bottom, following output)
        offset: usize,
    },

    /// Jump viewport to bottom (unpin and follow output)
    JumpToBottom { pane_id: Uuid },

    /// Send a reply to a pane awaiting input
    Reply { reply: crate::types::ReplyMessage },

    /// Send orchestration message to other sessions
    SendOrchestration {
        target: OrchestrationTarget,
        message: OrchestrationMessage,
    },

    // ==================== MCP Bridge Messages ====================

    /// List all panes across all sessions (for MCP bridge)
    ListAllPanes {
        /// Optional session name or ID to filter by
        session_filter: Option<String>,
    },

    /// List windows in a session (for MCP bridge)
    ListWindows {
        /// Session name or ID (uses first session if omitted)
        session_filter: Option<String>,
    },

    /// Read scrollback from a pane (for MCP bridge)
    ReadPane {
        pane_id: Uuid,
        /// Number of lines to read (default 100, max 1000)
        lines: usize,
    },

    /// Get detailed pane status (for MCP bridge)
    GetPaneStatus { pane_id: Uuid },

    /// Create a new pane with options (for MCP bridge)
    CreatePaneWithOptions {
        /// Session filter (name or ID, uses first if omitted)
        session_filter: Option<String>,
        /// Window filter (name or ID, uses first if omitted)
        window_filter: Option<String>,
        /// Split direction
        direction: SplitDirection,
        /// Command to run (default: shell)
        command: Option<String>,
        /// Working directory
        cwd: Option<String>,
    },

    /// Create a new session with options (for MCP bridge)
    CreateSessionWithOptions {
        /// Session name (auto-generated if omitted)
        name: Option<String>,
    },

    /// Create a new window with options (for MCP bridge)
    CreateWindowWithOptions {
        /// Session filter (name or ID, uses first if omitted)
        session_filter: Option<String>,
        /// Window name
        name: Option<String>,
        /// Command to run in default pane (default: shell)
        command: Option<String>,
    },
}

/// Messages sent from server to client
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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

    /// Viewport state updated for a pane
    ViewportUpdated {
        pane_id: Uuid,
        state: crate::types::ViewportState,
    },

    /// Reply was delivered successfully
    ReplyDelivered { result: crate::types::ReplyResult },

    /// Received orchestration message from another session
    OrchestrationReceived {
        from_session_id: Uuid,
        message: OrchestrationMessage,
    },

    /// Orchestration message was delivered
    OrchestrationDelivered {
        /// Number of sessions that received the message
        delivered_count: usize,
    },

    // ==================== MCP Bridge Response Messages ====================

    /// List of all panes across sessions
    AllPanesList {
        panes: Vec<PaneListEntry>,
    },

    /// List of windows in a session
    WindowList {
        session_name: String,
        windows: Vec<WindowInfo>,
    },

    /// Scrollback content from a pane
    PaneContent {
        pane_id: Uuid,
        content: String,
    },

    /// Detailed pane status
    PaneStatus {
        pane_id: Uuid,
        session_name: String,
        window_name: String,
        window_index: usize,
        pane_index: usize,
        cols: u16,
        rows: u16,
        title: Option<String>,
        cwd: Option<String>,
        state: PaneState,
        has_pty: bool,
        is_awaiting_input: bool,
        is_awaiting_confirmation: bool,
    },

    /// Pane created with full details (for MCP bridge)
    PaneCreatedWithDetails {
        pane_id: Uuid,
        session_id: Uuid,
        session_name: String,
        window_id: Uuid,
        direction: String,
    },

    /// Session created with full details (for MCP bridge)
    SessionCreatedWithDetails {
        session_id: Uuid,
        session_name: String,
        window_id: Uuid,
        pane_id: Uuid,
    },

    /// Window created with full details (for MCP bridge)
    WindowCreatedWithDetails {
        window_id: Uuid,
        pane_id: Uuid,
        session_name: String,
    },
}

/// Entry in the pane list (for MCP bridge)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PaneListEntry {
    pub id: Uuid,
    pub session_name: String,
    pub window_index: usize,
    pub window_name: String,
    pub pane_index: usize,
    pub cols: u16,
    pub rows: u16,
    pub title: Option<String>,
    pub cwd: Option<String>,
    pub state: PaneState,
    pub is_claude: bool,
    pub claude_state: Option<ClaudeState>,
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
    /// Target pane is not awaiting input
    NotAwaitingInput,
    /// Session not associated with a repository
    NoRepository,
    /// No recipients for orchestration message
    NoRecipients,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_message_connect() {
        let client_id = Uuid::new_v4();
        let msg = ClientMessage::Connect {
            client_id,
            protocol_version: 1,
        };

        // Test clone
        let cloned = msg.clone();
        assert_eq!(msg, cloned);

        // Test debug
        let debug = format!("{:?}", msg);
        assert!(debug.contains("Connect"));
        assert!(debug.contains(&client_id.to_string()));
    }

    #[test]
    fn test_client_message_list_sessions() {
        let msg = ClientMessage::ListSessions;
        assert_eq!(msg.clone(), ClientMessage::ListSessions);
    }

    #[test]
    fn test_client_message_create_session() {
        let msg = ClientMessage::CreateSession {
            name: "test-session".to_string(),
        };
        if let ClientMessage::CreateSession { name } = &msg {
            assert_eq!(name, "test-session");
        } else {
            panic!("Wrong variant");
        }
    }

    #[test]
    fn test_client_message_attach_session() {
        let session_id = Uuid::new_v4();
        let msg = ClientMessage::AttachSession { session_id };
        if let ClientMessage::AttachSession { session_id: id } = msg {
            assert_eq!(id, session_id);
        }
    }

    #[test]
    fn test_client_message_create_window() {
        let session_id = Uuid::new_v4();

        // With name
        let msg = ClientMessage::CreateWindow {
            session_id,
            name: Some("main".to_string()),
        };
        if let ClientMessage::CreateWindow {
            session_id: sid,
            name,
        } = msg
        {
            assert_eq!(sid, session_id);
            assert_eq!(name, Some("main".to_string()));
        }

        // Without name
        let msg2 = ClientMessage::CreateWindow {
            session_id,
            name: None,
        };
        if let ClientMessage::CreateWindow { name, .. } = msg2 {
            assert!(name.is_none());
        }
    }

    #[test]
    fn test_client_message_create_pane() {
        let window_id = Uuid::new_v4();

        let msg_h = ClientMessage::CreatePane {
            window_id,
            direction: SplitDirection::Horizontal,
        };
        let msg_v = ClientMessage::CreatePane {
            window_id,
            direction: SplitDirection::Vertical,
        };

        assert_ne!(msg_h, msg_v);
    }

    #[test]
    fn test_client_message_input() {
        let pane_id = Uuid::new_v4();
        let data = vec![0x1b, 0x5b, 0x41]; // Up arrow escape sequence

        let msg = ClientMessage::Input {
            pane_id,
            data: data.clone(),
        };

        if let ClientMessage::Input {
            pane_id: pid,
            data: d,
        } = msg
        {
            assert_eq!(pid, pane_id);
            assert_eq!(d, data);
        }
    }

    #[test]
    fn test_client_message_resize() {
        let pane_id = Uuid::new_v4();
        let msg = ClientMessage::Resize {
            pane_id,
            cols: 120,
            rows: 40,
        };

        if let ClientMessage::Resize { cols, rows, .. } = msg {
            assert_eq!(cols, 120);
            assert_eq!(rows, 40);
        }
    }

    #[test]
    fn test_client_message_close_pane() {
        let pane_id = Uuid::new_v4();
        let msg = ClientMessage::ClosePane { pane_id };
        if let ClientMessage::ClosePane { pane_id: pid } = msg {
            assert_eq!(pid, pane_id);
        }
    }

    #[test]
    fn test_client_message_select_pane() {
        let pane_id = Uuid::new_v4();
        let msg = ClientMessage::SelectPane { pane_id };
        if let ClientMessage::SelectPane { pane_id: pid } = msg {
            assert_eq!(pid, pane_id);
        }
    }

    #[test]
    fn test_client_message_simple_variants() {
        assert_eq!(ClientMessage::Detach.clone(), ClientMessage::Detach);
        assert_eq!(ClientMessage::Sync.clone(), ClientMessage::Sync);
        assert_eq!(ClientMessage::Ping.clone(), ClientMessage::Ping);

        // All should be different
        assert_ne!(ClientMessage::Detach, ClientMessage::Sync);
        assert_ne!(ClientMessage::Sync, ClientMessage::Ping);
        assert_ne!(ClientMessage::Ping, ClientMessage::Detach);
    }

    #[test]
    fn test_server_message_connected() {
        let msg = ServerMessage::Connected {
            server_version: "1.0.0".to_string(),
            protocol_version: 1,
        };

        if let ServerMessage::Connected {
            server_version,
            protocol_version,
        } = msg.clone()
        {
            assert_eq!(server_version, "1.0.0");
            assert_eq!(protocol_version, 1);
        }

        assert_eq!(msg.clone(), msg);
    }

    #[test]
    fn test_server_message_session_list() {
        let sessions = vec![
            SessionInfo {
                id: Uuid::new_v4(),
                name: "session1".to_string(),
                created_at: 1000,
                window_count: 2,
                attached_clients: 1,
                worktree: None,
                is_orchestrator: false,
            },
            SessionInfo {
                id: Uuid::new_v4(),
                name: "session2".to_string(),
                created_at: 2000,
                window_count: 1,
                attached_clients: 0,
                worktree: None,
                is_orchestrator: false,
            },
        ];

        let msg = ServerMessage::SessionList {
            sessions: sessions.clone(),
        };

        if let ServerMessage::SessionList { sessions: s } = msg {
            assert_eq!(s.len(), 2);
            assert_eq!(s[0].name, "session1");
            assert_eq!(s[1].name, "session2");
        }
    }

    #[test]
    fn test_server_message_session_created() {
        let session = SessionInfo {
            id: Uuid::new_v4(),
            name: "new-session".to_string(),
            created_at: 12345,
            window_count: 0,
            attached_clients: 1,
            worktree: None,
            is_orchestrator: false,
        };

        let msg = ServerMessage::SessionCreated {
            session: session.clone(),
        };

        if let ServerMessage::SessionCreated { session: s } = msg {
            assert_eq!(s.name, "new-session");
            assert_eq!(s.window_count, 0);
        }
    }

    #[test]
    fn test_server_message_attached() {
        let session_id = Uuid::new_v4();
        let window_id = Uuid::new_v4();
        let pane_id = Uuid::new_v4();

        let msg = ServerMessage::Attached {
            session: SessionInfo {
                id: session_id,
                name: "test".to_string(),
                created_at: 0,
                window_count: 1,
                attached_clients: 1,
                worktree: None,
                is_orchestrator: false,
            },
            windows: vec![WindowInfo {
                id: window_id,
                session_id,
                name: "main".to_string(),
                index: 0,
                pane_count: 1,
                active_pane_id: Some(pane_id),
            }],
            panes: vec![PaneInfo {
                id: pane_id,
                window_id,
                index: 0,
                cols: 80,
                rows: 24,
                state: PaneState::Normal,
                title: None,
                cwd: None,
            }],
        };

        if let ServerMessage::Attached {
            session,
            windows,
            panes,
        } = msg
        {
            assert_eq!(session.id, session_id);
            assert_eq!(windows.len(), 1);
            assert_eq!(panes.len(), 1);
        }
    }

    #[test]
    fn test_server_message_window_created() {
        let window = WindowInfo {
            id: Uuid::new_v4(),
            session_id: Uuid::new_v4(),
            name: "new-window".to_string(),
            index: 1,
            pane_count: 0,
            active_pane_id: None,
        };

        let msg = ServerMessage::WindowCreated {
            window: window.clone(),
        };

        if let ServerMessage::WindowCreated { window: w } = msg {
            assert_eq!(w.name, "new-window");
            assert_eq!(w.index, 1);
        }
    }

    #[test]
    fn test_server_message_pane_created() {
        let pane = PaneInfo {
            id: Uuid::new_v4(),
            window_id: Uuid::new_v4(),
            index: 0,
            cols: 80,
            rows: 24,
            state: PaneState::Normal,
            title: Some("bash".to_string()),
            cwd: Some("/home/user".to_string()),
        };

        let msg = ServerMessage::PaneCreated { pane: pane.clone() };

        if let ServerMessage::PaneCreated { pane: p } = msg {
            assert_eq!(p.title, Some("bash".to_string()));
            assert_eq!(p.cwd, Some("/home/user".to_string()));
        }
    }

    #[test]
    fn test_server_message_output() {
        let pane_id = Uuid::new_v4();
        let data = b"Hello, World!\n".to_vec();

        let msg = ServerMessage::Output {
            pane_id,
            data: data.clone(),
        };

        if let ServerMessage::Output {
            pane_id: pid,
            data: d,
        } = msg
        {
            assert_eq!(pid, pane_id);
            assert_eq!(d, data);
        }
    }

    #[test]
    fn test_server_message_pane_state_changed() {
        let pane_id = Uuid::new_v4();

        // Normal state
        let msg1 = ServerMessage::PaneStateChanged {
            pane_id,
            state: PaneState::Normal,
        };

        // Claude state
        let msg2 = ServerMessage::PaneStateChanged {
            pane_id,
            state: PaneState::Claude(ClaudeState::default()),
        };

        // Exited state
        let msg3 = ServerMessage::PaneStateChanged {
            pane_id,
            state: PaneState::Exited { code: Some(0) },
        };

        assert_ne!(msg1, msg2);
        assert_ne!(msg2, msg3);
        assert_ne!(msg1, msg3);
    }

    #[test]
    fn test_server_message_claude_state_changed() {
        let pane_id = Uuid::new_v4();
        let state = ClaudeState {
            session_id: Some("abc123".to_string()),
            activity: ClaudeActivity::Thinking,
            model: Some("claude-3-opus".to_string()),
            tokens_used: Some(1500),
        };

        let msg = ServerMessage::ClaudeStateChanged {
            pane_id,
            state: state.clone(),
        };

        if let ServerMessage::ClaudeStateChanged {
            pane_id: pid,
            state: s,
        } = msg
        {
            assert_eq!(pid, pane_id);
            assert_eq!(s.activity, ClaudeActivity::Thinking);
            assert_eq!(s.tokens_used, Some(1500));
        }
    }

    #[test]
    fn test_server_message_pane_closed() {
        let pane_id = Uuid::new_v4();

        // With exit code
        let msg1 = ServerMessage::PaneClosed {
            pane_id,
            exit_code: Some(0),
        };

        // Without exit code (killed)
        let msg2 = ServerMessage::PaneClosed {
            pane_id,
            exit_code: None,
        };

        assert_ne!(msg1, msg2);
    }

    #[test]
    fn test_server_message_window_closed() {
        let window_id = Uuid::new_v4();
        let msg = ServerMessage::WindowClosed { window_id };

        if let ServerMessage::WindowClosed { window_id: wid } = msg {
            assert_eq!(wid, window_id);
        }
    }

    #[test]
    fn test_server_message_session_ended() {
        let session_id = Uuid::new_v4();
        let msg = ServerMessage::SessionEnded { session_id };

        if let ServerMessage::SessionEnded { session_id: sid } = msg {
            assert_eq!(sid, session_id);
        }
    }

    #[test]
    fn test_server_message_error() {
        let msg = ServerMessage::Error {
            code: ErrorCode::SessionNotFound,
            message: "Session 'test' not found".to_string(),
        };

        if let ServerMessage::Error { code, message } = msg {
            assert_eq!(code, ErrorCode::SessionNotFound);
            assert!(message.contains("test"));
        }
    }

    #[test]
    fn test_server_message_pong() {
        assert_eq!(ServerMessage::Pong.clone(), ServerMessage::Pong);
    }

    #[test]
    fn test_error_code_equality() {
        assert_eq!(ErrorCode::SessionNotFound, ErrorCode::SessionNotFound);
        assert_ne!(ErrorCode::SessionNotFound, ErrorCode::WindowNotFound);
        assert_ne!(ErrorCode::WindowNotFound, ErrorCode::PaneNotFound);
        assert_ne!(ErrorCode::PaneNotFound, ErrorCode::InvalidOperation);
        assert_ne!(ErrorCode::InvalidOperation, ErrorCode::ProtocolMismatch);
        assert_ne!(ErrorCode::ProtocolMismatch, ErrorCode::InternalError);
    }

    #[test]
    fn test_error_code_clone() {
        let code = ErrorCode::InternalError;
        let cloned = code.clone();
        assert_eq!(code, cloned);
    }

    #[test]
    fn test_error_code_debug() {
        let code = ErrorCode::ProtocolMismatch;
        let debug = format!("{:?}", code);
        assert_eq!(debug, "ProtocolMismatch");
    }

    #[test]
    fn test_all_error_codes_covered() {
        // Ensure we have a test that touches all variants
        let codes = [
            ErrorCode::SessionNotFound,
            ErrorCode::WindowNotFound,
            ErrorCode::PaneNotFound,
            ErrorCode::InvalidOperation,
            ErrorCode::ProtocolMismatch,
            ErrorCode::InternalError,
            ErrorCode::NotAwaitingInput,
            ErrorCode::NoRepository,
            ErrorCode::NoRecipients,
        ];

        assert_eq!(codes.len(), 9);
        for (i, code) in codes.iter().enumerate() {
            // Each code should be unique
            for (j, other) in codes.iter().enumerate() {
                if i == j {
                    assert_eq!(code, other);
                } else {
                    assert_ne!(code, other);
                }
            }
        }
    }

    // ==================== Viewport Message Tests ====================

    #[test]
    fn test_client_message_set_viewport_offset() {
        let pane_id = Uuid::new_v4();
        let msg = ClientMessage::SetViewportOffset {
            pane_id,
            offset: 100,
        };

        if let ClientMessage::SetViewportOffset {
            pane_id: pid,
            offset,
        } = msg
        {
            assert_eq!(pid, pane_id);
            assert_eq!(offset, 100);
        } else {
            panic!("Wrong variant");
        }
    }

    #[test]
    fn test_client_message_set_viewport_offset_zero() {
        let pane_id = Uuid::new_v4();
        let msg = ClientMessage::SetViewportOffset {
            pane_id,
            offset: 0,
        };

        if let ClientMessage::SetViewportOffset { offset, .. } = msg {
            assert_eq!(offset, 0);
        }
    }

    #[test]
    fn test_client_message_jump_to_bottom() {
        let pane_id = Uuid::new_v4();
        let msg = ClientMessage::JumpToBottom { pane_id };

        if let ClientMessage::JumpToBottom { pane_id: pid } = msg {
            assert_eq!(pid, pane_id);
        } else {
            panic!("Wrong variant");
        }
    }

    #[test]
    fn test_server_message_viewport_updated() {
        use crate::types::ViewportState;

        let pane_id = Uuid::new_v4();
        let state = ViewportState::pinned(50);

        let msg = ServerMessage::ViewportUpdated {
            pane_id,
            state: state.clone(),
        };

        if let ServerMessage::ViewportUpdated {
            pane_id: pid,
            state: s,
        } = msg
        {
            assert_eq!(pid, pane_id);
            assert_eq!(s, state);
        } else {
            panic!("Wrong variant");
        }
    }

    #[test]
    fn test_server_message_viewport_updated_at_bottom() {
        use crate::types::ViewportState;

        let pane_id = Uuid::new_v4();
        let state = ViewportState::new();

        let msg = ServerMessage::ViewportUpdated { pane_id, state };

        if let ServerMessage::ViewportUpdated { state: s, .. } = msg {
            assert!(s.is_at_bottom());
            assert!(!s.is_pinned);
            assert_eq!(s.new_lines_since_pin, 0);
        }
    }

    #[test]
    fn test_server_message_viewport_updated_with_new_lines() {
        use crate::types::ViewportState;

        let pane_id = Uuid::new_v4();
        let state = ViewportState {
            offset_from_bottom: 100,
            is_pinned: true,
            new_lines_since_pin: 47,
        };

        let msg = ServerMessage::ViewportUpdated { pane_id, state };

        if let ServerMessage::ViewportUpdated { state: s, .. } = msg {
            assert_eq!(s.new_lines_since_pin, 47);
        }
    }

    #[test]
    fn test_viewport_messages_equality() {
        use crate::types::ViewportState;

        let pane_id = Uuid::new_v4();

        let msg1 = ClientMessage::SetViewportOffset {
            pane_id,
            offset: 50,
        };
        let msg2 = ClientMessage::SetViewportOffset {
            pane_id,
            offset: 50,
        };
        let msg3 = ClientMessage::SetViewportOffset {
            pane_id,
            offset: 100,
        };

        assert_eq!(msg1, msg2);
        assert_ne!(msg1, msg3);

        let state = ViewportState::pinned(10);
        let srv1 = ServerMessage::ViewportUpdated { pane_id, state };
        let srv2 = ServerMessage::ViewportUpdated { pane_id, state };

        assert_eq!(srv1, srv2);
    }

    // ==================== Reply Message Tests ====================

    #[test]
    fn test_client_message_reply_by_id() {
        use crate::types::{PaneTarget, ReplyMessage};

        let pane_id = Uuid::new_v4();
        let reply = ReplyMessage::by_id(pane_id, "yes, proceed");
        let msg = ClientMessage::Reply { reply: reply.clone() };

        if let ClientMessage::Reply { reply: r } = msg {
            assert_eq!(r.target, PaneTarget::Id(pane_id));
            assert_eq!(r.content, "yes, proceed");
        } else {
            panic!("Expected Reply variant");
        }
    }

    #[test]
    fn test_client_message_reply_by_name() {
        use crate::types::{PaneTarget, ReplyMessage};

        let reply = ReplyMessage::by_name("worker-3", "use async");
        let msg = ClientMessage::Reply { reply: reply.clone() };

        if let ClientMessage::Reply { reply: r } = msg {
            assert_eq!(r.target, PaneTarget::Name("worker-3".to_string()));
            assert_eq!(r.content, "use async");
        } else {
            panic!("Expected Reply variant");
        }
    }

    #[test]
    fn test_client_message_reply_clone() {
        use crate::types::ReplyMessage;

        let reply = ReplyMessage::by_name("test", "content");
        let msg = ClientMessage::Reply { reply };
        let cloned = msg.clone();
        assert_eq!(msg, cloned);
    }

    // ==================== ReplyDelivered Message Tests ====================

    #[test]
    fn test_server_message_reply_delivered() {
        use crate::types::ReplyResult;

        let pane_id = Uuid::new_v4();
        let result = ReplyResult {
            pane_id,
            bytes_written: 42,
        };
        let msg = ServerMessage::ReplyDelivered { result: result.clone() };

        if let ServerMessage::ReplyDelivered { result: r } = msg {
            assert_eq!(r.pane_id, pane_id);
            assert_eq!(r.bytes_written, 42);
        } else {
            panic!("Expected ReplyDelivered variant");
        }
    }

    #[test]
    fn test_server_message_reply_delivered_clone() {
        use crate::types::ReplyResult;

        let result = ReplyResult {
            pane_id: Uuid::new_v4(),
            bytes_written: 100,
        };
        let msg = ServerMessage::ReplyDelivered { result };
        let cloned = msg.clone();
        assert_eq!(msg, cloned);
    }

    #[test]
    fn test_error_code_not_awaiting_input() {
        let code = ErrorCode::NotAwaitingInput;
        assert_eq!(code, ErrorCode::NotAwaitingInput);
        assert_ne!(code, ErrorCode::PaneNotFound);

        let debug = format!("{:?}", code);
        assert_eq!(debug, "NotAwaitingInput");
    }

    // ==================== Orchestration Message Tests ====================

    #[test]
    fn test_orchestration_message_status_update() {
        let session_id = Uuid::new_v4();
        let msg = OrchestrationMessage::StatusUpdate {
            session_id,
            status: WorkerStatus::Working,
            message: Some("Processing files".to_string()),
        };

        if let OrchestrationMessage::StatusUpdate {
            session_id: sid,
            status,
            message,
        } = msg
        {
            assert_eq!(sid, session_id);
            assert_eq!(status, WorkerStatus::Working);
            assert_eq!(message, Some("Processing files".to_string()));
        } else {
            panic!("Wrong variant");
        }
    }

    #[test]
    fn test_orchestration_message_task_assignment() {
        let task_id = Uuid::new_v4();
        let msg = OrchestrationMessage::TaskAssignment {
            task_id,
            description: "Fix the login bug".to_string(),
            files: vec!["src/auth.rs".to_string(), "src/login.rs".to_string()],
        };

        if let OrchestrationMessage::TaskAssignment {
            task_id: tid,
            description,
            files,
        } = msg
        {
            assert_eq!(tid, task_id);
            assert_eq!(description, "Fix the login bug");
            assert_eq!(files.len(), 2);
        } else {
            panic!("Wrong variant");
        }
    }

    #[test]
    fn test_orchestration_message_task_complete() {
        let task_id = Uuid::new_v4();
        let msg = OrchestrationMessage::TaskComplete {
            task_id,
            success: true,
            summary: "Bug fixed and tests pass".to_string(),
        };

        if let OrchestrationMessage::TaskComplete {
            task_id: tid,
            success,
            summary,
        } = msg
        {
            assert_eq!(tid, task_id);
            assert!(success);
            assert_eq!(summary, "Bug fixed and tests pass");
        } else {
            panic!("Wrong variant");
        }
    }

    #[test]
    fn test_orchestration_message_help_request() {
        let session_id = Uuid::new_v4();
        let msg = OrchestrationMessage::HelpRequest {
            session_id,
            context: "Stuck on type inference".to_string(),
        };

        if let OrchestrationMessage::HelpRequest {
            session_id: sid,
            context,
        } = msg
        {
            assert_eq!(sid, session_id);
            assert_eq!(context, "Stuck on type inference");
        } else {
            panic!("Wrong variant");
        }
    }

    #[test]
    fn test_orchestration_message_broadcast() {
        let from_session_id = Uuid::new_v4();
        let msg = OrchestrationMessage::Broadcast {
            from_session_id,
            message: "All workers pause".to_string(),
        };

        if let OrchestrationMessage::Broadcast {
            from_session_id: sid,
            message,
        } = msg
        {
            assert_eq!(sid, from_session_id);
            assert_eq!(message, "All workers pause");
        } else {
            panic!("Wrong variant");
        }
    }

    #[test]
    fn test_orchestration_message_sync_request() {
        let msg = OrchestrationMessage::SyncRequest;
        assert_eq!(msg.clone(), OrchestrationMessage::SyncRequest);
    }

    #[test]
    fn test_worker_status_all_variants() {
        let statuses = [
            WorkerStatus::Idle,
            WorkerStatus::Working,
            WorkerStatus::WaitingForInput,
            WorkerStatus::Blocked,
            WorkerStatus::Complete,
            WorkerStatus::Error,
        ];

        assert_eq!(statuses.len(), 6);
        for (i, status) in statuses.iter().enumerate() {
            for (j, other) in statuses.iter().enumerate() {
                if i == j {
                    assert_eq!(status, other);
                } else {
                    assert_ne!(status, other);
                }
            }
        }
    }

    #[test]
    fn test_orchestration_target_orchestrator() {
        let target = OrchestrationTarget::Orchestrator;
        assert_eq!(target.clone(), OrchestrationTarget::Orchestrator);
    }

    #[test]
    fn test_orchestration_target_session() {
        let session_id = Uuid::new_v4();
        let target = OrchestrationTarget::Session(session_id);

        if let OrchestrationTarget::Session(id) = target {
            assert_eq!(id, session_id);
        } else {
            panic!("Wrong variant");
        }
    }

    #[test]
    fn test_orchestration_target_broadcast() {
        let target = OrchestrationTarget::Broadcast;
        assert_eq!(target.clone(), OrchestrationTarget::Broadcast);
    }

    #[test]
    fn test_orchestration_target_worktree() {
        let target = OrchestrationTarget::Worktree("/repo/feature-branch".to_string());

        if let OrchestrationTarget::Worktree(path) = target {
            assert_eq!(path, "/repo/feature-branch");
        } else {
            panic!("Wrong variant");
        }
    }

    #[test]
    fn test_client_message_send_orchestration() {
        let target = OrchestrationTarget::Orchestrator;
        let message = OrchestrationMessage::StatusUpdate {
            session_id: Uuid::new_v4(),
            status: WorkerStatus::Idle,
            message: None,
        };

        let msg = ClientMessage::SendOrchestration {
            target: target.clone(),
            message: message.clone(),
        };

        if let ClientMessage::SendOrchestration {
            target: t,
            message: m,
        } = msg
        {
            assert_eq!(t, target);
            assert_eq!(m, message);
        } else {
            panic!("Wrong variant");
        }
    }

    #[test]
    fn test_server_message_orchestration_received() {
        let from_session_id = Uuid::new_v4();
        let message = OrchestrationMessage::SyncRequest;

        let msg = ServerMessage::OrchestrationReceived {
            from_session_id,
            message: message.clone(),
        };

        if let ServerMessage::OrchestrationReceived {
            from_session_id: sid,
            message: m,
        } = msg
        {
            assert_eq!(sid, from_session_id);
            assert_eq!(m, message);
        } else {
            panic!("Wrong variant");
        }
    }

    #[test]
    fn test_server_message_orchestration_delivered() {
        let msg = ServerMessage::OrchestrationDelivered { delivered_count: 3 };

        if let ServerMessage::OrchestrationDelivered { delivered_count } = msg {
            assert_eq!(delivered_count, 3);
        } else {
            panic!("Wrong variant");
        }
    }

    #[test]
    fn test_error_code_no_repository() {
        let code = ErrorCode::NoRepository;
        assert_eq!(code, ErrorCode::NoRepository);
        assert_ne!(code, ErrorCode::NoRecipients);

        let debug = format!("{:?}", code);
        assert_eq!(debug, "NoRepository");
    }

    #[test]
    fn test_error_code_no_recipients() {
        let code = ErrorCode::NoRecipients;
        assert_eq!(code, ErrorCode::NoRecipients);
        assert_ne!(code, ErrorCode::NoRepository);

        let debug = format!("{:?}", code);
        assert_eq!(debug, "NoRecipients");
    }
}
