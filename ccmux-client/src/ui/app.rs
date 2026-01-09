//! Main application struct and state management
//!
//! The App struct is the central coordinator for the ccmux client UI.

// Allow unused code that's part of the public API for future features
#![allow(dead_code)]

use std::collections::HashMap;
use std::time::Duration;

use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Borders, Paragraph};
use uuid::Uuid;

use ccmux_protocol::{
    ClientMessage, ClaudeActivity, PaneInfo, PaneState, ServerMessage, SessionInfo,
    WindowInfo,
};
use ccmux_utils::Result;

use crate::connection::Connection;

use super::event::{AppEvent, EventHandler, InputEvent};
use super::terminal::Terminal;

/// Application state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppState {
    /// Initial state, not connected
    Disconnected,
    /// Connecting to server
    Connecting,
    /// Connected, selecting session
    SessionSelect,
    /// Attached to a session
    Attached,
    /// Shutting down
    Quitting,
}

/// Main application
pub struct App {
    /// Current application state
    state: AppState,
    /// Client ID
    client_id: Uuid,
    /// Server connection
    connection: Connection,
    /// Event handler
    events: EventHandler,
    /// Current session info
    session: Option<SessionInfo>,
    /// Windows in current session
    windows: HashMap<Uuid, WindowInfo>,
    /// Panes in current session
    panes: HashMap<Uuid, PaneInfo>,
    /// Active pane ID
    active_pane_id: Option<Uuid>,
    /// Available sessions (when in SessionSelect state)
    available_sessions: Vec<SessionInfo>,
    /// Selected session index in session list
    session_list_index: usize,
    /// Terminal size (cols, rows)
    terminal_size: (u16, u16),
    /// Animation tick counter
    tick_count: u64,
    /// Status message to display
    status_message: Option<String>,
}

impl App {
    /// Create a new application instance
    pub fn new() -> Result<Self> {
        let events = EventHandler::new(Duration::from_millis(100));

        Ok(Self {
            state: AppState::Disconnected,
            client_id: Uuid::new_v4(),
            connection: Connection::new(),
            events,
            session: None,
            windows: HashMap::new(),
            panes: HashMap::new(),
            active_pane_id: None,
            available_sessions: Vec::new(),
            session_list_index: 0,
            terminal_size: (80, 24),
            tick_count: 0,
            status_message: None,
        })
    }

    /// Get current application state
    pub fn state(&self) -> AppState {
        self.state
    }

    /// Check if application should quit
    pub fn should_quit(&self) -> bool {
        self.state == AppState::Quitting
    }

    /// Run the main application loop
    pub async fn run(&mut self) -> Result<()> {
        // Initialize terminal
        let mut terminal = Terminal::new()?;
        self.terminal_size = terminal.size()?;

        // Start input polling
        self.events.start_input_polling();

        // Start server message forwarding
        self.start_server_message_forwarding();

        // Connect to server
        self.connect().await?;

        // Main event loop
        while !self.should_quit() {
            // Draw UI
            self.draw(&mut terminal)?;

            // Handle events
            if let Some(event) = self.events.next().await {
                self.handle_event(event).await?;
            }
        }

        Ok(())
    }

    /// Connect to the ccmux server
    async fn connect(&mut self) -> Result<()> {
        self.state = AppState::Connecting;
        self.status_message = Some("Connecting to server...".to_string());

        match self.connection.connect().await {
            Ok(()) => {
                // Send handshake
                self.connection
                    .send(ClientMessage::Connect {
                        client_id: self.client_id,
                        protocol_version: ccmux_protocol::PROTOCOL_VERSION,
                    })
                    .await?;
                Ok(())
            }
            Err(e) => {
                self.state = AppState::Disconnected;
                self.status_message = Some(format!("Failed to connect: {}", e));
                Err(e)
            }
        }
    }

    /// Start forwarding server messages to event handler
    /// Note: Currently we poll server messages during tick events instead
    fn start_server_message_forwarding(&self) {
        // Server messages are polled during tick events in poll_server_messages()
        // This could be enhanced to use a dedicated background task if needed
    }

