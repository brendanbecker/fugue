//! Pane widget for terminal rendering
//!
//! Provides terminal emulation using tui-term and vt100 for rendering
//! PTY output in ratatui panes.

// Allow unused code that's part of the public API for future features
#![allow(dead_code)]

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders, Widget};
use tui_term::vt100::{Parser, Screen};
use tui_term::widget::PseudoTerminal;
use uuid::Uuid;

use ccmux_protocol::{ClaudeActivity, PaneState};

/// Pane focus state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FocusState {
    /// Pane is not focused
    #[default]
    Unfocused,
    /// Pane is focused and active
    Focused,
    /// Pane is in selection mode (for copy/paste)
    Selecting,
}

/// Terminal pane with VT100 emulation
pub struct Pane {
    /// Unique pane ID
    id: Uuid,
    /// VT100 parser for terminal emulation
    parser: Parser,
    /// Current pane title
    title: Option<String>,
    /// Current working directory
    cwd: Option<String>,
    /// Focus state
    focus_state: FocusState,
    /// Pane state (Normal, Claude, Exited)
    pane_state: PaneState,
    /// Scrollback offset (0 = bottom/live, positive = scrolled up)
    scroll_offset: usize,
    /// Whether to show scrollbar
    show_scrollbar: bool,
}

impl Pane {
    /// Create a new pane with given dimensions
    pub fn new(id: Uuid, rows: u16, cols: u16) -> Self {
        Self {
            id,
            parser: Parser::new(rows, cols, 1000), // 1000 lines of scrollback
            title: None,
            cwd: None,
            focus_state: FocusState::Unfocused,
            pane_state: PaneState::Normal,
            scroll_offset: 0,
            show_scrollbar: true,
        }
    }

    /// Get pane ID
    pub fn id(&self) -> Uuid {
        self.id
    }

    /// Get pane title
    pub fn title(&self) -> Option<&str> {
        self.title.as_deref()
    }

    /// Set pane title
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

    /// Get focus state
    pub fn focus_state(&self) -> FocusState {
        self.focus_state
    }

    /// Set focus state
    pub fn set_focus_state(&mut self, state: FocusState) {
        self.focus_state = state;
    }

    /// Check if pane is focused
    pub fn is_focused(&self) -> bool {
        self.focus_state == FocusState::Focused
    }

    /// Get pane state
    pub fn pane_state(&self) -> &PaneState {
        &self.pane_state
    }

    /// Set pane state
    pub fn set_pane_state(&mut self, state: PaneState) {
        self.pane_state = state;
    }

    /// Get Claude activity if in Claude state
    pub fn claude_activity(&self) -> Option<&ClaudeActivity> {
        match &self.pane_state {
            PaneState::Claude(state) => Some(&state.activity),
            _ => None,
        }
    }

    /// Process output data through the terminal parser
    pub fn process_output(&mut self, data: &[u8]) {
        self.parser.process(data);
        // Reset scroll to bottom when new output arrives
        self.scroll_offset = 0;
    }

    /// Resize the terminal
    pub fn resize(&mut self, rows: u16, cols: u16) {
        self.parser.set_size(rows, cols);
    }

    /// Get current terminal size (rows, cols)
    pub fn size(&self) -> (u16, u16) {
        let screen = self.parser.screen();
        (screen.size().0, screen.size().1)
    }

    /// Scroll up by given number of lines
    pub fn scroll_up(&mut self, lines: usize) {
        let max_scroll = self.parser.screen().scrollback();
        self.scroll_offset = (self.scroll_offset + lines).min(max_scroll);
    }

    /// Scroll down by given number of lines
    pub fn scroll_down(&mut self, lines: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(lines);
    }

    /// Scroll to top of scrollback
    pub fn scroll_to_top(&mut self) {
        self.scroll_offset = self.parser.screen().scrollback();
    }

    /// Scroll to bottom (live view)
    pub fn scroll_to_bottom(&mut self) {
        self.scroll_offset = 0;
    }

    /// Check if scrolled (not at bottom)
    pub fn is_scrolled(&self) -> bool {
        self.scroll_offset > 0
    }

    /// Get scroll position as percentage (0.0 = bottom, 1.0 = top)
    pub fn scroll_percentage(&self) -> f32 {
        let max = self.parser.screen().scrollback();
        if max == 0 {
            0.0
        } else {
            self.scroll_offset as f32 / max as f32
        }
    }

