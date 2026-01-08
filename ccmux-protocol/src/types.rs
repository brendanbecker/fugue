//! Shared data types for ccmux protocol

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Split direction for creating panes
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum SplitDirection {
    Horizontal,
    Vertical,
}

/// Session information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    pub id: Uuid,
    pub name: String,
    pub created_at: u64, // Unix timestamp
    pub window_count: usize,
    pub attached_clients: usize,
}

/// Window information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowInfo {
    pub id: Uuid,
    pub session_id: Uuid,
    pub name: String,
    pub index: usize,
    pub pane_count: usize,
    pub active_pane_id: Option<Uuid>,
}

/// Pane information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaneInfo {
    pub id: Uuid,
    pub window_id: Uuid,
    pub index: usize,
    pub cols: u16,
    pub rows: u16,
    pub state: PaneState,
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
