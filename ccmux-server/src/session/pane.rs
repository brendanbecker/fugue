// Allow unused fields that are reserved for future use
#![allow(dead_code)]

use std::fmt;
use std::time::SystemTime;
use uuid::Uuid;
use vt100::Parser;
use ccmux_protocol::{ClaudeActivity, ClaudeState, PaneInfo, PaneState};
use crate::claude::ClaudeDetector;
use crate::config::SessionType;
use crate::isolation;
use crate::pty::ScrollbackBuffer;

/// A terminal pane within a window
pub struct Pane {
    /// Unique pane identifier
    id: Uuid,
    /// Parent window ID
    window_id: Uuid,
    /// Index within the window
    index: usize,
    /// Terminal dimensions
    cols: u16,
    rows: u16,
    /// Current pane state
    state: PaneState,
    /// Terminal title (from escape sequences)
    title: Option<String>,
    /// Current working directory
    cwd: Option<String>,
    /// When the pane was created
    created_at: SystemTime,
    /// When state last changed
    state_changed_at: SystemTime,
    /// Session type for this pane
    session_type: SessionType,
    /// Scrollback buffer for terminal history
    scrollback: ScrollbackBuffer,
    /// vt100 parser for terminal emulation
    parser: Option<Parser>,
    /// Claude detector for state tracking
    claude_detector: ClaudeDetector,
}

impl fmt::Debug for Pane {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Pane")
            .field("id", &self.id)
            .field("window_id", &self.window_id)
            .field("index", &self.index)
            .field("cols", &self.cols)
            .field("rows", &self.rows)
            .field("state", &self.state)
            .field("title", &self.title)
            .field("cwd", &self.cwd)
            .field("created_at", &self.created_at)
            .field("state_changed_at", &self.state_changed_at)
            .field("session_type", &self.session_type)
            .field("scrollback", &self.scrollback)
            .field("parser", &self.parser.as_ref().map(|_| "Parser { ... }"))
            .field("claude_detector", &self.claude_detector)
            .finish()
    }
}

/// Default scrollback lines when not specified
const DEFAULT_SCROLLBACK_LINES: usize = 1000;

impl Pane {
    /// Create a new pane with default scrollback
    pub fn new(window_id: Uuid, index: usize) -> Self {
        Self::with_scrollback(window_id, index, SessionType::Default, DEFAULT_SCROLLBACK_LINES)
    }

    /// Create a new pane with specific session type and scrollback size
    pub fn with_scrollback(
        window_id: Uuid,
        index: usize,
        session_type: SessionType,
        scrollback_lines: usize,
    ) -> Self {
        let now = SystemTime::now();
        Self {
            id: Uuid::new_v4(),
            window_id,
            index,
            cols: 80,
            rows: 24,
            state: PaneState::Normal,
            title: None,
            cwd: None,
            created_at: now,
            state_changed_at: now,
            session_type,
            scrollback: ScrollbackBuffer::new(scrollback_lines),
            parser: None,
            claude_detector: ClaudeDetector::new(),
        }
    }

    /// Restore a pane from persisted state
    ///
    /// Used during crash recovery to recreate pane with original ID and attributes.
    #[allow(clippy::too_many_arguments)]
    pub fn restore(
        id: Uuid,
        window_id: Uuid,
        index: usize,
        cols: u16,
        rows: u16,
        state: PaneState,
        title: Option<String>,
        cwd: Option<String>,
        created_at: u64,
    ) -> Self {
        let created_at = SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(created_at);
        // If restoring a Claude pane, pre-mark the detector
        let mut claude_detector = ClaudeDetector::new();
        if matches!(state, PaneState::Claude(_)) {
            claude_detector.mark_as_claude();
        }
        Self {
            id,
            window_id,
            index,
            cols,
            rows,
            state,
            title,
            cwd,
            created_at,
            state_changed_at: created_at,
            session_type: SessionType::Default,
            scrollback: ScrollbackBuffer::new(DEFAULT_SCROLLBACK_LINES),
            parser: None,
            claude_detector,
        }
    }

    /// Get pane ID
    pub fn id(&self) -> Uuid {
        self.id
    }

    /// Get window ID
    pub fn window_id(&self) -> Uuid {
        self.window_id
    }

    /// Get pane index
    pub fn index(&self) -> usize {
        self.index
    }

    /// Set pane index
    pub fn set_index(&mut self, index: usize) {
        self.index = index;
    }

    /// Get dimensions
    pub fn dimensions(&self) -> (u16, u16) {
        (self.cols, self.rows)
    }

