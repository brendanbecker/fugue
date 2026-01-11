//! Shared data types for ccmux protocol

use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// A wrapper for serde_json::Value that serializes as a JSON string for bincode compatibility.
///
/// Bincode doesn't support `deserialize_any` which `serde_json::Value` requires.
/// This wrapper serializes the JSON value as a string, which bincode can handle.
#[derive(Debug, Clone, PartialEq)]
pub struct JsonValue(pub serde_json::Value);

impl JsonValue {
    /// Create a new JsonValue from a serde_json::Value
    pub fn new(value: serde_json::Value) -> Self {
        Self(value)
    }

    /// Get a reference to the inner value
    pub fn inner(&self) -> &serde_json::Value {
        &self.0
    }

    /// Consume the wrapper and return the inner value
    pub fn into_inner(self) -> serde_json::Value {
        self.0
    }
}

impl From<serde_json::Value> for JsonValue {
    fn from(value: serde_json::Value) -> Self {
        Self(value)
    }
}

impl From<JsonValue> for serde_json::Value {
    fn from(value: JsonValue) -> Self {
        value.0
    }
}

impl std::ops::Deref for JsonValue {
    type Target = serde_json::Value;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Serialize for JsonValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        // Serialize as a JSON string for bincode compatibility
        let json_string = serde_json::to_string(&self.0).map_err(serde::ser::Error::custom)?;
        serializer.serialize_str(&json_string)
    }
}

impl<'de> Deserialize<'de> for JsonValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        // Deserialize from a JSON string
        let json_string = String::deserialize(deserializer)?;
        let value: serde_json::Value =
            serde_json::from_str(&json_string).map_err(serde::de::Error::custom)?;
        Ok(Self(value))
    }
}

/// Split direction for creating panes
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum SplitDirection {
    Horizontal,
    Vertical,
}

/// Worktree information for protocol messages
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorktreeInfo {
    /// Absolute path to the worktree
    pub path: String,
    /// Branch name (if any)
    pub branch: Option<String>,
    /// Whether this is the main worktree
    pub is_main: bool,
}

/// Session information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SessionInfo {
    pub id: Uuid,
    pub name: String,
    pub created_at: u64, // Unix timestamp
    pub window_count: usize,
    pub attached_clients: usize,
    /// Associated worktree (if any)
    pub worktree: Option<WorktreeInfo>,
    /// Tags for session classification and routing (e.g., "orchestrator", "worker", "evaluator")
    #[serde(default)]
    pub tags: HashSet<String>,
    /// Arbitrary key-value metadata for application use
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

impl SessionInfo {
    /// Check if session has a specific tag
    pub fn has_tag(&self, tag: &str) -> bool {
        self.tags.contains(tag)
    }

    /// Add a tag to the session
    pub fn add_tag(&mut self, tag: impl Into<String>) {
        self.tags.insert(tag.into());
    }

    /// Remove a tag from the session
    pub fn remove_tag(&mut self, tag: &str) -> bool {
        self.tags.remove(tag)
    }
}

/// Window information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WindowInfo {
    pub id: Uuid,
    pub session_id: Uuid,
    pub name: String,
    pub index: usize,
    pub pane_count: usize,
    pub active_pane_id: Option<Uuid>,
}

/// Pane information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PaneInfo {
    pub id: Uuid,
    pub window_id: Uuid,
    pub index: usize,
    pub cols: u16,
    pub rows: u16,
    pub state: PaneState,
    /// User-assigned name for the pane (FEAT-036)
    pub name: Option<String>,
    /// Terminal title from escape sequences
    pub title: Option<String>,
    pub cwd: Option<String>,
}

/// Pane state
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum PaneState {
    /// Normal shell/process
    #[default]
    Normal,
    /// Claude Code detected
    Claude(ClaudeState),
    /// Process exited
    Exited { code: Option<i32> },
}

/// Claude Code specific state
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ClaudeState {
    /// Claude session ID if detected
    pub session_id: Option<String>,
    /// Current activity state
    pub activity: ClaudeActivity,
    /// Model being used
    pub model: Option<String>,
    /// Token usage if available
    pub tokens_used: Option<u64>,
}

impl Default for ClaudeState {
    fn default() -> Self {
        Self {
            session_id: None,
            activity: ClaudeActivity::Idle,
            model: None,
            tokens_used: None,
        }
    }
}

/// Claude activity states
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ClaudeActivity {
    /// Waiting for input
    Idle,
    /// Processing/thinking
    Thinking,
    /// Writing code
    Coding,
    /// Executing tools
    ToolUse,
    /// Waiting for user confirmation
    AwaitingConfirmation,
}

/// Terminal dimensions
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct Dimensions {
    pub cols: u16,
    pub rows: u16,
}

impl Dimensions {
    pub fn new(cols: u16, rows: u16) -> Self {
        Self { cols, rows }
    }
}

/// Viewport state for scroll position tracking
///
/// Tracks the scroll position within a pane and whether the user has
/// scrolled up (pinned) from the bottom. When pinned, new output is
/// buffered without yanking the viewport, and a count of new lines
/// is maintained for the "new content" indicator.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct ViewportState {
    /// Lines offset from bottom (0 = at bottom, following new content)
    pub offset_from_bottom: usize,
    /// True if user has scrolled up from bottom
    pub is_pinned: bool,
    /// Number of new lines received while viewport is pinned
    pub new_lines_since_pin: usize,
}

impl ViewportState {
    /// Create a new viewport state at the bottom (following output)
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a pinned viewport state at the given offset
    pub fn pinned(offset: usize) -> Self {
        Self {
            offset_from_bottom: offset,
            is_pinned: true,
            new_lines_since_pin: 0,
        }
    }

    /// Check if viewport is at the bottom (following new content)
    pub fn is_at_bottom(&self) -> bool {
        self.offset_from_bottom == 0 && !self.is_pinned
    }

    /// Pin the viewport at the current offset (user scrolled up)
    pub fn pin(&mut self, offset: usize) {
        self.offset_from_bottom = offset;
        self.is_pinned = true;
        // Don't reset new_lines_since_pin - keep counting
    }

