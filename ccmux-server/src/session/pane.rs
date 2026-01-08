use std::time::SystemTime;
use uuid::Uuid;
use ccmux_protocol::{PaneInfo, PaneState, ClaudeState};

/// A terminal pane within a window
#[derive(Debug)]
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
}

impl Pane {
    /// Create a new pane
    pub fn new(window_id: Uuid, index: usize) -> Self {
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

    /// Update Claude state
    pub fn set_claude_state(&mut self, state: ClaudeState) {
        self.state = PaneState::Claude(state);
        self.state_changed_at = SystemTime::now();
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
}