    /// Resize the pane
    pub fn resize(&mut self, cols: u16, rows: u16) {
        self.cols = cols;
        self.rows = rows;
        if let Some(parser) = &mut self.parser {
            parser.set_size(rows, cols);
        }
    }

    /// Get current state
    pub fn state(&self) -> &PaneState {
        &self.state
    }

    /// Set state
    pub fn set_state(&mut self, state: PaneState) {
        self.state = state;
        self.state_changed_at = SystemTime::now();
    }

    /// Check if this is a Claude pane
    pub fn is_claude(&self) -> bool {
        matches!(self.state, PaneState::Claude(_))
    }

    /// Get Claude state if this is a Claude pane
    pub fn claude_state(&self) -> Option<&ClaudeState> {
        match &self.state {
            PaneState::Claude(state) => Some(state),
            _ => None,
        }
    }

    /// Check if pane is awaiting user input (AwaitingConfirmation or Idle state)
    ///
    /// Returns true if:
    /// - This is a Claude pane in AwaitingConfirmation state
    /// - This is a Claude pane in Idle state (also waiting for input)
    pub fn is_awaiting_input(&self) -> bool {
        match &self.state {
            PaneState::Claude(state) => matches!(
                state.activity,
                ClaudeActivity::AwaitingConfirmation | ClaudeActivity::Idle
            ),
            _ => false,
        }
    }

    /// Check specifically if pane is awaiting confirmation (tool use approval, etc.)
    pub fn is_awaiting_confirmation(&self) -> bool {
        match &self.state {
            PaneState::Claude(state) => {
                matches!(state.activity, ClaudeActivity::AwaitingConfirmation)
            }
            _ => false,
        }
    }

    /// Update Claude state
    pub fn set_claude_state(&mut self, state: ClaudeState) {
        self.state = PaneState::Claude(state);
        self.state_changed_at = SystemTime::now();
        // Also mark the detector
        self.claude_detector.mark_as_claude();
    }

    /// Get reference to Claude detector
    pub fn claude_detector(&self) -> &ClaudeDetector {
        &self.claude_detector
    }

    /// Get mutable reference to Claude detector
    pub fn claude_detector_mut(&mut self) -> &mut ClaudeDetector {
        &mut self.claude_detector
    }

    /// Mark this pane as running Claude Code
    ///
    /// Call this when Claude is started via a known command.
    pub fn mark_as_claude(&mut self) {
        self.claude_detector.mark_as_claude();
        if let Some(state) = self.claude_detector.state() {
            self.state = PaneState::Claude(state);
            self.state_changed_at = SystemTime::now();
        }
    }

    /// Reset Claude detection state
    ///
    /// Call this when the process exits or restarts.
    pub fn reset_claude_detection(&mut self) {
        self.claude_detector.reset();
        self.state = PaneState::Normal;
        self.state_changed_at = SystemTime::now();
    }

    /// Clean up isolation directory for this pane
    ///
    /// Call this when a Claude pane is closed or the process exits.
    /// Safe to call on non-Claude panes (no-op).
    pub fn cleanup_isolation(&self) {
        if self.claude_detector.is_claude() || self.is_claude() {
            if let Err(e) = isolation::cleanup_config_dir(self.id) {
                tracing::warn!(
                    "Failed to cleanup isolation dir for pane {}: {}",
                    self.id, e
                );
            }
        }
    }

    /// Ensure isolation directory exists for this pane
    ///
    /// Call this when Claude is detected in a pane to set up isolation.
    /// Returns the path to the isolation directory if successful.
    pub fn ensure_isolation(&self) -> Option<std::path::PathBuf> {
        match isolation::ensure_config_dir(self.id) {
            Ok(dir) => Some(dir),
            Err(e) => {
                tracing::warn!(
                    "Failed to create isolation dir for pane {}: {}",
                    self.id, e
                );
                None
            }
        }
    }

    /// Get the isolation config directory path for this pane
    pub fn isolation_config_dir(&self) -> std::path::PathBuf {
        isolation::pane_config_dir(self.id)
    }

    /// Get title
    pub fn title(&self) -> Option<&str> {
        self.title.as_deref()
    }

    /// Set title
    pub fn set_title(&mut self, title: Option<String>) {
        self.title = title;
    }

    /// Get current working directory
    pub fn cwd(&self) -> Option<&str> {
        self.cwd.as_deref()
    }

    /// Set current working directory
    pub fn set_cwd(&mut self, cwd: Option<String>) {
        self.cwd = cwd;
    }

