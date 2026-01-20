//! Client-server message types

use serde::{Deserialize, Serialize};

use uuid::Uuid;

use crate::types::*;

// ==================== Orchestration Types ====================

/// Generic orchestration message with user-defined semantics
///
/// This is a flexible message format that allows workflows to define their own
/// message types and payloads. The `msg_type` field is a user-defined string
/// that identifies the message type (e.g., "status_update", "task_assignment",
/// "gas_town.auction"). The `payload` field contains the message data as JSON.
///
/// # Example
/// ```rust
/// use ccmux_protocol::OrchestrationMessage;
/// use serde_json::json;
///
/// let msg = OrchestrationMessage::new(
///     "workflow.task_assigned",
///     json!({
///         "task_id": "abc123",
///         "description": "Fix the bug",
///         "files": ["src/main.rs"]
///     }),
/// );
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OrchestrationMessage {
    /// User-defined message type identifier (e.g., "status_update", "task.assigned")
    pub msg_type: String,
    /// Message payload as JSON - structure is defined by the workflow
    /// Uses JsonValue wrapper for bincode compatibility (BUG-030)
    pub payload: crate::types::JsonValue,
}

impl OrchestrationMessage {
    /// Create a new orchestration message
    pub fn new(msg_type: impl Into<String>, payload: serde_json::Value) -> Self {
        Self {
            msg_type: msg_type.into(),
            payload: crate::types::JsonValue::new(payload),
        }
    }

    /// Get the payload as a serde_json::Value reference
    pub fn payload(&self) -> &serde_json::Value {
        self.payload.inner()
    }
}

