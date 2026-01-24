use super::agent::{AgentState, ClaudeActivity};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ==================== Pane State ====================

/// Pane state
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum PaneState {
    /// Normal shell/process
    #[default]
    Normal,
    /// AI agent detected (e.g., Claude, Copilot, Aider)
    Agent(AgentState),
    /// Process exited
    Exited { code: Option<i32> },
    /// Status pane (FEAT-102)
    Status,
}

impl PaneState {
    /// Check if this pane has an active agent
    pub fn is_agent(&self) -> bool {
        matches!(self, PaneState::Agent(_))
    }

    /// Get the agent state if this is an agent pane
    pub fn agent_state(&self) -> Option<AgentState> {
        match self {
            PaneState::Agent(state) => Some(state.clone()),
            _ => None,
        }
    }

    /// Get Claude activity if this is a Claude pane
    pub fn claude_activity(&self) -> Option<ClaudeActivity> {
        match self {
            PaneState::Agent(state) if state.is_claude() => Some(state.activity.clone().into()),
            _ => None,
        }
    }
}

/// Pane stuck/health status (FEAT-073)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PaneStuckStatus {
    /// Pane is healthy/normal
    None,
    /// Pane is slow (warning)
    Slow { duration: u64 },
    /// Pane is stuck (error)
    Stuck { duration: u64, reason: String },
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
    /// Stuck/health status of the pane (FEAT-073)
    pub stuck_status: Option<PaneStuckStatus>,
    /// Arbitrary key-value metadata for the pane (FEAT-076)
    #[serde(default)]
    pub metadata: std::collections::HashMap<String, String>,
    /// Whether this pane is a mirror of another pane (FEAT-062)
    #[serde(default)]
    pub is_mirror: bool,
    /// Source pane ID if this is a mirror pane (FEAT-062)
    #[serde(default)]
    pub mirror_source: Option<Uuid>,
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

/// Target specification for a pane (by ID or name)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PaneTarget {
    /// Target by UUID
    Id(Uuid),
    /// Target by pane name/title
    Name(String),
}

/// Message to send a reply to a pane awaiting input
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReplyMessage {
    /// Target pane (by ID or name)
    pub target: PaneTarget,
    /// Content to send to the pane's stdin
    pub content: String,
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

#[cfg(test)]
mod tests {
    use super::*;

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
    fn test_pane_state_agent() {
        let agent_state = AgentState::new("claude");
        let state = PaneState::Agent(agent_state.clone());

        if let PaneState::Agent(as_) = &state {
            assert_eq!(*as_, agent_state);
        } else {
            panic!("Expected Agent variant");
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
        let agent = PaneState::Agent(AgentState::new("claude"));
        let exited = PaneState::Exited { code: Some(0) };

        assert_eq!(normal1, normal2);
        assert_ne!(normal1, agent);
        assert_ne!(normal1, exited);
        assert_ne!(agent, exited);
    }

    #[test]
    fn test_pane_state_clone() {
        let states = [
            PaneState::Normal,
            PaneState::Agent(AgentState::new("claude")),
            PaneState::Exited { code: Some(42) },
        ];

        for state in states {
            let cloned = state.clone();
            assert_eq!(state, cloned);
        }
    }

    #[test]
    fn test_pane_state_serde() {
        let states = [
            PaneState::Normal,
            PaneState::Agent(AgentState::new("claude")),
            PaneState::Exited { code: Some(0) },
            PaneState::Exited { code: None },
        ];

        for state in states {
            let serialized = bincode::serialize(&state).unwrap();
            let deserialized: PaneState = bincode::deserialize(&serialized).unwrap();
            assert_eq!(state, deserialized);
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
            stuck_status: None,
            metadata: std::collections::HashMap::new(),
            is_mirror: false,
            mirror_source: None,
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
            state: PaneState::Agent(AgentState::new("claude")),
            name: None,
            title: Some("vim".to_string()),
            cwd: Some("/home/user/project".to_string()),
            stuck_status: Some(PaneStuckStatus::Slow { duration: 10 }),
            metadata: std::collections::HashMap::new(),
            is_mirror: false,
            mirror_source: None,
        };

        assert_eq!(pane.id, id);
        assert_eq!(pane.window_id, window_id);
        assert_eq!(pane.index, 2);
        assert_eq!(pane.title, Some("vim".to_string()));
        assert_eq!(pane.cwd, Some("/home/user/project".to_string()));
        assert_eq!(
            pane.stuck_status,
            Some(PaneStuckStatus::Slow { duration: 10 })
        );
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
            stuck_status: None,
            metadata: std::collections::HashMap::new(),
            is_mirror: false,
            mirror_source: None,
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
            stuck_status: None,
            metadata: std::collections::HashMap::new(),
            is_mirror: false,
            mirror_source: None,
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
            stuck_status: None,
            metadata: std::collections::HashMap::new(),
            is_mirror: false,
            mirror_source: None,
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
            stuck_status: None,
            metadata: std::collections::HashMap::new(),
            is_mirror: false,
            mirror_source: None,
        };

        assert_eq!(pane1, pane2);
        assert_ne!(pane1, pane3);
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
            stuck_status: None,
            metadata: std::collections::HashMap::new(),
            is_mirror: false,
            mirror_source: None,
        };

        let serialized = bincode::serialize(&pane).unwrap();
        let deserialized: PaneInfo = bincode::deserialize(&serialized).unwrap();
        assert_eq!(pane, deserialized);
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
}
