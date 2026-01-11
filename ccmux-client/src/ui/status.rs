//! Status bar widget for the application
//!
//! Displays session info, connection status, keybinding hints, and Claude indicators.

// Allow unused code that's part of the public API for future features
#![allow(dead_code)]

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Span;
use ratatui::widgets::Widget;

use ccmux_protocol::{ClaudeActivity, SessionInfo};

/// Connection status for display
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionStatus {
    Disconnected,
    Connecting,
    Connected,
    Reconnecting,
}

impl ConnectionStatus {
    /// Get display text for this status
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Disconnected => "Disconnected",
            Self::Connecting => "Connecting...",
            Self::Connected => "Connected",
            Self::Reconnecting => "Reconnecting...",
        }
    }

    /// Get the style for this status
    pub fn style(&self) -> Style {
        match self {
            Self::Disconnected => Style::default().fg(Color::Red),
            Self::Connecting => Style::default().fg(Color::Yellow),
            Self::Connected => Style::default().fg(Color::Green),
            Self::Reconnecting => Style::default().fg(Color::Yellow),
        }
    }
}

/// Status bar component
pub struct StatusBar {
    /// Current session info
    session: Option<SessionInfo>,
    /// Connection status
    connection_status: ConnectionStatus,
    /// Number of panes
    pane_count: usize,
    /// Active pane name/index
    active_pane: Option<String>,
    /// Claude activity for current pane
    claude_activity: Option<ClaudeActivity>,
    /// Custom message to display (errors, etc.)
    message: Option<(String, Style)>,
    /// Show keybinding hints
    show_hints: bool,
    /// Animation tick counter
    tick_count: u64,
    /// Whether the current pane is in a beads-tracked repo (FEAT-057)
    is_beads_tracked: bool,
}

impl StatusBar {
    /// Create a new status bar
    pub fn new() -> Self {
        Self {
            session: None,
            connection_status: ConnectionStatus::Disconnected,
            pane_count: 0,
            active_pane: None,
            claude_activity: None,
            message: None,
            show_hints: true,
            tick_count: 0,
            is_beads_tracked: false,
        }
    }

    /// Set the current session
    pub fn set_session(&mut self, session: Option<SessionInfo>) {
        self.session = session;
    }

    /// Set connection status
    pub fn set_connection_status(&mut self, status: ConnectionStatus) {
        self.connection_status = status;
    }

    /// Set pane count
    pub fn set_pane_count(&mut self, count: usize) {
        self.pane_count = count;
    }

    /// Set active pane name
    pub fn set_active_pane(&mut self, name: Option<String>) {
        self.active_pane = name;
    }

    /// Set Claude activity
    pub fn set_claude_activity(&mut self, activity: Option<ClaudeActivity>) {
        self.claude_activity = activity;
    }

    /// Set a temporary message
    pub fn set_message(&mut self, message: Option<String>, style: Option<Style>) {
        self.message = message.map(|m| (m, style.unwrap_or_default()));
    }

    /// Set error message
    pub fn set_error(&mut self, error: impl Into<String>) {
        self.message = Some((error.into(), Style::default().fg(Color::Red)));
    }

    /// Clear message
    pub fn clear_message(&mut self) {
        self.message = None;
    }

    /// Toggle keybinding hints
    pub fn toggle_hints(&mut self) {
        self.show_hints = !self.show_hints;
    }

    /// Set beads tracking status (FEAT-057)
    pub fn set_beads_tracked(&mut self, is_tracked: bool) {
        self.is_beads_tracked = is_tracked;
    }

    /// Check if beads tracking is enabled
    pub fn is_beads_tracked(&self) -> bool {
        self.is_beads_tracked
    }

    /// Update tick count for animations
    pub fn tick(&mut self) {
        self.tick_count = self.tick_count.wrapping_add(1);
    }

    /// Build the left section (session info)
    fn left_section(&self) -> Vec<Span<'static>> {
        let mut spans = Vec::new();

        // Session name
        if let Some(ref session) = self.session {
            spans.push(Span::styled(
                format!(" {} ", session.name),
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            ));
            spans.push(Span::raw("|"));
        }