    /// Toggle scrollbar visibility
    pub fn toggle_scrollbar(&mut self) {
        self.show_scrollbar = !self.show_scrollbar;
    }

    /// Get the underlying screen for rendering
    pub fn screen(&self) -> &Screen {
        self.parser.screen()
    }

    /// Build the title string for display
    pub fn display_title(&self) -> String {
        let base_title = self.title.as_deref().unwrap_or("pane");

        match &self.pane_state {
            PaneState::Normal => base_title.to_string(),
            PaneState::Claude(state) => {
                let activity = match state.activity {
                    ClaudeActivity::Idle => "Idle",
                    ClaudeActivity::Thinking => "Thinking...",
                    ClaudeActivity::Coding => "Coding",
                    ClaudeActivity::ToolUse => "Tool Use",
                    ClaudeActivity::AwaitingConfirmation => "Confirm?",
                };
                format!("{} [{}]", base_title, activity)
            }
            PaneState::Exited { code } => {
                let code_str = code.map(|c| c.to_string()).unwrap_or_else(|| "?".to_string());
                format!("{} [Exit:{}]", base_title, code_str)
            }
        }
    }
}

/// Widget state for pane rendering
pub struct PaneWidgetState<'a> {
    pane: &'a Pane,
    tick_count: u64,
}

impl<'a> PaneWidgetState<'a> {
    pub fn new(pane: &'a Pane, tick_count: u64) -> Self {
        Self { pane, tick_count }
    }
}

/// Pane widget for rendering
pub struct PaneWidget {
    /// Block for border styling
    block: Option<Block<'static>>,
    /// Whether to show Claude state indicator
    show_indicator: bool,
}

impl PaneWidget {
    pub fn new() -> Self {
        Self {
            block: None,
            show_indicator: true,
        }
    }

    /// Set the block (border) for this pane
    pub fn block(mut self, block: Block<'static>) -> Self {
        self.block = Some(block);
        self
    }

    /// Set whether to show Claude indicator
    pub fn show_indicator(mut self, show: bool) -> Self {
        self.show_indicator = show;
        self
    }

    /// Build the border style based on focus state
    fn border_style(focus_state: FocusState) -> Style {
        match focus_state {
            FocusState::Unfocused => Style::default().fg(Color::DarkGray),
            FocusState::Focused => Style::default().fg(Color::Cyan),
            FocusState::Selecting => Style::default().fg(Color::Yellow),
        }
    }

    /// Get the border block for a pane
    pub fn create_block(pane: &Pane) -> Block<'static> {
        let title = pane.display_title();
        let border_style = Self::border_style(pane.focus_state());

        let mut block = Block::default()
            .borders(Borders::ALL)
            .title(title)
            .border_style(border_style);

        // Add scroll indicator if scrolled
        if pane.is_scrolled() {
            let scroll_indicator = format!(" [{}↑] ", pane.scroll_offset);
            block = block.title_bottom(scroll_indicator);
        }

        block
    }
}

impl Default for PaneWidget {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget for PaneWidget {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Without state, we can only render an empty block
        let block = self.block.unwrap_or_else(|| {
            Block::default()
                .borders(Borders::ALL)
                .title("Pane")
        });
        block.render(area, buf);
    }
}

/// Render a pane with its terminal content
pub fn render_pane(pane: &Pane, area: Rect, buf: &mut Buffer, tick_count: u64) {
    // Create the border block
    let block = PaneWidget::create_block(pane);
    let inner = block.inner(area);

    // Render the block
    block.render(area, buf);

    // Render the terminal content
    if inner.width > 0 && inner.height > 0 {
        let pseudo_term = PseudoTerminal::new(pane.screen())
            .style(Style::default().fg(Color::White).bg(Color::Black));

        pseudo_term.render(inner, buf);

        // Render Claude state indicator if applicable
        if let Some(activity) = pane.claude_activity() {
            render_claude_indicator(activity, inner, buf, tick_count);
        }
    }
}