/// Target for orchestration messages
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum OrchestrationTarget {
    /// Send to sessions with a specific tag
    Tagged(String),
    /// Send to specific session by ID
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
        /// Type of client identifying its role (FEAT-079)
        client_type: ClientType,
    },

    /// Request list of sessions
    ListSessions,

    /// Create a new session
    CreateSession {
        name: String,
        /// Optional command to run instead of default_command from config
        command: Option<String>,
    },

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

    /// Send paste to pane (may be wrapped in bracketed paste markers)
    Paste { pane_id: Uuid, data: Vec<u8> },

    /// Resize pane
    Resize { pane_id: Uuid, cols: u16, rows: u16 },

    /// Close pane
    ClosePane { pane_id: Uuid },

    /// Select/focus pane
    SelectPane { pane_id: Uuid },

    /// Select/focus window (make it the active window in its session)
    SelectWindow { window_id: Uuid },

    /// Select/focus session (make it the active session)
    SelectSession { session_id: Uuid },

    /// Detach from session (keep session running)
    Detach,

    /// Request full state sync
    Sync,

    /// Request server-wide status (FEAT-074)
    GetServerStatus,

    /// Request screen redraw (triggers SIGWINCH to PTYs)
    Redraw {
        /// Optional specific pane to redraw. If None, redraw all panes in session.
        pane_id: Option<Uuid>,
    },

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

    /// Destroy/kill a session
    DestroySession { session_id: Uuid },

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
        /// If true, focus the new pane after creation (default: false)
        select: bool,
        /// Optional name for the pane (FEAT-036)
        name: Option<String>,
        /// Claude model override (FEAT-071)
        claude_model: Option<String>,
        /// Claude configuration override (FEAT-071)
        claude_config: Option<crate::types::JsonValue>,
        /// Configuration preset (FEAT-071)
        preset: Option<String>,
    },

    /// Create a new session with options (for MCP bridge)
    CreateSessionWithOptions {
        /// Session name (auto-generated if omitted)
        name: Option<String>,
        /// Command to run in the default pane (default: $SHELL or /bin/sh)
        command: Option<String>,
        /// Working directory for the session
        cwd: Option<String>,
        /// Claude model override (FEAT-071)
        claude_model: Option<String>,
        /// Claude configuration override (FEAT-071)
        claude_config: Option<crate::types::JsonValue>,
        /// Configuration preset (FEAT-071)
        preset: Option<String>,
    },

    /// Create a new window with options (for MCP bridge)
    CreateWindowWithOptions {
        /// Session filter (name or ID, uses first if omitted)
        session_filter: Option<String>,
        /// Window name
        name: Option<String>,
        /// Command to run in default pane (default: shell)
        command: Option<String>,
        /// Working directory for the window (BUG-050)
        cwd: Option<String>,
    },

    /// Rename a session (for MCP bridge)
    RenameSession {
        /// Session to rename (UUID or current name)
        session_filter: String,
        /// New name for the session
        new_name: String,
    },

    /// Rename a pane (FEAT-036)
    RenamPane {
        /// Pane to rename (UUID)
        pane_id: Uuid,
        /// New name for the pane
        new_name: String,
    },

    /// Rename a window (FEAT-036)
    RenameWindow {
        /// Window to rename (UUID)
        window_id: Uuid,
        /// New name for the window
        new_name: String,
    },

    /// Split an existing pane (for MCP bridge)
    SplitPane {
        /// The pane to split
        pane_id: Uuid,
        /// Split direction
        direction: SplitDirection,
        /// Size ratio for the original pane (0.1 to 0.9, default 0.5)
        ratio: f32,
        /// Command to run in the new pane (default: shell)
        command: Option<String>,
        /// Working directory for the new pane
        cwd: Option<String>,
        /// If true, focus the new pane after creation (default: false)
        select: bool,
    },

    /// Resize a pane by delta fraction (for MCP bridge)
    ResizePaneDelta {
        /// The pane to resize
        pane_id: Uuid,
        /// Size change as a fraction (-0.5 to 0.5)
        delta: f32,
    },

    /// Create a complex layout declaratively (for MCP bridge)
    CreateLayout {
        /// Session filter (name or ID, uses first if omitted)
        session_filter: Option<String>,
        /// Window filter (name or ID, uses first if omitted)
        window_filter: Option<String>,
        /// Layout specification as JSON
        /// Uses JsonValue wrapper for bincode compatibility (BUG-030)
        layout: crate::types::JsonValue,
    },

    /// Set an environment variable on a session (for MCP bridge)
    SetEnvironment {
        /// Session filter (name or ID)
        session_filter: String,
        /// Environment variable key
        key: String,
        /// Environment variable value
        value: String,
    },

    /// Get session environment variables (for MCP bridge)
    GetEnvironment {
        /// Session filter (name or ID)
        session_filter: String,
        /// Specific key to get (None = get all)
        key: Option<String>,
    },

    /// Set metadata on a session (for MCP bridge)
    SetMetadata {
        /// Session filter (name or ID)
        session_filter: String,
        /// Metadata key
        key: String,
        /// Metadata value
        value: String,
    },

    /// Get session metadata (for MCP bridge)
    GetMetadata {
        /// Session filter (name or ID)
        session_filter: String,
        /// Specific key to get (None = get all)
        key: Option<String>,
    },

    // ==================== Orchestration MCP Tools (FEAT-048) ====================

    /// Set tags on a session (add/remove) for MCP bridge
    SetTags {
        /// Session filter (name or ID, uses first if omitted)
        session_filter: Option<String>,
        /// Tags to add
        add: Vec<String>,
        /// Tags to remove
        remove: Vec<String>,
    },

    /// Get tags from a session for MCP bridge
    GetTags {
        /// Session filter (name or ID, uses first if omitted)
        session_filter: Option<String>,
    },

    // ==================== User Priority Lock Messages (FEAT-056) ====================

    /// User entered command mode (prefix key pressed)
    ///
    /// When the user presses the prefix key (e.g., Ctrl+B), the client sends this
    /// message to prevent MCP agents from interfering with focus-changing operations.
    UserCommandModeEntered {
        /// How long the lock should be held (ms) before auto-expiring
        timeout_ms: u32,
    },

    /// User exited command mode (command completed/cancelled/timed out)
    ///
    /// Sent when the user completes a command, presses Escape, or the prefix timeout expires.
    UserCommandModeExited,

    // ==================== Beads Query Integration (FEAT-058) ====================

    /// Request beads status for a pane's repository
    RequestBeadsStatus { pane_id: Uuid },

    /// Request full ready task list for the beads panel
    RequestBeadsReadyList { pane_id: Uuid },

    // ==================== Generic Widget System (FEAT-083) ====================

    /// Request widget update for a pane (generic alternative to RequestBeadsStatus)
    ///
    /// The server will delegate to the appropriate handler based on widget_type:
    /// - "beads.*" types delegate to existing beads handlers
    /// - Unknown types return an error
    ///
    /// This provides forward compatibility - new widget types can be added
    /// without protocol changes.
    RequestWidgetUpdate {
        /// Target pane ID
        pane_id: Uuid,
        /// Widget type to request (e.g., "beads.status", "beads.ready_list")
        widget_type: String,
    },

    // ==================== Resync / Event Log (FEAT-075) ====================

    /// Request events since a specific commit sequence
    GetEventsSince {
        /// Last seen commit sequence (0 = none)
        last_commit_seq: u64,
    },

    // ==================== Mirror Pane (FEAT-062) ====================

    /// Create a mirror pane that displays another pane's output (FEAT-062)
    ///
    /// Mirror panes are read-only views of a source pane's terminal output.
    /// They have independent scrollback and can be used for "plate spinning"
    /// visibility in multi-agent workflows.
    CreateMirror {
        /// The pane to mirror
        source_pane_id: Uuid,
        /// Optional target pane to convert to a mirror (creates new split if None)
        target_pane_id: Option<Uuid>,
        /// Split direction if creating a new pane (default: Vertical)
        direction: Option<SplitDirection>,
    },

    // ==================== FEAT-097: Orchestration Message Receive ====================

    /// Get status of a worker (session)
    GetWorkerStatus {
        /// Optional worker ID (session UUID or name)
        /// If None, returns status of all workers
        worker_id: Option<String>,
    },

    /// Poll for messages in a worker's inbox
    PollMessages {
        /// Worker ID (session UUID or name)
        worker_id: String,
    },

    // ==================== FEAT-102: Agent Status Pane ====================

    /// Create a dedicated agent status pane
    CreateStatusPane {
        /// Position relative to current pane
        position: Option<String>,
        /// Width as percentage (10-90)
        width_percent: Option<i64>,
        /// Whether to show activity feed
        show_activity_feed: bool,
        /// Whether to show output preview
        show_output_preview: bool,
        /// Filter tags
        filter_tags: Option<Vec<String>>,
    },

    // ==================== FEAT-104: Watchdog Timer ====================

    /// Start the watchdog timer that sends periodic messages to a pane
    WatchdogStart {
        /// Target pane to send messages to
        pane_id: Uuid,
        /// Interval between messages in seconds
        interval_secs: u64,
        /// Message to send (default: "check")
        message: Option<String>,
    },

    /// Stop the watchdog timer
    WatchdogStop,

    /// Get current watchdog status
    WatchdogStatus,
}