    /// Unpin and jump to bottom
    pub fn jump_to_bottom(&mut self) {
        self.offset_from_bottom = 0;
        self.is_pinned = false;
        self.new_lines_since_pin = 0;
    }

    /// Record new lines received while pinned
    pub fn add_new_lines(&mut self, count: usize) {
        if self.is_pinned {
            self.new_lines_since_pin = self.new_lines_since_pin.saturating_add(count);
        }
    }
}

/// Message to send a reply to a pane awaiting input
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReplyMessage {
    /// Target pane (by ID or name)
    pub target: PaneTarget,
    /// Content to send to the pane's stdin
    pub content: String,
}

/// Target specification for a pane (by ID or name)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PaneTarget {
    /// Target by UUID
    Id(Uuid),
    /// Target by pane name/title
    Name(String),
}

impl ReplyMessage {
    /// Create a reply message targeting a pane by ID
    pub fn by_id(pane_id: Uuid, content: impl Into<String>) -> Self {
        Self {
            target: PaneTarget::Id(pane_id),
            content: content.into(),
        }
    }

    /// Create a reply message targeting a pane by name
    pub fn by_name(name: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            target: PaneTarget::Name(name.into()),
            content: content.into(),
        }
    }
}

/// Result of a reply operation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReplyResult {
    /// The pane that received the reply
    pub pane_id: Uuid,
    /// Number of bytes written
    pub bytes_written: usize,
}

// ==================== Beads Query Types (FEAT-058) ====================

/// A task from the beads daemon work queue
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BeadsTask {
    /// Task ID (e.g., "BUG-042", "FEAT-015")
    pub id: String,
    /// Task title/summary
    pub title: String,
    /// Priority level (0 = highest, higher = lower priority)
    pub priority: i32,
    /// Current status (e.g., "open", "in_progress")
    pub status: String,
    /// Issue type (e.g., "bug", "feature")
    pub issue_type: String,
    /// Assigned user (if any)
    pub assignee: Option<String>,
    /// Labels attached to the task
    #[serde(default)]
    pub labels: Vec<String>,
}

impl BeadsTask {
    /// Check if this task has a specific label
    pub fn has_label(&self, label: &str) -> bool {
        self.labels.iter().any(|l| l.eq_ignore_ascii_case(label))
    }

    /// Get a short display string for the task
    pub fn short_display(&self) -> String {
        format!("{} P{} {}", self.id, self.priority, self.title)
    }
}

/// Beads daemon status for a pane's repository
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct BeadsStatus {
    /// Whether the daemon is available and responding
    pub daemon_available: bool,
    /// Number of ready tasks (no blockers)
    pub ready_count: usize,
    /// Ready tasks (may be limited/summarized)
    #[serde(default)]
    pub ready_tasks: Vec<BeadsTask>,
    /// Unix timestamp of last successful refresh
    pub last_refresh: Option<u64>,
    /// Error message if daemon unavailable
    pub error: Option<String>,
}

impl BeadsStatus {
    /// Create a status indicating daemon is unavailable
    pub fn unavailable() -> Self {
        Self {
            daemon_available: false,
            ready_count: 0,
            ready_tasks: Vec::new(),
            last_refresh: None,
            error: None,
        }
    }

    /// Create a status with an error message
    pub fn with_error(error: impl Into<String>) -> Self {
        Self {
            daemon_available: false,
            ready_count: 0,
            ready_tasks: Vec::new(),
            last_refresh: None,
            error: Some(error.into()),
        }
    }