/// Render Claude state indicator in the pane
fn render_claude_indicator(activity: &ClaudeActivity, area: Rect, buf: &mut Buffer, tick_count: u64) {
    // Small indicator in top-right corner
    if area.width < 5 || area.height < 1 {
        return;
    }

    let indicator = match activity {
        ClaudeActivity::Idle => "[ ]",
        ClaudeActivity::Thinking => {
            let frames = ["[.  ]", "[.. ]", "[...]", "[ ..]", "[  .]", "[   ]"];
            frames[(tick_count / 3) as usize % frames.len()]
        }
        ClaudeActivity::Coding => "[>]",
        ClaudeActivity::ToolUse => "[*]",
        ClaudeActivity::AwaitingConfirmation => "[?]",
    };

    let style = match activity {
        ClaudeActivity::Idle => Style::default().fg(Color::DarkGray),
        ClaudeActivity::Thinking => Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        ClaudeActivity::Coding => Style::default().fg(Color::Green),
        ClaudeActivity::ToolUse => Style::default().fg(Color::Blue),
        ClaudeActivity::AwaitingConfirmation => Style::default().fg(Color::Magenta),
    };

    let x = area.x + area.width.saturating_sub(indicator.len() as u16 + 1);
    let y = area.y;

    for (i, ch) in indicator.chars().enumerate() {
        let cell_x = x + (i as u16);
        if cell_x < area.x + area.width {
            if let Some(cell) = buf.cell_mut((cell_x, y)) {
                cell.set_char(ch).set_style(style);
            }
        }
    }
}

/// Collection of panes
#[derive(Default)]
pub struct PaneManager {
    panes: std::collections::HashMap<Uuid, Pane>,
    active_pane_id: Option<Uuid>,
}