impl ClientMessage {
    /// Return the message type name for metrics and logging (FEAT-074)
    pub fn type_name(&self) -> &'static str {
        match self {
            ClientMessage::Connect { .. } => "Connect",
            ClientMessage::ListSessions => "ListSessions",
            ClientMessage::CreateSession { .. } => "CreateSession",
            ClientMessage::AttachSession { .. } => "AttachSession",
            ClientMessage::CreateWindow { .. } => "CreateWindow",
            ClientMessage::CreatePane { .. } => "CreatePane",
            ClientMessage::Input { .. } => "Input",
            ClientMessage::Paste { .. } => "Paste",
            ClientMessage::Resize { .. } => "Resize",
            ClientMessage::ClosePane { .. } => "ClosePane",
            ClientMessage::SelectPane { .. } => "SelectPane",
            ClientMessage::SelectWindow { .. } => "SelectWindow",
            ClientMessage::SelectSession { .. } => "SelectSession",
            ClientMessage::Detach => "Detach",
            ClientMessage::Sync => "Sync",
            ClientMessage::GetServerStatus => "GetServerStatus",
            ClientMessage::Redraw { .. } => "Redraw",
            ClientMessage::Ping => "Ping",
            ClientMessage::SetViewportOffset { .. } => "SetViewportOffset",
            ClientMessage::JumpToBottom { .. } => "JumpToBottom",
            ClientMessage::Reply { .. } => "Reply",
            ClientMessage::SendOrchestration { .. } => "SendOrchestration",
            ClientMessage::DestroySession { .. } => "DestroySession",
            ClientMessage::ListAllPanes { .. } => "ListAllPanes",
            ClientMessage::ListWindows { .. } => "ListWindows",
            ClientMessage::ReadPane { .. } => "ReadPane",
            ClientMessage::GetPaneStatus { .. } => "GetPaneStatus",
            ClientMessage::CreatePaneWithOptions { .. } => "CreatePaneWithOptions",
            ClientMessage::CreateSessionWithOptions { .. } => "CreateSessionWithOptions",
            ClientMessage::CreateWindowWithOptions { .. } => "CreateWindowWithOptions",
            ClientMessage::RenameSession { .. } => "RenameSession",
            ClientMessage::RenamPane { .. } => "RenamePane",
            ClientMessage::RenameWindow { .. } => "RenameWindow",
            ClientMessage::SplitPane { .. } => "SplitPane",
            ClientMessage::ResizePaneDelta { .. } => "ResizePaneDelta",
            ClientMessage::CreateLayout { .. } => "CreateLayout",
            ClientMessage::SetEnvironment { .. } => "SetEnvironment",
            ClientMessage::GetEnvironment { .. } => "GetEnvironment",
            ClientMessage::SetMetadata { .. } => "SetMetadata",
            ClientMessage::GetMetadata { .. } => "GetMetadata",
            ClientMessage::SetTags { .. } => "SetTags",
            ClientMessage::GetTags { .. } => "GetTags",
            ClientMessage::UserCommandModeEntered { .. } => "UserCommandModeEntered",
            ClientMessage::UserCommandModeExited => "UserCommandModeExited",
            ClientMessage::RequestBeadsStatus { .. } => "RequestBeadsStatus",
            ClientMessage::RequestBeadsReadyList { .. } => "RequestBeadsReadyList",
            ClientMessage::GetEventsSince { .. } => "GetEventsSince",
            ClientMessage::RequestWidgetUpdate { .. } => "RequestWidgetUpdate",
            ClientMessage::CreateMirror { .. } => "CreateMirror",
            ClientMessage::GetWorkerStatus { .. } => "GetWorkerStatus",
            ClientMessage::PollMessages { .. } => "PollMessages",
            ClientMessage::CreateStatusPane { .. } => "CreateStatusPane",
            ClientMessage::WatchdogStart { .. } => "WatchdogStart",
            ClientMessage::WatchdogStop => "WatchdogStop",
            ClientMessage::WatchdogStatus => "WatchdogStatus",
        }
    }
}