    /// Get session type
    pub fn session_type(&self) -> SessionType {
        self.session_type
    }

    /// Get reference to scrollback buffer
    pub fn scrollback(&self) -> &ScrollbackBuffer {
        &self.scrollback
    }

    /// Get mutable reference to scrollback buffer
    pub fn scrollback_mut(&mut self) -> &mut ScrollbackBuffer {
        &mut self.scrollback
    }

    /// Push output to scrollback buffer
    pub fn push_output(&mut self, data: &[u8]) {
        self.scrollback.push_bytes(data);
    }

    /// Get scrollback line count
    pub fn scrollback_lines(&self) -> usize {
        self.scrollback.len()
    }

    /// Get scrollback memory usage in bytes
    pub fn scrollback_bytes(&self) -> usize {
        self.scrollback.total_bytes()
    }

    /// Initialize the vt100 parser with current dimensions
    pub fn init_parser(&mut self) {
        self.parser = Some(Parser::new(self.rows, self.cols, 0));
    }

    /// Process terminal output through the parser
    ///
    /// Returns `Some(ClaudeState)` if Claude state changed, `None` otherwise.
    pub fn process(&mut self, data: &[u8]) -> Option<ClaudeState> {
        if let Some(parser) = &mut self.parser {
            parser.process(data);
        }
        // Also push to scrollback
        self.scrollback.push_bytes(data);

        // Analyze output for Claude state changes
        let text = String::from_utf8_lossy(data);
        if let Some(_activity) = self.claude_detector.analyze(&text) {
            // State changed - update pane state and return new state
            if let Some(claude_state) = self.claude_detector.state() {
                self.state = PaneState::Claude(claude_state.clone());
                self.state_changed_at = SystemTime::now();
                return Some(claude_state);
            }
        }
        None
    }

    /// Get current screen contents
    pub fn screen(&self) -> Option<&vt100::Screen> {
        self.parser.as_ref().map(|p| p.screen())
    }

    /// Check if parser is initialized
    pub fn has_parser(&self) -> bool {
        self.parser.is_some()
    }

    /// Get creation timestamp as Unix time
    pub fn created_at_unix(&self) -> u64 {
        self.created_at
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    }