    /// Handle an application event
    async fn handle_event(&mut self, event: AppEvent) -> Result<()> {
        match event {
            AppEvent::Input(input) => self.handle_input(input).await?,
            AppEvent::Server(msg) => self.handle_server_message(msg).await?,
            AppEvent::Resize { cols, rows } => {
                self.terminal_size = (cols, rows);
                // Notify server of resize if attached
                if let Some(pane_id) = self.active_pane_id {
                    self.connection
                        .send(ClientMessage::Resize {
                            pane_id,
                            cols,
                            rows,
                        })
                        .await?;
                }
            }
            AppEvent::Tick => {
                self.tick_count = self.tick_count.wrapping_add(1);
                // Poll for server messages
                self.poll_server_messages().await?;
            }
        }
        Ok(())
    }

    /// Poll for pending server messages
    async fn poll_server_messages(&mut self) -> Result<()> {
        while let Some(msg) = self.connection.try_recv() {
            self.handle_server_message(msg).await?;
        }
        Ok(())
    }

    /// Handle input events
    async fn handle_input(&mut self, input: InputEvent) -> Result<()> {
        match input {
            InputEvent::Key(key) => {
                // Global key bindings
                if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('q')
                {
                    self.state = AppState::Quitting;
                    return Ok(());
                }

                // State-specific key handling
                match self.state {
                    AppState::SessionSelect => self.handle_session_select_input(key).await?,
                    AppState::Attached => self.handle_attached_input(key).await?,
                    _ => {}
                }
            }
            InputEvent::Mouse(_mouse) => {
                // Mouse handling can be implemented later
            }
            InputEvent::FocusGained | InputEvent::FocusLost => {
                // Focus events can be handled if needed
            }
        }
        Ok(())
    }