/// Messages sent from server to client
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ServerMessage {
    /// Connection accepted
    Connected {
        server_version: String,
        protocol_version: u32,
    },

    /// Server-wide status (FEAT-074)
    ServerStatus {
        /// Current commit sequence number
        commit_seq: u64,
        /// Number of connected clients
        client_count: usize,
        /// Number of active sessions
        session_count: usize,
        /// Replay buffer range (min_seq, max_seq)
        replay_range: (u64, u64),
        /// WAL health status
        wal_healthy: bool,
        /// Checkpoint health status
        checkpoint_healthy: bool,
        /// human control mode active?
        human_control_active: bool,
    },

    /// A sequenced message for event log/replay (FEAT-075)
    Sequenced {
        seq: u64,
        inner: Box<ServerMessage>,
    },

    /// Snapshot of the current state for resync (FEAT-075)
    StateSnapshot {
        commit_seq: u64,
        session: SessionInfo,
        windows: Vec<WindowInfo>,
        panes: Vec<PaneInfo>,
    },

    /// List of available sessions
    SessionList { sessions: Vec<SessionInfo> },

    /// Session created
    SessionCreated {
        session: SessionInfo,
        /// Whether the receiving client should focus this session
        #[serde(default)]
        should_focus: bool,
    },

    /// Attached to session - full state
    Attached {
        session: SessionInfo,
        windows: Vec<WindowInfo>,
        panes: Vec<PaneInfo>,
        /// Current commit sequence number for resync tracking (FEAT-075)
        #[serde(default)]
        commit_seq: u64,
    },

    /// Window created
    WindowCreated {
        window: WindowInfo,
        /// Whether the receiving client should focus this window
        #[serde(default)]
        should_focus: bool,
    },

    /// Pane created
    PaneCreated {
        pane: PaneInfo,
        /// Split direction for layout (how to arrange this pane relative to others)
        direction: SplitDirection,
        /// Whether the receiving client should focus this pane
        #[serde(default)]
        should_focus: bool,
    },

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

    /// Session list changed (broadcast notification)
    ///
    /// BUG-038 FIX: This is a broadcast-only message sent when sessions are created/destroyed.
    /// Unlike SessionList which is a direct response to ListSessions, this message is
    /// filtered out by the MCP bridge to prevent it from being picked up as a response
    /// to unrelated pending requests.
    SessionsChanged { sessions: Vec<SessionInfo> },

    /// Error response
    Error {
        code: ErrorCode,
        message: String,
        #[serde(default)]
        details: Option<ErrorDetails>,
    },

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

    /// Mail received from a worker pane (FEAT-073)
    MailReceived {
        pane_id: Uuid,
        priority: MailPriority,
        summary: String,
    },

    /// Orchestration message was delivered
    OrchestrationDelivered {
        /// Number of sessions that received the message
        delivered_count: usize,
    },

    // ==================== FEAT-097: Orchestration Message Receive ====================

    /// Status of a worker (or all workers)
    WorkerStatus {
        /// Status data (JSON)
        status: crate::types::JsonValue,
    },

    /// Messages polled from inbox
    MessagesPolled {
        /// List of (sender_id, message) tuples
        messages: Vec<(Uuid, OrchestrationMessage)>,
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
        /// Whether the receiving client should focus this pane
        #[serde(default)]
        should_focus: bool,
    },

    /// Session created with full details (for MCP bridge)
    SessionCreatedWithDetails {
        session_id: Uuid,
        session_name: String,
        window_id: Uuid,
        pane_id: Uuid,
        /// Whether the receiving client should focus this session
        #[serde(default)]
        should_focus: bool,
    },

    /// Window created with full details (for MCP bridge)
    WindowCreatedWithDetails {
        window_id: Uuid,
        pane_id: Uuid,
        session_name: String,
        /// Whether the receiving client should focus this window
        #[serde(default)]
        should_focus: bool,
    },

    /// Session was renamed (for MCP bridge)
    SessionRenamed {
        session_id: Uuid,
        previous_name: String,
        new_name: String,
    },

    /// Pane was renamed (FEAT-036)
    PaneRenamed {
        pane_id: Uuid,
        previous_name: Option<String>,
        new_name: String,
    },

    /// Window was renamed (FEAT-036)
    WindowRenamed {
        window_id: Uuid,
        previous_name: String,
        new_name: String,
    },

    PaneSplit {
        new_pane_id: Uuid,
        original_pane_id: Uuid,
        session_id: Uuid,
        session_name: String,
        window_id: Uuid,
        direction: String,
        /// Whether the receiving client should focus the new pane
        #[serde(default)]
        should_focus: bool,
    },

    /// Pane was resized successfully (for MCP bridge)
    PaneResized {
        pane_id: Uuid,
        /// New size after resize
        new_cols: u16,
        new_rows: u16,
    },

    /// Layout was created successfully (for MCP bridge)
    LayoutCreated {
        /// Session the layout was created in
        session_id: Uuid,
        session_name: String,
        /// Window the layout was created in
        window_id: Uuid,
        /// All panes created as part of the layout
        pane_ids: Vec<Uuid>,
    },

    /// Session was destroyed/killed (for MCP bridge)
    SessionDestroyed {
        session_id: Uuid,
        session_name: String,
    },

    /// Environment variable was set (for MCP bridge)
    EnvironmentSet {
        session_id: Uuid,
        session_name: String,
        key: String,
        value: String,
    },

    /// Session environment variables (for MCP bridge)
    EnvironmentList {
        session_id: Uuid,
        session_name: String,
        /// All environment variables (or single requested variable)
        environment: std::collections::HashMap<String, String>,
    },

    /// Metadata was set on a session (for MCP bridge)
    MetadataSet {
        session_id: Uuid,
        session_name: String,
        key: String,
        value: String,
    },

    /// Session metadata (for MCP bridge)
    MetadataList {
        session_id: Uuid,
        session_name: String,
        /// All metadata (or single requested key)
        metadata: std::collections::HashMap<String, String>,
    },

    // ==================== Orchestration MCP Tools (FEAT-048) ====================

    /// Tags were set on a session (for MCP bridge)
    TagsSet {
        session_id: Uuid,
        session_name: String,
        /// Current tags after the operation
        tags: std::collections::HashSet<String>,
    },

    /// Session tags (for MCP bridge)
    TagsList {
        session_id: Uuid,
        session_name: String,
        /// All tags for the session
        tags: std::collections::HashSet<String>,
    },

    // ==================== Focus Change Broadcasts (BUG-026) ====================

    /// Pane focus changed - broadcast to TUI clients
    PaneFocused {
        session_id: Uuid,
        window_id: Uuid,
        pane_id: Uuid,
    },

    /// Window selection changed - broadcast to TUI clients
    WindowFocused {
        session_id: Uuid,
        window_id: Uuid,
    },

    /// Active session changed - broadcast to TUI clients
    SessionFocused {
        session_id: Uuid,
    },

    // ==================== Beads Query Integration (FEAT-058) ====================

    /// Beads status update for a pane (daemon availability, ready count)
    BeadsStatusUpdate {
        pane_id: Uuid,
        status: crate::types::BeadsStatus,
    },

    /// Full list of ready tasks for the beads panel
    BeadsReadyList {
        pane_id: Uuid,
        tasks: Vec<crate::types::BeadsTask>,
    },

    // ==================== Generic Widget System (FEAT-083) ====================

    /// Generic widget update response (alternative to BeadsStatusUpdate)
    ///
    /// This provides a generic mechanism for returning widget data without
    /// requiring protocol changes for new widget types.
    WidgetUpdate {
        /// Target pane ID
        pane_id: Uuid,
        /// The widget update containing metadata and widget items
        update: crate::types::WidgetUpdate,
    },

    // ==================== Mirror Pane (FEAT-062) ====================

    /// Mirror pane created successfully (FEAT-062)
    MirrorCreated {
        /// The newly created mirror pane
        mirror_pane: PaneInfo,
        /// The source pane being mirrored
        source_pane_id: Uuid,
        /// Session information
        session_id: Uuid,
        session_name: String,
        /// Window information
        window_id: Uuid,
        /// Split direction for layout
        direction: SplitDirection,
        /// Whether the receiving client should focus this pane
        #[serde(default)]
        should_focus: bool,
    },

    /// Mirror source pane closed (FEAT-062)
    ///
    /// Sent to mirror panes when their source pane closes.
    /// The mirror pane should display a message and allow the user to close it.
    MirrorSourceClosed {
        /// The mirror pane affected
        mirror_pane_id: Uuid,
        /// The source pane that closed
        source_pane_id: Uuid,
        /// Exit code of the source pane (if available)
        exit_code: Option<i32>,
    },

    // ==================== FEAT-104: Watchdog Timer ====================

    /// Watchdog timer started successfully
    WatchdogStarted {
        /// Target pane receiving the periodic messages
        pane_id: Uuid,
        /// Interval between messages in seconds
        interval_secs: u64,
        /// Message being sent
        message: String,
    },

    /// Watchdog timer stopped
    WatchdogStopped,

    /// Current watchdog status
    WatchdogStatusResponse {
        /// Whether a watchdog timer is currently running
        is_running: bool,
        /// Target pane (if running)
        pane_id: Option<Uuid>,
        /// Interval in seconds (if running)
        interval_secs: Option<u64>,
        /// Message being sent (if running)
        message: Option<String>,
    },
}

