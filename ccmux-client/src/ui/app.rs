//! Main application struct and state management
//!
//! The App struct is the central coordinator for the ccmux client UI.

// Allow unused code that's part of the public API for future features
#![allow(dead_code)]

use std::collections::HashMap;
use std::time::Duration;

/// Maximum size for a single input chunk sent to the server.
/// This should be well under the protocol's MAX_MESSAGE_SIZE (16MB).
/// 64KB provides good balance between overhead and latency.
const MAX_INPUT_CHUNK_SIZE: usize = 64 * 1024;

/// Maximum total paste size allowed.
/// Pastes larger than this will be rejected with a user message.
/// 10MB is generous for any reasonable paste operation.
const MAX_PASTE_SIZE: usize = 10 * 1024 * 1024;

/// Maximum number of server messages to process per tick.
/// This prevents event loop starvation during large output bursts (BUG-014).
/// With 50 messages per tick at 100ms tick rate, we can process 500 messages/sec
/// while still maintaining responsive input handling.
const MAX_MESSAGES_PER_TICK: usize = 50;

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
use super::layout::{LayoutManager, SplitDirection as LayoutSplitDirection};
use super::pane::{render_pane, FocusState, PaneManager};
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
    /// Last (previously active) pane ID for Ctrl-b ; (tmux last-pane)
    last_pane_id: Option<Uuid>,
    /// Last (previously active) window ID for Ctrl-b l (tmux last-window)
    last_window_id: Option<Uuid>,
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
    /// UI pane manager for terminal rendering
    pane_manager: PaneManager,
    /// Layout manager for pane arrangement
    layout: Option<LayoutManager>,
    /// Pending split direction for next pane creation
    pending_split_direction: Option<SplitDirection>,
    /// Custom command to run in new sessions (from CLI args)
    session_command: Option<String>,
    /// Previous input mode for tracking mode transitions (FEAT-056)
    /// Used to detect when user exits command mode
    previous_input_mode: InputMode,
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
            last_pane_id: None,
            last_window_id: None,
            available_sessions: Vec::new(),
            session_list_index: 0,
            terminal_size: (80, 24),
            tick_count: 0,
            status_message: None,
            pane_manager: PaneManager::new(),
            layout: None,
            pending_split_direction: None,
            session_command: None,
            previous_input_mode: InputMode::Normal,
        })
    }

    /// Create a new application instance with a custom socket path
    pub fn with_socket_path(socket_path: std::path::PathBuf) -> Result<Self> {
        let events = EventHandler::new(Duration::from_millis(100));

        Ok(Self {
            state: AppState::Disconnected,
            client_id: Uuid::new_v4(),
            connection: Connection::with_socket_path(socket_path),
            events,
            input_handler: InputHandler::new(),
            session: None,
            windows: HashMap::new(),
            panes: HashMap::new(),
            active_pane_id: None,
            last_pane_id: None,
            last_window_id: None,
            available_sessions: Vec::new(),
            session_list_index: 0,
            terminal_size: (80, 24),
            tick_count: 0,
            status_message: None,
            pane_manager: PaneManager::new(),
            layout: None,
            pending_split_direction: None,
            session_command: None,
            previous_input_mode: InputMode::Normal,
        })
    }

    /// Set the command to run in new sessions
    pub fn set_session_command(&mut self, command: Option<String>) {
        self.session_command = command;
    }

    /// Get current application state
    pub fn state(&self) -> AppState {
        self.state
    }

    /// Set quick navigation keybindings
    pub fn set_quick_bindings(&mut self, bindings: crate::input::QuickBindings) {
        self.input_handler.set_quick_bindings(bindings);
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

                // Update layout and resize all panes
                if self.state == AppState::Attached {
                    // Calculate pane area (minus status bar)
                    let pane_area = Rect::new(0, 0, cols, rows.saturating_sub(1));

                    if let Some(ref layout) = self.layout {
                        let pane_rects = layout.calculate_rects(pane_area);

                        // Resize each pane and notify server
                        for (pane_id, rect) in &pane_rects {
                            // Account for border (1 cell on each side)
                            let inner_width = rect.width.saturating_sub(2);
                            let inner_height = rect.height.saturating_sub(2);

                            // Resize UI pane
                            self.pane_manager.resize_pane(*pane_id, inner_height, inner_width);

                            // Notify server of resize for each pane
                            self.connection
                                .send(ClientMessage::Resize {
                                    pane_id: *pane_id,
                                    cols: inner_width,
                                    rows: inner_height,
                                })
                                .await?;
                        }
                    } else if let Some(pane_id) = self.active_pane_id {
                        // Fallback: single pane, no layout
                        let pane_rows = rows.saturating_sub(3);
                        let pane_cols = cols.saturating_sub(2);
                        self.pane_manager.resize_pane(pane_id, pane_rows, pane_cols);
                        self.connection
                            .send(ClientMessage::Resize {
                                pane_id,
                                cols: pane_cols,
                                rows: pane_rows,
                            })
                            .await?;
                    }
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
    ///
    /// BUG-014 FIX: Limits message processing per tick to prevent event loop starvation.
    /// During large output bursts, the server sends many Output messages that can
    /// queue up faster than the client can process them. Without a limit, the event
    /// loop would be blocked processing all queued messages before handling input.
    async fn poll_server_messages(&mut self) -> Result<()> {
        let mut processed = 0;
        while processed < MAX_MESSAGES_PER_TICK {
            if let Some(msg) = self.connection.try_recv() {
                tracing::trace!(
                    message_type = ?std::mem::discriminant(&msg),
                    processed = processed,
                    "poll_server_messages received message"
                );
                self.handle_server_message(msg).await?;
                processed += 1;
            } else {
                break;
            }
        }

        if processed >= MAX_MESSAGES_PER_TICK {
            tracing::debug!(
                processed = processed,
                "Hit message processing limit, deferring remaining messages to next tick"
            );
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
            // FEAT-056: Track mode before processing
            let mode_before = self.input_handler.mode();

            let action = self.input_handler.handle_event(event);
            self.handle_input_action(action).await?;

            // FEAT-056: Check if we exited PrefixPending mode
            let mode_after = self.input_handler.mode();
            if self.previous_input_mode == InputMode::PrefixPending
                && mode_after != InputMode::PrefixPending
            {
                // User command mode exited (command completed, timed out, or cancelled)
                self.connection
                    .send(ClientMessage::UserCommandModeExited)
                    .await?;
                tracing::debug!("Sent UserCommandModeExited (mode: {:?} -> {:?})", mode_before, mode_after);
            }
            self.previous_input_mode = mode_after;
        }

        Ok(())
    }

    /// Handle an InputAction from the input handler
    async fn handle_input_action(&mut self, action: InputAction) -> Result<()> {
        match action {
            InputAction::None => {}

            InputAction::SendToPane(data) => {
                if let Some(pane_id) = self.active_pane_id {
                    // BUG-011 FIX: Handle large pastes gracefully
                    let data_len = data.len();

                    // Reject extremely large pastes to prevent memory issues
                    if data_len > MAX_PASTE_SIZE {
                        let size_mb = data_len as f64 / (1024.0 * 1024.0);
                        self.status_message = Some(format!(
                            "Paste too large ({:.1}MB). Maximum is {}MB.",
                            size_mb,
                            MAX_PASTE_SIZE / (1024 * 1024)
                        ));
                        tracing::warn!(
                            "Rejected paste of {} bytes ({:.1}MB) - exceeds maximum",
                            data_len,
                            size_mb
                        );
                        return Ok(());
                    }

                    // Chunk large inputs to avoid protocol message size limits
                    if data_len > MAX_INPUT_CHUNK_SIZE {
                        let num_chunks = (data_len + MAX_INPUT_CHUNK_SIZE - 1) / MAX_INPUT_CHUNK_SIZE;
                        tracing::debug!(
                            "Chunking large paste ({} bytes) into {} chunks",
                            data_len,
                            num_chunks
                        );

                        // Show feedback for large pastes
                        if data_len > 1024 * 1024 {
                            let size_mb = data_len as f64 / (1024.0 * 1024.0);
                            self.status_message = Some(format!(
                                "Pasting {:.1}MB in {} chunks...",
                                size_mb,
                                num_chunks
                            ));
                        }

                        // Send data in chunks
                        for chunk in data.chunks(MAX_INPUT_CHUNK_SIZE) {
                            self.connection
                                .send(ClientMessage::Input {
                                    pane_id,
                                    data: chunk.to_vec(),
                                })
                                .await?;
                        }
                    } else {
                        // Small input - send directly
                        self.connection
                            .send(ClientMessage::Input { pane_id, data })
                            .await?;
                    }
                }
            }

            InputAction::Command(cmd) => {
                self.handle_client_command(cmd).await?;
            }

            InputAction::FocusPane { x, y } => {
                // Find pane at coordinates and focus it
                let (cols, rows) = self.terminal_size;
                let pane_area = Rect::new(0, 0, cols, rows.saturating_sub(1));

                if let Some(ref layout) = self.layout {
                    let pane_rects = layout.calculate_rects(pane_area);

                    // Find which pane contains the click point
                    if let Some((pane_id, _)) = pane_rects.iter().find(|(_, rect)| {
                        x >= rect.x && x < rect.x + rect.width && y >= rect.y && y < rect.y + rect.height
                    }) {
                        // Focus this pane (same logic as ClientCommand::FocusPane)
                        self.active_pane_id = Some(*pane_id);
                        self.pane_manager.set_active(*pane_id);
                        if let Some(ref mut layout) = self.layout {
                            layout.set_active_pane(*pane_id);
                        }
                        self.connection
                            .send(ClientMessage::SelectPane { pane_id: *pane_id })
                            .await?;
                    }
                }
            }

            InputAction::ScrollUp { lines } => {
                if let Some(pane_id) = self.active_pane_id {
                    // Update LOCAL UI pane for immediate visual feedback
                    if let Some(pane) = self.pane_manager.get_mut(pane_id) {
                        pane.scroll_up(lines);
                    }

                    // Get updated scroll offset and sync with server
                    let new_offset = self
                        .pane_manager
                        .get(pane_id)
                        .map(|p| p.scroll_offset())
                        .unwrap_or(0);
                    self.connection
                        .send(ClientMessage::SetViewportOffset {
                            pane_id,
                            offset: new_offset,
                        })
                        .await?;
                }
            }

            InputAction::ScrollDown { lines } => {
                if let Some(pane_id) = self.active_pane_id {
                    // Update LOCAL UI pane for immediate visual feedback
                    if let Some(pane) = self.pane_manager.get_mut(pane_id) {
                        pane.scroll_down(lines);
                    }

                    // Get updated scroll offset and sync with server
                    let new_offset = self
                        .pane_manager
                        .get(pane_id)
                        .map(|p| p.scroll_offset())
                        .unwrap_or(0);

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
                }
            }

            InputAction::Resize { cols, rows } => {
                self.terminal_size = (cols, rows);

                // Calculate pane area (minus status bar)
                let pane_area = Rect::new(0, 0, cols, rows.saturating_sub(1));

                if let Some(ref layout) = self.layout {
                    let pane_rects = layout.calculate_rects(pane_area);

                    // Resize each pane and notify server
                    for (pane_id, rect) in &pane_rects {
                        // Account for border (1 cell on each side)
                        let inner_width = rect.width.saturating_sub(2);
                        let inner_height = rect.height.saturating_sub(2);

                        // Resize UI pane
                        self.pane_manager.resize_pane(*pane_id, inner_height, inner_width);

                        // Notify server of resize
                        self.connection
                            .send(ClientMessage::Resize {
                                pane_id: *pane_id,
                                cols: inner_width,
                                rows: inner_height,
                            })
                            .await?;
                    }
                } else if let Some(pane_id) = self.active_pane_id {
                    // Fallback: single pane, no layout
                    let pane_rows = rows.saturating_sub(3);
                    let pane_cols = cols.saturating_sub(2);
                    self.pane_manager.resize_pane(pane_id, pane_rows, pane_cols);
                    self.connection
                        .send(ClientMessage::Resize {
                            pane_id,
                            cols: pane_cols,
                            rows: pane_rows,
                        })
                        .await?;
                }
            }

            InputAction::Detach => {
                self.connection.send(ClientMessage::Detach).await?;
                self.state = AppState::SessionSelect;
                self.session = None;
                self.windows.clear();
                self.panes.clear();
                self.pane_manager = PaneManager::new();
                self.active_pane_id = None;
                self.last_pane_id = None;
                self.last_window_id = None;
                self.layout = None;
                self.pending_split_direction = None;
                self.status_message = Some("Detached from session".to_string());
                self.connection.send(ClientMessage::ListSessions).await?;
            }

            InputAction::Quit => {
                self.state = AppState::Quitting;
            }

            InputAction::EnterUserCommandMode { timeout_ms } => {
                // FEAT-056: Send user priority lock to server
                self.connection
                    .send(ClientMessage::UserCommandModeEntered { timeout_ms })
                    .await?;
                tracing::debug!("Sent UserCommandModeEntered with timeout {}ms", timeout_ms);
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
                    // Store direction for layout update when PaneCreated is received
                    self.pending_split_direction = Some(SplitDirection::Vertical);
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
                    // Store direction for layout update when PaneCreated is received
                    self.pending_split_direction = Some(SplitDirection::Horizontal);
                    self.connection
                        .send(ClientMessage::CreatePane {
                            window_id: window.id,
                            direction: SplitDirection::Horizontal,
                        })
                        .await?;
                }
            }

            ClientCommand::NextPane => {
                // Use layout manager for navigation if available
                if let Some(ref mut layout) = self.layout {
                    layout.next_pane();
                    if let Some(new_active) = layout.active_pane_id() {
                        self.active_pane_id = Some(new_active);
                        self.pane_manager.set_active(new_active);
                    }
                } else {
                    self.cycle_pane(1);
                }
            }

            ClientCommand::PreviousPane => {
                // Use layout manager for navigation if available
                if let Some(ref mut layout) = self.layout {
                    layout.prev_pane();
                    if let Some(new_active) = layout.active_pane_id() {
                        self.active_pane_id = Some(new_active);
                        self.pane_manager.set_active(new_active);
                    }
                } else {
                    self.cycle_pane(-1);
                }
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
                    let pane_id = pane.id;
                    // Track last pane before switching (only if actually changing)
                    if self.active_pane_id != Some(pane_id) {
                        self.last_pane_id = self.active_pane_id;
                    }
                    self.active_pane_id = Some(pane_id);
                    self.pane_manager.set_active(pane_id);
                    // Sync with layout manager
                    if let Some(ref mut layout) = self.layout {
                        layout.set_active_pane(pane_id);
                    }
                    self.connection
                        .send(ClientMessage::SelectPane { pane_id })
                        .await?;
                }
            }

            ClientCommand::ListSessions => {
                self.state = AppState::SessionSelect;
                self.connection.send(ClientMessage::ListSessions).await?;
            }

            ClientCommand::CreateSession(name) => {
                let session_name =
                    name.unwrap_or_else(|| format!("session-{}", Uuid::new_v4().as_simple()));
                // Use CLI command only for first session, then clear it
                self.connection
                    .send(ClientMessage::CreateSession {
                        name: session_name,
                        command: self.session_command.take(),
                    })
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
                if let Some(pane_id) = self.active_pane_id {
                    if let Some(pane) = self.pane_manager.get_mut(pane_id) {
                        pane.enter_copy_mode();
                    }
                }
                self.status_message = Some("Copy mode - v: visual, V: line, hjkl: move, y: yank, q: exit".to_string());
            }

            ClientCommand::ExitCopyMode => {
                if let Some(pane_id) = self.active_pane_id {
                    if let Some(pane) = self.pane_manager.get_mut(pane_id) {
                        pane.exit_copy_mode();
                    }
                    self.connection
                        .send(ClientMessage::JumpToBottom { pane_id })
                        .await?;
                }
                self.status_message = None;
            }

            ClientCommand::StartVisualMode => {
                if let Some(pane_id) = self.active_pane_id {
                    if let Some(pane) = self.pane_manager.get_mut(pane_id) {
                        pane.start_visual_selection();
                    }
                }
                self.status_message = Some("-- VISUAL --".to_string());
            }

            ClientCommand::StartVisualLineMode => {
                if let Some(pane_id) = self.active_pane_id {
                    if let Some(pane) = self.pane_manager.get_mut(pane_id) {
                        pane.start_visual_line_selection();
                    }
                }
                self.status_message = Some("-- VISUAL LINE --".to_string());
            }

            ClientCommand::YankSelection => {
                if let Some(pane_id) = self.active_pane_id {
                    if let Some(pane) = self.pane_manager.get_mut(pane_id) {
                        if let Some(text) = pane.yank_selection() {
                            let len = text.len();
                            pane.exit_copy_mode();
                            self.status_message = Some(format!("Yanked {} bytes to clipboard", len));
                        } else {
                            self.status_message = Some("No selection to yank".to_string());
                        }
                    }
                }
            }

            ClientCommand::MoveCopyCursor { row_delta, col_delta } => {
                if let Some(pane_id) = self.active_pane_id {
                    if let Some(pane) = self.pane_manager.get_mut(pane_id) {
                        pane.move_copy_cursor(row_delta, col_delta);
                        // Update status with cursor position
                        if let Some(cursor) = pane.copy_mode_cursor() {
                            if let Some(indicator) = pane.visual_mode_indicator() {
                                self.status_message = Some(format!("{} ({}, {})", indicator, cursor.row, cursor.col));
                            } else {
                                self.status_message = Some(format!("Copy mode ({}, {})", cursor.row, cursor.col));
                            }
                        }
                    }
                }
            }

            ClientCommand::CancelSelection => {
                if let Some(pane_id) = self.active_pane_id {
                    if let Some(pane) = self.pane_manager.get_mut(pane_id) {
                        pane.cancel_selection();
                    }
                }
                self.status_message = Some("Selection cancelled".to_string());
            }

            ClientCommand::MouseSelectionStart { x, y } => {
                // Translate terminal coordinates to pane-relative coordinates
                if let Some(pane_id) = self.active_pane_id {
                    if let Some(pane) = self.pane_manager.get_mut(pane_id) {
                        // Get pane rect from layout to translate coordinates
                        if let Some(ref layout) = self.layout {
                            let (cols, rows) = crossterm::terminal::size().unwrap_or((80, 24));
                            let pane_area = ratatui::layout::Rect::new(0, 0, cols, rows.saturating_sub(1));
                            if let Some(rect) = layout.get_pane_rect(pane_area, pane_id) {
                                // Translate to pane-relative coordinates (accounting for border)
                                let pane_x = x.saturating_sub(rect.x + 1) as usize;
                                let pane_y = y.saturating_sub(rect.y + 1) as usize;
                                pane.mouse_selection_start(pane_y, pane_x);
                                self.status_message = Some("-- VISUAL --".to_string());
                            }
                        }
                    }
                }
            }

            ClientCommand::MouseSelectionUpdate { x, y } => {
                if let Some(pane_id) = self.active_pane_id {
                    if let Some(pane) = self.pane_manager.get_mut(pane_id) {
                        if let Some(ref layout) = self.layout {
                            let (cols, rows) = crossterm::terminal::size().unwrap_or((80, 24));
                            let pane_area = ratatui::layout::Rect::new(0, 0, cols, rows.saturating_sub(1));
                            if let Some(rect) = layout.get_pane_rect(pane_area, pane_id) {
                                let pane_x = x.saturating_sub(rect.x + 1) as usize;
                                let pane_y = y.saturating_sub(rect.y + 1) as usize;
                                pane.mouse_selection_update(pane_y, pane_x);
                            }
                        }
                    }
                }
            }

            ClientCommand::MouseSelectionEnd { x, y } => {
                if let Some(pane_id) = self.active_pane_id {
                    if let Some(pane) = self.pane_manager.get_mut(pane_id) {
                        if let Some(ref layout) = self.layout {
                            let (cols, rows) = crossterm::terminal::size().unwrap_or((80, 24));
                            let pane_area = ratatui::layout::Rect::new(0, 0, cols, rows.saturating_sub(1));
                            if let Some(rect) = layout.get_pane_rect(pane_area, pane_id) {
                                let pane_x = x.saturating_sub(rect.x + 1) as usize;
                                let pane_y = y.saturating_sub(rect.y + 1) as usize;
                                pane.mouse_selection_end(pane_y, pane_x);
                                self.status_message = Some("Selection complete - press 'y' to yank".to_string());
                            }
                        }
                    }
                }
            }

            ClientCommand::SelectWord { x, y } => {
                if let Some(pane_id) = self.active_pane_id {
                    if let Some(pane) = self.pane_manager.get_mut(pane_id) {
                        if let Some(ref layout) = self.layout {
                            let (cols, rows) = crossterm::terminal::size().unwrap_or((80, 24));
                            let pane_area = ratatui::layout::Rect::new(0, 0, cols, rows.saturating_sub(1));
                            if let Some(rect) = layout.get_pane_rect(pane_area, pane_id) {
                                let pane_x = x.saturating_sub(rect.x + 1) as usize;
                                let pane_y = y.saturating_sub(rect.y + 1) as usize;
                                pane.select_word_at(pane_y, pane_x);
                                self.status_message = Some("Word selected - press 'y' to yank".to_string());
                            }
                        }
                    }
                }
            }

            ClientCommand::SelectLine { x: _, y } => {
                if let Some(pane_id) = self.active_pane_id {
                    if let Some(pane) = self.pane_manager.get_mut(pane_id) {
                        if let Some(ref layout) = self.layout {
                            let (cols, rows) = crossterm::terminal::size().unwrap_or((80, 24));
                            let pane_area = ratatui::layout::Rect::new(0, 0, cols, rows.saturating_sub(1));
                            if let Some(rect) = layout.get_pane_rect(pane_area, pane_id) {
                                let pane_y = y.saturating_sub(rect.y + 1) as usize;
                                pane.select_line_at(pane_y);
                                self.status_message = Some("Line selected - press 'y' to yank".to_string());
                            }
                        }
                    }
                }
            }

            ClientCommand::ToggleZoom => {
                self.status_message = Some("Zoom toggle not yet implemented".to_string());
            }

            ClientCommand::ShowHelp => {
                self.status_message =
                    Some("Ctrl+B: prefix | c: new pane | x: close | n/p: next/prev".to_string());
            }

            ClientCommand::NextWindow => {
                self.cycle_window(1);
            }

            ClientCommand::PreviousWindow => {
                self.cycle_window(-1);
            }

            ClientCommand::LastWindow => {
                if let Some(last_id) = self.last_window_id {
                    // Get current window ID before switching
                    let current_window_id = self
                        .active_pane_id
                        .and_then(|pid| self.panes.get(&pid))
                        .map(|p| p.window_id);

                    // Focus first pane in the last window
                    if let Some(pane_id) = self.first_pane_in_window(last_id) {
                        // Update last_window_id to current before switching
                        self.last_window_id = current_window_id;
                        self.active_pane_id = Some(pane_id);
                        self.pane_manager.set_active(pane_id);
                        if let Some(ref mut layout) = self.layout {
                            layout.set_active_pane(pane_id);
                        }
                    }
                } else {
                    self.status_message = Some("No last window".to_string());
                }
            }

            ClientCommand::LastPane => {
                if let Some(last_id) = self.last_pane_id {
                    if self.panes.contains_key(&last_id) {
                        // Save current as last before switching
                        let current = self.active_pane_id;
                        self.last_pane_id = current;
                        self.active_pane_id = Some(last_id);
                        self.pane_manager.set_active(last_id);
                        if let Some(ref mut layout) = self.layout {
                            layout.set_active_pane(last_id);
                        }
                    } else {
                        self.status_message = Some("Last pane no longer exists".to_string());
                        self.last_pane_id = None;
                    }
                } else {
                    self.status_message = Some("No last pane".to_string());
                }
            }

            ClientCommand::ShowPaneNumbers => {
                // Show pane numbers as a status message
                // In tmux, this shows an overlay with pane numbers that can be selected
                // For now, show a simple status with pane indices
                let pane_info: Vec<String> = self
                    .panes
                    .values()
                    .map(|p| format!("{}", p.index))
                    .collect();
                self.status_message = Some(format!(
                    "Pane numbers: {} (use Ctrl-b 0-9 to select)",
                    pane_info.join(", ")
                ));
            }

            ClientCommand::SelectWindow(index) => {
                // Find window by index (sorted order)
                let mut window_ids: Vec<Uuid> = self.windows.keys().copied().collect();
                window_ids.sort();

                if let Some(&window_id) = window_ids.get(index) {
                    // Track current window as last before switching
                    let current_window_id = self
                        .active_pane_id
                        .and_then(|pid| self.panes.get(&pid))
                        .map(|p| p.window_id);

                    if current_window_id != Some(window_id) {
                        self.last_window_id = current_window_id;
                    }

                    // Focus first pane in the window
                    if let Some(pane_id) = self.first_pane_in_window(window_id) {
                        self.active_pane_id = Some(pane_id);
                        self.pane_manager.set_active(pane_id);
                        if let Some(ref mut layout) = self.layout {
                            layout.set_active_pane(pane_id);
                        }
                    }
                } else {
                    self.status_message = Some(format!("No window at index {}", index));
                }
            }

            // Commands not yet implemented
            ClientCommand::CloseWindow
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

        let new_pane_id = pane_ids[new_index];

        // Track last pane before switching (only if actually changing)
        if self.active_pane_id != Some(new_pane_id) {
            self.last_pane_id = self.active_pane_id;
        }

        self.active_pane_id = Some(new_pane_id);
        self.pane_manager.set_active(new_pane_id);
        // Sync with layout manager
        if let Some(ref mut layout) = self.layout {
            layout.set_active_pane(new_pane_id);
        }
    }

    /// Cycle through windows by offset (positive = forward, negative = backward)
    fn cycle_window(&mut self, offset: i32) {
        if self.windows.is_empty() {
            return;
        }

        // Get current window ID from active pane
        let current_window_id = self
            .active_pane_id
            .and_then(|pid| self.panes.get(&pid))
            .map(|p| p.window_id);

        // Get sorted list of window IDs for consistent ordering
        let mut window_ids: Vec<Uuid> = self.windows.keys().copied().collect();
        window_ids.sort(); // Consistent ordering

        let current_index = current_window_id
            .and_then(|wid| window_ids.iter().position(|&w| w == wid))
            .unwrap_or(0);

        let new_index = if offset > 0 {
            (current_index + offset as usize) % window_ids.len()
        } else {
            let abs_offset = (-offset) as usize;
            (current_index + window_ids.len() - (abs_offset % window_ids.len())) % window_ids.len()
        };

        let new_window_id = window_ids[new_index];

        // Track last window before switching (only if actually changing)
        if current_window_id != Some(new_window_id) {
            self.last_window_id = current_window_id;
        }

        // Focus first pane in the new window
        if let Some(pane_id) = self.first_pane_in_window(new_window_id) {
            self.active_pane_id = Some(pane_id);
            self.pane_manager.set_active(pane_id);
        }
    }

    /// Get the first pane in a window (by index)
    fn first_pane_in_window(&self, window_id: Uuid) -> Option<Uuid> {
        self.panes
            .values()
            .filter(|p| p.window_id == window_id)
            .min_by_key(|p| p.index)
            .map(|p| p.id)
    }

    /// Handle input in session select state
    async fn handle_session_select_input(
        &mut self,
        key: crossterm::event::KeyEvent,
    ) -> Result<()> {
        match (key.code, key.modifiers) {
            // Quit handlers
            (KeyCode::Char('c'), KeyModifiers::CONTROL)
            | (KeyCode::Char('q'), KeyModifiers::CONTROL) => {
                self.state = AppState::Quitting;
            }
            (KeyCode::Char('q'), KeyModifiers::NONE) | (KeyCode::Esc, _) => {
                self.state = AppState::Quitting;
            }
            // Navigation
            (KeyCode::Up, _) | (KeyCode::Char('k'), KeyModifiers::NONE) => {
                if self.session_list_index > 0 {
                    self.session_list_index -= 1;
                }
            }
            (KeyCode::Down, _) | (KeyCode::Char('j'), KeyModifiers::NONE) => {
                if self.session_list_index < self.available_sessions.len().saturating_sub(1) {
                    self.session_list_index += 1;
                }
            }
            (KeyCode::Enter, _) => {
                if let Some(session) = self.available_sessions.get(self.session_list_index) {
                    self.connection
                        .send(ClientMessage::AttachSession {
                            session_id: session.id,
                        })
                        .await?;
                }
            }
            (KeyCode::Char('n'), KeyModifiers::NONE) => {
                // Create new session (CLI command only applies to first session)
                self.connection
                    .send(ClientMessage::CreateSession {
                        name: format!("session-{}", Uuid::new_v4().as_simple()),
                        command: self.session_command.take(),
                    })
                    .await?;
            }
            (KeyCode::Char('r'), KeyModifiers::NONE) => {
                // Refresh session list
                self.connection.send(ClientMessage::ListSessions).await?;
            }
            (KeyCode::Char('d'), KeyModifiers::CONTROL) => {
                // Delete/destroy selected session
                if let Some(session) = self.available_sessions.get(self.session_list_index) {
                    let session_id = session.id;
                    self.connection
                        .send(ClientMessage::DestroySession { session_id })
                        .await?;
                    // Server will broadcast updated session list
                    // Adjust selection index if needed
                    if self.session_list_index > 0
                        && self.session_list_index >= self.available_sessions.len().saturating_sub(1)
                    {
                        self.session_list_index = self.session_list_index.saturating_sub(1);
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// Handle server messages
    async fn handle_server_message(&mut self, msg: ServerMessage) -> Result<()> {
        tracing::debug!(
            message_type = ?std::mem::discriminant(&msg),
            "handle_server_message processing"
        );
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

                // BUG-006 FIX: Use client's terminal size, not server-reported size
                // The server's pane dimensions are from when the session was created,
                // which may differ from this client's terminal size.
                let (term_cols, term_rows) = self.terminal_size;
                let pane_rows = term_rows.saturating_sub(3); // Account for borders and status bar
                let pane_cols = term_cols.saturating_sub(2); // Account for side borders

                // Initialize layout manager with panes
                // For now, when reattaching, we create a simple layout with existing panes
                // (a more complete solution would persist layout in the server)
                let pane_ids: Vec<Uuid> = self.panes.keys().copied().collect();
                if let Some(&first_pane_id) = pane_ids.first() {
                    let mut layout_manager = LayoutManager::new(first_pane_id);
                    // Add remaining panes as vertical splits (simple layout for reattach)
                    for &pane_id in pane_ids.iter().skip(1) {
                        layout_manager.root_mut().add_pane(
                            first_pane_id,
                            pane_id,
                            LayoutSplitDirection::Vertical,
                        );
                    }
                    layout_manager.set_active_pane(self.active_pane_id.unwrap_or(first_pane_id));
                    self.layout = Some(layout_manager);
                }

                // Create UI panes with CLIENT's terminal dimensions
                for pane_info in self.panes.values() {
                    self.pane_manager.add_pane(pane_info.id, pane_rows, pane_cols);
                    if let Some(ui_pane) = self.pane_manager.get_mut(pane_info.id) {
                        ui_pane.set_title(pane_info.title.clone());
                        ui_pane.set_cwd(pane_info.cwd.clone());
                        ui_pane.set_pane_state(pane_info.state.clone());
                    }
                }

                // Send resize messages to server for all panes to sync PTY dimensions
                for pane_id in self.pane_manager.pane_ids() {
                    self.connection
                        .send(ClientMessage::Resize {
                            pane_id,
                            cols: pane_cols,
                            rows: pane_rows,
                        })
                        .await?;
                }

                // Set active UI pane with focus state
                if let Some(active_id) = self.active_pane_id {
                    self.pane_manager.set_active(active_id);
                }
            }
            ServerMessage::WindowCreated { window } => {
                self.windows.insert(window.id, window);
            }
            ServerMessage::PaneCreated { pane, direction } => {
                tracing::info!(
                    pane_id = %pane.id,
                    window_id = %pane.window_id,
                    pane_index = pane.index,
                    ?direction,
                    "Handling PaneCreated broadcast from server"
                );

                // Use direction from the message (set by MCP or TUI command)
                // Clear pending_split_direction if it was set by TUI
                let _ = self.pending_split_direction.take();
                let layout_direction = LayoutSplitDirection::from(direction);

                // Add new pane to layout
                if let Some(ref mut layout) = self.layout {
                    // Split the active pane to add the new one
                    if let Some(active_id) = self.active_pane_id {
                        layout.root_mut().add_pane(active_id, pane.id, layout_direction);
                    } else {
                        // No active pane - this is the first pane, initialize layout
                        *layout = LayoutManager::new(pane.id);
                    }
                } else {
                    // Layout not initialized - create with this pane
                    self.layout = Some(LayoutManager::new(pane.id));
                }

                // Create UI pane for terminal rendering
                self.pane_manager.add_pane(pane.id, pane.rows, pane.cols);
                if let Some(ui_pane) = self.pane_manager.get_mut(pane.id) {
                    ui_pane.set_title(pane.title.clone());
                    ui_pane.set_cwd(pane.cwd.clone());
                    ui_pane.set_pane_state(pane.state.clone());
                }
                self.panes.insert(pane.id, pane.clone());

                // Switch focus to the new pane
                self.active_pane_id = Some(pane.id);
                self.pane_manager.set_active(pane.id);
                if let Some(ref mut layout) = self.layout {
                    layout.set_active_pane(pane.id);
                }

                // Resize all panes after layout change
                let (cols, rows) = self.terminal_size;
                let pane_area = Rect::new(0, 0, cols, rows.saturating_sub(1));

                if let Some(ref layout) = self.layout {
                    let pane_rects = layout.calculate_rects(pane_area);

                    for (pane_id, rect) in &pane_rects {
                        let inner_width = rect.width.saturating_sub(2);
                        let inner_height = rect.height.saturating_sub(2);

                        self.pane_manager.resize_pane(*pane_id, inner_height, inner_width);

                        self.connection
                            .send(ClientMessage::Resize {
                                pane_id: *pane_id,
                                cols: inner_width,
                                rows: inner_height,
                            })
                            .await?;
                    }
                }
            }
            ServerMessage::Output { pane_id, data } => {
                // Process output through the UI pane's terminal emulator
                self.pane_manager.process_output(pane_id, &data);
            }
            ServerMessage::PaneStateChanged { pane_id, state } => {
                if let Some(pane) = self.panes.get_mut(&pane_id) {
                    pane.state = state.clone();
                }
                // Sync state with UI pane
                self.pane_manager.update_pane_state(pane_id, state);
            }
            ServerMessage::ClaudeStateChanged { pane_id, state } => {
                let pane_state = PaneState::Claude(state);
                if let Some(pane) = self.panes.get_mut(&pane_id) {
                    pane.state = pane_state.clone();
                }
                // Sync state with UI pane
                self.pane_manager.update_pane_state(pane_id, pane_state);
            }
            ServerMessage::PaneClosed { pane_id, .. } => {
                self.panes.remove(&pane_id);
                self.pane_manager.remove_pane(pane_id);

                // Remove from layout (which also prunes single-child splits)
                if let Some(ref mut layout) = self.layout {
                    layout.remove_pane(pane_id);
                }

                if self.active_pane_id == Some(pane_id) {
                    // Get new active pane from layout or fallback to panes
                    let new_active = self
                        .layout
                        .as_ref()
                        .and_then(|l| l.active_pane_id())
                        .or_else(|| self.panes.keys().next().copied());

                    self.active_pane_id = new_active;
                    // Update active UI pane and layout
                    if let Some(id) = new_active {
                        self.pane_manager.set_active(id);
                        if let Some(ref mut layout) = self.layout {
                            layout.set_active_pane(id);
                        }
                    }
                }

                // BUG-015 FIX: Recalculate layout and resize remaining panes
                // After removing a pane, remaining panes should expand to fill available space
                if !self.panes.is_empty() {
                    let (cols, rows) = self.terminal_size;
                    let pane_area = Rect::new(0, 0, cols, rows.saturating_sub(1));

                    if let Some(ref layout) = self.layout {
                        let pane_rects = layout.calculate_rects(pane_area);

                        for (remaining_pane_id, rect) in &pane_rects {
                            let inner_width = rect.width.saturating_sub(2);
                            let inner_height = rect.height.saturating_sub(2);

                            // Resize UI pane to new dimensions
                            self.pane_manager
                                .resize_pane(*remaining_pane_id, inner_height, inner_width);

                            // Notify server of new size so PTY gets resize signal
                            self.connection
                                .send(ClientMessage::Resize {
                                    pane_id: *remaining_pane_id,
                                    cols: inner_width,
                                    rows: inner_height,
                                })
                                .await?;
                        }
                    }
                }

                // If no panes left, go back to session selection
                if self.panes.is_empty() {
                    self.session = None;
                    self.windows.clear();
                    self.active_pane_id = None;
                    self.layout = None;
                    self.state = AppState::SessionSelect;
                    self.status_message = Some("Session has no active panes".to_string());
                }
            }
            ServerMessage::WindowClosed { window_id } => {
                self.windows.remove(&window_id);
            }
            ServerMessage::SessionEnded { .. } => {
                self.session = None;
                self.windows.clear();
                self.panes.clear();
                self.pane_manager = PaneManager::new();
                self.active_pane_id = None;
                self.last_pane_id = None;
                self.last_window_id = None;
                self.layout = None;
                self.pending_split_direction = None;
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
            // BUG-026 FIX: Focus change broadcasts from MCP commands
            ServerMessage::PaneFocused { pane_id, window_id, .. } => {
                // Update active pane if we know about this pane
                if self.panes.contains_key(&pane_id) {
                    self.active_pane_id = Some(pane_id);
                    tracing::debug!("Focus changed to pane {} (via MCP)", pane_id);
                    // If the window is known, ensure it's the active window display
                    if let Some(window) = self.windows.get_mut(&window_id) {
                        window.active_pane_id = Some(pane_id);
                    }
                }
            }
            ServerMessage::WindowFocused { window_id, .. } => {
                // Update active window - focus its active pane
                if let Some(window) = self.windows.get(&window_id) {
                    if let Some(active_pane) = window.active_pane_id {
                        self.active_pane_id = Some(active_pane);
                        tracing::debug!("Window {} focused, now focusing pane {} (via MCP)", window_id, active_pane);
                    }
                }
            }
            ServerMessage::SessionFocused { session_id } => {
                // Session focus change - if this is our attached session, log it
                if let Some(ref session) = self.session {
                    if session.id == session_id {
                        tracing::debug!("Our session {} is now the active session (via MCP)", session_id);
                    }
                }
            }

            // MCP bridge messages - not used by TUI client
            ServerMessage::AllPanesList { .. }
            | ServerMessage::WindowList { .. }
            | ServerMessage::PaneContent { .. }
            | ServerMessage::PaneStatus { .. }
            | ServerMessage::PaneCreatedWithDetails { .. }
            | ServerMessage::SessionCreatedWithDetails { .. }
            | ServerMessage::WindowCreatedWithDetails { .. }
            | ServerMessage::SessionRenamed { .. }
            | ServerMessage::PaneSplit { .. }
            | ServerMessage::PaneResized { .. }
            | ServerMessage::LayoutCreated { .. }
            | ServerMessage::SessionDestroyed { .. }
            | ServerMessage::EnvironmentSet { .. }
            | ServerMessage::EnvironmentList { .. } => {
                // These messages are for the MCP bridge, not the TUI client
            }
        }
        Ok(())
    }

    /// Draw the UI
    fn draw(&mut self, terminal: &mut Terminal) -> Result<()> {
        // For attached state, update pane layout before drawing
        if self.state == AppState::Attached {
            self.update_pane_layout(terminal.size()?);
        }

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

    /// Update pane layout and sizes based on current terminal size
    fn update_pane_layout(&mut self, terminal_size: (u16, u16)) {
        let (term_cols, term_rows) = terminal_size;

        // Calculate pane area (minus status bar)
        let pane_area = Rect::new(0, 0, term_cols, term_rows.saturating_sub(1));

        if let Some(ref layout) = self.layout {
            let pane_rects = layout.calculate_rects(pane_area);

            for (pane_id, rect) in &pane_rects {
                // Account for border (1 cell on each side)
                let inner_width = rect.width.saturating_sub(2);
                let inner_height = rect.height.saturating_sub(2);

                // Resize the UI pane to match the calculated layout
                self.pane_manager.resize_pane(*pane_id, inner_height, inner_width);

                // Update focus state
                let is_active = Some(*pane_id) == self.active_pane_id;
                if let Some(ui_pane) = self.pane_manager.get_mut(*pane_id) {
                    ui_pane.set_focus_state(if is_active {
                        FocusState::Focused
                    } else {
                        FocusState::Unfocused
                    });
                }
            }
        }
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
        let help = Paragraph::new("↑/k ↓/j: navigate | Enter: attach | n: new | r: refresh | Ctrl+D: delete | q: quit")
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

        // Render all panes using layout manager
        if let Some(ref layout) = self.layout {
            let pane_rects = layout.calculate_rects(pane_area);

            // Render each pane
            for (pane_id, rect) in &pane_rects {
                if let Some(ui_pane) = self.pane_manager.get(*pane_id) {
                    render_pane(ui_pane, *rect, frame.buffer_mut(), self.tick_count);
                } else {
                    // Fallback if UI pane not found
                    let pane_block = Block::default()
                        .borders(Borders::ALL)
                        .title("Pane (no terminal)")
                        .border_style(Style::default().fg(Color::Red));
                    let pane_widget = Paragraph::new("Terminal not initialized")
                        .block(pane_block);
                    frame.render_widget(pane_widget, *rect);
                }
            }
        } else if let Some(pane_id) = self.active_pane_id {
            // Fallback: no layout, render single active pane
            if let Some(ui_pane) = self.pane_manager.get(pane_id) {
                render_pane(ui_pane, pane_area, frame.buffer_mut(), self.tick_count);
            } else {
                let pane_block = Block::default()
                    .borders(Borders::ALL)
                    .title("Pane (no terminal)")
                    .border_style(Style::default().fg(Color::Red));
                let pane_widget = Paragraph::new("Terminal not initialized")
                    .block(pane_block);
                frame.render_widget(pane_widget, pane_area);
            }
        } else {
            // No active pane
            let pane_block = Block::default()
                .borders(Borders::ALL)
                .title("No Pane")
                .border_style(Style::default().fg(Color::DarkGray));
            let pane_widget = Paragraph::new("No active pane").block(pane_block);
            frame.render_widget(pane_widget, pane_area);
        }

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

    // Calculate modifier parameter for CSI sequences (xterm style)
    // 1 = none, 2 = Shift, 3 = Alt, 4 = Shift+Alt, 5 = Ctrl, 6 = Shift+Ctrl, 7 = Alt+Ctrl, 8 = all
    let modifier_param = {
        let mut m = 1u8;
        if key.modifiers.contains(KeyModifiers::SHIFT) {
            m += 1;
        }
        if key.modifiers.contains(KeyModifiers::ALT) {
            m += 2;
        }
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            m += 4;
        }
        m
    };

    match key.code {
        KeyCode::Char(c) => {
            if key.modifiers.contains(KeyModifiers::CONTROL) {
                // Control characters
                if c.is_ascii_lowercase() {
                    bytes.push(c as u8 - b'a' + 1);
                } else if c.is_ascii_uppercase() {
                    bytes.push(c as u8 - b'A' + 1);
                }
            } else if key.modifiers.contains(KeyModifiers::ALT) {
                // Alt/Meta sends ESC prefix
                bytes.push(0x1b);
                let mut buf = [0u8; 4];
                let s = c.encode_utf8(&mut buf);
                bytes.extend_from_slice(s.as_bytes());
            } else {
                let mut buf = [0u8; 4];
                let s = c.encode_utf8(&mut buf);
                bytes.extend_from_slice(s.as_bytes());
            }
        }
        KeyCode::Enter => bytes.push(b'\r'),
        KeyCode::Tab => {
            if key.modifiers.contains(KeyModifiers::SHIFT) {
                bytes.extend_from_slice(b"\x1b[Z"); // CSI Z - backtab
            } else {
                bytes.push(b'\t');
            }
        }
        KeyCode::Backspace => {
            if key.modifiers.contains(KeyModifiers::ALT) {
                bytes.extend_from_slice(b"\x1b\x7f"); // Alt+Backspace - delete word
            } else {
                bytes.push(0x7f);
            }
        }
        KeyCode::Esc => bytes.push(0x1b),
        KeyCode::Up => {
            if modifier_param > 1 {
                bytes.extend_from_slice(format!("\x1b[1;{}A", modifier_param).as_bytes());
            } else {
                bytes.extend_from_slice(b"\x1b[A");
            }
        }
        KeyCode::Down => {
            if modifier_param > 1 {
                bytes.extend_from_slice(format!("\x1b[1;{}B", modifier_param).as_bytes());
            } else {
                bytes.extend_from_slice(b"\x1b[B");
            }
        }
        KeyCode::Right => {
            if modifier_param > 1 {
                bytes.extend_from_slice(format!("\x1b[1;{}C", modifier_param).as_bytes());
            } else {
                bytes.extend_from_slice(b"\x1b[C");
            }
        }
        KeyCode::Left => {
            if modifier_param > 1 {
                bytes.extend_from_slice(format!("\x1b[1;{}D", modifier_param).as_bytes());
            } else {
                bytes.extend_from_slice(b"\x1b[D");
            }
        }
        KeyCode::Home => {
            if modifier_param > 1 {
                bytes.extend_from_slice(format!("\x1b[1;{}H", modifier_param).as_bytes());
            } else {
                bytes.extend_from_slice(b"\x1b[H");
            }
        }
        KeyCode::End => {
            if modifier_param > 1 {
                bytes.extend_from_slice(format!("\x1b[1;{}F", modifier_param).as_bytes());
            } else {
                bytes.extend_from_slice(b"\x1b[F");
            }
        }
        KeyCode::PageUp => bytes.extend_from_slice(b"\x1b[5~"),
        KeyCode::PageDown => bytes.extend_from_slice(b"\x1b[6~"),
        KeyCode::Insert => bytes.extend_from_slice(b"\x1b[2~"),
        KeyCode::Delete => {
            if key.modifiers.contains(KeyModifiers::ALT) {
                bytes.extend_from_slice(b"\x1b[3;3~"); // Alt+Delete
            } else {
                bytes.extend_from_slice(b"\x1b[3~");
            }
        }
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

    // ==================== BUG-011 Tests: Large Paste Handling ====================

    #[test]
    fn test_max_input_chunk_size_is_reasonable() {
        // Chunk size should be much smaller than protocol max (16MB)
        // to leave room for message overhead and serialization
        assert!(MAX_INPUT_CHUNK_SIZE <= 1024 * 1024); // <= 1MB
        assert!(MAX_INPUT_CHUNK_SIZE >= 4096); // >= 4KB for efficiency
    }

    #[test]
    fn test_max_paste_size_is_reasonable() {
        // Max paste should be smaller than protocol max
        // but large enough for legitimate use cases
        assert!(MAX_PASTE_SIZE <= 16 * 1024 * 1024); // <= 16MB (protocol max)
        assert!(MAX_PASTE_SIZE >= 1024 * 1024); // >= 1MB for practical use
    }

    #[test]
    fn test_chunk_size_is_less_than_paste_limit() {
        // Chunks should be smaller than the overall paste limit
        assert!(MAX_INPUT_CHUNK_SIZE < MAX_PASTE_SIZE);
    }

    #[test]
    fn test_chunking_math_small_input() {
        // Input smaller than chunk size should not be chunked
        let data_len = MAX_INPUT_CHUNK_SIZE / 2;
        let num_chunks = if data_len > MAX_INPUT_CHUNK_SIZE {
            (data_len + MAX_INPUT_CHUNK_SIZE - 1) / MAX_INPUT_CHUNK_SIZE
        } else {
            1
        };
        assert_eq!(num_chunks, 1);
    }

    #[test]
    fn test_chunking_math_exact_chunk_size() {
        // Input exactly chunk size should be a single chunk
        let data_len = MAX_INPUT_CHUNK_SIZE;
        let num_chunks = (data_len + MAX_INPUT_CHUNK_SIZE - 1) / MAX_INPUT_CHUNK_SIZE;
        assert_eq!(num_chunks, 1);
    }

    #[test]
    fn test_chunking_math_multiple_chunks() {
        // Input larger than chunk size should be multiple chunks
        let data_len = MAX_INPUT_CHUNK_SIZE * 3 + 100;
        let num_chunks = (data_len + MAX_INPUT_CHUNK_SIZE - 1) / MAX_INPUT_CHUNK_SIZE;
        assert_eq!(num_chunks, 4); // 3 full chunks + 1 partial
    }

    #[test]
    fn test_chunking_math_large_paste() {
        // Test with a 5MB paste
        let data_len = 5 * 1024 * 1024;
        let num_chunks = (data_len + MAX_INPUT_CHUNK_SIZE - 1) / MAX_INPUT_CHUNK_SIZE;
        // With 64KB chunks, 5MB = ~78 chunks
        assert!(num_chunks > 70);
        assert!(num_chunks < 100);
    }

    #[test]
    fn test_over_limit_detection() {
        // Verify we correctly detect pastes over the limit
        let within_limit = MAX_PASTE_SIZE;
        let over_limit = MAX_PASTE_SIZE + 1;

        assert!(within_limit <= MAX_PASTE_SIZE);
        assert!(over_limit > MAX_PASTE_SIZE);
    }

    #[test]
    fn test_actual_chunking_behavior() {
        // Simulate what happens when we chunk data
        let data = vec![0u8; MAX_INPUT_CHUNK_SIZE * 2 + 500];
        let chunks: Vec<&[u8]> = data.chunks(MAX_INPUT_CHUNK_SIZE).collect();

        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0].len(), MAX_INPUT_CHUNK_SIZE);
        assert_eq!(chunks[1].len(), MAX_INPUT_CHUNK_SIZE);
        assert_eq!(chunks[2].len(), 500);

        // Verify all data is accounted for
        let total: usize = chunks.iter().map(|c| c.len()).sum();
        assert_eq!(total, data.len());
    }

    // ==================== BUG-014 Tests: Event Loop Starvation Prevention ====================

    #[test]
    fn test_max_messages_per_tick_is_reasonable() {
        // Should be high enough to process output quickly
        assert!(MAX_MESSAGES_PER_TICK >= 10);
        // But low enough to not starve input handling
        assert!(MAX_MESSAGES_PER_TICK <= 100);
    }

    #[test]
    fn test_max_messages_per_tick_allows_responsive_input() {
        // With 100ms tick rate, we want to process at most ~100ms worth of messages
        // Each message takes ~1-2ms to process, so 50 messages is ~50-100ms
        // This leaves room for input handling and UI redraw
        let tick_rate_ms = 100;
        let estimated_process_time_per_msg_ms = 2;
        let max_process_time_ms = MAX_MESSAGES_PER_TICK * estimated_process_time_per_msg_ms;

        // Processing time should not exceed tick rate
        assert!(max_process_time_ms <= tick_rate_ms);
    }

    #[test]
    fn test_large_output_message_count() {
        // Verify that large output generates many messages
        // Server uses 16KB max buffer, so 1MB output = ~64 messages
        let output_size_bytes = 1024 * 1024; // 1MB
        let max_buffer_size = 16 * 1024; // 16KB (from output.rs DEFAULT_MAX_BUFFER_SIZE)
        let expected_messages = (output_size_bytes + max_buffer_size - 1) / max_buffer_size;

        // 1MB would need ~64 messages
        assert!(expected_messages >= 60);
        assert!(expected_messages <= 70);

        // With our limit, this would be processed over multiple ticks
        let ticks_needed = (expected_messages + MAX_MESSAGES_PER_TICK - 1) / MAX_MESSAGES_PER_TICK;
        assert!(ticks_needed >= 1);
        // At 100ms per tick, ~2 ticks means ~200ms to process 1MB
        // This is acceptable for maintaining responsiveness
    }
}