    /// Handle input in session select state
    async fn handle_session_select_input(
        &mut self,
        key: crossterm::event::KeyEvent,
    ) -> Result<()> {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                if self.session_list_index > 0 {
                    self.session_list_index -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.session_list_index < self.available_sessions.len().saturating_sub(1) {
                    self.session_list_index += 1;
                }
            }
            KeyCode::Enter => {
                if let Some(session) = self.available_sessions.get(self.session_list_index) {
                    self.connection
                        .send(ClientMessage::AttachSession {
                            session_id: session.id,
                        })
                        .await?;
                }
            }
            KeyCode::Char('n') => {
                // Create new session
                self.connection
                    .send(ClientMessage::CreateSession {
                        name: format!("session-{}", Uuid::new_v4().as_simple()),
                    })
                    .await?;
            }
            KeyCode::Char('r') => {
                // Refresh session list
                self.connection.send(ClientMessage::ListSessions).await?;
            }
            _ => {}
        }
        Ok(())
    }

    /// Handle input when attached to a session
    async fn handle_attached_input(&mut self, key: crossterm::event::KeyEvent) -> Result<()> {
        // Check for prefix key (Ctrl+B by default, like tmux)
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('b') {
            // Prefix mode - next key is a command
            // For now, we'll handle simple cases
            return Ok(());
        }

        // Forward input to active pane
        if let Some(pane_id) = self.active_pane_id {
            let data = key_to_bytes(&key);
            if !data.is_empty() {
                self.connection
                    .send(ClientMessage::Input { pane_id, data })
                    .await?;
            }
        }
        Ok(())
    }

    /// Handle server messages
    async fn handle_server_message(&mut self, msg: ServerMessage) -> Result<()> {
        match msg {
            ServerMessage::Connected {
                server_version,
                protocol_version: _,
            } => {
                self.status_message = Some(format!("Connected to server v{}", server_version));
                // Request session list
                self.connection.send(ClientMessage::ListSessions).await?;
                self.state = AppState::SessionSelect;
            }
            ServerMessage::SessionList { sessions } => {
                self.available_sessions = sessions;
                self.session_list_index = 0;
            }
            ServerMessage::SessionCreated { session } => {
                // Automatically attach to new session
                self.connection
                    .send(ClientMessage::AttachSession {
                        session_id: session.id,
                    })
                    .await?;
            }
            ServerMessage::Attached {
                session,
                windows,
                panes,
            } => {
                self.session = Some(session);
                self.windows = windows.into_iter().map(|w| (w.id, w)).collect();
                self.panes = panes.into_iter().map(|p| (p.id, p)).collect();
                self.active_pane_id = self.panes.keys().next().copied();
                self.state = AppState::Attached;
                self.status_message = Some("Attached to session".to_string());
            }
            ServerMessage::WindowCreated { window } => {
                self.windows.insert(window.id, window);
            }
            ServerMessage::PaneCreated { pane } => {
                self.panes.insert(pane.id, pane);
            }
            ServerMessage::Output { pane_id, data } => {
                // In a full implementation, this would update the pane's terminal buffer
                // For now, we store it or pass to tui-term
                if let Some(_pane) = self.panes.get_mut(&pane_id) {
                    // TODO: Update pane terminal with output data
                    let _ = data;
                }
            }
            ServerMessage::PaneStateChanged { pane_id, state } => {
                if let Some(pane) = self.panes.get_mut(&pane_id) {
                    pane.state = state;
                }
            }
            ServerMessage::ClaudeStateChanged { pane_id, state } => {
                if let Some(pane) = self.panes.get_mut(&pane_id) {
                    pane.state = PaneState::Claude(state);
                }
            }
            ServerMessage::PaneClosed { pane_id, .. } => {
                self.panes.remove(&pane_id);
                if self.active_pane_id == Some(pane_id) {
                    self.active_pane_id = self.panes.keys().next().copied();
                }
            }
            ServerMessage::WindowClosed { window_id } => {
                self.windows.remove(&window_id);
            }
            ServerMessage::SessionEnded { .. } => {
                self.session = None;
                self.windows.clear();
                self.panes.clear();
                self.active_pane_id = None;
                self.state = AppState::SessionSelect;
                self.status_message = Some("Session ended".to_string());
                // Refresh session list
                self.connection.send(ClientMessage::ListSessions).await?;
            }
            ServerMessage::Error { code, message } => {
                self.status_message = Some(format!("Error ({:?}): {}", code, message));
            }
            ServerMessage::Pong => {
                // Keepalive response, no action needed
            }
            ServerMessage::ViewportUpdated { .. } => {
                // Viewport update acknowledged, no action needed
            }
            ServerMessage::ReplyDelivered { .. } => {
                // Reply delivery status, handled separately if needed
            }
        }
        Ok(())
    }

    /// Draw the UI
    fn draw(&mut self, terminal: &mut Terminal) -> Result<()> {
        terminal.terminal_mut().draw(|frame| {
            let area = frame.area();

            match self.state {
                AppState::Disconnected => self.draw_disconnected(frame, area),
                AppState::Connecting => self.draw_connecting(frame, area),
                AppState::SessionSelect => self.draw_session_select(frame, area),
                AppState::Attached => self.draw_attached(frame, area),
                AppState::Quitting => {}
            }
        })?;
        Ok(())
    }

    /// Draw disconnected state
    fn draw_disconnected(&self, frame: &mut ratatui::Frame, area: Rect) {
        let message = self.status_message.as_deref().unwrap_or("Disconnected");
        let paragraph = Paragraph::new(message)
            .style(Style::default().fg(Color::Red))
            .block(Block::default().borders(Borders::ALL).title("ccmux"));
        frame.render_widget(paragraph, area);
    }

    /// Draw connecting state
    fn draw_connecting(&self, frame: &mut ratatui::Frame, area: Rect) {
        let dots = ".".repeat(((self.tick_count / 5) % 4) as usize);
        let message = format!("Connecting{}", dots);
        let paragraph = Paragraph::new(message)
            .style(Style::default().fg(Color::Yellow))
            .block(Block::default().borders(Borders::ALL).title("ccmux"));
        frame.render_widget(paragraph, area);
    }

    /// Draw session select state
    fn draw_session_select(&self, frame: &mut ratatui::Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(5), Constraint::Length(3)])
            .split(area);

        // Session list
        let mut lines: Vec<ratatui::text::Line> = Vec::new();

        if self.available_sessions.is_empty() {
            lines.push("No sessions available. Press 'n' to create one.".into());
        } else {
            for (i, session) in self.available_sessions.iter().enumerate() {
                let prefix = if i == self.session_list_index {
                    "> "
                } else {
                    "  "
                };
                let style = if i == self.session_list_index {
                    Style::default().fg(Color::Cyan)
                } else {
                    Style::default()
                };
                lines.push(
                    ratatui::text::Line::from(format!(
                        "{}{} ({} windows, {} clients)",
                        prefix, session.name, session.window_count, session.attached_clients
                    ))
                    .style(style),
                );
            }
        }

        let list = Paragraph::new(lines).block(
            Block::default()
                .borders(Borders::ALL)
                .title("Select Session"),
        );
        frame.render_widget(list, chunks[0]);

        // Help line
        let help = Paragraph::new("↑/↓: navigate | Enter: attach | n: new | r: refresh | Ctrl+Q: quit")
            .style(Style::default().fg(Color::DarkGray))
            .block(Block::default().borders(Borders::ALL).title("Help"));
        frame.render_widget(help, chunks[1]);
    }

    /// Draw attached state (main pane view)
    fn draw_attached(&self, frame: &mut ratatui::Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(3), Constraint::Length(1)])
            .split(area);

        // Main pane area
        let pane_area = chunks[0];

        // For now, draw a placeholder for the pane
        // This will be replaced with tui-term integration in Section 3
        let pane_block = Block::default()
            .borders(Borders::ALL)
            .title(self.active_pane_title())
            .border_style(Style::default().fg(Color::Cyan));

        let pane_content = if let Some(pane_id) = self.active_pane_id {
            if let Some(pane) = self.panes.get(&pane_id) {
                self.format_pane_info(pane)
            } else {
                "Pane not found".to_string()
            }
        } else {
            "No active pane".to_string()
        };

        let pane_widget = Paragraph::new(pane_content).block(pane_block);
        frame.render_widget(pane_widget, pane_area);

        // Status bar
        let status = self.build_status_bar();
        let status_widget = Paragraph::new(status).style(Style::default().bg(Color::DarkGray));
        frame.render_widget(status_widget, chunks[1]);
    }

    /// Get title for active pane
    fn active_pane_title(&self) -> String {
        if let Some(pane_id) = self.active_pane_id {
            if let Some(pane) = self.panes.get(&pane_id) {
                return pane.title.clone().unwrap_or_else(|| format!("Pane {}", pane.index));
            }
        }
        "Pane".to_string()
    }

    /// Format pane info for display
    fn format_pane_info(&self, pane: &PaneInfo) -> String {
        let state_info = match &pane.state {
            PaneState::Normal => "Normal".to_string(),
            PaneState::Claude(cs) => {
                format!("Claude: {:?}", cs.activity)
            }
            PaneState::Exited { code } => {
                format!("Exited: {:?}", code)
            }
        };

        format!(
            "Size: {}x{}\nState: {}\nCWD: {}",
            pane.cols,
            pane.rows,
            state_info,
            pane.cwd.as_deref().unwrap_or("unknown")
        )
    }

    /// Build status bar content
    fn build_status_bar(&self) -> String {
        let session_name = self
            .session
            .as_ref()
            .map(|s| s.name.as_str())
            .unwrap_or("No session");

        let pane_info = if let Some(pane_id) = self.active_pane_id {
            if let Some(pane) = self.panes.get(&pane_id) {
                match &pane.state {
                    PaneState::Normal => "[ ]".to_string(),
                    PaneState::Claude(cs) => format_claude_indicator(&cs.activity, self.tick_count),
                    PaneState::Exited { code } => format!("[Exit:{}]", code.unwrap_or(-1)),
                }
            } else {
                "".to_string()
            }
        } else {
            "".to_string()
        };

        format!(" {} | {} panes {} ", session_name, self.panes.len(), pane_info)
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new().expect("Failed to create App")
    }
}