impl ServerMessage {
    /// Return the message type name for metrics and logging (FEAT-074/FEAT-109)
    pub fn type_name(&self) -> &'static str {
        match self {
            ServerMessage::Connected { .. } => "Connected",
            ServerMessage::ServerStatus { .. } => "ServerStatus",
            ServerMessage::Sequenced { .. } => "Sequenced",
            ServerMessage::StateSnapshot { .. } => "StateSnapshot",
            ServerMessage::SessionList { .. } => "SessionList",
            ServerMessage::SessionCreated { .. } => "SessionCreated",
            ServerMessage::Attached { .. } => "Attached",
            ServerMessage::WindowCreated { .. } => "WindowCreated",
            ServerMessage::PaneCreated { .. } => "PaneCreated",
            ServerMessage::Output { .. } => "Output",
            ServerMessage::PaneStateChanged { .. } => "PaneStateChanged",
            ServerMessage::ClaudeStateChanged { .. } => "ClaudeStateChanged",
            ServerMessage::PaneClosed { .. } => "PaneClosed",
            ServerMessage::WindowClosed { .. } => "WindowClosed",
            ServerMessage::SessionEnded { .. } => "SessionEnded",
            ServerMessage::SessionsChanged { .. } => "SessionsChanged",
            ServerMessage::Error { .. } => "Error",
            ServerMessage::Pong => "Pong",
            ServerMessage::ViewportUpdated { .. } => "ViewportUpdated",
            ServerMessage::ReplyDelivered { .. } => "ReplyDelivered",
            ServerMessage::OrchestrationReceived { .. } => "OrchestrationReceived",
            ServerMessage::MailReceived { .. } => "MailReceived",
            ServerMessage::OrchestrationDelivered { .. } => "OrchestrationDelivered",
            ServerMessage::WorkerStatus { .. } => "WorkerStatus",
            ServerMessage::MessagesPolled { .. } => "MessagesPolled",
            ServerMessage::AllPanesList { .. } => "AllPanesList",
            ServerMessage::WindowList { .. } => "WindowList",
            ServerMessage::PaneContent { .. } => "PaneContent",
            ServerMessage::PaneStatus { .. } => "PaneStatus",
            ServerMessage::PaneCreatedWithDetails { .. } => "PaneCreatedWithDetails",
            ServerMessage::SessionCreatedWithDetails { .. } => "SessionCreatedWithDetails",
            ServerMessage::WindowCreatedWithDetails { .. } => "WindowCreatedWithDetails",
            ServerMessage::SessionRenamed { .. } => "SessionRenamed",
            ServerMessage::PaneRenamed { .. } => "PaneRenamed",
            ServerMessage::WindowRenamed { .. } => "WindowRenamed",
            ServerMessage::PaneSplit { .. } => "PaneSplit",
            ServerMessage::PaneResized { .. } => "PaneResized",
            ServerMessage::LayoutCreated { .. } => "LayoutCreated",
            ServerMessage::SessionDestroyed { .. } => "SessionDestroyed",
            ServerMessage::EnvironmentSet { .. } => "EnvironmentSet",
            ServerMessage::EnvironmentList { .. } => "EnvironmentList",
            ServerMessage::MetadataSet { .. } => "MetadataSet",
            ServerMessage::MetadataList { .. } => "MetadataList",
            ServerMessage::TagsSet { .. } => "TagsSet",
            ServerMessage::TagsList { .. } => "TagsList",
            ServerMessage::PaneFocused { .. } => "PaneFocused",
            ServerMessage::WindowFocused { .. } => "WindowFocused",
            ServerMessage::SessionFocused { .. } => "SessionFocused",
            ServerMessage::BeadsStatusUpdate { .. } => "BeadsStatusUpdate",
            ServerMessage::BeadsReadyList { .. } => "BeadsReadyList",
            ServerMessage::WidgetUpdate { .. } => "WidgetUpdate",
            ServerMessage::MirrorCreated { .. } => "MirrorCreated",
            ServerMessage::MirrorSourceClosed { .. } => "MirrorSourceClosed",
            ServerMessage::WatchdogStarted { .. } => "WatchdogStarted",
            ServerMessage::WatchdogStopped => "WatchdogStopped",
            ServerMessage::WatchdogStatusResponse { .. } => "WatchdogStatusResponse",
        }
    }
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
    /// User-assigned name for the pane (FEAT-036)
    pub name: Option<String>,
    /// Terminal title from escape sequences
    pub title: Option<String>,
    pub cwd: Option<String>,
    pub state: PaneState,
    pub is_claude: bool,
    pub claude_state: Option<ClaudeState>,
    /// Whether this pane is currently focused (active pane in active window)
    pub is_focused: bool,
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
    /// Session name already exists
    SessionNameExists,
    /// User priority lock is active - MCP focus operations blocked (FEAT-056)
    UserPriorityActive,
}