impl PaneManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a new pane
    pub fn add_pane(&mut self, id: Uuid, rows: u16, cols: u16) {
        let pane = Pane::new(id, rows, cols);
        self.panes.insert(id, pane);

        // Set as active if it's the first pane
        if self.active_pane_id.is_none() {
            self.active_pane_id = Some(id);
        }
    }

    /// Remove a pane
    pub fn remove_pane(&mut self, id: Uuid) -> Option<Pane> {
        let pane = self.panes.remove(&id);

        // Update active pane if needed
        if self.active_pane_id == Some(id) {
            self.active_pane_id = self.panes.keys().next().copied();
        }

        pane
    }

    /// Get a pane by ID
    pub fn get(&self, id: Uuid) -> Option<&Pane> {
        self.panes.get(&id)
    }

    /// Get a mutable pane by ID
    pub fn get_mut(&mut self, id: Uuid) -> Option<&mut Pane> {
        self.panes.get_mut(&id)
    }

    /// Get the active pane
    pub fn active_pane(&self) -> Option<&Pane> {
        self.active_pane_id.and_then(|id| self.panes.get(&id))
    }

    /// Get the active pane mutably
    pub fn active_pane_mut(&mut self) -> Option<&mut Pane> {
        self.active_pane_id.and_then(|id| self.panes.get_mut(&id))
    }

    /// Get active pane ID
    pub fn active_pane_id(&self) -> Option<Uuid> {
        self.active_pane_id
    }

    /// Set the active pane
    pub fn set_active(&mut self, id: Uuid) {
        if self.panes.contains_key(&id) {
            // Update focus states
            if let Some(old_id) = self.active_pane_id {
                if let Some(pane) = self.panes.get_mut(&old_id) {
                    pane.set_focus_state(FocusState::Unfocused);
                }
            }
            if let Some(pane) = self.panes.get_mut(&id) {
                pane.set_focus_state(FocusState::Focused);
            }
            self.active_pane_id = Some(id);
        }
    }

    /// Process output for a pane
    pub fn process_output(&mut self, id: Uuid, data: &[u8]) {
        if let Some(pane) = self.panes.get_mut(&id) {
            pane.process_output(data);
        }
    }

    /// Resize a pane
    pub fn resize_pane(&mut self, id: Uuid, rows: u16, cols: u16) {
        if let Some(pane) = self.panes.get_mut(&id) {
            pane.resize(rows, cols);
        }
    }

    /// Update pane state
    pub fn update_pane_state(&mut self, id: Uuid, state: PaneState) {
        if let Some(pane) = self.panes.get_mut(&id) {
            pane.set_pane_state(state);
        }
    }

    /// Get all pane IDs
    pub fn pane_ids(&self) -> Vec<Uuid> {
        self.panes.keys().copied().collect()
    }

    /// Get number of panes
    pub fn count(&self) -> usize {
        self.panes.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.panes.is_empty()
    }

    /// Iterate over all panes
    pub fn iter(&self) -> impl Iterator<Item = (&Uuid, &Pane)> {
        self.panes.iter()
    }

    /// Iterate over all panes mutably
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&Uuid, &mut Pane)> {
        self.panes.iter_mut()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pane_creation() {
        let id = Uuid::new_v4();
        let pane = Pane::new(id, 24, 80);

        assert_eq!(pane.id(), id);
        assert_eq!(pane.focus_state(), FocusState::Unfocused);
        assert!(!pane.is_focused());
    }

    #[test]
    fn test_pane_title() {
        let id = Uuid::new_v4();
        let mut pane = Pane::new(id, 24, 80);

        assert!(pane.title().is_none());

        pane.set_title(Some("test".to_string()));
        assert_eq!(pane.title(), Some("test"));
    }

    #[test]
    fn test_pane_focus() {
        let id = Uuid::new_v4();
        let mut pane = Pane::new(id, 24, 80);

        pane.set_focus_state(FocusState::Focused);
        assert!(pane.is_focused());
        assert_eq!(pane.focus_state(), FocusState::Focused);

        pane.set_focus_state(FocusState::Unfocused);
        assert!(!pane.is_focused());
    }

    #[test]
    fn test_pane_output() {
        let id = Uuid::new_v4();
        let mut pane = Pane::new(id, 24, 80);

        pane.process_output(b"Hello, World!");
        // Output was processed (we can't easily verify content)
    }

    #[test]
    fn test_pane_resize() {
        let id = Uuid::new_v4();
        let mut pane = Pane::new(id, 24, 80);

        pane.resize(40, 120);
        let (rows, cols) = pane.size();
        assert_eq!(rows, 40);
        assert_eq!(cols, 120);
    }

    #[test]
    fn test_pane_scrolling() {
        let id = Uuid::new_v4();
        let mut pane = Pane::new(id, 24, 80);

        assert!(!pane.is_scrolled());
        assert_eq!(pane.scroll_offset, 0);

        pane.scroll_up(10);
        // Note: scrollback is empty, so scroll_up may not change offset
    }

    #[test]
    fn test_display_title_normal() {
        let id = Uuid::new_v4();
        let mut pane = Pane::new(id, 24, 80);
        pane.set_title(Some("bash".to_string()));

        assert_eq!(pane.display_title(), "bash");
    }

    #[test]
    fn test_display_title_claude() {
        let id = Uuid::new_v4();
        let mut pane = Pane::new(id, 24, 80);
        pane.set_title(Some("claude".to_string()));
        pane.set_pane_state(PaneState::Claude(ccmux_protocol::ClaudeState {
            session_id: None,
            activity: ClaudeActivity::Thinking,
            model: None,
            tokens_used: None,
        }));

        assert!(pane.display_title().contains("Thinking"));
    }

    #[test]
    fn test_display_title_exited() {
        let id = Uuid::new_v4();
        let mut pane = Pane::new(id, 24, 80);
        pane.set_title(Some("process".to_string()));
        pane.set_pane_state(PaneState::Exited { code: Some(0) });

        assert!(pane.display_title().contains("Exit:0"));
    }

    #[test]
    fn test_pane_manager_new() {
        let manager = PaneManager::new();
        assert!(manager.is_empty());
        assert_eq!(manager.count(), 0);
    }

    #[test]
    fn test_pane_manager_add() {
        let mut manager = PaneManager::new();
        let id = Uuid::new_v4();

        manager.add_pane(id, 24, 80);

        assert_eq!(manager.count(), 1);
        assert!(manager.get(id).is_some());
        assert_eq!(manager.active_pane_id(), Some(id));
    }

    #[test]
    fn test_pane_manager_remove() {
        let mut manager = PaneManager::new();
        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();

        manager.add_pane(id1, 24, 80);
        manager.add_pane(id2, 24, 80);

        manager.remove_pane(id1);
        assert_eq!(manager.count(), 1);
        assert!(manager.get(id1).is_none());
    }

    #[test]
    fn test_pane_manager_set_active() {
        let mut manager = PaneManager::new();
        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();

        manager.add_pane(id1, 24, 80);
        manager.add_pane(id2, 24, 80);

        manager.set_active(id2);
        assert_eq!(manager.active_pane_id(), Some(id2));

        let pane1 = manager.get(id1).unwrap();
        let pane2 = manager.get(id2).unwrap();
        assert_eq!(pane1.focus_state(), FocusState::Unfocused);
        assert_eq!(pane2.focus_state(), FocusState::Focused);
    }

    #[test]
    fn test_pane_widget_border_style() {
        let unfocused = PaneWidget::border_style(FocusState::Unfocused);
        let focused = PaneWidget::border_style(FocusState::Focused);
        let selecting = PaneWidget::border_style(FocusState::Selecting);

        assert_ne!(unfocused, focused);
        assert_ne!(focused, selecting);
    }
}