    /// Create a successful status with tasks
    pub fn with_tasks(tasks: Vec<BeadsTask>, timestamp: u64) -> Self {
        let ready_count = tasks.len();
        Self {
            daemon_available: true,
            ready_count,
            ready_tasks: tasks,
            last_refresh: Some(timestamp),
            error: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== SplitDirection Tests ====================

    #[test]
    fn test_split_direction_horizontal() {
        let dir = SplitDirection::Horizontal;
        assert_eq!(dir, SplitDirection::Horizontal);
        assert_ne!(dir, SplitDirection::Vertical);
    }

    #[test]
    fn test_split_direction_vertical() {
        let dir = SplitDirection::Vertical;
        assert_eq!(dir, SplitDirection::Vertical);
        assert_ne!(dir, SplitDirection::Horizontal);
    }

    #[test]
    fn test_split_direction_clone() {
        let dir = SplitDirection::Horizontal;
        let cloned = dir.clone();
        assert_eq!(dir, cloned);
    }

    #[test]
    fn test_split_direction_copy() {
        let dir = SplitDirection::Vertical;
        let copied = dir; // Copy semantics
        assert_eq!(dir, copied);
    }

    #[test]
    fn test_split_direction_debug() {
        assert_eq!(format!("{:?}", SplitDirection::Horizontal), "Horizontal");
        assert_eq!(format!("{:?}", SplitDirection::Vertical), "Vertical");
    }

    // ==================== Dimensions Tests ====================

    #[test]
    fn test_dimensions_new() {
        let dims = Dimensions::new(80, 24);
        assert_eq!(dims.cols, 80);
        assert_eq!(dims.rows, 24);
    }

    #[test]
    fn test_dimensions_equality() {
        let dims1 = Dimensions::new(80, 24);
        let dims2 = Dimensions::new(80, 24);
        let dims3 = Dimensions::new(120, 40);

        assert_eq!(dims1, dims2);
        assert_ne!(dims1, dims3);
    }

    #[test]
    fn test_dimensions_clone_copy() {
        let dims = Dimensions::new(100, 50);
        let cloned = dims.clone();
        let copied = dims; // Copy

        assert_eq!(dims, cloned);
        assert_eq!(dims, copied);
    }

    #[test]
    fn test_dimensions_debug() {
        let dims = Dimensions::new(80, 24);
        let debug = format!("{:?}", dims);
        assert!(debug.contains("80"));
        assert!(debug.contains("24"));
    }

    #[test]
    fn test_dimensions_zero() {
        let dims = Dimensions::new(0, 0);
        assert_eq!(dims.cols, 0);
        assert_eq!(dims.rows, 0);
    }

    #[test]
    fn test_dimensions_max_values() {
        let dims = Dimensions::new(u16::MAX, u16::MAX);
        assert_eq!(dims.cols, u16::MAX);
        assert_eq!(dims.rows, u16::MAX);
    }

    // ==================== ClaudeActivity Tests ====================

    #[test]
    fn test_claude_activity_all_variants() {
        let activities = [
            ClaudeActivity::Idle,
            ClaudeActivity::Thinking,
            ClaudeActivity::Coding,
            ClaudeActivity::ToolUse,
            ClaudeActivity::AwaitingConfirmation,
        ];

        assert_eq!(activities.len(), 5);

        // All should be unique
        for (i, a) in activities.iter().enumerate() {
            for (j, b) in activities.iter().enumerate() {
                if i == j {
                    assert_eq!(a, b);
                } else {
                    assert_ne!(a, b);
                }
            }
        }
    }

    #[test]
    fn test_claude_activity_clone() {
        let activity = ClaudeActivity::Thinking;
        let cloned = activity.clone();
        assert_eq!(activity, cloned);
    }

    #[test]
    fn test_claude_activity_debug() {
        assert_eq!(format!("{:?}", ClaudeActivity::Idle), "Idle");
        assert_eq!(format!("{:?}", ClaudeActivity::Thinking), "Thinking");
        assert_eq!(format!("{:?}", ClaudeActivity::Coding), "Coding");
        assert_eq!(format!("{:?}", ClaudeActivity::ToolUse), "ToolUse");
        assert_eq!(
            format!("{:?}", ClaudeActivity::AwaitingConfirmation),
            "AwaitingConfirmation"
        );
    }

    // ==================== ClaudeState Tests ====================

    #[test]
    fn test_claude_state_default() {
        let state = ClaudeState::default();

        assert!(state.session_id.is_none());
        assert_eq!(state.activity, ClaudeActivity::Idle);
        assert!(state.model.is_none());
        assert!(state.tokens_used.is_none());
    }

    #[test]
    fn test_claude_state_with_all_fields() {
        let state = ClaudeState {
            session_id: Some("session-123".to_string()),
            activity: ClaudeActivity::Coding,
            model: Some("claude-3-opus".to_string()),
            tokens_used: Some(5000),
        };

        assert_eq!(state.session_id, Some("session-123".to_string()));
        assert_eq!(state.activity, ClaudeActivity::Coding);
        assert_eq!(state.model, Some("claude-3-opus".to_string()));
        assert_eq!(state.tokens_used, Some(5000));
    }

    #[test]
    fn test_claude_state_clone() {
        let state = ClaudeState {
            session_id: Some("test".to_string()),
            activity: ClaudeActivity::ToolUse,
            model: Some("claude-3-sonnet".to_string()),
            tokens_used: Some(1000),
        };

        let cloned = state.clone();
        assert_eq!(state, cloned);
    }

    #[test]
    fn test_claude_state_equality() {
        let state1 = ClaudeState::default();
        let state2 = ClaudeState::default();
        let state3 = ClaudeState {
            session_id: Some("x".to_string()),
            ..ClaudeState::default()
        };

        assert_eq!(state1, state2);
        assert_ne!(state1, state3);
    }

    #[test]
    fn test_claude_state_debug() {
        let state = ClaudeState::default();
        let debug = format!("{:?}", state);
        assert!(debug.contains("ClaudeState"));
        assert!(debug.contains("Idle"));
    }

    // ==================== PaneState Tests ====================

    #[test]
    fn test_pane_state_default() {
        let state = PaneState::default();
        assert_eq!(state, PaneState::Normal);
    }

    #[test]
    fn test_pane_state_normal() {
        let state = PaneState::Normal;
        assert_eq!(state.clone(), PaneState::Normal);
    }

    #[test]
    fn test_pane_state_claude() {
        let claude_state = ClaudeState::default();
        let state = PaneState::Claude(claude_state.clone());

        if let PaneState::Claude(cs) = &state {
            assert_eq!(*cs, claude_state);
        } else {
            panic!("Expected Claude variant");
        }
    }

    #[test]
    fn test_pane_state_exited_with_code() {
        let state = PaneState::Exited { code: Some(0) };

        if let PaneState::Exited { code } = state {
            assert_eq!(code, Some(0));
        }
    }

    #[test]
    fn test_pane_state_exited_without_code() {
        let state = PaneState::Exited { code: None };

        if let PaneState::Exited { code } = state {
            assert!(code.is_none());
        }
    }

    #[test]
    fn test_pane_state_exited_error_code() {
        let state = PaneState::Exited { code: Some(1) };

        if let PaneState::Exited { code } = state {
            assert_eq!(code, Some(1));
        }
    }

    #[test]
    fn test_pane_state_exited_signal() {
        // Simulating killed by signal (128 + signal number)
        let state = PaneState::Exited { code: Some(137) }; // SIGKILL

        if let PaneState::Exited { code } = state {
            assert_eq!(code, Some(137));
        }
    }

    #[test]
    fn test_pane_state_equality() {
        let normal1 = PaneState::Normal;
        let normal2 = PaneState::Normal;
        let claude = PaneState::Claude(ClaudeState::default());
        let exited = PaneState::Exited { code: Some(0) };

        assert_eq!(normal1, normal2);
        assert_ne!(normal1, claude);
        assert_ne!(normal1, exited);
        assert_ne!(claude, exited);
    }

    #[test]
    fn test_pane_state_clone() {
        let states = [
            PaneState::Normal,
            PaneState::Claude(ClaudeState::default()),
            PaneState::Exited { code: Some(42) },
        ];

        for state in states {
            let cloned = state.clone();
            assert_eq!(state, cloned);
        }
    }

    // ==================== PaneInfo Tests ====================

    #[test]
    fn test_pane_info_minimal() {
        let pane = PaneInfo {
            id: Uuid::new_v4(),
            window_id: Uuid::new_v4(),
            index: 0,
            cols: 80,
            rows: 24,
            state: PaneState::Normal,
            name: None,
            title: None,
            cwd: None,
        };

        assert_eq!(pane.index, 0);
        assert_eq!(pane.cols, 80);
        assert_eq!(pane.rows, 24);
        assert!(pane.title.is_none());
        assert!(pane.cwd.is_none());
    }

    #[test]
    fn test_pane_info_full() {
        let id = Uuid::new_v4();
        let window_id = Uuid::new_v4();

        let pane = PaneInfo {
            id,
            window_id,
            index: 2,
            cols: 120,
            rows: 40,
            state: PaneState::Claude(ClaudeState::default()),
            name: None,
            title: Some("vim".to_string()),
            cwd: Some("/home/user/project".to_string()),
        };

        assert_eq!(pane.id, id);
        assert_eq!(pane.window_id, window_id);
        assert_eq!(pane.index, 2);
        assert_eq!(pane.title, Some("vim".to_string()));
        assert_eq!(pane.cwd, Some("/home/user/project".to_string()));
    }

    #[test]
    fn test_pane_info_clone() {
        let pane = PaneInfo {
            id: Uuid::new_v4(),
            window_id: Uuid::new_v4(),
            index: 0,
            cols: 80,
            rows: 24,
            state: PaneState::Normal,
            name: None,
            title: Some("bash".to_string()),
            cwd: Some("/tmp".to_string()),
        };

        let cloned = pane.clone();
        assert_eq!(pane, cloned);
    }

    #[test]
    fn test_pane_info_equality() {
        let id = Uuid::new_v4();
        let window_id = Uuid::new_v4();

        let pane1 = PaneInfo {
            id,
            window_id,
            index: 0,
            cols: 80,
            rows: 24,
            state: PaneState::Normal,
            name: None,
            title: None,
            cwd: None,
        };

        let pane2 = PaneInfo {
            id,
            window_id,
            index: 0,
            cols: 80,
            rows: 24,
            state: PaneState::Normal,
            name: None,
            title: None,
            cwd: None,
        };

        let pane3 = PaneInfo {
            id,
            window_id,
            index: 1, // Different index
            cols: 80,
            rows: 24,
            state: PaneState::Normal,
            name: None,
            title: None,
            cwd: None,
        };

        assert_eq!(pane1, pane2);
        assert_ne!(pane1, pane3);
    }

    // ==================== WindowInfo Tests ====================

    #[test]
    fn test_window_info_minimal() {
        let window = WindowInfo {
            id: Uuid::new_v4(),
            session_id: Uuid::new_v4(),
            name: "main".to_string(),
            index: 0,
            pane_count: 1,
            active_pane_id: None,
        };

        assert_eq!(window.name, "main");
        assert_eq!(window.index, 0);
        assert_eq!(window.pane_count, 1);
        assert!(window.active_pane_id.is_none());
    }

    #[test]
    fn test_window_info_with_active_pane() {
        let pane_id = Uuid::new_v4();

        let window = WindowInfo {
            id: Uuid::new_v4(),
            session_id: Uuid::new_v4(),
            name: "editor".to_string(),
            index: 1,
            pane_count: 3,
            active_pane_id: Some(pane_id),
        };

        assert_eq!(window.active_pane_id, Some(pane_id));
        assert_eq!(window.pane_count, 3);
    }

    #[test]
    fn test_window_info_clone() {
        let window = WindowInfo {
            id: Uuid::new_v4(),
            session_id: Uuid::new_v4(),
            name: "test".to_string(),
            index: 0,
            pane_count: 2,
            active_pane_id: Some(Uuid::new_v4()),
        };

        let cloned = window.clone();
        assert_eq!(window, cloned);
    }

    #[test]
    fn test_window_info_equality() {
        let id = Uuid::new_v4();
        let session_id = Uuid::new_v4();

        let window1 = WindowInfo {
            id,
            session_id,
            name: "main".to_string(),
            index: 0,
            pane_count: 1,
            active_pane_id: None,
        };

        let window2 = WindowInfo {
            id,
            session_id,
            name: "main".to_string(),
            index: 0,
            pane_count: 1,
            active_pane_id: None,
        };

        let window3 = WindowInfo {
            id,
            session_id,
            name: "other".to_string(), // Different name
            index: 0,
            pane_count: 1,
            active_pane_id: None,
        };

        assert_eq!(window1, window2);
        assert_ne!(window1, window3);
    }

    // ==================== SessionInfo Tests ====================

    #[test]
    fn test_session_info_creation() {
        let id = Uuid::new_v4();

        let session = SessionInfo {
            id,
            name: "my-session".to_string(),
            created_at: 1704067200, // 2024-01-01 00:00:00 UTC
            window_count: 2,
            attached_clients: 1,
            worktree: None,
tags: HashSet::new(),
                    metadata: HashMap::new(),
        };

        assert_eq!(session.id, id);
        assert_eq!(session.name, "my-session");
        assert_eq!(session.created_at, 1704067200);
        assert_eq!(session.window_count, 2);
        assert_eq!(session.attached_clients, 1);
    }

    #[test]
    fn test_session_info_no_clients() {
        let session = SessionInfo {
            id: Uuid::new_v4(),
            name: "detached".to_string(),
            created_at: 0,
            window_count: 1,
            attached_clients: 0,
            worktree: None,
tags: HashSet::new(),
                    metadata: HashMap::new(),
        };

        assert_eq!(session.attached_clients, 0);
    }

    #[test]
    fn test_session_info_multiple_clients() {
        let session = SessionInfo {
            id: Uuid::new_v4(),
            name: "shared".to_string(),
            created_at: 0,
            window_count: 1,
            attached_clients: 5,
            worktree: None,
tags: HashSet::new(),
                    metadata: HashMap::new(),
        };

        assert_eq!(session.attached_clients, 5);
    }

    #[test]
    fn test_session_info_clone() {
        let session = SessionInfo {
            id: Uuid::new_v4(),
            name: "test".to_string(),
            created_at: 12345,
            window_count: 3,
            attached_clients: 2,
            worktree: None,
tags: HashSet::new(),
                    metadata: HashMap::new(),
        };

        let cloned = session.clone();
        assert_eq!(session, cloned);
    }

    #[test]
    fn test_session_info_equality() {
        let id = Uuid::new_v4();

        let session1 = SessionInfo {
            id,
            name: "test".to_string(),
            created_at: 1000,
            window_count: 1,
            attached_clients: 0,
            worktree: None,
tags: HashSet::new(),
                    metadata: HashMap::new(),
        };

        let session2 = SessionInfo {
            id,
            name: "test".to_string(),
            created_at: 1000,
            window_count: 1,
            attached_clients: 0,
            worktree: None,
tags: HashSet::new(),
                    metadata: HashMap::new(),
        };

        let session3 = SessionInfo {
            id,
            name: "different".to_string(),
            created_at: 1000,
            window_count: 1,
            attached_clients: 0,
            worktree: None,
tags: HashSet::new(),
                    metadata: HashMap::new(),
        };

        assert_eq!(session1, session2);
        assert_ne!(session1, session3);
    }

    #[test]
    fn test_session_info_debug() {
        let session = SessionInfo {
            id: Uuid::nil(),
            name: "debug-test".to_string(),
            created_at: 0,
            window_count: 0,
            attached_clients: 0,
            worktree: None,
tags: HashSet::new(),
                    metadata: HashMap::new(),
        };

        let debug = format!("{:?}", session);
        assert!(debug.contains("SessionInfo"));
        assert!(debug.contains("debug-test"));
    }

    // ==================== Serialization Round-trip Tests ====================

    #[test]
    fn test_split_direction_serde() {
        let dir = SplitDirection::Horizontal;
        let serialized = bincode::serialize(&dir).unwrap();
        let deserialized: SplitDirection = bincode::deserialize(&serialized).unwrap();
        assert_eq!(dir, deserialized);
    }

    #[test]
    fn test_dimensions_serde() {
        let dims = Dimensions::new(80, 24);
        let serialized = bincode::serialize(&dims).unwrap();
        let deserialized: Dimensions = bincode::deserialize(&serialized).unwrap();
        assert_eq!(dims, deserialized);
    }

    #[test]
    fn test_claude_activity_serde() {
        for activity in [
            ClaudeActivity::Idle,
            ClaudeActivity::Thinking,
            ClaudeActivity::Coding,
            ClaudeActivity::ToolUse,
            ClaudeActivity::AwaitingConfirmation,
        ] {
            let serialized = bincode::serialize(&activity).unwrap();
            let deserialized: ClaudeActivity = bincode::deserialize(&serialized).unwrap();
            assert_eq!(activity, deserialized);
        }
    }

    #[test]
    fn test_claude_state_serde() {
        let state = ClaudeState {
            session_id: Some("abc".to_string()),
            activity: ClaudeActivity::Coding,
            model: Some("claude-3".to_string()),
            tokens_used: Some(100),
        };

        let serialized = bincode::serialize(&state).unwrap();
        let deserialized: ClaudeState = bincode::deserialize(&serialized).unwrap();
        assert_eq!(state, deserialized);
    }

    #[test]
    fn test_pane_state_serde() {
        let states = [
            PaneState::Normal,
            PaneState::Claude(ClaudeState::default()),
            PaneState::Exited { code: Some(0) },
            PaneState::Exited { code: None },
        ];

        for state in states {
            let serialized = bincode::serialize(&state).unwrap();
            let deserialized: PaneState = bincode::deserialize(&serialized).unwrap();
            assert_eq!(state, deserialized);
        }
    }

    #[test]
    fn test_pane_info_serde() {
        let pane = PaneInfo {
            id: Uuid::new_v4(),
            window_id: Uuid::new_v4(),
            index: 0,
            cols: 80,
            rows: 24,
            state: PaneState::Normal,
            name: None,
            title: Some("test".to_string()),
            cwd: Some("/home".to_string()),
        };

        let serialized = bincode::serialize(&pane).unwrap();
        let deserialized: PaneInfo = bincode::deserialize(&serialized).unwrap();
        assert_eq!(pane, deserialized);
    }

    #[test]
    fn test_window_info_serde() {
        let window = WindowInfo {
            id: Uuid::new_v4(),
            session_id: Uuid::new_v4(),
            name: "main".to_string(),
            index: 0,
            pane_count: 2,
            active_pane_id: Some(Uuid::new_v4()),
        };

        let serialized = bincode::serialize(&window).unwrap();
        let deserialized: WindowInfo = bincode::deserialize(&serialized).unwrap();
        assert_eq!(window, deserialized);
    }

    #[test]
    fn test_session_info_serde() {
        let session = SessionInfo {
            id: Uuid::new_v4(),
            name: "test-session".to_string(),
            created_at: 1234567890,
            window_count: 3,
            attached_clients: 1,
            worktree: None,
tags: HashSet::new(),
                    metadata: HashMap::new(),
        };

        let serialized = bincode::serialize(&session).unwrap();
        let deserialized: SessionInfo = bincode::deserialize(&serialized).unwrap();
        assert_eq!(session, deserialized);
    }

    // ==================== ViewportState Tests ====================

    #[test]
    fn test_viewport_state_default() {
        let state = ViewportState::default();

        assert_eq!(state.offset_from_bottom, 0);
        assert!(!state.is_pinned);
        assert_eq!(state.new_lines_since_pin, 0);
    }

    #[test]
    fn test_viewport_state_new() {
        let state = ViewportState::new();

        assert_eq!(state, ViewportState::default());
        assert!(state.is_at_bottom());
    }

    #[test]
    fn test_viewport_state_pinned() {
        let state = ViewportState::pinned(50);

        assert_eq!(state.offset_from_bottom, 50);
        assert!(state.is_pinned);
        assert_eq!(state.new_lines_since_pin, 0);
        assert!(!state.is_at_bottom());
    }

    #[test]
    fn test_viewport_state_is_at_bottom() {
        // At bottom when offset is 0 and not pinned
        let at_bottom = ViewportState::new();
        assert!(at_bottom.is_at_bottom());

        // Not at bottom when pinned (even with offset 0)
        let pinned_at_zero = ViewportState {
            offset_from_bottom: 0,
            is_pinned: true,
            new_lines_since_pin: 0,
        };
        assert!(!pinned_at_zero.is_at_bottom());

        // Not at bottom when offset > 0
        let scrolled_up = ViewportState::pinned(10);
        assert!(!scrolled_up.is_at_bottom());
    }

    #[test]
    fn test_viewport_state_pin() {
        let mut state = ViewportState::new();

        state.pin(100);
        assert!(state.is_pinned);
        assert_eq!(state.offset_from_bottom, 100);
        assert!(!state.is_at_bottom());

        // Pin again at different offset
        state.pin(50);
        assert_eq!(state.offset_from_bottom, 50);
    }

    #[test]
    fn test_viewport_state_jump_to_bottom() {
        let mut state = ViewportState::pinned(100);
        state.new_lines_since_pin = 50;

        state.jump_to_bottom();

        assert_eq!(state.offset_from_bottom, 0);
        assert!(!state.is_pinned);
        assert_eq!(state.new_lines_since_pin, 0);
        assert!(state.is_at_bottom());
    }

    #[test]
    fn test_viewport_state_add_new_lines() {
        let mut state = ViewportState::pinned(10);

        state.add_new_lines(5);
        assert_eq!(state.new_lines_since_pin, 5);

        state.add_new_lines(10);
        assert_eq!(state.new_lines_since_pin, 15);
    }

    #[test]
    fn test_viewport_state_add_new_lines_not_pinned() {
        let mut state = ViewportState::new();

        // When not pinned, adding lines should not increment counter
        state.add_new_lines(10);
        assert_eq!(state.new_lines_since_pin, 0);
    }

    #[test]
    fn test_viewport_state_add_new_lines_overflow() {
        let mut state = ViewportState::pinned(10);
        state.new_lines_since_pin = usize::MAX - 5;

        // Should saturate instead of overflow
        state.add_new_lines(10);
        assert_eq!(state.new_lines_since_pin, usize::MAX);
    }

    #[test]
    fn test_viewport_state_pin_preserves_new_lines() {
        let mut state = ViewportState::pinned(10);
        state.add_new_lines(20);

        // Pinning again should preserve the line count
        state.pin(50);
        assert_eq!(state.new_lines_since_pin, 20);
    }

    #[test]
    fn test_viewport_state_clone() {
        let state = ViewportState {
            offset_from_bottom: 42,
            is_pinned: true,
            new_lines_since_pin: 100,
        };

        let cloned = state.clone();
        assert_eq!(state, cloned);
    }

    #[test]
    fn test_viewport_state_copy() {
        let state = ViewportState::pinned(10);
        let copied = state; // Copy semantics

        assert_eq!(state, copied);
    }

    #[test]
    fn test_viewport_state_debug() {
        let state = ViewportState::pinned(25);
        let debug = format!("{:?}", state);

        assert!(debug.contains("ViewportState"));
        assert!(debug.contains("25"));
        assert!(debug.contains("true"));
    }

    #[test]
    fn test_viewport_state_equality() {
        let state1 = ViewportState::pinned(10);
        let state2 = ViewportState::pinned(10);
        let state3 = ViewportState::pinned(20);

        assert_eq!(state1, state2);
        assert_ne!(state1, state3);
    }

    #[test]
    fn test_viewport_state_serde() {
        let states = [
            ViewportState::new(),
            ViewportState::pinned(100),
            ViewportState {
                offset_from_bottom: 50,
                is_pinned: true,
                new_lines_since_pin: 25,
            },
        ];

        for state in states {
            let serialized = bincode::serialize(&state).unwrap();
            let deserialized: ViewportState = bincode::deserialize(&serialized).unwrap();
            assert_eq!(state, deserialized);
        }
    }

    // ==================== ReplyMessage Tests ====================

    #[test]
    fn test_reply_message_by_id() {
        let pane_id = Uuid::new_v4();
        let msg = ReplyMessage::by_id(pane_id, "hello world");

        assert_eq!(msg.target, PaneTarget::Id(pane_id));
        assert_eq!(msg.content, "hello world");
    }

    #[test]
    fn test_reply_message_by_name() {
        let msg = ReplyMessage::by_name("worker-3", "use async");

        assert_eq!(msg.target, PaneTarget::Name("worker-3".to_string()));
        assert_eq!(msg.content, "use async");
    }

    #[test]
    fn test_reply_message_clone() {
        let msg = ReplyMessage::by_name("test", "content");
        let cloned = msg.clone();
        assert_eq!(msg, cloned);
    }

    #[test]
    fn test_reply_message_serde() {
        let msg = ReplyMessage::by_id(Uuid::new_v4(), "test content");
        let serialized = bincode::serialize(&msg).unwrap();
        let deserialized: ReplyMessage = bincode::deserialize(&serialized).unwrap();
        assert_eq!(msg, deserialized);
    }

    // ==================== PaneTarget Tests ====================

    #[test]
    fn test_pane_target_id() {
        let id = Uuid::new_v4();
        let target = PaneTarget::Id(id);
        assert_eq!(target, PaneTarget::Id(id));
    }

    #[test]
    fn test_pane_target_name() {
        let target = PaneTarget::Name("my-pane".to_string());
        assert_eq!(target, PaneTarget::Name("my-pane".to_string()));
    }

    #[test]
    fn test_pane_target_equality() {
        let id = Uuid::new_v4();
        let target1 = PaneTarget::Id(id);
        let target2 = PaneTarget::Id(id);
        let target3 = PaneTarget::Name("test".to_string());

        assert_eq!(target1, target2);
        assert_ne!(target1, target3);
    }

    #[test]
    fn test_pane_target_serde() {
        let targets = [
            PaneTarget::Id(Uuid::new_v4()),
            PaneTarget::Name("worker-1".to_string()),
        ];

        for target in targets {
            let serialized = bincode::serialize(&target).unwrap();
            let deserialized: PaneTarget = bincode::deserialize(&serialized).unwrap();
            assert_eq!(target, deserialized);
        }
    }

    // ==================== ReplyResult Tests ====================

    #[test]
    fn test_reply_result_creation() {
        let pane_id = Uuid::new_v4();
        let result = ReplyResult {
            pane_id,
            bytes_written: 42,
        };

        assert_eq!(result.pane_id, pane_id);
        assert_eq!(result.bytes_written, 42);
    }

    #[test]
    fn test_reply_result_clone() {
        let result = ReplyResult {
            pane_id: Uuid::new_v4(),
            bytes_written: 100,
        };
        let cloned = result.clone();
        assert_eq!(result, cloned);
    }

    #[test]
    fn test_reply_result_serde() {
        let result = ReplyResult {
            pane_id: Uuid::new_v4(),
            bytes_written: 256,
        };
        let serialized = bincode::serialize(&result).unwrap();
        let deserialized: ReplyResult = bincode::deserialize(&serialized).unwrap();
        assert_eq!(result, deserialized);
    }

    // ==================== WorktreeInfo Tests ====================

    #[test]
    fn test_worktree_info_creation() {
        let wt = WorktreeInfo {
            path: "/path/to/worktree".to_string(),
            branch: Some("feature-1".to_string()),
            is_main: false,
        };

        assert_eq!(wt.path, "/path/to/worktree");
        assert_eq!(wt.branch, Some("feature-1".to_string()));
        assert!(!wt.is_main);
    }

    #[test]
    fn test_worktree_info_main() {
        let wt = WorktreeInfo {
            path: "/path/to/repo".to_string(),
            branch: Some("main".to_string()),
            is_main: true,
        };

        assert!(wt.is_main);
    }

    #[test]
    fn test_worktree_info_no_branch() {
        let wt = WorktreeInfo {
            path: "/path/to/worktree".to_string(),
            branch: None,
            is_main: false,
        };

        assert!(wt.branch.is_none());
    }

    #[test]
    fn test_worktree_info_clone() {
        let wt = WorktreeInfo {
            path: "/path/to/worktree".to_string(),
            branch: Some("main".to_string()),
            is_main: true,
        };

        let cloned = wt.clone();
        assert_eq!(wt, cloned);
    }

    #[test]
    fn test_worktree_info_equality() {
        let wt1 = WorktreeInfo {
            path: "/path/a".to_string(),
            branch: Some("main".to_string()),
            is_main: true,
        };

        let wt2 = WorktreeInfo {
            path: "/path/a".to_string(),
            branch: Some("main".to_string()),
            is_main: true,
        };

        let wt3 = WorktreeInfo {
            path: "/path/b".to_string(),
            branch: Some("main".to_string()),
            is_main: true,
        };

        assert_eq!(wt1, wt2);
        assert_ne!(wt1, wt3);
    }

    #[test]
    fn test_worktree_info_debug() {
        let wt = WorktreeInfo {
            path: "/debug/path".to_string(),
            branch: Some("test".to_string()),
            is_main: false,
        };

        let debug = format!("{:?}", wt);
        assert!(debug.contains("WorktreeInfo"));
        assert!(debug.contains("/debug/path"));
    }

    #[test]
    fn test_worktree_info_serde() {
        let wt = WorktreeInfo {
            path: "/path/to/worktree".to_string(),
            branch: Some("feature".to_string()),
            is_main: false,
        };

        let serialized = bincode::serialize(&wt).unwrap();
        let deserialized: WorktreeInfo = bincode::deserialize(&serialized).unwrap();
        assert_eq!(wt, deserialized);
    }

    #[test]
    fn test_worktree_info_serde_no_branch() {
        let wt = WorktreeInfo {
            path: "/path/to/worktree".to_string(),
            branch: None,
            is_main: true,
        };

        let serialized = bincode::serialize(&wt).unwrap();
        let deserialized: WorktreeInfo = bincode::deserialize(&serialized).unwrap();
        assert_eq!(wt, deserialized);
    }

    #[test]
    fn test_session_info_with_worktree() {
        let mut tags = HashSet::new();
        tags.insert("orchestrator".to_string());

        let session = SessionInfo {
            id: Uuid::new_v4(),
            name: "test".to_string(),
            created_at: 1234567890,
            window_count: 1,
            attached_clients: 0,
            worktree: Some(WorktreeInfo {
                path: "/path/to/repo".to_string(),
                branch: Some("main".to_string()),
                is_main: true,
            }),
            tags,
            metadata: HashMap::new(),
        };

        assert!(session.worktree.is_some());
        assert!(session.has_tag("orchestrator"));
    }

    #[test]
    fn test_session_info_without_worktree() {
        let session = SessionInfo {
            id: Uuid::new_v4(),
            name: "test".to_string(),
            created_at: 1234567890,
            window_count: 1,
            attached_clients: 0,
            worktree: None,
tags: HashSet::new(),
                    metadata: HashMap::new(),
        };

        assert!(session.worktree.is_none());
        assert!(!session.has_tag("orchestrator"));
    }

    #[test]
    fn test_session_info_tags() {
        let mut session = SessionInfo {
            id: Uuid::new_v4(),
            name: "test".to_string(),
            created_at: 0,
            window_count: 0,
            attached_clients: 0,
            worktree: None,
            tags: HashSet::new(),
            metadata: HashMap::new(),
        };

        // Initially no tags
        assert!(!session.has_tag("worker"));
        assert!(session.tags.is_empty());

        // Add a tag
        session.add_tag("worker");
        assert!(session.has_tag("worker"));
        assert_eq!(session.tags.len(), 1);

        // Add another tag
        session.add_tag("evaluator");
        assert!(session.has_tag("evaluator"));
        assert_eq!(session.tags.len(), 2);

        // Remove a tag
        assert!(session.remove_tag("worker"));
        assert!(!session.has_tag("worker"));
        assert_eq!(session.tags.len(), 1);

        // Removing non-existent tag returns false
        assert!(!session.remove_tag("nonexistent"));
    }

    #[test]
    fn test_session_info_tags_clone() {
        let mut tags = HashSet::new();
        tags.insert("tag1".to_string());
        tags.insert("tag2".to_string());

        let session = SessionInfo {
            id: Uuid::new_v4(),
            name: "test".to_string(),
            created_at: 0,
            window_count: 0,
            attached_clients: 0,
            worktree: None,
            tags,
            metadata: HashMap::new(),
        };

        let cloned = session.clone();
        assert_eq!(session.tags, cloned.tags);
    }

    #[test]
    fn test_session_info_with_worktree_serde() {
        let session = SessionInfo {
            id: Uuid::new_v4(),
            name: "test".to_string(),
            created_at: 1234567890,
            window_count: 2,
            attached_clients: 1,
            worktree: Some(WorktreeInfo {
                path: "/path/to/worktree".to_string(),
                branch: Some("feature".to_string()),
                is_main: false,
            }),
tags: HashSet::new(),
                    metadata: HashMap::new(),
        };

        let serialized = bincode::serialize(&session).unwrap();
        let deserialized: SessionInfo = bincode::deserialize(&serialized).unwrap();
        assert_eq!(session, deserialized);
    }

    // ==================== BeadsTask Tests (FEAT-058) ====================

    #[test]
    fn test_beads_task_creation() {
        let task = BeadsTask {
            id: "BUG-042".to_string(),
            title: "Fix login timeout".to_string(),
            priority: 1,
            status: "open".to_string(),
            issue_type: "bug".to_string(),
            assignee: Some("alice@example.com".to_string()),
            labels: vec!["auth".to_string(), "urgent".to_string()],
        };

        assert_eq!(task.id, "BUG-042");
        assert_eq!(task.priority, 1);
        assert!(task.assignee.is_some());
        assert_eq!(task.labels.len(), 2);
    }

    #[test]
    fn test_beads_task_has_label() {
        let task = BeadsTask {
            id: "FEAT-015".to_string(),
            title: "Add dark mode".to_string(),
            priority: 2,
            status: "open".to_string(),
            issue_type: "feature".to_string(),
            assignee: None,
            labels: vec!["UI".to_string(), "enhancement".to_string()],
        };

        assert!(task.has_label("ui")); // Case insensitive
        assert!(task.has_label("UI"));
        assert!(task.has_label("enhancement"));
        assert!(!task.has_label("bug"));
    }

    #[test]
    fn test_beads_task_short_display() {
        let task = BeadsTask {
            id: "BUG-042".to_string(),
            title: "Fix login timeout".to_string(),
            priority: 1,
            status: "open".to_string(),
            issue_type: "bug".to_string(),
            assignee: None,
            labels: vec![],
        };

        let display = task.short_display();
        assert!(display.contains("BUG-042"));
        assert!(display.contains("P1"));
        assert!(display.contains("Fix login timeout"));
    }

    #[test]
    fn test_beads_task_clone() {
        let task = BeadsTask {
            id: "TEST-001".to_string(),
            title: "Test task".to_string(),
            priority: 0,
            status: "open".to_string(),
            issue_type: "test".to_string(),
            assignee: Some("bob".to_string()),
            labels: vec!["test".to_string()],
        };

        let cloned = task.clone();
        assert_eq!(task, cloned);
    }

    #[test]
    fn test_beads_task_serde() {
        let task = BeadsTask {
            id: "FEAT-100".to_string(),
            title: "New feature".to_string(),
            priority: 2,
            status: "in_progress".to_string(),
            issue_type: "feature".to_string(),
            assignee: Some("dev@example.com".to_string()),
            labels: vec!["backend".to_string()],
        };

        let serialized = bincode::serialize(&task).unwrap();
        let deserialized: BeadsTask = bincode::deserialize(&serialized).unwrap();
        assert_eq!(task, deserialized);
    }

    // ==================== BeadsStatus Tests (FEAT-058) ====================

    #[test]
    fn test_beads_status_default() {
        let status = BeadsStatus::default();

        assert!(!status.daemon_available);
        assert_eq!(status.ready_count, 0);
        assert!(status.ready_tasks.is_empty());
        assert!(status.last_refresh.is_none());
        assert!(status.error.is_none());
    }

    #[test]
    fn test_beads_status_unavailable() {
        let status = BeadsStatus::unavailable();

        assert!(!status.daemon_available);
        assert_eq!(status.ready_count, 0);
        assert!(status.error.is_none());
    }

    #[test]
    fn test_beads_status_with_error() {
        let status = BeadsStatus::with_error("Connection refused");

        assert!(!status.daemon_available);
        assert_eq!(status.error, Some("Connection refused".to_string()));
    }

    #[test]
    fn test_beads_status_with_tasks() {
        let tasks = vec![
            BeadsTask {
                id: "BUG-001".to_string(),
                title: "First bug".to_string(),
                priority: 1,
                status: "open".to_string(),
                issue_type: "bug".to_string(),
                assignee: None,
                labels: vec![],
            },
            BeadsTask {
                id: "FEAT-002".to_string(),
                title: "Second feature".to_string(),
                priority: 2,
                status: "open".to_string(),
                issue_type: "feature".to_string(),
                assignee: None,
                labels: vec![],
            },
        ];

        let status = BeadsStatus::with_tasks(tasks.clone(), 1704067200);

        assert!(status.daemon_available);
        assert_eq!(status.ready_count, 2);
        assert_eq!(status.ready_tasks.len(), 2);
        assert_eq!(status.last_refresh, Some(1704067200));
        assert!(status.error.is_none());
    }

    #[test]
    fn test_beads_status_clone() {
        let status = BeadsStatus::with_tasks(vec![], 1234567890);
        let cloned = status.clone();
        assert_eq!(status, cloned);
    }

    #[test]
    fn test_beads_status_serde() {
        let status = BeadsStatus {
            daemon_available: true,
            ready_count: 5,
            ready_tasks: vec![BeadsTask {
                id: "TEST-001".to_string(),
                title: "Test".to_string(),
                priority: 1,
                status: "open".to_string(),
                issue_type: "test".to_string(),
                assignee: None,
                labels: vec![],
            }],
            last_refresh: Some(1704067200),
            error: None,
        };

        let serialized = bincode::serialize(&status).unwrap();
        let deserialized: BeadsStatus = bincode::deserialize(&serialized).unwrap();
        assert_eq!(status, deserialized);
    }

    #[test]
    fn test_beads_status_equality() {
        let status1 = BeadsStatus::unavailable();
        let status2 = BeadsStatus::unavailable();
        let status3 = BeadsStatus::with_error("error");

        assert_eq!(status1, status2);
        assert_ne!(status1, status3);
    }
}