/// Detailed error information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ErrorDetails {
    /// Human control mode is active
    HumanControl {
        /// Remaining time in milliseconds until lock expires
        remaining_ms: u64,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{HashMap, HashSet};

    #[test]
    fn test_client_message_connect() {
        let client_id = Uuid::new_v4();
        let msg = ClientMessage::Connect {
            client_id,
            protocol_version: 1,
            client_type: crate::ClientType::Tui,
        };

        // Test clone
        let cloned = msg.clone();
        assert_eq!(msg, cloned);

        // Test debug
        let debug = format!("{:?}", msg);
        assert!(debug.contains("Connect"));
        assert!(debug.contains(&client_id.to_string()));
        assert!(debug.contains("Tui"));
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
            command: None,
        };
        if let ClientMessage::CreateSession { name, command } = &msg {
            assert_eq!(name, "test-session");
            assert!(command.is_none());
        } else {
            panic!("Wrong variant");
        }

        let msg_with_cmd = ClientMessage::CreateSession {
            name: "test".to_string(),
            command: Some("claude --resume".to_string()),
        };
        if let ClientMessage::CreateSession { name, command } = &msg_with_cmd {
            assert_eq!(name, "test");
            assert_eq!(command.as_deref(), Some("claude --resume"));
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
tags: HashSet::new(),
                    metadata: HashMap::new(),
            },
            SessionInfo {
                id: Uuid::new_v4(),
                name: "session2".to_string(),
                created_at: 2000,
                window_count: 1,
                attached_clients: 0,
                worktree: None,
tags: HashSet::new(),
                    metadata: HashMap::new(),
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

    // BUG-038: Test for SessionsChanged broadcast message
    #[test]
    fn test_server_message_sessions_changed() {
        let sessions = vec![
            SessionInfo {
                id: Uuid::new_v4(),
                name: "session1".to_string(),
                created_at: 1000,
                window_count: 2,
                attached_clients: 1,
                worktree: None,
                tags: HashSet::new(),
                metadata: HashMap::new(),
            },
        ];

        let msg = ServerMessage::SessionsChanged {
            sessions: sessions.clone(),
        };

        if let ServerMessage::SessionsChanged { sessions: s } = msg.clone() {
            assert_eq!(s.len(), 1);
            assert_eq!(s[0].name, "session1");
        } else {
            panic!("Expected SessionsChanged variant");
        }

        // SessionsChanged should be distinct from SessionList
        let list_msg = ServerMessage::SessionList { sessions };
        assert_ne!(std::mem::discriminant(&msg), std::mem::discriminant(&list_msg));
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
tags: HashSet::new(),
                    metadata: HashMap::new(),
        };

        let msg = ServerMessage::SessionCreated {
            session: session.clone(),
            should_focus: true,
        };

        if let ServerMessage::SessionCreated { session: s, should_focus } = msg {
            assert_eq!(s.name, "new-session");
            assert_eq!(s.window_count, 0);
            assert!(should_focus);
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
tags: HashSet::new(),
                    metadata: HashMap::new(),
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
                                name: None,
                                title: None,
                                cwd: None,
                                stuck_status: None,
                                metadata: HashMap::new(),
                                is_mirror: false,
                                mirror_source: None,
                            }],            commit_seq: 100,
        };

        if let ServerMessage::Attached {
            session,
            windows,
            panes,
            commit_seq,
        } = msg
        {
            assert_eq!(session.id, session_id);
            assert_eq!(windows.len(), 1);
            assert_eq!(panes.len(), 1);
            assert_eq!(commit_seq, 100);
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
            should_focus: true,
        };

        if let ServerMessage::WindowCreated { window: w, should_focus } = msg {
            assert_eq!(w.name, "new-window");
            assert_eq!(w.index, 1);
            assert!(should_focus);
        }
    }

    #[test]
    fn test_server_message_pane_created() {
        let pane_info = PaneInfo {
            id: Uuid::new_v4(),
            window_id: Uuid::new_v4(),
            index: 0,
            cols: 80,
            rows: 24,
            state: PaneState::Normal,
            name: None,
            title: Some("bash".to_string()),
            cwd: Some("/home/user".to_string()),
            stuck_status: None,
            metadata: HashMap::new(),
            is_mirror: false,
            mirror_source: None,
        };

        let msg = ServerMessage::PaneCreated {
            pane: pane_info.clone(),
            direction: SplitDirection::Horizontal,
            should_focus: false,
        };

        if let ServerMessage::PaneCreated {
            pane: p,
            direction,
            should_focus,
        } = msg
        {
            assert_eq!(p.id, pane_info.id);
            assert_eq!(direction, SplitDirection::Horizontal);
            assert!(!should_focus);
        } else {
            panic!("Expected PaneCreated");
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

        // Agent state
        let msg2 = ServerMessage::PaneStateChanged {
            pane_id,
            state: PaneState::Agent(AgentState::new("claude")),
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
            details: None,
        };

        if let ServerMessage::Error { code, message, details } = msg {
            assert_eq!(code, ErrorCode::SessionNotFound);
            assert!(message.contains("test"));
            assert!(details.is_none());
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
            ErrorCode::SessionNameExists,
            ErrorCode::UserPriorityActive,
        ];

        assert_eq!(codes.len(), 11);
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
    fn test_orchestration_message_new() {
        use serde_json::json;

        let msg = OrchestrationMessage::new(
            "status.update",
            json!({"status": "working", "message": "Processing files"}),
        );

        assert_eq!(msg.msg_type, "status.update");
        assert_eq!(msg.payload["status"], "working");
        assert_eq!(msg.payload["message"], "Processing files");
    }

    #[test]
    fn test_orchestration_message_struct_creation() {
        use serde_json::json;

        let msg = OrchestrationMessage {
            msg_type: "task.assigned".to_string(),
            payload: json!({
                "task_id": "abc123",
                "description": "Fix the bug",
                "files": ["src/main.rs", "src/lib.rs"]
            }).into(),
        };

        assert_eq!(msg.msg_type, "task.assigned");
        assert!(msg.payload.is_object());
        assert_eq!(msg.payload["files"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn test_orchestration_message_clone() {
        use serde_json::json;

        let msg = OrchestrationMessage::new("sync.request", json!({}));
        let cloned = msg.clone();

        assert_eq!(msg, cloned);
    }

    #[test]
    fn test_orchestration_message_equality() {
        use serde_json::json;

        let msg1 = OrchestrationMessage::new("test", json!({"key": "value"}));
        let msg2 = OrchestrationMessage::new("test", json!({"key": "value"}));
        let msg3 = OrchestrationMessage::new("test", json!({"key": "other"}));
        let msg4 = OrchestrationMessage::new("other", json!({"key": "value"}));

        assert_eq!(msg1, msg2);
        assert_ne!(msg1, msg3);
        assert_ne!(msg1, msg4);
    }

    #[test]
    fn test_orchestration_message_with_nested_payload() {
        use serde_json::json;

        let msg = OrchestrationMessage::new(
            "complex.message",
            json!({
                "metadata": {
                    "version": "1.0",
                    "timestamp": 1234567890
                },
                "data": {
                    "items": [1, 2, 3],
                    "nested": {
                        "deep": true
                    }
                }
            }),
        );

        assert_eq!(msg.payload["metadata"]["version"], "1.0");
        assert_eq!(msg.payload["data"]["nested"]["deep"], true);
    }

    #[test]
    fn test_orchestration_message_with_null_payload() {
        use serde_json::json;

        let msg = OrchestrationMessage::new("ping", json!(null));
        assert!(msg.payload.is_null());
    }

    #[test]
    fn test_orchestration_message_debug() {
        use serde_json::json;

        let msg = OrchestrationMessage::new("debug.test", json!({"value": 42}));
        let debug = format!("{:?}", msg);

        assert!(debug.contains("OrchestrationMessage"));
        assert!(debug.contains("debug.test"));
    }

    #[test]
    fn test_orchestration_target_tagged() {
        let target = OrchestrationTarget::Tagged("orchestrator".to_string());

        if let OrchestrationTarget::Tagged(tag) = target {
            assert_eq!(tag, "orchestrator");
        } else {
            panic!("Wrong variant");
        }
    }

    #[test]
    fn test_orchestration_target_tagged_clone() {
        let target = OrchestrationTarget::Tagged("worker".to_string());
        let cloned = target.clone();
        assert_eq!(target, cloned);
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
    fn test_orchestration_target_equality() {
        let tagged1 = OrchestrationTarget::Tagged("test".to_string());
        let tagged2 = OrchestrationTarget::Tagged("test".to_string());
        let tagged3 = OrchestrationTarget::Tagged("other".to_string());
        let broadcast = OrchestrationTarget::Broadcast;

        assert_eq!(tagged1, tagged2);
        assert_ne!(tagged1, tagged3);
        assert_ne!(tagged1, broadcast);
    }

    #[test]
    fn test_client_message_send_orchestration() {
        use serde_json::json;

        let target = OrchestrationTarget::Tagged("mayor".to_string());
        let message = OrchestrationMessage::new(
            "status.update",
            json!({"status": "idle"}),
        );

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
        use serde_json::json;

        let from_session_id = Uuid::new_v4();
        let message = OrchestrationMessage::new("sync.request", json!({}));

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
    fn test_server_message_mail_received() {
        let pane_id = Uuid::new_v4();
        let msg = ServerMessage::MailReceived {
            pane_id,
            priority: MailPriority::Warning,
            summary: "Disk space low".to_string(),
        };

        if let ServerMessage::MailReceived {
            pane_id: pid,
            priority,
            summary,
        } = msg
        {
            assert_eq!(pid, pane_id);
            assert_eq!(priority, MailPriority::Warning);
            assert_eq!(summary, "Disk space low");
        } else {
            panic!("Expected MailReceived variant");
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

    // ==================== FEAT-056: User Priority Lock Tests ====================

    #[test]
    fn test_client_message_user_command_mode_entered() {
        let msg = ClientMessage::UserCommandModeEntered { timeout_ms: 500 };

        if let ClientMessage::UserCommandModeEntered { timeout_ms } = msg {
            assert_eq!(timeout_ms, 500);
        } else {
            panic!("Wrong variant");
        }
    }

    #[test]
    fn test_client_message_user_command_mode_exited() {
        let msg = ClientMessage::UserCommandModeExited;
        assert_eq!(msg.clone(), ClientMessage::UserCommandModeExited);
    }

    #[test]
    fn test_user_command_mode_messages_clone() {
        let entered = ClientMessage::UserCommandModeEntered { timeout_ms: 1000 };
        let cloned = entered.clone();
        assert_eq!(entered, cloned);

        let exited = ClientMessage::UserCommandModeExited;
        let cloned = exited.clone();
        assert_eq!(exited, cloned);
    }

    #[test]
    fn test_error_code_user_priority_active() {
        let code = ErrorCode::UserPriorityActive;
        assert_eq!(code, ErrorCode::UserPriorityActive);
        assert_ne!(code, ErrorCode::InvalidOperation);

        let debug = format!("{:?}", code);
        assert_eq!(debug, "UserPriorityActive");
    }

    #[test]
    fn test_user_command_mode_serialization() {
        // Test that the messages can be serialized and deserialized correctly
        let entered = ClientMessage::UserCommandModeEntered { timeout_ms: 750 };
        let json = serde_json::to_string(&entered).unwrap();
        let deserialized: ClientMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(entered, deserialized);

        let exited = ClientMessage::UserCommandModeExited;
        let json = serde_json::to_string(&exited).unwrap();
        let deserialized: ClientMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(exited, deserialized);
    }
}