    /// Convert to protocol PaneInfo
    pub fn to_info(&self) -> PaneInfo {
        PaneInfo {
            id: self.id,
            window_id: self.window_id,
            index: self.index,
            cols: self.cols,
            rows: self.rows,
            state: self.state.clone(),
            title: self.title.clone(),
            cwd: self.cwd.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ccmux_protocol::ClaudeActivity;

    #[test]
    fn test_pane_creation() {
        let window_id = Uuid::new_v4();
        let pane = Pane::new(window_id, 0);

        assert_eq!(pane.window_id(), window_id);
        assert_eq!(pane.index(), 0);
        assert_eq!(pane.dimensions(), (80, 24));
        assert!(!pane.is_claude());
    }

    #[test]
    fn test_pane_resize() {
        let window_id = Uuid::new_v4();
        let mut pane = Pane::new(window_id, 0);

        pane.resize(120, 40);
        assert_eq!(pane.dimensions(), (120, 40));
    }

    #[test]
    fn test_pane_claude_state() {
        let window_id = Uuid::new_v4();
        let mut pane = Pane::new(window_id, 0);

        assert!(!pane.is_claude());

        pane.set_claude_state(ClaudeState::default());
        assert!(pane.is_claude());
        assert!(pane.claude_state().is_some());
    }

    #[test]
    fn test_pane_to_info() {
        let window_id = Uuid::new_v4();
        let pane = Pane::new(window_id, 0);
        let info = pane.to_info();

        assert_eq!(info.id, pane.id());
        assert_eq!(info.window_id, window_id);
    }

    #[test]
    fn test_pane_id_is_unique() {
        let window_id = Uuid::new_v4();
        let pane1 = Pane::new(window_id, 0);
        let pane2 = Pane::new(window_id, 1);

        assert_ne!(pane1.id(), pane2.id());
    }

    #[test]
    fn test_pane_set_index() {
        let window_id = Uuid::new_v4();
        let mut pane = Pane::new(window_id, 0);

        assert_eq!(pane.index(), 0);
        pane.set_index(5);
        assert_eq!(pane.index(), 5);
    }

    #[test]
    fn test_pane_state_getter_setter() {
        let window_id = Uuid::new_v4();
        let mut pane = Pane::new(window_id, 0);

        assert!(matches!(pane.state(), PaneState::Normal));

        pane.set_state(PaneState::Exited { code: Some(0) });
        assert!(matches!(pane.state(), PaneState::Exited { code: Some(0) }));
    }

    #[test]
    fn test_pane_title_getter_setter() {
        let window_id = Uuid::new_v4();
        let mut pane = Pane::new(window_id, 0);

        assert!(pane.title().is_none());

        pane.set_title(Some("my-title".to_string()));
        assert_eq!(pane.title(), Some("my-title"));

        pane.set_title(None);
        assert!(pane.title().is_none());
    }

    #[test]
    fn test_pane_cwd_getter_setter() {
        let window_id = Uuid::new_v4();
        let mut pane = Pane::new(window_id, 0);

        assert!(pane.cwd().is_none());

        pane.set_cwd(Some("/home/user".to_string()));
        assert_eq!(pane.cwd(), Some("/home/user"));

        pane.set_cwd(None);
        assert!(pane.cwd().is_none());
    }

    #[test]
    fn test_pane_claude_state_none_when_not_claude() {
        let window_id = Uuid::new_v4();
        let pane = Pane::new(window_id, 0);

        assert!(pane.claude_state().is_none());
    }

    #[test]
    fn test_pane_claude_state_with_activity() {
        let window_id = Uuid::new_v4();
        let mut pane = Pane::new(window_id, 0);

        let state = ClaudeState {
            session_id: Some("test-session".to_string()),
            activity: ClaudeActivity::Thinking,
            model: Some("claude-3-opus".to_string()),
            tokens_used: Some(5000),
        };

        pane.set_claude_state(state.clone());
        let claude_state = pane.claude_state().unwrap();
        assert_eq!(claude_state.activity, ClaudeActivity::Thinking);
        assert_eq!(claude_state.session_id, Some("test-session".to_string()));
        assert_eq!(claude_state.tokens_used, Some(5000));
    }

    #[test]
    fn test_pane_resize_to_zero() {
        let window_id = Uuid::new_v4();
        let mut pane = Pane::new(window_id, 0);

        pane.resize(0, 0);
        assert_eq!(pane.dimensions(), (0, 0));
    }

    #[test]
    fn test_pane_resize_large_dimensions() {
        let window_id = Uuid::new_v4();
        let mut pane = Pane::new(window_id, 0);

        pane.resize(u16::MAX, u16::MAX);
        assert_eq!(pane.dimensions(), (u16::MAX, u16::MAX));
    }

    #[test]
    fn test_pane_to_info_includes_all_fields() {
        let window_id = Uuid::new_v4();
        let mut pane = Pane::new(window_id, 3);
        pane.resize(100, 50);
        pane.set_title(Some("test-title".to_string()));
        pane.set_cwd(Some("/tmp".to_string()));
        pane.set_state(PaneState::Exited { code: Some(1) });

        let info = pane.to_info();

        assert_eq!(info.id, pane.id());
        assert_eq!(info.window_id, window_id);
        assert_eq!(info.index, 3);
        assert_eq!(info.cols, 100);
        assert_eq!(info.rows, 50);
        assert_eq!(info.title, Some("test-title".to_string()));
        assert_eq!(info.cwd, Some("/tmp".to_string()));
        assert!(matches!(info.state, PaneState::Exited { code: Some(1) }));
    }

    #[test]
    fn test_pane_debug_format() {
        let window_id = Uuid::new_v4();
        let pane = Pane::new(window_id, 0);

        let debug_str = format!("{:?}", pane);
        assert!(debug_str.contains("Pane"));
        assert!(debug_str.contains("cols: 80"));
        assert!(debug_str.contains("rows: 24"));
    }

    #[test]
    fn test_pane_state_transition_exited() {
        let window_id = Uuid::new_v4();
        let mut pane = Pane::new(window_id, 0);

        pane.set_state(PaneState::Exited { code: Some(0) });
        assert!(matches!(pane.state(), PaneState::Exited { code: Some(0) }));
        assert!(!pane.is_claude());
    }

    #[test]
    fn test_pane_state_transition_exited_no_code() {
        let window_id = Uuid::new_v4();
        let mut pane = Pane::new(window_id, 0);

        pane.set_state(PaneState::Exited { code: None });
        assert!(matches!(pane.state(), PaneState::Exited { code: None }));
    }

    #[test]
    fn test_pane_multiple_resizes() {
        let window_id = Uuid::new_v4();
        let mut pane = Pane::new(window_id, 0);

        pane.resize(100, 50);
        assert_eq!(pane.dimensions(), (100, 50));

        pane.resize(200, 100);
        assert_eq!(pane.dimensions(), (200, 100));

        pane.resize(80, 24);
        assert_eq!(pane.dimensions(), (80, 24));
    }

    #[test]
    fn test_pane_with_scrollback() {
        let window_id = Uuid::new_v4();
        let pane = Pane::with_scrollback(window_id, 0, SessionType::Orchestrator, 50000);

        assert_eq!(pane.session_type(), SessionType::Orchestrator);
        assert_eq!(pane.scrollback().max_lines(), 50000);
    }

    #[test]
    fn test_pane_default_session_type() {
        let window_id = Uuid::new_v4();
        let pane = Pane::new(window_id, 0);

        assert_eq!(pane.session_type(), SessionType::Default);
    }

    #[test]
    fn test_pane_push_output() {
        let window_id = Uuid::new_v4();
        let mut pane = Pane::new(window_id, 0);

        pane.push_output(b"Hello\nWorld\n");
        assert_eq!(pane.scrollback_lines(), 2);
    }

    #[test]
    fn test_pane_scrollback_access() {
        let window_id = Uuid::new_v4();
        let mut pane = Pane::new(window_id, 0);

        pane.push_output(b"Line 1\nLine 2\n");

        let lines: Vec<_> = pane.scrollback().get_lines().collect();
        assert_eq!(lines, vec!["Line 1", "Line 2"]);
    }

    #[test]
    fn test_pane_scrollback_bytes() {
        let window_id = Uuid::new_v4();
        let mut pane = Pane::new(window_id, 0);

        pane.push_output(b"12345\n");
        assert!(pane.scrollback_bytes() > 0);
    }

    #[test]
    fn test_pane_scrollback_mut() {
        let window_id = Uuid::new_v4();
        let mut pane = Pane::new(window_id, 0);

        pane.scrollback_mut().push_line("Direct push".to_string());
        assert_eq!(pane.scrollback_lines(), 1);
    }

    #[test]
    fn test_pane_worker_small_scrollback() {
        let window_id = Uuid::new_v4();
        let pane = Pane::with_scrollback(window_id, 0, SessionType::Worker, 500);

        assert_eq!(pane.session_type(), SessionType::Worker);
        assert_eq!(pane.scrollback().max_lines(), 500);
    }

    // ==================== Input-Wait Detection Tests ====================

    #[test]
    fn test_pane_is_awaiting_input_normal_pane() {
        let window_id = Uuid::new_v4();
        let pane = Pane::new(window_id, 0);

        // Normal panes are never awaiting input
        assert!(!pane.is_awaiting_input());
        assert!(!pane.is_awaiting_confirmation());
    }

    #[test]
    fn test_pane_is_awaiting_input_idle_claude() {
        let window_id = Uuid::new_v4();
        let mut pane = Pane::new(window_id, 0);

        pane.set_claude_state(ClaudeState {
            session_id: None,
            activity: ClaudeActivity::Idle,
            model: None,
            tokens_used: None,
        });

        // Idle Claude panes are awaiting input
        assert!(pane.is_awaiting_input());
        assert!(!pane.is_awaiting_confirmation());
    }

    #[test]
    fn test_pane_is_awaiting_input_awaiting_confirmation() {
        let window_id = Uuid::new_v4();
        let mut pane = Pane::new(window_id, 0);

        pane.set_claude_state(ClaudeState {
            session_id: None,
            activity: ClaudeActivity::AwaitingConfirmation,
            model: None,
            tokens_used: None,
        });

        // AwaitingConfirmation Claude panes are awaiting input
        assert!(pane.is_awaiting_input());
        assert!(pane.is_awaiting_confirmation());
    }

    #[test]
    fn test_pane_is_awaiting_input_thinking() {
        let window_id = Uuid::new_v4();
        let mut pane = Pane::new(window_id, 0);

        pane.set_claude_state(ClaudeState {
            session_id: None,
            activity: ClaudeActivity::Thinking,
            model: None,
            tokens_used: None,
        });

        // Thinking Claude panes are NOT awaiting input
        assert!(!pane.is_awaiting_input());
        assert!(!pane.is_awaiting_confirmation());
    }

    #[test]
    fn test_pane_is_awaiting_input_coding() {
        let window_id = Uuid::new_v4();
        let mut pane = Pane::new(window_id, 0);

        pane.set_claude_state(ClaudeState {
            session_id: None,
            activity: ClaudeActivity::Coding,
            model: None,
            tokens_used: None,
        });

        // Coding Claude panes are NOT awaiting input
        assert!(!pane.is_awaiting_input());
        assert!(!pane.is_awaiting_confirmation());
    }

    #[test]
    fn test_pane_is_awaiting_input_tool_use() {
        let window_id = Uuid::new_v4();
        let mut pane = Pane::new(window_id, 0);

        pane.set_claude_state(ClaudeState {
            session_id: None,
            activity: ClaudeActivity::ToolUse,
            model: None,
            tokens_used: None,
        });

        // ToolUse Claude panes are NOT awaiting input
        assert!(!pane.is_awaiting_input());
        assert!(!pane.is_awaiting_confirmation());
    }

    #[test]
    fn test_pane_is_awaiting_input_exited_pane() {
        let window_id = Uuid::new_v4();
        let mut pane = Pane::new(window_id, 0);

        pane.set_state(PaneState::Exited { code: Some(0) });

        // Exited panes are never awaiting input
        assert!(!pane.is_awaiting_input());
        assert!(!pane.is_awaiting_confirmation());
    }

    #[test]
    fn test_pane_is_awaiting_input_state_transitions() {
        let window_id = Uuid::new_v4();
        let mut pane = Pane::new(window_id, 0);

        // Start as Claude Idle - awaiting input
        pane.set_claude_state(ClaudeState {
            session_id: None,
            activity: ClaudeActivity::Idle,
            model: None,
            tokens_used: None,
        });
        assert!(pane.is_awaiting_input());

        // Transition to Thinking - NOT awaiting input
        pane.set_claude_state(ClaudeState {
            session_id: None,
            activity: ClaudeActivity::Thinking,
            model: None,
            tokens_used: None,
        });
        assert!(!pane.is_awaiting_input());

        // Transition to AwaitingConfirmation - awaiting input
        pane.set_claude_state(ClaudeState {
            session_id: None,
            activity: ClaudeActivity::AwaitingConfirmation,
            model: None,
            tokens_used: None,
        });
        assert!(pane.is_awaiting_input());
        assert!(pane.is_awaiting_confirmation());

        // Back to Idle - awaiting input
        pane.set_claude_state(ClaudeState {
            session_id: None,
            activity: ClaudeActivity::Idle,
            model: None,
            tokens_used: None,
        });
        assert!(pane.is_awaiting_input());
        assert!(!pane.is_awaiting_confirmation());
    }

    // ==================== vt100 Parser Tests ====================

    #[test]
    fn test_pane_parser_init() {
        let window_id = Uuid::new_v4();
        let mut pane = Pane::new(window_id, 0);
        assert!(!pane.has_parser());

        pane.init_parser();
        assert!(pane.has_parser());
    }

    #[test]
    fn test_pane_process_output() {
        let window_id = Uuid::new_v4();
        let mut pane = Pane::new(window_id, 0);
        pane.init_parser();

        pane.process(b"Hello, World!");

        let screen = pane.screen().unwrap();
        assert!(screen.contents().contains("Hello, World!"));
    }

    #[test]
    fn test_pane_parser_resize() {
        let window_id = Uuid::new_v4();
        let mut pane = Pane::new(window_id, 0);
        pane.init_parser();
        pane.resize(120, 40);

        let screen = pane.screen().unwrap();
        assert_eq!(screen.size(), (40, 120));
    }

    #[test]
    fn test_pane_process_without_parser() {
        let window_id = Uuid::new_v4();
        let mut pane = Pane::new(window_id, 0);

        // Should not panic - just adds to scrollback
        pane.process(b"Test data\n");

        assert!(pane.screen().is_none());
        assert_eq!(pane.scrollback_lines(), 1);
    }

    #[test]
    fn test_pane_process_also_writes_scrollback() {
        let window_id = Uuid::new_v4();
        let mut pane = Pane::new(window_id, 0);
        pane.init_parser();

        pane.process(b"Line 1\nLine 2\n");

        // Verify data went to both parser and scrollback
        let screen = pane.screen().unwrap();
        assert!(screen.contents().contains("Line 1"));
        assert_eq!(pane.scrollback_lines(), 2);
    }

    #[test]
    fn test_pane_resize_without_parser() {
        let window_id = Uuid::new_v4();
        let mut pane = Pane::new(window_id, 0);

        // Should not panic
        pane.resize(120, 40);

        assert_eq!(pane.dimensions(), (120, 40));
        assert!(!pane.has_parser());
    }
}