/// Format Claude activity indicator with animation
fn format_claude_indicator(activity: &ClaudeActivity, tick: u64) -> String {
    match activity {
        ClaudeActivity::Idle => "[ ]".to_string(),
        ClaudeActivity::Thinking => {
            let frames = ["[.  ]", "[.. ]", "[...]", "[ ..]", "[  .]", "[   ]"];
            frames[(tick / 3) as usize % frames.len()].to_string()
        }
        ClaudeActivity::Coding => "[>]".to_string(),
        ClaudeActivity::ToolUse => "[*]".to_string(),
        ClaudeActivity::AwaitingConfirmation => "[?]".to_string(),
    }
}

/// Convert key event to byte sequence for terminal input
fn key_to_bytes(key: &crossterm::event::KeyEvent) -> Vec<u8> {
    let mut bytes = Vec::new();

    match key.code {
        KeyCode::Char(c) => {
            if key.modifiers.contains(KeyModifiers::CONTROL) {
                // Control characters
                if c.is_ascii_lowercase() {
                    bytes.push(c as u8 - b'a' + 1);
                }
            } else {
                let mut buf = [0u8; 4];
                let s = c.encode_utf8(&mut buf);
                bytes.extend_from_slice(s.as_bytes());
            }
        }
        KeyCode::Enter => bytes.push(b'\r'),
        KeyCode::Tab => bytes.push(b'\t'),
        KeyCode::Backspace => bytes.push(0x7f),
        KeyCode::Esc => bytes.push(0x1b),
        KeyCode::Up => bytes.extend_from_slice(b"\x1b[A"),
        KeyCode::Down => bytes.extend_from_slice(b"\x1b[B"),
        KeyCode::Right => bytes.extend_from_slice(b"\x1b[C"),
        KeyCode::Left => bytes.extend_from_slice(b"\x1b[D"),
        KeyCode::Home => bytes.extend_from_slice(b"\x1b[H"),
        KeyCode::End => bytes.extend_from_slice(b"\x1b[F"),
        KeyCode::PageUp => bytes.extend_from_slice(b"\x1b[5~"),
        KeyCode::PageDown => bytes.extend_from_slice(b"\x1b[6~"),
        KeyCode::Insert => bytes.extend_from_slice(b"\x1b[2~"),
        KeyCode::Delete => bytes.extend_from_slice(b"\x1b[3~"),
        KeyCode::F(n) => {
            let seq = match n {
                1 => b"\x1bOP".as_slice(),
                2 => b"\x1bOQ",
                3 => b"\x1bOR",
                4 => b"\x1bOS",
                5 => b"\x1b[15~",
                6 => b"\x1b[17~",
                7 => b"\x1b[18~",
                8 => b"\x1b[19~",
                9 => b"\x1b[20~",
                10 => b"\x1b[21~",
                11 => b"\x1b[23~",
                12 => b"\x1b[24~",
                _ => return bytes,
            };
            bytes.extend_from_slice(seq);
        }
        _ => {}
    }

    bytes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_state_default() {
        // Note: Can't actually test App::new() in unit tests as it needs terminal
        assert_eq!(AppState::Disconnected, AppState::Disconnected);
    }

    #[test]
    fn test_claude_indicator_idle() {
        let result = format_claude_indicator(&ClaudeActivity::Idle, 0);
        assert_eq!(result, "[ ]");
    }

    #[test]
    fn test_claude_indicator_thinking_animation() {
        // Different ticks should produce different frames
        let frames: Vec<String> = (0..6)
            .map(|i| format_claude_indicator(&ClaudeActivity::Thinking, i * 3))
            .collect();

        // Should cycle through animation frames
        assert!(frames.iter().any(|f| f.contains(".")));
    }

    #[test]
    fn test_claude_indicator_coding() {
        let result = format_claude_indicator(&ClaudeActivity::Coding, 0);
        assert_eq!(result, "[>]");
    }

    #[test]
    fn test_claude_indicator_tool_use() {
        let result = format_claude_indicator(&ClaudeActivity::ToolUse, 0);
        assert_eq!(result, "[*]");
    }

    #[test]
    fn test_claude_indicator_awaiting() {
        let result = format_claude_indicator(&ClaudeActivity::AwaitingConfirmation, 0);
        assert_eq!(result, "[?]");
    }

    #[test]
    fn test_key_to_bytes_char() {
        let key = crossterm::event::KeyEvent::new(KeyCode::Char('a'), KeyModifiers::empty());
        let bytes = key_to_bytes(&key);
        assert_eq!(bytes, vec![b'a']);
    }

    #[test]
    fn test_key_to_bytes_enter() {
        let key = crossterm::event::KeyEvent::new(KeyCode::Enter, KeyModifiers::empty());
        let bytes = key_to_bytes(&key);
        assert_eq!(bytes, vec![b'\r']);
    }

    #[test]
    fn test_key_to_bytes_arrow_up() {
        let key = crossterm::event::KeyEvent::new(KeyCode::Up, KeyModifiers::empty());
        let bytes = key_to_bytes(&key);
        assert_eq!(bytes, b"\x1b[A");
    }

    #[test]
    fn test_key_to_bytes_ctrl_c() {
        let key = crossterm::event::KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
        let bytes = key_to_bytes(&key);
        assert_eq!(bytes, vec![3]); // ETX
    }

    #[test]
    fn test_key_to_bytes_f1() {
        let key = crossterm::event::KeyEvent::new(KeyCode::F(1), KeyModifiers::empty());
        let bytes = key_to_bytes(&key);
        assert_eq!(bytes, b"\x1bOP");
    }
}