        // Pane info
        if self.pane_count > 0 {
            let pane_text = if let Some(ref name) = self.active_pane {
                format!(" {} ({} panes) ", name, self.pane_count)
            } else {
                format!(" {} panes ", self.pane_count)
            };
            spans.push(Span::styled(pane_text, Style::default().fg(Color::White)));
        }

        // FEAT-057: Beads indicator
        if self.is_beads_tracked {
            spans.push(Span::raw("|"));
            spans.push(Span::styled(
                " beads ",
                Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD),
            ));
        }

        spans
    }

    /// Build the center section (messages, Claude indicator)
    fn center_section(&self) -> Vec<Span<'static>> {
        let mut spans = Vec::new();

        // Priority: message > Claude activity
        if let Some((ref msg, style)) = self.message {
            spans.push(Span::styled(msg.clone(), style));
        } else if let Some(ref activity) = self.claude_activity {
            let (indicator, style) = self.claude_indicator_styled(activity);
            spans.push(Span::styled(indicator, style));
        }

        spans
    }

    /// Build the right section (connection status, hints)
    fn right_section(&self) -> Vec<Span<'static>> {
        let mut spans = Vec::new();

        // Connection status
        spans.push(Span::styled(
            format!(" {} ", self.connection_status.as_str()),
            self.connection_status.style(),
        ));

        // Keybinding hints
        if self.show_hints && self.connection_status == ConnectionStatus::Connected {
            spans.push(Span::raw("|"));
            spans.push(Span::styled(
                " ^b:prefix ^q:quit ",
                Style::default().fg(Color::DarkGray),
            ));
        }

        spans
    }

    /// Get Claude indicator with style
    fn claude_indicator_styled(&self, activity: &ClaudeActivity) -> (String, Style) {
        match activity {
            ClaudeActivity::Idle => (
                "[ ] Idle".to_string(),
                Style::default().fg(Color::DarkGray),
            ),
            ClaudeActivity::Thinking => {
                let frames = ["[.  ]", "[.. ]", "[...]", "[ ..]", "[  .]", "[   ]"];
                let frame = frames[(self.tick_count / 3) as usize % frames.len()];
                (
                    format!("{} Thinking", frame),
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                )
            }
            ClaudeActivity::Coding => (
                "[>] Coding".to_string(),
                Style::default().fg(Color::Green),
            ),
            ClaudeActivity::ToolUse => (
                "[*] Tool Use".to_string(),
                Style::default().fg(Color::Blue),
            ),
            ClaudeActivity::AwaitingConfirmation => (
                "[?] Confirm".to_string(),
                Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD),
            ),
        }
    }
}

impl Default for StatusBar {
    fn default() -> Self {
        Self::new()
    }
}

/// Widget implementation for status bar
pub struct StatusBarWidget<'a> {
    status_bar: &'a StatusBar,
}

impl<'a> StatusBarWidget<'a> {
    pub fn new(status_bar: &'a StatusBar) -> Self {
        Self { status_bar }
    }
}

impl<'a> Widget for StatusBarWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.height < 1 {
            return;
        }

        // Fill background
        let bg_style = Style::default().bg(Color::DarkGray).fg(Color::White);
        for x in area.x..area.x + area.width {
            if let Some(cell) = buf.cell_mut((x, area.y)) {
                cell.set_char(' ').set_style(bg_style);
            }
        }

        // Get sections
        let left_spans = self.status_bar.left_section();
        let center_spans = self.status_bar.center_section();
        let right_spans = self.status_bar.right_section();

        // Render left section
        let mut x = area.x;
        for span in &left_spans {
            for ch in span.content.chars() {
                if x < area.x + area.width {
                    if let Some(cell) = buf.cell_mut((x, area.y)) {
                        cell.set_char(ch).set_style(bg_style.patch(span.style));
                    }
                    x += 1;
                }
            }
        }

        // Calculate right section width
        let right_width: usize = right_spans.iter().map(|s| s.content.len()).sum();
        let right_start = area.x + area.width.saturating_sub(right_width as u16);

        // Render center section (centered between left and right)
        if !center_spans.is_empty() {
            let center_width: usize = center_spans.iter().map(|s| s.content.len()).sum();
            let center_start = area.x + (area.width.saturating_sub(center_width as u16)) / 2;

            if center_start > x && center_start + center_width as u16 <= right_start {
                let mut cx = center_start;
                for span in &center_spans {
                    for ch in span.content.chars() {
                        if cx < right_start {
                            if let Some(cell) = buf.cell_mut((cx, area.y)) {
                                cell.set_char(ch).set_style(bg_style.patch(span.style));
                            }
                            cx += 1;
                        }
                    }
                }
            }
        }

        // Render right section
        let mut rx = right_start;
        for span in &right_spans {
            for ch in span.content.chars() {
                if rx < area.x + area.width {
                    if let Some(cell) = buf.cell_mut((rx, area.y)) {
                        cell.set_char(ch).set_style(bg_style.patch(span.style));
                    }
                    rx += 1;
                }
            }
        }
    }
}

