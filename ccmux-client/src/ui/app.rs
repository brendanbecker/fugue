//! Main application struct and state management
//!
//! The App struct is the central coordinator for the ccmux client UI.

// Allow unused code that's part of the public API for future features
#![allow(dead_code)]

use std::time::{Duration, Instant};

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

/// Beads status refresh interval in ticks (FEAT-058).
/// Default config is 30 seconds, tick rate is 100ms, so 300 ticks.
const BEADS_REFRESH_INTERVAL_TICKS: u64 = 300;

use crossterm::event::{Event as CrosstermEvent, KeyCode, KeyModifiers};
use ratatui::layout::Rect;
use uuid::Uuid;

use ccmux_protocol::{
    ClientMessage, ClientType, PaneState,
    ServerMessage, SplitDirection,
};
use ccmux_utils::Result;

use crate::connection::Connection;
use crate::input::{ClientCommand, InputAction, InputHandler, InputMode};

use super::event::{AppEvent, EventHandler, InputEvent};
use super::layout::{LayoutManager, LayoutPolicy, SplitDirection as LayoutSplitDirection};
use super::pane::PaneManager;
use super::state::{AppState, ClientState, MailboxMessage, ViewMode};
use super::terminal::Terminal;

/// Main application
pub struct App {
    /// Client state
    pub state: ClientState,
    /// Server connection
    connection: Connection,
    /// Event handler
    events: EventHandler,
    /// Input handler with prefix key state machine
    input_handler: InputHandler,
}

impl App {
    /// Create a new application instance
    pub fn new() -> Result<Self> {
        let events = EventHandler::new(Duration::from_millis(100));
        let client_id = Uuid::new_v4();

        Ok(Self {
            state: ClientState::new(client_id),
            connection: Connection::new(),
            events,
            input_handler: InputHandler::new(),
        })
    }

    /// Create a new application instance with a custom connection address
    pub fn with_addr(addr: String) -> Result<Self> {
        let events = EventHandler::new(Duration::from_millis(100));
        let client_id = Uuid::new_v4();

        Ok(Self {
            state: ClientState::new(client_id),
            connection: Connection::with_addr(addr),
            events,
            input_handler: InputHandler::new(),
        })
    }

    /// Create a new application instance with a custom socket path
    pub fn with_socket_path(socket_path: std::path::PathBuf) -> Result<Self> {
        let events = EventHandler::new(Duration::from_millis(100));
        let client_id = Uuid::new_v4();

        Ok(Self {
            state: ClientState::new(client_id),
            connection: Connection::with_socket_path(socket_path),
            events,
            input_handler: InputHandler::new(),
        })
    }

    /// Set the command to run in new sessions
    pub fn set_session_command(&mut self, command: Option<String>) {
        self.state.session_command = command;
    }

    /// Get current application state
    pub fn state(&self) -> AppState {
        self.state.state
    }

    /// Set quick navigation keybindings
    pub fn set_quick_bindings(&mut self, bindings: crate::input::QuickBindings) {
        self.input_handler.set_quick_bindings(bindings);
    }

    /// Check if application should quit
    pub fn should_quit(&self) -> bool {
        self.state.should_quit()
    }

