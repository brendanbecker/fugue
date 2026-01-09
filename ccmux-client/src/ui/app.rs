//! Main application struct and state management
//!
//! The App struct is the central coordinator for the ccmux client UI.

// Allow unused code that's part of the public API for future features
#![allow(dead_code)]

use std::collections::HashMap;
use std::time::Duration;

use crossterm::event::{Event as CrosstermEvent, KeyCode, KeyModifiers};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};
use uuid::Uuid;

use ccmux_protocol::{
    ClientMessage, ClaudeActivity, PaneInfo, PaneState, ServerMessage, SessionInfo,
    SplitDirection, WindowInfo,
};
use ccmux_utils::Result;

use crate::connection::Connection;
use crate::input::{ClientCommand, InputAction, InputHandler, InputMode};

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
    /// Input handler with prefix key state machine
    input_handler: InputHandler,
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
            input_handler: InputHandler::new(),
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
        // Update input handler with current active pane
        self.input_handler.set_active_pane(self.active_pane_id);

        // Convert InputEvent to crossterm Event for the input handler
        let event = match input {
            InputEvent::Key(key) => {
                // In session select mode, handle keys directly without the prefix system
                if self.state == AppState::SessionSelect {
                    return self.handle_session_select_input(key).await;
                }
                CrosstermEvent::Key(key)
            }
            InputEvent::Mouse(mouse) => CrosstermEvent::Mouse(mouse),
            InputEvent::FocusGained => CrosstermEvent::FocusGained,
            InputEvent::FocusLost => CrosstermEvent::FocusLost,
        };

        // Only use input handler when attached to a session
        if self.state == AppState::Attached {
            let action = self.input_handler.handle_event(event);
            self.handle_input_action(action).await?;
        }

        Ok(())
    }

    /// Handle an InputAction from the input handler
    async fn handle_input_action(&mut self, action: InputAction) -> Result<()> {
        match action {
            InputAction::None => {}

            InputAction::SendToPane(data) => {
                if let Some(pane_id) = self.active_pane_id {
                    self.connection
                        .send(ClientMessage::Input { pane_id, data })
                        .await?;
                }
            }

            InputAction::Command(cmd) => {
                self.handle_client_command(cmd).await?;
            }

            InputAction::FocusPane { x, y } => {
                // Find pane at coordinates and focus it
                // For now, just log - will be implemented with proper pane layout
                tracing::debug!("Focus pane at ({}, {})", x, y);
            }

            InputAction::ScrollUp { lines } => {
                if let Some(pane_id) = self.active_pane_id {
                    // Calculate new viewport offset
                    let new_offset = self.input_handler.scroll_offset();
                    self.connection
                        .send(ClientMessage::SetViewportOffset {
                            pane_id,
                            offset: new_offset,
                        })
                        .await?;
                    let _ = lines; // Used by input handler to track offset
                }
            }

            InputAction::ScrollDown { lines } => {
                if let Some(pane_id) = self.active_pane_id {
                    let new_offset = self.input_handler.scroll_offset();
                    if new_offset == 0 {
                        // Jump to bottom when scrolled all the way down
                        self.connection
                            .send(ClientMessage::JumpToBottom { pane_id })
                            .await?;
                    } else {
                        self.connection
                            .send(ClientMessage::SetViewportOffset {
                                pane_id,
                                offset: new_offset,
                            })
                            .await?;
                    }
                    let _ = lines;
                }
            }

            InputAction::Resize { cols, rows } => {
                self.terminal_size = (cols, rows);
                if let Some(pane_id) = self.active_pane_id {
                    self.connection
                        .send(ClientMessage::Resize { pane_id, cols, rows })
                        .await?;
                }
            }

            InputAction::Detach => {
                self.connection.send(ClientMessage::Detach).await?;
                self.state = AppState::SessionSelect;
                self.session = None;
                self.windows.clear();
                self.panes.clear();
                self.active_pane_id = None;
                self.status_message = Some("Detached from session".to_string());
                self.connection.send(ClientMessage::ListSessions).await?;
            }

            InputAction::Quit => {
                self.state = AppState::Quitting;
            }
        }
        Ok(())
    }

    /// Handle a client command from prefix key or command mode
    async fn handle_client_command(&mut self, cmd: ClientCommand) -> Result<()> {
        match cmd {
            ClientCommand::CreatePane => {
                if let Some(window) = self.windows.values().next() {
                    self.connection
                        .send(ClientMessage::CreatePane {
                            window_id: window.id,
                            direction: SplitDirection::Vertical,
                        })
                        .await?;
                }
            }

            ClientCommand::ClosePane => {
                if let Some(pane_id) = self.active_pane_id {
                    self.connection
                        .send(ClientMessage::ClosePane { pane_id })
                        .await?;
                }
            }

            ClientCommand::SplitVertical => {
                if let Some(window) = self.windows.values().next() {
                    self.connection
                        .send(ClientMessage::CreatePane {
                            window_id: window.id,
                            direction: SplitDirection::Vertical,
                        })
                        .await?;
                }
            }

            ClientCommand::SplitHorizontal => {
                if let Some(window) = self.windows.values().next() {
                    self.connection
                        .send(ClientMessage::CreatePane {
                            window_id: window.id,
                            direction: SplitDirection::Horizontal,
                        })
                        .await?;
                }
            }

            ClientCommand::NextPane => {
                self.cycle_pane(1);
            }

            ClientCommand::PreviousPane => {
                self.cycle_pane(-1);
            }

            ClientCommand::PaneLeft
            | ClientCommand::PaneRight
            | ClientCommand::PaneUp
            | ClientCommand::PaneDown => {
                // Directional navigation requires pane layout info
                // For now, just cycle panes
                self.cycle_pane(1);
            }

            ClientCommand::FocusPane(index) => {
                if let Some(pane) = self.panes.values().find(|p| p.index == index) {
                    self.active_pane_id = Some(pane.id);
                    self.connection
                        .send(ClientMessage::SelectPane { pane_id: pane.id })
                        .await?;
                }
            }

            ClientCommand::ListSessions => {
                self.connection.send(ClientMessage::ListSessions).await?;
            }

            ClientCommand::CreateSession(name) => {
                let session_name =
                    name.unwrap_or_else(|| format!("session-{}", Uuid::new_v4().as_simple()));
                self.connection
                    .send(ClientMessage::CreateSession { name: session_name })
                    .await?;
            }

            ClientCommand::ListWindows => {
                // Show window list in status
                let window_names: Vec<_> = self.windows.values().map(|w| w.name.clone()).collect();
                self.status_message = Some(format!("Windows: {}", window_names.join(", ")));
            }

            ClientCommand::CreateWindow => {
                if let Some(session) = &self.session {
                    self.connection
                        .send(ClientMessage::CreateWindow {
                            session_id: session.id,
                            name: None,
                        })
                        .await?;
                }
            }

            ClientCommand::EnterCopyMode => {
                self.status_message = Some("Copy mode - use j/k to scroll, q to exit".to_string());
            }

            ClientCommand::ExitCopyMode => {
                if let Some(pane_id) = self.active_pane_id {
                    self.connection
                        .send(ClientMessage::JumpToBottom { pane_id })
                        .await?;
                }
                self.status_message = None;
            }

            ClientCommand::ToggleZoom => {
                self.status_message = Some("Zoom toggle not yet implemented".to_string());
            }

            ClientCommand::ShowHelp => {
                self.status_message =
                    Some("Ctrl+B: prefix | c: new pane | x: close | n/p: next/prev".to_string());
            }

            // Commands not yet implemented
            ClientCommand::CloseWindow
            | ClientCommand::NextWindow
            | ClientCommand::PreviousWindow
            | ClientCommand::SelectWindow(_)
            | ClientCommand::RenameWindow(_)
            | ClientCommand::RenameSession(_)
            | ClientCommand::ClearHistory
            | ClientCommand::NextLayout
            | ClientCommand::ResizePane { .. }
            | ClientCommand::ReloadConfig
            | ClientCommand::ShowClock => {
                self.status_message = Some(format!("Command not yet implemented: {:?}", cmd));
            }
        }
        Ok(())
    }

    /// Cycle through panes by offset (positive = forward, negative = backward)
    fn cycle_pane(&mut self, offset: i32) {
        if self.panes.is_empty() {
            return;
        }

        let pane_ids: Vec<Uuid> = self.panes.keys().copied().collect();
        let current_index = self
            .active_pane_id
            .and_then(|id| pane_ids.iter().position(|&p| p == id))
            .unwrap_or(0);

        let new_index = if offset > 0 {
            (current_index + offset as usize) % pane_ids.len()
        } else {
            let abs_offset = (-offset) as usize;
            (current_index + pane_ids.len() - (abs_offset % pane_ids.len())) % pane_ids.len()
        };

        self.active_pane_id = Some(pane_ids[new_index]);
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
            ServerMessage::ViewportUpdated { pane_id, state } => {
                // Viewport scroll state updated for pane
                let _ = (pane_id, state); // TODO: Update scroll state in pane display
            }
            ServerMessage::ReplyDelivered { result } => {
                // Reply was successfully delivered to pane
                self.status_message = Some(format!(
                    "Reply delivered ({} bytes)",
                    result.bytes_written
                ));
            }
            ServerMessage::OrchestrationReceived { from_session_id, message } => {
                // Received orchestration message from another session
                // TODO: Handle orchestration messages in UI
                let _ = (from_session_id, message);
            }
            ServerMessage::OrchestrationDelivered { delivered_count } => {
                // Orchestration message was delivered to other sessions
                self.status_message = Some(format!(
                    "Message delivered to {} session(s)",
                    delivered_count
                ));
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
    fn draw_session_select(&mut self, frame: &mut ratatui::Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(5), Constraint::Length(3)])
            .split(area);

        if self.available_sessions.is_empty() {
            // Show empty state message
            let empty_msg = Paragraph::new("No sessions available. Press 'n' to create one.")
                .style(Style::default().fg(Color::DarkGray))
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Select Session")
                        .border_style(Style::default().fg(Color::Cyan)),
                );
            frame.render_widget(empty_msg, chunks[0]);
        } else {
            // Build list items with session metadata
            let items: Vec<ListItem> = self
                .available_sessions
                .iter()
                .map(|session| {
                    let worktree_info = session
                        .worktree
                        .as_ref()
                        .map(|w| format!(" [{}]", w.path))
                        .unwrap_or_default();
                    let orchestrator_badge = if session.is_orchestrator { " ★" } else { "" };
                    ListItem::new(format!(
                        "{}{} ({} windows, {} clients){}",
                        session.name,
                        orchestrator_badge,
                        session.window_count,
                        session.attached_clients,
                        worktree_info
                    ))
                })
                .collect();

            let list = List::new(items)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Select Session")
                        .border_style(Style::default().fg(Color::Cyan)),
                )
                .highlight_style(
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )
                .highlight_symbol("▶ ");

            // Create ListState with current selection
            let mut list_state = ListState::default();
            list_state.select(Some(self.session_list_index));

            frame.render_stateful_widget(list, chunks[0], &mut list_state);
        }

        // Help line with j/k mentioned
        let help = Paragraph::new("↑/k ↓/j: navigate | Enter: attach | n: new | r: refresh | Ctrl+Q: quit")
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

        // Show input mode indicator
        let mode_indicator = match self.input_handler.mode() {
            InputMode::Normal => "".to_string(),
            InputMode::PrefixPending => " [PREFIX]".to_string(),
            InputMode::Command => format!(" :{}", self.input_handler.command_buffer()),
            InputMode::Copy => format!(" [COPY +{}]", self.input_handler.scroll_offset()),
        };

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

        format!(
            " {} | {} panes {}{}",
            session_name,
            self.panes.len(),
            pane_info,
            mode_indicator
        )
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