/// Render a status bar directly
pub fn render_status_bar(status_bar: &StatusBar, area: Rect, buf: &mut Buffer) {
    StatusBarWidget::new(status_bar).render(area, buf);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_status_bar_new() {
        let bar = StatusBar::new();
        assert!(bar.session.is_none());
        assert_eq!(bar.connection_status, ConnectionStatus::Disconnected);
        assert_eq!(bar.pane_count, 0);
    }

    #[test]
    fn test_connection_status_display() {
        assert_eq!(ConnectionStatus::Disconnected.as_str(), "Disconnected");
        assert_eq!(ConnectionStatus::Connecting.as_str(), "Connecting...");
        assert_eq!(ConnectionStatus::Connected.as_str(), "Connected");
        assert_eq!(ConnectionStatus::Reconnecting.as_str(), "Reconnecting...");
    }

    #[test]
    fn test_set_session() {
        let mut bar = StatusBar::new();

        let session = SessionInfo {
            id: uuid::Uuid::new_v4(),
            name: "test-session".to_string(),
            created_at: 0,
            window_count: 1,
            attached_clients: 1,
            worktree: None,
            tags: std::collections::HashSet::new(),
            metadata: HashMap::new(),
        };

        bar.set_session(Some(session.clone()));
        assert!(bar.session.is_some());
        assert_eq!(bar.session.as_ref().unwrap().name, "test-session");
    }

    #[test]
    fn test_set_connection_status() {
        let mut bar = StatusBar::new();

        bar.set_connection_status(ConnectionStatus::Connected);
        assert_eq!(bar.connection_status, ConnectionStatus::Connected);

        bar.set_connection_status(ConnectionStatus::Reconnecting);
        assert_eq!(bar.connection_status, ConnectionStatus::Reconnecting);
    }

    #[test]
    fn test_set_pane_count() {
        let mut bar = StatusBar::new();

        bar.set_pane_count(5);
        assert_eq!(bar.pane_count, 5);
    }

    #[test]
    fn test_set_claude_activity() {
        let mut bar = StatusBar::new();

        bar.set_claude_activity(Some(ClaudeActivity::Thinking));
        assert_eq!(bar.claude_activity, Some(ClaudeActivity::Thinking));

        bar.set_claude_activity(None);
        assert!(bar.claude_activity.is_none());
    }

    #[test]
    fn test_set_message() {
        let mut bar = StatusBar::new();

        bar.set_message(Some("Test message".to_string()), None);
        assert!(bar.message.is_some());
        assert_eq!(bar.message.as_ref().unwrap().0, "Test message");

        bar.clear_message();
        assert!(bar.message.is_none());
    }

    #[test]
    fn test_set_error() {
        let mut bar = StatusBar::new();

        bar.set_error("Error occurred");
        assert!(bar.message.is_some());
        assert_eq!(bar.message.as_ref().unwrap().0, "Error occurred");
    }

    #[test]
    fn test_toggle_hints() {
        let mut bar = StatusBar::new();
        assert!(bar.show_hints);

        bar.toggle_hints();
        assert!(!bar.show_hints);

        bar.toggle_hints();
        assert!(bar.show_hints);
    }

    #[test]
    fn test_tick() {
        let mut bar = StatusBar::new();
        assert_eq!(bar.tick_count, 0);

        bar.tick();
        assert_eq!(bar.tick_count, 1);

        bar.tick();
        bar.tick();
        assert_eq!(bar.tick_count, 3);
    }

    #[test]
    fn test_left_section_empty() {
        let bar = StatusBar::new();
        let spans = bar.left_section();
        assert!(spans.is_empty());
    }

    #[test]
    fn test_left_section_with_session() {
        let mut bar = StatusBar::new();
        bar.set_session(Some(SessionInfo {
            id: uuid::Uuid::new_v4(),
            name: "my-session".to_string(),
            created_at: 0,
            window_count: 1,
            attached_clients: 1,
            worktree: None,
            tags: std::collections::HashSet::new(),
            metadata: HashMap::new(),
        }));

        let spans = bar.left_section();
        assert!(!spans.is_empty());

        let text: String = spans.iter().map(|s| s.content.as_ref()).collect();
        assert!(text.contains("my-session"));
    }

    #[test]
    fn test_claude_indicator_idle() {
        let bar = StatusBar::new();
        let (text, _) = bar.claude_indicator_styled(&ClaudeActivity::Idle);
        assert!(text.contains("Idle"));
    }

    #[test]
    fn test_claude_indicator_thinking() {
        let mut bar = StatusBar::new();
        bar.tick_count = 0;

        let (text, _) = bar.claude_indicator_styled(&ClaudeActivity::Thinking);
        assert!(text.contains("Thinking"));
    }

    #[test]
    fn test_status_bar_widget_render() {
        let bar = StatusBar::new();
        let widget = StatusBarWidget::new(&bar);

        let area = Rect::new(0, 0, 80, 1);
        let mut buf = Buffer::empty(area);

        widget.render(area, &mut buf);
        // Widget rendered successfully
    }

    #[test]
    fn test_render_status_bar() {
        let mut bar = StatusBar::new();
        bar.set_connection_status(ConnectionStatus::Connected);
        bar.set_pane_count(3);

        let area = Rect::new(0, 0, 100, 1);
        let mut buf = Buffer::empty(area);

        render_status_bar(&bar, area, &mut buf);
        // Should not panic
    }

    // ==================== FEAT-057 Beads Indicator Tests ====================

    #[test]
    fn test_beads_tracked_default_false() {
        let bar = StatusBar::new();
        assert!(!bar.is_beads_tracked());
    }

    #[test]
    fn test_set_beads_tracked() {
        let mut bar = StatusBar::new();

        bar.set_beads_tracked(true);
        assert!(bar.is_beads_tracked());

        bar.set_beads_tracked(false);
        assert!(!bar.is_beads_tracked());
    }

    #[test]
    fn test_left_section_with_beads() {
        let mut bar = StatusBar::new();
        bar.set_session(Some(SessionInfo {
            id: uuid::Uuid::new_v4(),
            name: "my-session".to_string(),
            created_at: 0,
            window_count: 1,
            attached_clients: 1,
            worktree: None,
            tags: std::collections::HashSet::new(),
            metadata: HashMap::new(),
        }));
        bar.set_beads_tracked(true);

        let spans = bar.left_section();
        let text: String = spans.iter().map(|s| s.content.as_ref()).collect();

        assert!(text.contains("beads"), "Status bar should show beads indicator");
    }

    #[test]
    fn test_left_section_without_beads() {
        let mut bar = StatusBar::new();
        bar.set_session(Some(SessionInfo {
            id: uuid::Uuid::new_v4(),
            name: "my-session".to_string(),
            created_at: 0,
            window_count: 1,
            attached_clients: 1,
            worktree: None,
            tags: std::collections::HashSet::new(),
            metadata: HashMap::new(),
        }));
        bar.set_beads_tracked(false);

        let spans = bar.left_section();
        let text: String = spans.iter().map(|s| s.content.as_ref()).collect();

        assert!(!text.contains("beads"), "Status bar should not show beads indicator when not tracked");
    }
}