    /// Run the main application loop
    pub async fn run(&mut self) -> Result<()> {
        // Initialize terminal
        let mut terminal = Terminal::new()?;
        self.state.terminal_size = terminal.size()?;

        // Start input polling
        self.events.start_input_polling();

        // Start server message forwarding
        self.start_server_message_forwarding();

        // Connect to server
        self.connect().await?;

        // Main event loop
        while !self.should_quit() {
            // Draw UI
            if self.state.needs_redraw {
                // NOTE: Commented out to fix flicker on session completion.
                // PaneStateChanged/ClaudeStateChanged set needs_redraw=true, which
                // triggered terminal.clear() causing visible flash. Ratatui's
                // differential rendering should handle layout changes without clearing.
                // If visual artifacts appear, uncomment this line.
                // terminal.clear()?;
                self.state.needs_redraw = false;
            }
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
        self.state.state = AppState::Connecting;
        self.state.status_message = Some("Connecting to server...".to_string());

        match self.connection.connect().await {
            Ok(()) => {
                // Send handshake
                self.connection
                    .send(ClientMessage::Connect {
                        client_id: self.state.client_id,
                        protocol_version: ccmux_protocol::PROTOCOL_VERSION,
                        client_type: ClientType::Tui,
                    })
                    .await?;
                Ok(())
            }
            Err(e) => {
                self.state.state = AppState::Disconnected;
                self.state.status_message = Some(format!("Failed to connect: {}", e));
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
            AppEvent::Server(msg) => self.handle_server_message(*msg).await?,
            AppEvent::Resize { cols, rows } => {
                self.state.terminal_size = (cols, rows);

                // Update layout and resize all panes
                if self.state.state == AppState::Attached {
                    // Calculate pane area (minus status bar)
                    let pane_area = Rect::new(0, 0, cols, rows.saturating_sub(1));

                    if let Some(ref layout) = self.state.layout {
                        let weights = self.state.calculate_pane_weights();
                        let pane_rects = layout.calculate_rects(pane_area, &weights);

                        // Resize each pane and notify server
                        for (pane_id, rect) in &pane_rects {
                            // Account for border (1 cell on each side)
                            let inner_width = rect.width.saturating_sub(2);
                            let inner_height = rect.height.saturating_sub(2);

                            // Resize UI pane
                            self.state.pane_manager.resize_pane(*pane_id, inner_height, inner_width);

                            // Notify server of resize for each pane
                            self.connection
                                .send(ClientMessage::Resize {
                                    pane_id: *pane_id,
                                    cols: inner_width,
                                    rows: inner_height,
                                })
                                .await?;
                        }
                    } else if let Some(pane_id) = self.state.active_pane_id {
                        // Fallback: single pane, no layout
                        let pane_rows = rows.saturating_sub(3);
                        let pane_cols = cols.saturating_sub(2);
                        self.state.pane_manager.resize_pane(pane_id, pane_rows, pane_cols);
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
                self.state.tick_count = self.state.tick_count.wrapping_add(1);
                // Poll for server messages
                self.poll_server_messages().await?;

                // FEAT-058: Periodic beads status refresh when attached
                if self.state.state == AppState::Attached {
                    let since_last_request = self.state.tick_count.saturating_sub(self.state.last_beads_request_tick);
                    if since_last_request >= BEADS_REFRESH_INTERVAL_TICKS {
                        self.request_beads_status().await?;
                    }
                }
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

    /// Request beads status for the active pane (FEAT-058)
    ///
    /// Sends a RequestBeadsStatus message to get the ready task count
    /// from the beads daemon for the active pane's working directory.
    async fn request_beads_status(&mut self) -> Result<()> {
        // Only request if we have an active pane and beads tracking is enabled
        if let Some(pane_id) = self.state.active_pane_id {
            if self.state.is_beads_tracked {
                tracing::trace!(pane_id = %pane_id, "Requesting beads status");
                self.connection
                    .send(ClientMessage::RequestBeadsStatus { pane_id })
                    .await?;
                self.state.last_beads_request_tick = self.state.tick_count;
            }
        }
        Ok(())
    }

    /// Handle input events
    async fn handle_input(&mut self, input: InputEvent) -> Result<()> {
        // Update input handler with current active pane
        self.input_handler.set_active_pane(self.state.active_pane_id);

        // Convert InputEvent to crossterm Event for the input handler
        let event = match input {
            InputEvent::Key(key) => {
                // In session select mode, handle keys directly without the prefix system
                if self.state.state == AppState::SessionSelect {
                    return self.handle_session_select_input(key).await;
                }
                
                // In Dashboard mode, handle keys directly if not a prefix combo
                if self.state.state == AppState::Attached && self.state.view_mode == ViewMode::Dashboard
                    && !self.input_handler.is_prefix_key(&key) && self.input_handler.mode() == InputMode::Normal {
                        return self.handle_dashboard_input(key).await;
                    }
                
                CrosstermEvent::Key(key)
            }
            InputEvent::Mouse(mouse) => CrosstermEvent::Mouse(mouse),
            InputEvent::FocusGained => CrosstermEvent::FocusGained,
            InputEvent::FocusLost => CrosstermEvent::FocusLost,
            InputEvent::Paste(text) => CrosstermEvent::Paste(text),
        };

        // Only use input handler when attached to a session
        if self.state.state == AppState::Attached {
            // FEAT-056: Track mode before processing
            let mode_before = self.input_handler.mode();

            let action = self.input_handler.handle_event(event);
            self.handle_input_action(action).await?;

            // FEAT-056: Check if we exited PrefixPending mode
            let mode_after = self.input_handler.mode();
            if self.state.previous_input_mode == InputMode::PrefixPending
                && mode_after != InputMode::PrefixPending
            {
                // User command mode exited (command completed, timed out, or cancelled)
                self.connection
                    .send(ClientMessage::UserCommandModeExited)
                    .await?;
                tracing::debug!("Sent UserCommandModeExited (mode: {:?} -> {:?})", mode_before, mode_after);
            }
            self.state.previous_input_mode = mode_after;
        }

        Ok(())
    }

    /// Handle input specifically for the dashboard view
    async fn handle_dashboard_input(&mut self, key: crossterm::event::KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                let i = match self.state.mailbox_state.selected() {
                    Some(i) => {
                        if i == 0 {
                            self.state.mailbox.len().saturating_sub(1)
                        } else {
                            i - 1
                        }
                    }
                    None => 0,
                };
                self.state.mailbox_state.select(Some(i));
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let i = match self.state.mailbox_state.selected() {
                    Some(i) => {
                        if i >= self.state.mailbox.len().saturating_sub(1) {
                            0
                        } else {
                            i + 1
                        }
                    }
                    None => 0,
                };
                self.state.mailbox_state.select(Some(i));
            }
            KeyCode::Enter => {
                // Jump to pane associated with the mailbox message
                if let Some(i) = self.state.mailbox_state.selected() {
                    // mailbox is rev() in draw, so index is reversed
                    let actual_idx = self.state.mailbox.len().saturating_sub(1).saturating_sub(i);
                    if let Some(msg) = self.state.mailbox.get(actual_idx) {
                        self.state.active_pane_id = Some(msg.pane_id);
                        self.state.pane_manager.set_active(msg.pane_id);
                        if let Some(ref mut layout) = self.state.layout {
                            layout.set_active_pane(msg.pane_id);
                        }
                        self.state.view_mode = ViewMode::Panes;
                        self.connection
                            .send(ClientMessage::SelectPane { pane_id: msg.pane_id })
                            .await?;
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// Handle an InputAction from the input handler
    async fn handle_input_action(&mut self, action: InputAction) -> Result<()> {
        match action {
            InputAction::None => {}

            InputAction::SendToPane(data) => {
                if let Some(pane_id) = self.state.active_pane_id {
                    // FEAT-062: Don't send input to mirror panes (read-only)
                    if self.state.pane_manager.get(pane_id).map(|p| p.is_mirror()).unwrap_or(false) {
                        // Mirror panes are read-only - ignore input
                        tracing::trace!("Ignoring input for mirror pane {}", pane_id);
                    } else {
                        // FEAT-077: Update local human control lock
                        self.state.human_control_lock_expiry = Some(Instant::now() + Duration::from_millis(2000));

                        // Small input - send directly
                        self.connection
                            .send(ClientMessage::Input { pane_id, data })
                            .await?;
                    }
                }
            }

            InputAction::PasteToPane(data) => {
                if let Some(pane_id) = self.state.active_pane_id {
                    // FEAT-062: Don't paste to mirror panes (read-only)
                    if self.state.pane_manager.get(pane_id).map(|p| p.is_mirror()).unwrap_or(false) {
                        tracing::trace!("Ignoring paste for mirror pane {}", pane_id);
                        return Ok(());
                    }

                    // FEAT-077: Update local human control lock
                    self.state.human_control_lock_expiry = Some(Instant::now() + Duration::from_millis(2000));

                    // BUG-041 FIX: Check if bracketed paste mode is enabled for this pane.
                    // If enabled, we wrap the ENTIRE paste client-side BEFORE chunking.
                    // This prevents the server from wrapping each chunk separately, which
                    // would send multiple bracketed paste sequences and crash Claude Code.
                    let use_bracketed = self.state.pane_manager
                        .get(pane_id)
                        .map(|p| p.is_bracketed_paste_enabled())
                        .unwrap_or(false);

                    let data = if use_bracketed {
                        tracing::debug!(
                            "Wrapping paste in brackets client-side for pane {} ({} bytes)",
                            pane_id,
                            data.len()
                        );
                        let mut wrapped = Vec::with_capacity(data.len() + 12);
                        wrapped.extend_from_slice(b"\x1b[200~");
                        wrapped.extend_from_slice(&data);
                        wrapped.extend_from_slice(b"\x1b[201~");
                        wrapped
                    } else {
                        data
                    };

                    // BUG-011 FIX: Handle large pastes gracefully
                    let data_len = data.len();

                    // Reject extremely large pastes to prevent memory issues
                    if data_len > MAX_PASTE_SIZE {
                        let size_mb = data_len as f64 / (1024.0 * 1024.0);
                        self.state.status_message = Some(format!(
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
                    // BUG-041 FIX: Send as Input since wrapping is already done client-side
                    if data_len > MAX_INPUT_CHUNK_SIZE {
                        let num_chunks = data_len.div_ceil(MAX_INPUT_CHUNK_SIZE);
                        tracing::debug!(
                            "Chunking large paste ({} bytes) into {} chunks",
                            data_len,
                            num_chunks
                        );

                        // Show feedback for large pastes
                        if data_len > 1024 * 1024 {
                            let size_mb = data_len as f64 / (1024.0 * 1024.0);
                            self.state.status_message = Some(format!(
                                "Pasting {:.1}MB in {} chunks...",
                                size_mb,
                                num_chunks
                            ));
                        }

                        // Send data in chunks as Input (wrapping already done)
                        for chunk in data.chunks(MAX_INPUT_CHUNK_SIZE) {
                            self.connection
                                .send(ClientMessage::Input {
                                    pane_id,
                                    data: chunk.to_vec(),
                                })
                                .await?;
                        }
                    } else {
                        // Small input - send directly as Input (wrapping already done if needed)
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
                let (cols, rows) = self.state.terminal_size;
                let pane_area = Rect::new(0, 0, cols, rows.saturating_sub(1));

                if let Some(ref layout) = self.state.layout {
                    let weights = self.state.calculate_pane_weights();
                    let pane_rects = layout.calculate_rects(pane_area, &weights);

                    // Find which pane contains the click point
                    if let Some((pane_id, _)) = pane_rects.iter().find(|(_, rect)| {
                        x >= rect.x && x < rect.x + rect.width && y >= rect.y && y < rect.y + rect.height
                    }) {
                        // Focus this pane (same logic as ClientCommand::FocusPane)
                        self.state.active_pane_id = Some(*pane_id);
                        self.state.pane_manager.set_active(*pane_id);
                        if let Some(ref mut layout) = self.state.layout {
                            layout.set_active_pane(*pane_id);
                        }
                        self.connection
                            .send(ClientMessage::SelectPane { pane_id: *pane_id })
                            .await?;
                    }
                }
            }

            InputAction::ScrollUp { lines } => {
                if let Some(pane_id) = self.state.active_pane_id {
                    // Update LOCAL UI pane for immediate visual feedback
                    if let Some(pane) = self.state.pane_manager.get_mut(pane_id) {
                        pane.scroll_up(lines);
                    }

                    // Get updated scroll offset and sync with server
                    let new_offset = self.state.pane_manager
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
                if let Some(pane_id) = self.state.active_pane_id {
                    // Update LOCAL UI pane for immediate visual feedback
                    if let Some(pane) = self.state.pane_manager.get_mut(pane_id) {
                        pane.scroll_down(lines);
                    }

                    // Get updated scroll offset and sync with server
                    let new_offset = self.state.pane_manager
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
                // FEAT-077: Update local human control lock (layout change)
                self.state.human_control_lock_expiry = Some(Instant::now() + Duration::from_millis(5000));

                self.state.terminal_size = (cols, rows);

                // Calculate pane area (minus status bar)
                let pane_area = Rect::new(0, 0, cols, rows.saturating_sub(1));

                if let Some(ref layout) = self.state.layout {
                    let weights = self.state.calculate_pane_weights();
                    let pane_rects = layout.calculate_rects(pane_area, &weights);

                    // Resize each pane and notify server
                    for (pane_id, rect) in &pane_rects {
                        // Account for border (1 cell on each side)
                        let inner_width = rect.width.saturating_sub(2);
                        let inner_height = rect.height.saturating_sub(2);

                        // Resize UI pane
                        self.state.pane_manager.resize_pane(*pane_id, inner_height, inner_width);

                        // Notify server of resize
                        self.connection
                            .send(ClientMessage::Resize {
                                pane_id: *pane_id,
                                cols: inner_width,
                                rows: inner_height,
                            })
                            .await?;
                    }
                } else if let Some(pane_id) = self.state.active_pane_id {
                    // Fallback: single pane, no layout
                    let pane_rows = rows.saturating_sub(3);
                    let pane_cols = cols.saturating_sub(2);
                    self.state.pane_manager.resize_pane(pane_id, pane_rows, pane_cols);
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
                self.state.state = AppState::SessionSelect;
                self.state.session = None;
                self.state.windows.clear();
                self.state.panes.clear();
                self.state.pane_manager = PaneManager::new();
                self.state.active_pane_id = None;
                self.state.last_pane_id = None;
                self.state.last_window_id = None;
                self.state.layout = None;
                self.state.pending_split_direction = None;
                self.state.status_message = Some("Detached from session".to_string());
                self.connection.send(ClientMessage::ListSessions).await?;
            }

            InputAction::Quit => {
                self.state.state = AppState::Quitting;
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
        // FEAT-077: Update local human control lock for layout-changing commands
        match cmd {
            ClientCommand::CreatePane
            | ClientCommand::ClosePane
            | ClientCommand::SplitVertical
            | ClientCommand::SplitHorizontal
            | ClientCommand::CreateSession(_)
            | ClientCommand::CreateWindow => {
                self.state.human_control_lock_expiry = Some(Instant::now() + Duration::from_millis(5000));
            }
            _ => {}
        }

        match cmd {
            ClientCommand::CreatePane => {
                if let Some(window) = self.state.windows.values().next() {
                    self.connection
                        .send(ClientMessage::CreatePane {
                            window_id: window.id,
                            direction: SplitDirection::Vertical,
                        })
                        .await?;
                }
            }

            ClientCommand::ClosePane => {
                if let Some(pane_id) = self.state.active_pane_id {
                    self.connection
                        .send(ClientMessage::ClosePane { pane_id })
                        .await?;
                }
            }

            ClientCommand::SplitVertical => {
                if let Some(window) = self.state.windows.values().next() {
                    // Store direction for layout update when PaneCreated is received
                    self.state.pending_split_direction = Some(SplitDirection::Vertical);
                    self.connection
                        .send(ClientMessage::CreatePane {
                            window_id: window.id,
                            direction: SplitDirection::Vertical,
                        })
                        .await?;
                }
            }

            ClientCommand::SplitHorizontal => {
                if let Some(window) = self.state.windows.values().next() {
                    // Store direction for layout update when PaneCreated is received
                    self.state.pending_split_direction = Some(SplitDirection::Horizontal);
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
                if let Some(ref mut layout) = self.state.layout {
                    layout.next_pane();
                    if let Some(new_active) = layout.active_pane_id() {
                        self.state.active_pane_id = Some(new_active);
                        self.state.pane_manager.set_active(new_active);
                    }
                } else {
                    self.cycle_pane(1);
                }
            }

            ClientCommand::PreviousPane => {
                // Use layout manager for navigation if available
                if let Some(ref mut layout) = self.state.layout {
                    layout.prev_pane();
                    if let Some(new_active) = layout.active_pane_id() {
                        self.state.active_pane_id = Some(new_active);
                        self.state.pane_manager.set_active(new_active);
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
                if let Some(pane) = self.state.panes.values().find(|p| p.index == index) {
                    let pane_id = pane.id;
                    // Track last pane before switching (only if actually changing)
                    if self.state.active_pane_id != Some(pane_id) {
                        self.state.last_pane_id = self.state.active_pane_id;
                    }
                    self.state.active_pane_id = Some(pane_id);
                    self.state.pane_manager.set_active(pane_id);
                    // Sync with layout manager
                    if let Some(ref mut layout) = self.state.layout {
                        layout.set_active_pane(pane_id);
                    }
                    self.connection
                        .send(ClientMessage::SelectPane { pane_id })
                        .await?;
                }
            }

            ClientCommand::ListSessions => {
                self.state.state = AppState::SessionSelect;
                self.connection.send(ClientMessage::ListSessions).await?;
            }

            ClientCommand::CreateSession(name) => {
                // Use CLI command only for first session, then clear it
                // Pass client's cwd so new session starts in the right directory
                let cwd = std::env::current_dir().ok().map(|p| p.to_string_lossy().into_owned());
                self.connection
                    .send(ClientMessage::CreateSessionWithOptions {
                        name,
                        command: self.state.session_command.take(),
                        cwd,
                        claude_model: None,
                        claude_config: None,
                        preset: None,
                    })
                    .await?;
            }

            ClientCommand::ListWindows => {
                // Show window list in status
                let window_names: Vec<_> = self.state.windows.values().map(|w| w.name.clone()).collect();
                self.state.status_message = Some(format!("Windows: {}", window_names.join(", ")));
            }

            ClientCommand::CreateWindow => {
                if let Some(session) = &self.state.session {
                    self.connection
                        .send(ClientMessage::CreateWindow {
                            session_id: session.id,
                            name: None,
                        })
                        .await?;
                }
            }

            ClientCommand::EnterCopyMode => {
                if let Some(pane_id) = self.state.active_pane_id {
                    if let Some(pane) = self.state.pane_manager.get_mut(pane_id) {
                        pane.enter_copy_mode();
                    }
                }
                self.state.status_message = Some("Copy mode - v: visual, V: line, hjkl: move, y: yank, q: exit".to_string());
            }

            ClientCommand::ExitCopyMode => {
                if let Some(pane_id) = self.state.active_pane_id {
                    if let Some(pane) = self.state.pane_manager.get_mut(pane_id) {
                        pane.exit_copy_mode();
                    }
                    self.connection
                        .send(ClientMessage::JumpToBottom { pane_id })
                        .await?;
                }
                self.state.status_message = None;
            }

            ClientCommand::StartVisualMode => {
                if let Some(pane_id) = self.state.active_pane_id {
                    if let Some(pane) = self.state.pane_manager.get_mut(pane_id) {
                        pane.start_visual_selection();
                    }
                }
                self.state.status_message = Some("-- VISUAL --".to_string());
            }

            ClientCommand::StartVisualLineMode => {
                if let Some(pane_id) = self.state.active_pane_id {
                    if let Some(pane) = self.state.pane_manager.get_mut(pane_id) {
                        pane.start_visual_line_selection();
                    }
                }
                self.state.status_message = Some("-- VISUAL LINE --".to_string());
            }

            ClientCommand::YankSelection => {
                if let Some(pane_id) = self.state.active_pane_id {
                    if let Some(pane) = self.state.pane_manager.get_mut(pane_id) {
                        if let Some(text) = pane.yank_selection() {
                            let len = text.len();
                            pane.exit_copy_mode();
                            self.state.status_message = Some(format!("Yanked {} bytes to clipboard", len));
                        } else {
                            // BUG-039 FIX: Always exit copy mode when yank is triggered,
                            // even if there's no selection. The input handler has already
                            // set its mode to Normal, so the pane must match.
                            pane.exit_copy_mode();
                            self.state.status_message = Some("No selection to yank".to_string());
                        }
                    }
                }
            }

            ClientCommand::MoveCopyCursor { row_delta, col_delta } => {
                if let Some(pane_id) = self.state.active_pane_id {
                    if let Some(pane) = self.state.pane_manager.get_mut(pane_id) {
                        pane.move_copy_cursor(row_delta, col_delta);
                        // Update status with cursor position
                        if let Some(cursor) = pane.copy_mode_cursor() {
                            if let Some(indicator) = pane.visual_mode_indicator() {
                                self.state.status_message = Some(format!("{} ({}, {})", indicator, cursor.row, cursor.col));
                            } else {
                                self.state.status_message = Some(format!("Copy mode ({}, {})", cursor.row, cursor.col));
                            }
                        }
                    }
                }
            }

            ClientCommand::CancelSelection => {
                if let Some(pane_id) = self.state.active_pane_id {
                    if let Some(pane) = self.state.pane_manager.get_mut(pane_id) {
                        pane.cancel_selection();
                    }
                }
                self.state.status_message = Some("Selection cancelled".to_string());
            }

            ClientCommand::MouseSelectionStart { x, y } => {
                // Translate terminal coordinates to pane-relative coordinates
                if let Some(pane_id) = self.state.active_pane_id {
                    let weights = self.state.calculate_pane_weights();
                    if let Some(pane) = self.state.pane_manager.get_mut(pane_id) {
                        // Get pane rect from layout to translate coordinates
                        if let Some(ref layout) = self.state.layout {
                            let (cols, rows) = crossterm::terminal::size().unwrap_or((80, 24));
                            let pane_area = ratatui::layout::Rect::new(0, 0, cols, rows.saturating_sub(1));
                            if let Some(rect) = layout.get_pane_rect(pane_area, pane_id, &weights) {
                                // Translate to pane-relative coordinates (accounting for border)
                                let pane_x = x.saturating_sub(rect.x + 1) as usize;
                                let pane_y = y.saturating_sub(rect.y + 1) as usize;
                                pane.mouse_selection_start(pane_y, pane_x);
                                self.state.status_message = Some("-- VISUAL --".to_string());
                            }
                        }
                    }
                }
            }

            ClientCommand::MouseSelectionUpdate { x, y } => {
                if let Some(pane_id) = self.state.active_pane_id {
                    let weights = self.state.calculate_pane_weights();
                    if let Some(pane) = self.state.pane_manager.get_mut(pane_id) {
                        if let Some(ref layout) = self.state.layout {
                            let (cols, rows) = crossterm::terminal::size().unwrap_or((80, 24));
                            let pane_area = ratatui::layout::Rect::new(0, 0, cols, rows.saturating_sub(1));
                            if let Some(rect) = layout.get_pane_rect(pane_area, pane_id, &weights) {
                                let pane_x = x.saturating_sub(rect.x + 1) as usize;
                                let pane_y = y.saturating_sub(rect.y + 1) as usize;
                                pane.mouse_selection_update(pane_y, pane_x);
                            }
                        }
                    }
                }
            }

            ClientCommand::MouseSelectionEnd { x, y } => {
                if let Some(pane_id) = self.state.active_pane_id {
                    let weights = self.state.calculate_pane_weights();
                    if let Some(pane) = self.state.pane_manager.get_mut(pane_id) {
                        if let Some(ref layout) = self.state.layout {
                            let (cols, rows) = crossterm::terminal::size().unwrap_or((80, 24));
                            let pane_area = ratatui::layout::Rect::new(0, 0, cols, rows.saturating_sub(1));
                            if let Some(rect) = layout.get_pane_rect(pane_area, pane_id, &weights) {
                                let pane_x = x.saturating_sub(rect.x + 1) as usize;
                                let pane_y = y.saturating_sub(rect.y + 1) as usize;
                                pane.mouse_selection_end(pane_y, pane_x);
                                self.state.status_message = Some("Selection complete - press 'y' to yank".to_string());
                            }
                        }
                    }
                }
            }

            ClientCommand::SelectWord { x, y } => {
                if let Some(pane_id) = self.state.active_pane_id {
                    let weights = self.state.calculate_pane_weights();
                    if let Some(pane) = self.state.pane_manager.get_mut(pane_id) {
                        if let Some(ref layout) = self.state.layout {
                            let (cols, rows) = crossterm::terminal::size().unwrap_or((80, 24));
                            let pane_area = ratatui::layout::Rect::new(0, 0, cols, rows.saturating_sub(1));
                            if let Some(rect) = layout.get_pane_rect(pane_area, pane_id, &weights) {
                                let pane_x = x.saturating_sub(rect.x + 1) as usize;
                                let pane_y = y.saturating_sub(rect.y + 1) as usize;
                                pane.select_word_at(pane_y, pane_x);
                                self.state.status_message = Some("Word selected - press 'y' to yank".to_string());
                            }
                        }
                    }
                }
            }

            ClientCommand::SelectLine { x: _, y } => {
                if let Some(pane_id) = self.state.active_pane_id {
                    let weights = self.state.calculate_pane_weights();
                    if let Some(pane) = self.state.pane_manager.get_mut(pane_id) {
                        if let Some(ref layout) = self.state.layout {
                            let (cols, rows) = crossterm::terminal::size().unwrap_or((80, 24));
                            let pane_area = ratatui::layout::Rect::new(0, 0, cols, rows.saturating_sub(1));
                            if let Some(rect) = layout.get_pane_rect(pane_area, pane_id, &weights) {
                                let pane_y = y.saturating_sub(rect.y + 1) as usize;
                                pane.select_line_at(pane_y);
                                self.state.status_message = Some("Line selected - press 'y' to yank".to_string());
                            }
                        }
                    }
                }
            }

            ClientCommand::ToggleZoom => {
                self.state.status_message = Some("Zoom toggle not yet implemented".to_string());
            }

            ClientCommand::CycleLayoutPolicy => {
                if let Some(ref mut layout) = self.state.layout {
                    let next_policy = match layout.policy() {
                        LayoutPolicy::Fixed => LayoutPolicy::Balanced,
                        LayoutPolicy::Balanced => LayoutPolicy::Adaptive,
                        LayoutPolicy::Adaptive => LayoutPolicy::Fixed,
                    };
                    layout.set_policy(next_policy);
                    self.state.status_message = Some(format!("Layout policy: {:?}", next_policy));
                    self.state.needs_redraw = true;
                }
            }

            ClientCommand::ToggleDashboard => {
                self.state.view_mode = if self.state.view_mode == ViewMode::Panes {
                    ViewMode::Dashboard
                } else {
                    ViewMode::Panes
                };
                self.state.status_message = Some(format!("View mode: {:?}", self.state.view_mode));
            }

            ClientCommand::ShowHelp => {
                self.state.status_message =
                    Some("Ctrl+B: prefix | c: new pane | x: close | n/p: next/prev".to_string());
            }

            ClientCommand::Redraw => {
                self.state.needs_redraw = true;
                self.state.status_message = Some("Screen redrawn".to_string());
                // Also notify server to signal child PTYs
                self.connection.send(ClientMessage::Redraw { pane_id: None }).await?;
            }

            ClientCommand::NextWindow => {
                self.cycle_window(1);
            }

            ClientCommand::PreviousWindow => {
                self.cycle_window(-1);
            }

            ClientCommand::LastWindow => {
                if let Some(last_id) = self.state.last_window_id {
                    // Get current window ID before switching
                    let current_window_id = self.state.active_pane_id
                        .and_then(|pid| self.state.panes.get(&pid))
                        .map(|p| p.window_id);

                    // Focus first pane in the last window
                    if let Some(pane_id) = self.first_pane_in_window(last_id) {
                        // Update last_window_id to current before switching
                        self.state.last_window_id = current_window_id;
                        self.state.active_pane_id = Some(pane_id);
                        self.state.pane_manager.set_active(pane_id);
                        // BUG-045: Rebuild layout to show only the new window's panes
                        self.rebuild_layout_for_active_window();
                    }
                } else {
                    self.state.status_message = Some("No last window".to_string());
                }
            }

            ClientCommand::LastPane => {
                if let Some(last_id) = self.state.last_pane_id {
                    if self.state.panes.contains_key(&last_id) {
                        // Save current as last before switching
                        let current = self.state.active_pane_id;
                        self.state.last_pane_id = current;
                        self.state.active_pane_id = Some(last_id);
                        self.state.pane_manager.set_active(last_id);
                        if let Some(ref mut layout) = self.state.layout {
                            layout.set_active_pane(last_id);
                        }
                    } else {
                        self.state.status_message = Some("Last pane no longer exists".to_string());
                        self.state.last_pane_id = None;
                    }
                } else {
                    self.state.status_message = Some("No last pane".to_string());
                }
            }

            ClientCommand::ShowPaneNumbers => {
                // Show pane numbers as a status message
                // In tmux, this shows an overlay with pane numbers that can be selected
                // For now, show a simple status with pane indices
                let pane_info: Vec<String> = self.state.panes
                    .values()
                    .map(|p| format!("{}", p.index))
                    .collect();
                self.state.status_message = Some(format!(
                    "Pane numbers: {} (use Ctrl-b 0-9 to select)",
                    pane_info.join(", ")
                ));
            }

            ClientCommand::SelectWindow(index) => {
                // Find window by index (sorted order)
                let mut window_ids: Vec<Uuid> = self.state.windows.keys().copied().collect();
                window_ids.sort();

                if let Some(&window_id) = window_ids.get(index) {
                    // Track current window as last before switching
                    let current_window_id = self.state.active_pane_id
                        .and_then(|pid| self.state.panes.get(&pid))
                        .map(|p| p.window_id);

                    let switching_windows = current_window_id != Some(window_id);
                    if switching_windows {
                        self.state.last_window_id = current_window_id;
                    }

                    // Focus first pane in the window
                    if let Some(pane_id) = self.first_pane_in_window(window_id) {
                        self.state.active_pane_id = Some(pane_id);
                        self.state.pane_manager.set_active(pane_id);
                        // BUG-045: Rebuild layout to show only the new window's panes
                        if switching_windows {
                            self.rebuild_layout_for_active_window();
                        } else if let Some(ref mut layout) = self.state.layout {
                            layout.set_active_pane(pane_id);
                        }
                    }
                } else {
                    self.state.status_message = Some(format!("No window at index {}", index));
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
                self.state.status_message = Some(format!("Command not yet implemented: {:?}", cmd));
            }
        }
        Ok(())
    }

    /// Cycle through panes by offset (positive = forward, negative = backward)
    fn cycle_pane(&mut self, offset: i32) {
        if self.state.panes.is_empty() {
            return;
        }

        let pane_ids: Vec<Uuid> = self.state.panes.keys().copied().collect();
        let current_index = self.state.active_pane_id
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
        if self.state.active_pane_id != Some(new_pane_id) {
            self.state.last_pane_id = self.state.active_pane_id;
        }

        self.state.active_pane_id = Some(new_pane_id);
        self.state.pane_manager.set_active(new_pane_id);
        // Sync with layout manager
        if let Some(ref mut layout) = self.state.layout {
            layout.set_active_pane(new_pane_id);
        }
    }

    /// Cycle through windows by offset (positive = forward, negative = backward)
    fn cycle_window(&mut self, offset: i32) {
        if self.state.windows.is_empty() {
            return;
        }

        // Get current window ID from active pane
        let current_window_id = self.state.active_pane_id
            .and_then(|pid| self.state.panes.get(&pid))
            .map(|p| p.window_id);

        // Get sorted list of window IDs for consistent ordering
        let mut window_ids: Vec<Uuid> = self.state.windows.keys().copied().collect();
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
            self.state.last_window_id = current_window_id;
        }

        // Focus first pane in the new window
        if let Some(pane_id) = self.first_pane_in_window(new_window_id) {
            self.state.active_pane_id = Some(pane_id);
            self.state.pane_manager.set_active(pane_id);
        }

        // BUG-045: Rebuild layout to show only the new window's panes
        if current_window_id != Some(new_window_id) {
            self.rebuild_layout_for_active_window();
        }
    }

    /// Get the first pane in a window (by index)
    fn first_pane_in_window(&self, window_id: Uuid) -> Option<Uuid> {
        self.state.panes
            .values()
            .filter(|p| p.window_id == window_id)
            .min_by_key(|p| p.index)
            .map(|p| p.id)
    }

    /// Get the active window ID (from the active pane)
    fn active_window_id(&self) -> Option<Uuid> {
        self.state.active_pane_id
            .and_then(|pid| self.state.panes.get(&pid))
            .map(|p| p.window_id)
    }

    /// Rebuild the layout manager to only include panes from the active window.
    /// This ensures windows act like tabs - only one window's panes are visible at a time.
    fn rebuild_layout_for_active_window(&mut self) {
        let active_window_id = match self.active_window_id() {
            Some(id) => id,
            None => {
                // No active window - clear layout
                self.state.layout = None;
                return;
            }
        };

        // Filter panes to only those in the active window
        let mut pane_ids: Vec<Uuid> = self.state.panes
            .values()
            .filter(|p| p.window_id == active_window_id)
            .map(|p| p.id)
            .collect();

        if pane_ids.is_empty() {
            self.state.layout = None;
            return;
        }

        // Sort by index for consistent layout ordering
        pane_ids.sort_by_key(|id| self.state.panes.get(id).map(|p| p.index).unwrap_or(0));

        // Build a new layout manager with only the active window's panes
        let first_pane_id = pane_ids[0];
        let mut layout_manager = LayoutManager::new(first_pane_id);

        // Add remaining panes as vertical splits (simple layout)
        // TODO: Persist per-window layouts in the server for better reconstruction
        for &pane_id in pane_ids.iter().skip(1) {
            layout_manager.root_mut().add_pane(
                first_pane_id,
                pane_id,
                LayoutSplitDirection::Vertical,
            );
        }

        layout_manager.set_active_pane(self.state.active_pane_id.unwrap_or(first_pane_id));
        self.state.layout = Some(layout_manager);
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
                self.state.state = AppState::Quitting;
            }
            (KeyCode::Char('q'), KeyModifiers::NONE) | (KeyCode::Esc, _) => {
                self.state.state = AppState::Quitting;
            }
            // Navigation
            (KeyCode::Up, _) | (KeyCode::Char('k'), KeyModifiers::NONE) => {
                if self.state.session_list_index > 0 {
                    self.state.session_list_index -= 1;
                }
            }
            (KeyCode::Down, _) | (KeyCode::Char('j'), KeyModifiers::NONE) => {
                if self.state.session_list_index < self.state.available_sessions.len().saturating_sub(1) {
                    self.state.session_list_index += 1;
                }
            }
            (KeyCode::Enter, _) => {
                if let Some(session) = self.state.available_sessions.get(self.state.session_list_index) {
                    self.connection
                        .send(ClientMessage::AttachSession {
                            session_id: session.id,
                        })
                        .await?;
                }
            }
            (KeyCode::Char('n'), KeyModifiers::NONE) => {
                // Create new session (CLI command only applies to first session)
                // Pass client's cwd so new session starts in the right directory
                let cwd = std::env::current_dir().ok().map(|p| p.to_string_lossy().into_owned());
                self.connection
                    .send(ClientMessage::CreateSessionWithOptions {
                        name: None,
                        command: self.state.session_command.take(),
                        cwd,
                        claude_model: None,
                        claude_config: None,
                        preset: None,
                    })
                    .await?;
            }
            (KeyCode::Char('r'), KeyModifiers::NONE) => {
                // Refresh session list
                self.connection.send(ClientMessage::ListSessions).await?;
            }
            (KeyCode::Char('d'), KeyModifiers::CONTROL) => {
                // Delete/destroy selected session
                if let Some(session) = self.state.available_sessions.get(self.state.session_list_index) {
                    let session_id = session.id;
                    self.connection
                        .send(ClientMessage::DestroySession { session_id })
                        .await?;
                    // Server will broadcast updated session list
                    // Adjust selection index if needed
                    if self.state.session_list_index > 0
                        && self.state.session_list_index >= self.state.available_sessions.len().saturating_sub(1)
                    {
                        self.state.session_list_index = self.state.session_list_index.saturating_sub(1);
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// Handle server messages
    async fn handle_server_message(&mut self, mut msg: ServerMessage) -> Result<()> {
        loop {
            tracing::debug!(
                message_type = ?std::mem::discriminant(&msg),
                "handle_server_message processing"
            );

            match msg {
                // FEAT-075: Sequence tracking and resync
                ServerMessage::Sequenced { seq, inner } => {
                    if self.state.last_seen_commit_seq > 0 && seq > self.state.last_seen_commit_seq + 1 {
                        tracing::warn!(
                            "Gap detected: last_seen={}, got {}. Requesting resync.",
                            self.state.last_seen_commit_seq,
                            seq
                        );
                        self.connection
                            .send(ClientMessage::GetEventsSince {
                                last_commit_seq: self.state.last_seen_commit_seq,
                            })
                            .await?;
                        // Drop this message as we'll get it during replay
                        return Ok(());
                    }
                    if seq > self.state.last_seen_commit_seq {
                        self.state.last_seen_commit_seq = seq;
                    }

                    // Process inner message
                    msg = *inner;
                    continue;
                }

                ServerMessage::Connected {
                    server_version,
                    protocol_version: _,
                } => {
                    self.state.status_message = Some(format!("Connected to server v{}", server_version));
                    // Request session list
                    self.connection.send(ClientMessage::ListSessions).await?;
                    self.state.state = AppState::SessionSelect;
                }
                ServerMessage::SessionList { sessions } => {

                self.state.available_sessions = sessions;
                self.state.session_list_index = 0;
            }
            // BUG-038 FIX: Handle session list change broadcasts
            // This updates the session list when sessions are created/destroyed by others
            ServerMessage::SessionsChanged { sessions } => {
                self.state.available_sessions = sessions;
                // Don't reset session_list_index to preserve user's scroll position
            }
            ServerMessage::SessionCreated { session, should_focus } => {
                if should_focus {
                    // Automatically attach to new session if we requested it
                    self.connection
                        .send(ClientMessage::AttachSession {
                            session_id: session.id,
                        })
                        .await?;
                }
            }
            ServerMessage::Attached {
                session,
                windows,
                panes,
                commit_seq,
            } => {
                self.state.last_seen_commit_seq = commit_seq;
                self.state.session = Some(session);
                self.state.windows = windows.into_iter().map(|w| (w.id, w)).collect();
                self.state.panes = panes.into_iter().map(|p| (p.id, p)).collect();
                self.state.active_pane_id = self.state.panes.keys().next().copied();
                self.state.state = AppState::Attached;
                self.state.status_message = Some("Attached to session".to_string());

                // BUG-006 FIX: Use client's terminal size, not server-reported size
                // The server's pane dimensions are from when the session was created,
                // which may differ from this client's terminal size.
                let (term_cols, term_rows) = self.state.terminal_size;
                let pane_rows = term_rows.saturating_sub(3); // Account for borders and status bar
                let pane_cols = term_cols.saturating_sub(2); // Account for side borders

                // Initialize layout manager with panes from the active window only
                // (BUG-045: windows are tabs, only show panes from active window)
                self.rebuild_layout_for_active_window();

                // Create UI panes with CLIENT's terminal dimensions
                for pane_info in self.state.panes.values() {
                    self.state.pane_manager.add_pane(pane_info.id, pane_rows, pane_cols);
                    if let Some(ui_pane) = self.state.pane_manager.get_mut(pane_info.id) {
                        ui_pane.set_title(pane_info.title.clone());
                        ui_pane.set_cwd(pane_info.cwd.clone());
                        ui_pane.set_pane_state(pane_info.state.clone());
                    }
                }

                // Send resize messages to server for all panes to sync PTY dimensions
                for pane_id in self.state.pane_manager.pane_ids() {
                    self.connection
                        .send(ClientMessage::Resize {
                            pane_id,
                            cols: pane_cols,
                            rows: pane_rows,
                        })
                        .await?;
                }

                // Set active UI pane with focus state
                if let Some(active_id) = self.state.active_pane_id {
                    self.state.pane_manager.set_active(active_id);
                }

                // FEAT-057/058: Check if session is in a beads-tracked repo
                self.state.is_beads_tracked = self.state.session
                    .as_ref()
                    .map(|s| s.metadata.contains_key("beads.root"))
                    .unwrap_or(false);

                // FEAT-058: Trigger immediate beads status request on attach
                self.state.last_beads_request_tick = 0;
                // Clear any stale beads count
                self.state.beads_ready_count = None;
            }
            ServerMessage::StateSnapshot {
                commit_seq,
                session,
                windows,
                panes,
            } => {
                tracing::info!("Received StateSnapshot (seq={})", commit_seq);
                self.state.last_seen_commit_seq = commit_seq;
                self.state.session = Some(session);
                self.state.windows = windows.into_iter().map(|w| (w.id, w)).collect();
                self.state.panes = panes.into_iter().map(|p| (p.id, p)).collect();
                self.state.active_pane_id = self.state.panes.keys().next().copied();
                self.state.state = AppState::Attached;
                self.state.status_message = Some("State resynchronized".to_string());

                // BUG-006 FIX: Use client's terminal size, not server-reported size
                let (term_cols, term_rows) = self.state.terminal_size;
                let pane_rows = term_rows.saturating_sub(3);
                let pane_cols = term_cols.saturating_sub(2);

                // Initialize layout manager with panes from the active window only
                // (BUG-045: windows are tabs, only show panes from active window)
                self.rebuild_layout_for_active_window();

                // Create UI panes with CLIENT's terminal dimensions
                // Note: We might be replacing existing panes, so we clear/recreate or update?
                // PaneManager::add_pane overwrites if exists? No, check impl.
                // Assuming it resets or we should clear first.
                // But we just updated self.state.panes.
                // Let's clear pane_manager to be safe?
                self.state.pane_manager = PaneManager::new(); // Reset UI state
                for pane_info in self.state.panes.values() {
                    self.state.pane_manager.add_pane(pane_info.id, pane_rows, pane_cols);
                    if let Some(ui_pane) = self.state.pane_manager.get_mut(pane_info.id) {
                        ui_pane.set_title(pane_info.title.clone());
                        ui_pane.set_cwd(pane_info.cwd.clone());
                        ui_pane.set_pane_state(pane_info.state.clone());
                    }
                }

                // Send resize messages to server
                for pane_id in self.state.pane_manager.pane_ids() {
                    self.connection
                        .send(ClientMessage::Resize {
                            pane_id,
                            cols: pane_cols,
                            rows: pane_rows,
                        })
                        .await?;
                }

                // Set active UI pane
                if let Some(active_id) = self.state.active_pane_id {
                    self.state.pane_manager.set_active(active_id);
                }

                // Update beads tracking
                self.state.is_beads_tracked = self.state.session
                    .as_ref()
                    .map(|s| s.metadata.contains_key("beads.root"))
                    .unwrap_or(false);

                self.state.last_beads_request_tick = 0;
                self.state.beads_ready_count = None;
            }
            ServerMessage::WindowCreated { window, should_focus: _ } => {
                self.state.windows.insert(window.id, window);
            }
            ServerMessage::PaneCreated { pane, direction, should_focus } => {
                tracing::info!(
                    pane_id = %pane.id,
                    window_id = %pane.window_id,
                    pane_index = pane.index,
                    ?direction,
                    should_focus,
                    "Handling PaneCreated broadcast from server"
                );

                // Use direction from the message (set by MCP or TUI command)
                // Clear pending_split_direction if it was set by TUI
                let _ = self.state.pending_split_direction.take();
                let layout_direction = LayoutSplitDirection::from(direction);

                // Create UI pane for terminal rendering (always do this)
                self.state.pane_manager.add_pane(pane.id, pane.rows, pane.cols);
                if let Some(ui_pane) = self.state.pane_manager.get_mut(pane.id) {
                    ui_pane.set_title(pane.title.clone());
                    ui_pane.set_cwd(pane.cwd.clone());
                    ui_pane.set_pane_state(pane.state.clone());
                }

                // Store pane info (always)
                let pane_window_id = pane.window_id;
                self.state.panes.insert(pane.id, pane.clone());

                // BUG-045: Determine if this pane is in the active window
                let active_window_id = self.active_window_id();
                let pane_in_active_window = active_window_id == Some(pane_window_id);

                if pane_in_active_window {
                    // Add new pane to layout (only if in active window)
                    if let Some(ref mut layout) = self.state.layout {
                        // Split the active pane to add the new one
                        if let Some(active_id) = self.state.active_pane_id {
                            layout.root_mut().add_pane(active_id, pane.id, layout_direction);
                        } else {
                            // No active pane - this is the first pane, initialize layout
                            *layout = LayoutManager::new(pane.id);
                        }
                    } else {
                        // Layout not initialized - create with this pane
                        self.state.layout = Some(LayoutManager::new(pane.id));
                    }
                }

                // Switch focus to the new pane if requested
                if should_focus {
                    self.state.active_pane_id = Some(pane.id);
                    self.state.pane_manager.set_active(pane.id);

                    // If the pane is in a different window, rebuild layout for that window
                    if !pane_in_active_window {
                        self.rebuild_layout_for_active_window();
                    } else if let Some(ref mut layout) = self.state.layout {
                        layout.set_active_pane(pane.id);
                    }
                }

                // Resize all panes after layout change
                let (cols, rows) = self.state.terminal_size;
                let pane_area = Rect::new(0, 0, cols, rows.saturating_sub(1));

                if let Some(ref layout) = self.state.layout {
                    let weights = self.state.calculate_pane_weights();
                    let pane_rects = layout.calculate_rects(pane_area, &weights);

                    for (pane_id, rect) in &pane_rects {
                        let inner_width = rect.width.saturating_sub(2);
                        let inner_height = rect.height.saturating_sub(2);

                        self.state.pane_manager.resize_pane(*pane_id, inner_height, inner_width);

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
                self.state.pane_manager.process_output(pane_id, &data);

                // FEAT-062: Forward output to any mirror panes of this source
                let mirror_ids: Vec<Uuid> = self.state.panes
                    .iter()
                    .filter_map(|(id, info)| {
                        if info.mirror_source == Some(pane_id) {
                            Some(*id)
                        } else {
                            None
                        }
                    })
                    .collect();

                for mirror_id in mirror_ids {
                    self.state.pane_manager.process_output(mirror_id, &data);
                }
            }
            ServerMessage::PaneStateChanged { pane_id, state } => {
                if let Some(pane) = self.state.panes.get_mut(&pane_id) {
                    pane.state = state.clone();
                }
                // Sync state with UI pane
                self.state.pane_manager.update_pane_state(pane_id, state);
                self.state.needs_redraw = true;
            }
            ServerMessage::ClaudeStateChanged { pane_id, state } => {
                // Convert ClaudeState to AgentState
                let pane_state = PaneState::Agent(state.into());
                if let Some(pane) = self.state.panes.get_mut(&pane_id) {
                    pane.state = pane_state.clone();
                }
                // Sync state with UI pane
                self.state.pane_manager.update_pane_state(pane_id, pane_state);
                self.state.needs_redraw = true;
            }
            ServerMessage::PaneClosed { pane_id, .. } => {
                self.state.panes.remove(&pane_id);
                self.state.pane_manager.remove_pane(pane_id);

                // Remove from layout (which also prunes single-child splits)
                if let Some(ref mut layout) = self.state.layout {
                    layout.remove_pane(pane_id);
                }

                if self.state.active_pane_id == Some(pane_id) {
                    // Get new active pane from layout or fallback to panes
                    let new_active = self.state.layout
                        .as_ref()
                        .and_then(|l| l.active_pane_id())
                        .or_else(|| self.state.panes.keys().next().copied());

                    self.state.active_pane_id = new_active;
                    // Update active UI pane and layout
                    if let Some(id) = new_active {
                        self.state.pane_manager.set_active(id);
                        if let Some(ref mut layout) = self.state.layout {
                            layout.set_active_pane(id);
                        }
                    }
                }

                // BUG-015 FIX: Recalculate layout and resize remaining panes
                // After removing a pane, remaining panes should expand to fill available space
                if !self.state.panes.is_empty() {
                    let (cols, rows) = self.state.terminal_size;
                    let pane_area = Rect::new(0, 0, cols, rows.saturating_sub(1));

                    if let Some(ref layout) = self.state.layout {
                        let weights = self.state.calculate_pane_weights();
                        let pane_rects = layout.calculate_rects(pane_area, &weights);

                        for (remaining_pane_id, rect) in &pane_rects {
                            let inner_width = rect.width.saturating_sub(2);
                            let inner_height = rect.height.saturating_sub(2);

                            // Resize UI pane to new dimensions
                            self.state.pane_manager
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
                if self.state.panes.is_empty() {
                    self.state.session = None;
                    self.state.windows.clear();
                    self.state.active_pane_id = None;
                    self.state.layout = None;
                    self.state.state = AppState::SessionSelect;
                    self.state.status_message = Some("Session has no active panes".to_string());
                }
            }
            ServerMessage::WindowClosed { window_id } => {
                self.state.windows.remove(&window_id);
            }
            ServerMessage::SessionEnded { .. } => {
                self.state.session = None;
                self.state.windows.clear();
                self.state.panes.clear();
                self.state.pane_manager = PaneManager::new();
                self.state.active_pane_id = None;
                self.state.last_pane_id = None;
                self.state.last_window_id = None;
                self.state.layout = None;
                self.state.pending_split_direction = None;
                self.state.state = AppState::SessionSelect;
                self.state.status_message = Some("Session ended".to_string());
                // Refresh session list
                self.connection.send(ClientMessage::ListSessions).await?;
            }
            ServerMessage::Error { code, message, details } => {
                self.state.status_message = Some(format!("Error ({:?}): {}", code, message));
                
                if let Some(ccmux_protocol::messages::ErrorDetails::HumanControl { remaining_ms }) = details {
                    self.state.human_control_lock_expiry = Some(Instant::now() + Duration::from_millis(remaining_ms));
                }
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
                self.state.status_message = Some(format!(
                    "Reply delivered ({} bytes)",
                    result.bytes_written
                ));
            }
            ServerMessage::OrchestrationReceived { from_session_id, message } => {
                // Received orchestration message from another session
                // TODO: Handle orchestration messages in UI
                let _ = (from_session_id, message);
            }
            ServerMessage::MailReceived { pane_id, priority, summary } => {
                let msg = MailboxMessage {
                    pane_id,
                    timestamp: std::time::SystemTime::now(),
                    priority,
                    summary,
                };
                self.state.mailbox.push(msg);
                // Keep mailbox size reasonable (e.g., last 100 messages)
                if self.state.mailbox.len() > 100 {
                    self.state.mailbox.remove(0);
                }
                
                // If we are in Dashboard view, ensure selection is valid
                if self.state.view_mode == ViewMode::Dashboard && self.state.mailbox_state.selected().is_none() {
                    self.state.mailbox_state.select(Some(0));
                }
            }
            ServerMessage::OrchestrationDelivered { delivered_count } => {
                // Orchestration message was delivered to other sessions
                self.state.status_message = Some(format!(
                    "Message delivered to {} session(s)",
                    delivered_count
                ));
            }
            // BUG-026 FIX: Focus change broadcasts from MCP commands
            // BUG-036 FIX: Switch sessions when focusing pane in different session
            ServerMessage::PaneFocused { session_id, window_id, pane_id } => {
                let should_switch = match &self.state.session {
                    Some(current) => current.id != session_id,
                    None => true,
                };

                if should_switch {
                    tracing::debug!("Switching to session {} for pane {} (via MCP)", session_id, pane_id);
                    self.connection
                        .send(ClientMessage::AttachSession { session_id })
                        .await?;
                } else {
                    // Update active pane if we know about this pane
                    if self.state.panes.contains_key(&pane_id) {
                        self.state.active_pane_id = Some(pane_id);
                        tracing::debug!("Focus changed to pane {} (via MCP)", pane_id);
                        // If the window is known, ensure it's the active window display
                        if let Some(window) = self.state.windows.get_mut(&window_id) {
                            window.active_pane_id = Some(pane_id);
                        }
                    }
                }
            }
            ServerMessage::WindowFocused { session_id, window_id } => {
                // BUG-036 FIX: Switch to the session if different from current
                let should_switch = match &self.state.session {
                    Some(current) => current.id != session_id,
                    None => true,
                };

                if should_switch {
                    tracing::debug!("Switching to session {} for window {} (via MCP)", session_id, window_id);
                    self.connection
                        .send(ClientMessage::AttachSession { session_id })
                        .await?;
                } else {
                    // Get current window before switching
                    let current_window_id = self.active_window_id();
                    let switching_windows = current_window_id != Some(window_id);

                    // Update active window - focus its active pane
                    if let Some(window) = self.state.windows.get(&window_id) {
                        if let Some(active_pane) = window.active_pane_id {
                            self.state.active_pane_id = Some(active_pane);
                            tracing::debug!("Window {} focused, now focusing pane {} (via MCP)", window_id, active_pane);
                        }
                    }

                    // BUG-045: Rebuild layout to show only the new window's panes
                    if switching_windows {
                        self.state.last_window_id = current_window_id;
                        self.rebuild_layout_for_active_window();
                    }
                }
            }
            ServerMessage::SessionFocused { session_id } => {
                // BUG-036 FIX: Switch to the focused session if different from current
                let should_switch = match &self.state.session {
                    Some(current) => current.id != session_id,
                    None => true,
                };

                if should_switch {
                    tracing::debug!("Switching to focused session {} (via MCP)", session_id);
                    self.connection
                        .send(ClientMessage::AttachSession { session_id })
                        .await?;
                } else {
                    tracing::debug!("Our session {} is now the active session (via MCP)", session_id);
                }
            }

            ServerMessage::SessionRenamed { session_id, new_name, .. } => {
                if let Some(session) = &mut self.state.session {
                    if session.id == session_id {
                        session.name = new_name;
                    }
                }
            }

            ServerMessage::WindowRenamed { window_id, new_name, .. } => {
                if let Some(window) = self.state.windows.get_mut(&window_id) {
                    window.name = new_name;
                }
            }

            ServerMessage::PaneRenamed { pane_id, new_name, .. } => {
                if let Some(pane) = self.state.panes.get_mut(&pane_id) {
                    pane.name = Some(new_name.clone());
                }
                if let Some(ui_pane) = self.state.pane_manager.get_mut(pane_id) {
                    ui_pane.set_title(Some(new_name));
                }
            }

            ServerMessage::PaneResized { pane_id, new_cols, new_rows } => {
                // Update local pane dimensions
                if let Some(pane) = self.state.panes.get_mut(&pane_id) {
                    pane.cols = new_cols;
                    pane.rows = new_rows;
                }
                // Resize the UI pane
                self.state.pane_manager.resize_pane(pane_id, new_rows, new_cols);
                
                // Trigger a full layout recalculation on next draw
            }

            ServerMessage::PaneSplit { new_pane_id, .. } => {
                // Usually followed by PaneCreated, but we could handle it here if needed.
                // For now, let PaneCreated handle the UI update.
                let _ = new_pane_id;
            }

            ServerMessage::SessionDestroyed { .. } => {
                // Handled by SessionEnded?
            }

            // FEAT-062: Mirror pane created
            ServerMessage::MirrorCreated {
                mirror_pane,
                source_pane_id,
                direction,
                should_focus: _,
                ..
            } => {
                tracing::info!(
                    mirror_id = %mirror_pane.id,
                    source_id = %source_pane_id,
                    ?direction,
                    "Handling MirrorCreated broadcast from server"
                );

                let layout_direction = LayoutSplitDirection::from(direction);

                // Create UI pane for the mirror
                self.state.pane_manager.add_pane(mirror_pane.id, mirror_pane.rows, mirror_pane.cols);
                if let Some(ui_pane) = self.state.pane_manager.get_mut(mirror_pane.id) {
                    // Set title to indicate this is a mirror
                    let title = format!("[MIRROR: {}]", source_pane_id.to_string().split('-').next().unwrap_or("?"));
                    ui_pane.set_title(Some(title));
                    // FEAT-062: Mark as mirror for styling and read-only handling
                    ui_pane.set_is_mirror(true);
                }

                // Store pane info
                let pane_window_id = mirror_pane.window_id;
                self.state.panes.insert(mirror_pane.id, mirror_pane.clone());

                // Add to layout if in active window
                let active_window_id = self.active_window_id();
                let pane_in_active_window = active_window_id == Some(pane_window_id);

                if pane_in_active_window {
                    if let Some(ref mut layout) = self.state.layout {
                        // Find a suitable reference pane for the split
                        let ref_pane = self.state.active_pane_id.or_else(|| {
                            layout.root().pane_ids().first().copied()
                        });

                        if let Some(ref_id) = ref_pane {
                            layout.root_mut().add_pane(ref_id, mirror_pane.id, layout_direction);
                        }
                    }
                }

                // Note: Mirror will receive output via forwarding as source produces it
                // Initial content sync could be added later if needed

                self.state.needs_redraw = true;
            }

            // FEAT-062: Mirror source closed
            ServerMessage::MirrorSourceClosed {
                mirror_pane_id,
                source_pane_id,
                exit_code,
            } => {
                tracing::info!(
                    mirror_id = %mirror_pane_id,
                    source_id = %source_pane_id,
                    ?exit_code,
                    "Mirror source pane closed"
                );

                // Display a message in the mirror pane
                if let Some(_ui_pane) = self.state.pane_manager.get_mut(mirror_pane_id) {
                    let msg = format!(
                        "\r\n\x1b[1;33m[Source pane closed{}]\x1b[0m\r\n\x1b[2mPress 'q' or Escape to close this mirror\x1b[0m\r\n",
                        exit_code.map(|c| format!(" with exit code {}", c)).unwrap_or_default()
                    );
                    self.state.pane_manager.process_output(mirror_pane_id, msg.as_bytes());
                }

                self.state.needs_redraw = true;
            }

            // MCP bridge messages - not used by TUI client
            ServerMessage::AllPanesList { .. }
            | ServerMessage::WindowList { .. }
            | ServerMessage::PaneContent { .. }
            | ServerMessage::PaneStatus { .. }
            | ServerMessage::PaneCreatedWithDetails { .. }
            | ServerMessage::WindowCreatedWithDetails { .. }
            | ServerMessage::LayoutCreated { .. }
            | ServerMessage::SessionCreatedWithDetails { .. }
            | ServerMessage::EnvironmentSet { .. }
            | ServerMessage::EnvironmentList { .. }
            | ServerMessage::MetadataSet { .. }
            | ServerMessage::MetadataList { .. }
            | ServerMessage::TagsSet { .. }
            | ServerMessage::TagsList { .. }
            | ServerMessage::ServerStatus { .. }
            | ServerMessage::WorkerStatus { .. }
            | ServerMessage::MessagesPolled { .. } => {
                // These messages are for the MCP bridge or observability, not the TUI client
            }

            // FEAT-058: Beads status updates
            ServerMessage::BeadsStatusUpdate { pane_id, status } => {
                // Update status bar if this is the active pane
                if Some(pane_id) == self.state.active_pane_id {
                    if status.daemon_available {
                        self.state.beads_ready_count = Some(status.ready_count);
                    } else {
                        // Daemon not available - clear count so we fall back to basic "beads" indicator
                        self.state.beads_ready_count = None;
                    }
                }
            }

            ServerMessage::BeadsReadyList { pane_id: _, tasks: _ } => {
                // TODO: Implement beads panel display (FEAT-058 Section 4)
                // For now, this message is received but not displayed
            }

            // FEAT-083: Generic widget updates
            ServerMessage::WidgetUpdate { pane_id, update } => {
                // Handle widget updates based on type
                // For backward compatibility, beads.status updates are processed
                // the same way as BeadsStatusUpdate
                if update.update_type == "beads.status"
                    && Some(pane_id) == self.state.active_pane_id {
                        if update.metadata()["daemon_available"].as_bool().unwrap_or(false) {
                            if let Some(count) = update.metadata()["ready_count"].as_u64() {
                                self.state.beads_ready_count = Some(count as usize);
                            }
                        } else {
                            self.state.beads_ready_count = None;
                        }
                    }
                // Other widget types can be handled here in the future
            }

            // FEAT-104: Watchdog timer responses (only handled by MCP bridge)
            ServerMessage::WatchdogStarted { .. } => {}
            ServerMessage::WatchdogStopped => {}
            ServerMessage::WatchdogStatusResponse { .. } => {}
        }
        break;
    }
    Ok(())
}    /// Draw the UI
    fn draw(&mut self, terminal: &mut Terminal) -> Result<()> {
        // For attached state, update pane layout before drawing
        if self.state.state == AppState::Attached {
            self.state.update_pane_layout(terminal.size()?);
        }
        
        // Construct input status string
        let input_status = match self.input_handler.mode() {
            InputMode::Normal => "".to_string(),
            InputMode::PrefixPending => " [PREFIX]".to_string(),
            InputMode::Command => format!(" :{}", self.input_handler.command_buffer()),
            InputMode::Copy => format!(" [COPY +{}]", self.input_handler.scroll_offset()),
        };

        terminal.terminal_mut().draw(|frame| {
             super::render::draw(&mut self.state, frame, &input_status);
        })?;
        Ok(())
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new().expect("Failed to create App")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_state_default() {
        // Note: Can't actually test App::new() in unit tests as it needs terminal
        assert_eq!(AppState::Disconnected, AppState::Disconnected);
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
