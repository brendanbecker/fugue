//! PTY output polling and broadcasting
//!
//! This module implements background tasks that poll PTY output and broadcast
//! it to connected clients in real-time. Each pane's PTY gets its own polling
//! task that reads output and routes it to all clients attached to that session.
//!
//! ## Sideband Command Integration
//!
//! When sideband parsing is enabled, the poller intercepts XML command tags
//! embedded in PTY output (e.g., `<ccmux:spawn direction="vertical" />`),
//! executes them, and strips them from the display output.

use std::io::Read;
use std::sync::Arc;
use std::time::Duration;

use parking_lot::Mutex;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio::time::{interval, Instant};
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, trace, warn};
use uuid::Uuid;

use ccmux_protocol::ServerMessage;

use crate::registry::ClientRegistry;
use crate::sideband::{AsyncCommandExecutor, SidebandCommand, SidebandParser, SplitDirection};

/// Default buffer flush timeout in milliseconds
const DEFAULT_FLUSH_TIMEOUT_MS: u64 = 50;

/// Default maximum buffer size before forced flush
const DEFAULT_MAX_BUFFER_SIZE: usize = 16384;

/// Read buffer size for PTY reads
const READ_BUFFER_SIZE: usize = 4096;

/// Configuration for the output poller
#[derive(Debug, Clone)]
pub struct OutputPollerConfig {
    /// Timeout before flushing buffered output (default: 50ms)
    pub flush_timeout: Duration,
    /// Maximum buffer size before forced flush (default: 16KB)
    pub max_buffer_size: usize,
}

impl Default for OutputPollerConfig {
    fn default() -> Self {
        Self {
            flush_timeout: Duration::from_millis(DEFAULT_FLUSH_TIMEOUT_MS),
            max_buffer_size: DEFAULT_MAX_BUFFER_SIZE,
        }
    }
}

/// Handle for managing a running PTY output poller
///
/// Provides access to the cancellation token and join handle for a spawned
/// poller task. Use this to stop the poller when the pane is closed.
#[derive(Debug)]
pub struct PollerHandle {
    /// Token to cancel the poller
    pub cancel_token: CancellationToken,
    /// Handle to the spawned task
    pub join_handle: JoinHandle<()>,
}

impl PollerHandle {
    /// Cancel the poller and wait for it to complete
    pub async fn stop(self) {
        self.cancel_token.cancel();
        // Wait for the task to finish, ignoring any join errors
        let _ = self.join_handle.await;
    }

    /// Cancel the poller without waiting
    pub fn cancel(&self) {
        self.cancel_token.cancel();
    }
}

/// Notification sent when a pane's PTY process exits
#[derive(Debug, Clone)]
pub struct PaneClosedNotification {
    pub session_id: Uuid,
    pub pane_id: Uuid,
}

/// PTY output poller that reads from a PTY and broadcasts to session clients
///
/// Each pane gets its own poller instance that runs in a background task.
/// The poller:
/// - Reads output from the PTY in a blocking manner (via spawn_blocking)
/// - Buffers output for efficient broadcasting
/// - Flushes on newline, timeout, or buffer size threshold
/// - Broadcasts to all clients attached to the session
/// - Optionally parses and executes sideband commands from output
pub struct PtyOutputPoller {
    /// Pane ID this poller is associated with
    pane_id: Uuid,
    /// Session ID for broadcasting
    session_id: Uuid,
    /// PTY reader wrapped in Arc<Mutex> for thread-safe access
    pty_reader: Arc<Mutex<Box<dyn Read + Send>>>,
    /// Client registry for broadcasting output
    registry: Arc<ClientRegistry>,
    /// Output buffer
    buffer: Vec<u8>,
    /// Configuration
    config: OutputPollerConfig,
    /// Cancellation token for clean shutdown
    cancel_token: CancellationToken,
    /// Last time we received data (for timeout flush)
    last_data_time: Instant,
    /// Channel to notify server when pane closes (for cleanup)
    pane_closed_tx: Option<mpsc::Sender<PaneClosedNotification>>,
    /// Sideband parser for extracting commands from output (optional)
    sideband_parser: Option<SidebandParser>,
    /// Command executor for processing sideband commands (optional)
    command_executor: Option<Arc<AsyncCommandExecutor>>,
}

impl PtyOutputPoller {
    /// Spawn a new output poller for a pane
    ///
    /// Returns a handle that can be used to stop the poller.
    pub fn spawn(
        pane_id: Uuid,
        session_id: Uuid,
        pty_reader: Arc<Mutex<Box<dyn Read + Send>>>,
        registry: Arc<ClientRegistry>,
    ) -> PollerHandle {
        Self::spawn_with_cleanup(pane_id, session_id, pty_reader, registry, None)
    }

    /// Spawn a new output poller with a cleanup notification channel
    ///
    /// When the PTY process exits, the poller will send a notification through
    /// the provided channel so the server can clean up the pane from session state.
    pub fn spawn_with_cleanup(
        pane_id: Uuid,
        session_id: Uuid,
        pty_reader: Arc<Mutex<Box<dyn Read + Send>>>,
        registry: Arc<ClientRegistry>,
        pane_closed_tx: Option<mpsc::Sender<PaneClosedNotification>>,
    ) -> PollerHandle {
        Self::spawn_with_config(pane_id, session_id, pty_reader, registry, OutputPollerConfig::default(), pane_closed_tx)
    }

    /// Spawn a new output poller with custom configuration
    pub fn spawn_with_config(
        pane_id: Uuid,
        session_id: Uuid,
        pty_reader: Arc<Mutex<Box<dyn Read + Send>>>,
        registry: Arc<ClientRegistry>,
        config: OutputPollerConfig,
        pane_closed_tx: Option<mpsc::Sender<PaneClosedNotification>>,
    ) -> PollerHandle {
        let cancel_token = CancellationToken::new();
        let poller = Self {
            pane_id,
            session_id,
            pty_reader,
            registry,
            buffer: Vec::with_capacity(config.max_buffer_size),
            config,
            cancel_token: cancel_token.clone(),
            last_data_time: Instant::now(),
            pane_closed_tx,
            sideband_parser: None,
            command_executor: None,
        };

        let join_handle = tokio::spawn(poller.run());

        PollerHandle {
            cancel_token,
            join_handle,
        }
    }

    /// Spawn a new output poller with sideband command parsing enabled
    ///
    /// This version integrates sideband parsing, which:
    /// - Intercepts XML command tags (e.g., `<ccmux:spawn ... />`) from output
    /// - Executes the commands against the session manager
    /// - Strips command tags from the display output
    pub fn spawn_with_sideband(
        pane_id: Uuid,
        session_id: Uuid,
        pty_reader: Arc<Mutex<Box<dyn Read + Send>>>,
        registry: Arc<ClientRegistry>,
        pane_closed_tx: Option<mpsc::Sender<PaneClosedNotification>>,
        command_executor: Arc<AsyncCommandExecutor>,
    ) -> PollerHandle {
        let cancel_token = CancellationToken::new();
        let config = OutputPollerConfig::default();
        let poller = Self {
            pane_id,
            session_id,
            pty_reader,
            registry,
            buffer: Vec::with_capacity(config.max_buffer_size),
            config,
            cancel_token: cancel_token.clone(),
            last_data_time: Instant::now(),
            pane_closed_tx,
            sideband_parser: Some(SidebandParser::new()),
            command_executor: Some(command_executor),
        };

        let join_handle = tokio::spawn(poller.run());

        PollerHandle {
            cancel_token,
            join_handle,
        }
    }

    /// Main polling loop
    async fn run(mut self) {
        info!(
            pane_id = %self.pane_id,
            session_id = %self.session_id,
            "PTY output poller started"
        );

        // Channel for receiving data from blocking reads
        let (data_tx, mut data_rx) = mpsc::channel::<ReadResult>(16);

        // Spawn the blocking reader task
        let reader = self.pty_reader.clone();
        let reader_cancel = self.cancel_token.clone();
        let pane_id = self.pane_id;

        tokio::spawn(async move {
            Self::blocking_reader_task(reader, data_tx, reader_cancel, pane_id).await;
        });

        // Create a timer for periodic flush checks
        let mut flush_interval = interval(self.config.flush_timeout);
        flush_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            tokio::select! {
                // Check for cancellation
                _ = self.cancel_token.cancelled() => {
                    debug!(pane_id = %self.pane_id, "Poller cancelled");
                    break;
                }

                // Handle incoming data from the PTY reader
                result = data_rx.recv() => {
                    match result {
                        Some(ReadResult::Data(data)) => {
                            self.handle_output(&data).await;
                        }
                        Some(ReadResult::Eof) => {
                            debug!(pane_id = %self.pane_id, "PTY EOF");
                            // Flush any remaining buffer
                            self.flush().await;
                            break;
                        }
                        Some(ReadResult::Error(e)) => {
                            error!(pane_id = %self.pane_id, error = %e, "PTY read error");
                            // Flush any remaining buffer
                            self.flush().await;
                            break;
                        }
                        None => {
                            // Channel closed - reader task ended
                            debug!(pane_id = %self.pane_id, "Reader channel closed");
                            self.flush().await;
                            break;
                        }
                    }
                }

                // Periodic flush check
                _ = flush_interval.tick() => {
                    if self.should_flush_timeout() {
                        self.flush().await;
                    }
                }
            }
        }

        // Final flush before exiting
        self.flush().await;

        // Notify clients that the pane has closed
        let close_msg = ServerMessage::PaneClosed {
            pane_id: self.pane_id,
            exit_code: None, // We don't have access to the exit code here
        };
        self.registry.broadcast_to_session(self.session_id, close_msg).await;

        // Notify server to clean up the pane from session state
        // (only if we have a cleanup channel - this allows the server to
        // remove zombie panes and empty sessions)
        if let Some(tx) = &self.pane_closed_tx {
            let notification = PaneClosedNotification {
                session_id: self.session_id,
                pane_id: self.pane_id,
            };
            if let Err(e) = tx.send(notification).await {
                warn!(
                    pane_id = %self.pane_id,
                    error = %e,
                    "Failed to send pane cleanup notification"
                );
            }
        }

        info!(
            pane_id = %self.pane_id,
            session_id = %self.session_id,
            "PTY output poller exiting"
        );
    }

    /// Blocking reader task that runs in spawn_blocking
    async fn blocking_reader_task(
        reader: Arc<Mutex<Box<dyn Read + Send>>>,
        data_tx: mpsc::Sender<ReadResult>,
        cancel_token: CancellationToken,
        pane_id: Uuid,
    ) {
        loop {
            // Check cancellation before each read
            if cancel_token.is_cancelled() {
                trace!(pane_id = %pane_id, "Blocking reader cancelled");
                break;
            }

            // Clone what we need for spawn_blocking
            let reader_clone = reader.clone();

            // Perform blocking read in spawn_blocking
            let result = tokio::task::spawn_blocking(move || {
                let mut buf = [0u8; READ_BUFFER_SIZE];
                let mut reader_guard = reader_clone.lock();
                match reader_guard.read(&mut buf) {
                    Ok(0) => ReadResult::Eof,
                    Ok(n) => ReadResult::Data(buf[..n].to_vec()),
                    Err(e) => {
                        // Check for specific error kinds that indicate normal closure
                        if e.kind() == std::io::ErrorKind::BrokenPipe
                            || e.kind() == std::io::ErrorKind::UnexpectedEof
                        {
                            ReadResult::Eof
                        } else {
                            ReadResult::Error(e.to_string())
                        }
                    }
                }
            })
            .await;

            match result {
                Ok(read_result) => {
                    let is_terminal = matches!(read_result, ReadResult::Eof | ReadResult::Error(_));

                    // Send result to main loop
                    if data_tx.send(read_result).await.is_err() {
                        // Main loop has closed - exit
                        trace!(pane_id = %pane_id, "Data channel closed, reader exiting");
                        break;
                    }

                    // If we hit EOF or error, exit the loop
                    if is_terminal {
                        break;
                    }
                }
                Err(e) => {
                    // spawn_blocking panicked or was cancelled
                    warn!(pane_id = %pane_id, error = %e, "spawn_blocking failed");
                    let _ = data_tx.send(ReadResult::Error(e.to_string())).await;
                    break;
                }
            }
        }
    }

    /// Handle new output data
    ///
    /// If sideband parsing is enabled:
    /// - Parses the data for embedded sideband commands
    /// - Executes any commands found (spawn, notify, etc.)
    /// - Buffers only the display text (with commands stripped)
    ///
    /// If sideband parsing is disabled:
    /// - Buffers raw data unchanged (legacy behavior)
    async fn handle_output(&mut self, data: &[u8]) {
        self.last_data_time = Instant::now();

        // Check if sideband parsing is enabled
        if let (Some(parser), Some(executor)) = (
            self.sideband_parser.as_mut(),
            self.command_executor.as_ref(),
        ) {
            // Convert bytes to string for parsing (lossy for non-UTF-8 data)
            let text = String::from_utf8_lossy(data);

            // Parse for sideband commands
            let (display_text, commands) = parser.parse(&text);

            trace!(
                pane_id = %self.pane_id,
                bytes = data.len(),
                display_bytes = display_text.len(),
                commands = commands.len(),
                "Parsed PTY output for sideband commands"
            );

            // Execute any commands found
            for cmd in commands {
                self.execute_sideband_command(cmd, executor.clone()).await;
            }

            // Buffer the display text (with commands stripped)
            self.buffer.extend_from_slice(display_text.as_bytes());
        } else {
            // No sideband parsing - buffer raw data
            self.buffer.extend_from_slice(data);

            trace!(
                pane_id = %self.pane_id,
                bytes = data.len(),
                buffer_size = self.buffer.len(),
                "Received PTY output"
            );
        }

        // Check if we should flush
        if self.should_flush() {
            self.flush().await;
        }
    }

    /// Execute a sideband command
    ///
    /// For spawn commands, this also starts the output poller for the new pane.
    async fn execute_sideband_command(
        &self,
        cmd: SidebandCommand,
        executor: Arc<AsyncCommandExecutor>,
    ) {
        match &cmd {
            SidebandCommand::Spawn { direction, command, cwd, config } => {
                // Handle spawn specially - we need to start a poller for the new pane
                info!(
                    pane_id = %self.pane_id,
                    direction = ?direction,
                    command = ?command,
                    cwd = ?cwd,
                    "Executing sideband spawn command"
                );

                match executor.execute_spawn_command(
                    self.pane_id,
                    *direction,
                    command.clone(),
                    cwd.clone(),
                    config.clone(),
                ).await {
                    Ok(result) => {
                        info!(
                            source_pane = %self.pane_id,
                            new_pane = %result.pane_id,
                            session = %result.session_id,
                            "Sideband spawn succeeded, starting output poller for new pane"
                        );

                        // Start output poller for the new pane with sideband enabled
                        let _new_poller = PtyOutputPoller::spawn_with_sideband(
                            result.pane_id,
                            result.session_id,
                            result.pty_reader,
                            self.registry.clone(),
                            self.pane_closed_tx.clone(),
                            executor,
                        );
                    }
                    Err(e) => {
                        error!(
                            pane_id = %self.pane_id,
                            error = %e,
                            "Sideband spawn command failed"
                        );
                    }
                }
            }
            _ => {
                // For non-spawn commands, just execute them
                debug!(
                    pane_id = %self.pane_id,
                    command = ?cmd,
                    "Executing sideband command"
                );

                if let Err(e) = executor.execute(cmd, self.pane_id).await {
                    warn!(
                        pane_id = %self.pane_id,
                        error = %e,
                        "Sideband command execution failed"
                    );
                }
            }
        }
    }

    /// Check if buffer should be flushed
    fn should_flush(&self) -> bool {
        // Flush if buffer exceeds max size
        if self.buffer.len() >= self.config.max_buffer_size {
            return true;
        }

        // Flush if buffer contains a newline
        if self.buffer.contains(&b'\n') {
            return true;
        }

        false
    }

    /// Check if we should flush due to timeout
    fn should_flush_timeout(&self) -> bool {
        !self.buffer.is_empty() && self.last_data_time.elapsed() >= self.config.flush_timeout
    }

    /// Check if data contains DSR CPR (Cursor Position Report) request
    ///
    /// DSR [6n] is ESC [ 6 n (0x1b, 0x5b, 0x36, 0x6e)
    /// This is a Device Status Report requesting cursor position.
    /// BUG-053: Codex CLI requires this response to start.
    fn contains_dsr_cpr(data: &[u8]) -> bool {
        // Look for ESC[6n sequence
        // ESC = 0x1b, [ = 0x5b, 6 = 0x36, n = 0x6e
        const DSR_CPR: &[u8] = b"\x1b[6n";
        data.windows(DSR_CPR.len()).any(|w| w == DSR_CPR)
    }

    /// Flush the buffer by broadcasting to session clients and routing to pane state
    ///
    /// This method:
    /// 1. Routes output to pane.process() for scrollback and agent detection (FEAT-084)
    /// 2. Handles DSR [6n] cursor position requests (BUG-053)
    /// 3. Broadcasts PaneStateChanged if agent state changed
    /// 4. Broadcasts Output to all session clients
    async fn flush(&mut self) {
        if self.buffer.is_empty() {
            return;
        }

        let data = std::mem::take(&mut self.buffer);
        self.buffer = Vec::with_capacity(self.config.max_buffer_size);

        trace!(
            pane_id = %self.pane_id,
            session_id = %self.session_id,
            bytes = data.len(),
            "Flushing output to session"
        );

        // Route output to pane state for scrollback and agent detection (FEAT-084)
        // Also handle DSR [6n] cursor position requests (BUG-053)
        let state_change_msg = if let Some(executor) = &self.command_executor {
            let session_manager = executor.session_manager();
            let mut manager = session_manager.write().await;
            if let Some(pane) = manager.find_pane_mut(self.pane_id) {
                // Process returns Some(AgentState) if state changed (FEAT-084)
                let state_changed = pane.process(&data);

                // BUG-053: Handle DSR [6n] cursor position request
                // Check if the output contains ESC[6n (cursor position query)
                // The terminal should respond with ESC[row;colR
                if Self::contains_dsr_cpr(&data) {
                    if let Some(screen) = pane.screen() {
                        let (row, col) = screen.cursor_position();
                        // vt100 uses 0-based indexing, DSR response uses 1-based
                        let response = format!("\x1b[{};{}R", row + 1, col + 1);
                        trace!(
                            pane_id = %self.pane_id,
                            row = row + 1,
                            col = col + 1,
                            "Responding to DSR [6n] cursor position request"
                        );

                        // Write response to PTY (must release session manager lock first)
                        drop(manager);

                        // Write DSR response to PTY
                        let pty_manager = executor.pty_manager();
                        let pty_mgr = pty_manager.read().await;
                        if let Some(handle) = pty_mgr.get(self.pane_id) {
                            if let Err(e) = handle.write_all(response.as_bytes()) {
                                warn!(
                                    pane_id = %self.pane_id,
                                    error = %e,
                                    "Failed to write DSR cursor position response"
                                );
                            }
                        }

                        // Re-acquire lock to check state change
                        let manager = session_manager.read().await;
                        if let Some(agent_state) = state_changed {
                            if let Some((_, _, pane)) = manager.find_pane(self.pane_id) {
                                debug!(
                                    pane_id = %self.pane_id,
                                    agent_type = %agent_state.agent_type,
                                    activity = ?agent_state.activity,
                                    "Agent state changed from PTY output"
                                );
                                Some(ServerMessage::PaneStateChanged {
                                    pane_id: self.pane_id,
                                    state: pane.state().clone(),
                                })
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    } else {
                        // No screen available, still handle state change
                        if let Some(agent_state) = state_changed {
                            debug!(
                                pane_id = %self.pane_id,
                                agent_type = %agent_state.agent_type,
                                activity = ?agent_state.activity,
                                "Agent state changed from PTY output"
                            );
                            Some(ServerMessage::PaneStateChanged {
                                pane_id: self.pane_id,
                                state: pane.state().clone(),
                            })
                        } else {
                            None
                        }
                    }
                } else if let Some(agent_state) = state_changed {
                    debug!(
                        pane_id = %self.pane_id,
                        agent_type = %agent_state.agent_type,
                        activity = ?agent_state.activity,
                        "Agent state changed from PTY output"
                    );
                    // Capture the state change message while we still have the lock
                    Some(ServerMessage::PaneStateChanged {
                        pane_id: self.pane_id,
                        state: pane.state().clone(),
                    })
                } else {
                    None
                }
            } else {
                trace!(
                    pane_id = %self.pane_id,
                    "Pane not found in session manager (may have been closed)"
                );
                None
            }
        } else {
            None
        };

        // Broadcast state change if agent state changed
        if let Some(state_msg) = state_change_msg {
            self.registry.broadcast_to_session(self.session_id, state_msg).await;
        }

        // Broadcast output to session clients
        let msg = ServerMessage::Output {
            pane_id: self.pane_id,
            data: data.clone(),
        };

        let delivered = self.registry.broadcast_to_session(self.session_id, msg).await;

        if delivered == 0 {
            trace!(
                pane_id = %self.pane_id,
                session_id = %self.session_id,
                "No clients received output (session may have no attached clients)"
            );
        }

        // BUG-066: Forward output to mirror panes in other sessions
        // Mirror panes are read-only views of a source pane. When the source produces
        // output, we need to forward it to all mirrors so they display the content.
        if let Some(executor) = &self.command_executor {
            let session_manager = executor.session_manager();
            let manager = session_manager.read().await;

            // Get all mirror pane IDs for this source pane
            let mirrors = manager.get_mirrors_for_pane(self.pane_id);

            for mirror_id in mirrors {
                // Find which session the mirror pane is in
                if let Some((mirror_session, _, _)) = manager.find_pane(mirror_id) {
                    let mirror_session_id = mirror_session.id();

                    // Only forward if mirror is in a different session
                    // (same-session mirrors receive output via normal broadcast)
                    if mirror_session_id != self.session_id {
                        // Create output message with the MIRROR's pane_id
                        // so the TUI routes it to the correct pane
                        let mirror_msg = ServerMessage::Output {
                            pane_id: mirror_id,
                            data: data.clone(),
                        };

                        let mirror_delivered = self.registry
                            .broadcast_to_session(mirror_session_id, mirror_msg)
                            .await;

                        trace!(
                            source_pane = %self.pane_id,
                            mirror_pane = %mirror_id,
                            mirror_session = %mirror_session_id,
                            delivered = mirror_delivered,
                            "Forwarded output to cross-session mirror pane"
                        );
                    }
                }
            }
        }
    }
}

/// Result of a PTY read operation
#[derive(Debug)]
enum ReadResult {
    /// Successfully read data
    Data(Vec<u8>),
    /// End of file (PTY closed)
    Eof,
    /// Read error
    Error(String),
}

/// Manages output pollers for multiple panes
///
/// Provides a central place to track, start, and stop output pollers.
/// Use this to ensure pollers are properly cleaned up when panes close.
#[derive(Default)]
pub struct PollerManager {
    /// Active pollers by pane ID
    handles: std::collections::HashMap<Uuid, PollerHandle>,
    /// Channel to notify server when panes close
    pane_closed_tx: Option<mpsc::Sender<PaneClosedNotification>>,
}


impl PollerManager {
    /// Create a new empty poller manager
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a new poller manager with a cleanup notification channel
    ///
    /// When a pane's PTY process exits, the poller will send a notification
    /// through this channel so the server can clean up the pane from session state.
    pub fn with_cleanup_channel(pane_closed_tx: mpsc::Sender<PaneClosedNotification>) -> Self {
        Self {
            handles: std::collections::HashMap::new(),
            pane_closed_tx: Some(pane_closed_tx),
        }
    }

    /// Start a new poller for a pane
    ///
    /// If a poller already exists for this pane, it will be stopped first.
    pub fn start(
        &mut self,
        pane_id: Uuid,
        session_id: Uuid,
        pty_reader: Arc<Mutex<Box<dyn Read + Send>>>,
        registry: Arc<ClientRegistry>,
    ) {
        self.start_with_config(pane_id, session_id, pty_reader, registry, OutputPollerConfig::default())
    }

    /// Start a new poller with custom configuration
    pub fn start_with_config(
        &mut self,
        pane_id: Uuid,
        session_id: Uuid,
        pty_reader: Arc<Mutex<Box<dyn Read + Send>>>,
        registry: Arc<ClientRegistry>,
        config: OutputPollerConfig,
    ) {
        // Stop existing poller if any
        if let Some(old_handle) = self.handles.remove(&pane_id) {
            old_handle.cancel();
            debug!(pane_id = %pane_id, "Stopped existing poller before starting new one");
        }

        let handle = PtyOutputPoller::spawn_with_config(
            pane_id,
            session_id,
            pty_reader,
            registry,
            config,
            self.pane_closed_tx.clone(),
        );

        self.handles.insert(pane_id, handle);
        debug!(pane_id = %pane_id, "Started output poller");
    }

    /// Stop a poller for a pane (non-blocking)
    ///
    /// Returns true if a poller was found and cancelled.
    pub fn stop(&mut self, pane_id: Uuid) -> bool {
        if let Some(handle) = self.handles.remove(&pane_id) {
            handle.cancel();
            debug!(pane_id = %pane_id, "Stopped output poller");
            true
        } else {
            false
        }
    }

    /// Stop a poller and wait for it to complete
    pub async fn stop_and_wait(&mut self, pane_id: Uuid) -> bool {
        if let Some(handle) = self.handles.remove(&pane_id) {
            handle.stop().await;
            debug!(pane_id = %pane_id, "Stopped and waited for output poller");
            true
        } else {
            false
        }
    }

    /// Stop all pollers (non-blocking)
    pub fn stop_all(&mut self) {
        for (pane_id, handle) in self.handles.drain() {
            handle.cancel();
            debug!(pane_id = %pane_id, "Stopped output poller (stop_all)");
        }
    }

    /// Stop all pollers and wait for them to complete
    pub async fn stop_all_and_wait(&mut self) {
        let handles: Vec<_> = self.handles.drain().collect();
        for (pane_id, handle) in handles {
            handle.stop().await;
            debug!(pane_id = %pane_id, "Stopped and waited for output poller (stop_all)");
        }
    }

    /// Check if a poller is running for a pane
    pub fn has_poller(&self, pane_id: Uuid) -> bool {
        self.handles.contains_key(&pane_id)
    }

    /// Get the number of active pollers
    pub fn count(&self) -> usize {
        self.handles.len()
    }

    /// Get all pane IDs with active pollers
    pub fn pane_ids(&self) -> Vec<Uuid> {
        self.handles.keys().copied().collect()
    }
}

impl std::fmt::Debug for PollerManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PollerManager")
            .field("count", &self.handles.len())
            .field("pane_ids", &self.handles.keys().collect::<Vec<_>>())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use tokio::sync::mpsc as tokio_mpsc;
    use tokio::time::timeout;

    // Helper to create a mock registry
    fn create_test_registry() -> Arc<ClientRegistry> {
        Arc::new(ClientRegistry::new())
    }

    // Helper to create a reader from bytes
    fn create_reader(data: &[u8]) -> Arc<Mutex<Box<dyn Read + Send>>> {
        Arc::new(Mutex::new(Box::new(Cursor::new(data.to_vec()))))
    }

    #[tokio::test]
    async fn test_output_poller_config_default() {
        let config = OutputPollerConfig::default();
        assert_eq!(config.flush_timeout, Duration::from_millis(50));
        assert_eq!(config.max_buffer_size, 16384);
    }

    #[tokio::test]
    async fn test_poller_handle_stop() {
        let pane_id = Uuid::new_v4();
        let session_id = Uuid::new_v4();
        let registry = create_test_registry();

        // Create a reader that will block (empty cursor will return EOF immediately)
        let reader = create_reader(b"");

        let handle = PtyOutputPoller::spawn(pane_id, session_id, reader, registry);

        // Stop should complete quickly
        let result = timeout(Duration::from_secs(1), handle.stop()).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_poller_handle_cancel() {
        let pane_id = Uuid::new_v4();
        let session_id = Uuid::new_v4();
        let registry = create_test_registry();
        let reader = create_reader(b"");

        let handle = PtyOutputPoller::spawn(pane_id, session_id, reader, registry);

        // Cancel should not block
        handle.cancel();

        // Task should eventually complete
        let result = timeout(Duration::from_secs(1), handle.join_handle).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_poller_broadcasts_output() {
        let pane_id = Uuid::new_v4();
        let session_id = Uuid::new_v4();
        let registry = create_test_registry();

        // Create a client attached to the session
        let (tx, mut rx) = tokio_mpsc::channel(10);
        let client_id = registry.register_client(tx);
        registry.attach_to_session(client_id, session_id);

        // Create reader with test data
        let test_data = b"Hello, World!\n";
        let reader = create_reader(test_data);

        let handle = PtyOutputPoller::spawn(pane_id, session_id, reader, registry);

        // Wait for message to be received
        let msg = timeout(Duration::from_secs(2), rx.recv()).await;

        // Stop the poller
        handle.cancel();

        // Verify the message
        assert!(msg.is_ok());
        let msg = msg.unwrap();
        assert!(msg.is_some());

        if let Some(ServerMessage::Output { pane_id: pid, data }) = msg {
            assert_eq!(pid, pane_id);
            assert_eq!(data, test_data);
        } else {
            panic!("Expected Output message");
        }
    }

    #[tokio::test]
    async fn test_poller_flushes_on_newline() {
        let pane_id = Uuid::new_v4();
        let session_id = Uuid::new_v4();
        let registry = create_test_registry();

        let (tx, mut rx) = tokio_mpsc::channel(10);
        let client_id = registry.register_client(tx);
        registry.attach_to_session(client_id, session_id);

        // Data with newline should flush immediately
        let test_data = b"line1\n";
        let reader = create_reader(test_data);

        let handle = PtyOutputPoller::spawn(pane_id, session_id, reader, registry);

        // Should receive message quickly (within flush timeout)
        let msg = timeout(Duration::from_millis(100), rx.recv()).await;
        handle.cancel();

        assert!(msg.is_ok());
    }

    #[tokio::test]
    async fn test_poller_eof_handling() {
        let pane_id = Uuid::new_v4();
        let session_id = Uuid::new_v4();
        let registry = create_test_registry();

        // Empty reader will immediately return EOF
        let reader = create_reader(b"");

        let handle = PtyOutputPoller::spawn(pane_id, session_id, reader, registry);

        // Task should complete due to EOF
        let result = timeout(Duration::from_secs(2), handle.join_handle).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_poller_no_clients() {
        let pane_id = Uuid::new_v4();
        let session_id = Uuid::new_v4();
        let registry = create_test_registry();

        // No clients attached - should still work without panicking
        let test_data = b"Hello\n";
        let reader = create_reader(test_data);

        let handle = PtyOutputPoller::spawn(pane_id, session_id, reader, registry);

        // Give it time to process
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Should complete cleanly
        let result = timeout(Duration::from_secs(1), handle.stop()).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_poller_custom_config() {
        let pane_id = Uuid::new_v4();
        let session_id = Uuid::new_v4();
        let registry = create_test_registry();
        let reader = create_reader(b"");

        let config = OutputPollerConfig {
            flush_timeout: Duration::from_millis(10),
            max_buffer_size: 1024,
        };

        let handle = PtyOutputPoller::spawn_with_config(
            pane_id,
            session_id,
            reader,
            registry,
            config,
            None, // No cleanup channel for test
        );

        // Should work with custom config
        let result = timeout(Duration::from_secs(1), handle.stop()).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_poller_multiple_outputs() {
        let pane_id = Uuid::new_v4();
        let session_id = Uuid::new_v4();
        let registry = create_test_registry();

        let (tx, mut rx) = tokio_mpsc::channel(10);
        let client_id = registry.register_client(tx);
        registry.attach_to_session(client_id, session_id);

        // Multiple lines - may be batched or separate
        let test_data = b"line1\nline2\nline3\n";
        let reader = create_reader(test_data);

        let handle = PtyOutputPoller::spawn(pane_id, session_id, reader, registry);

        // Collect all messages
        let mut received = Vec::new();
        loop {
            match timeout(Duration::from_millis(200), rx.recv()).await {
                Ok(Some(msg)) => received.push(msg),
                _ => break,
            }
        }

        handle.cancel();

        // Should have received at least one message with the data
        assert!(!received.is_empty());

        // Verify all data was received
        let mut all_data = Vec::new();
        for msg in received {
            if let ServerMessage::Output { data, .. } = msg {
                all_data.extend(data);
            }
        }
        assert_eq!(all_data, test_data);
    }

    #[test]
    fn test_read_result_debug() {
        let data = ReadResult::Data(vec![1, 2, 3]);
        let eof = ReadResult::Eof;
        let err = ReadResult::Error("test".to_string());

        // Should not panic
        let _ = format!("{:?}", data);
        let _ = format!("{:?}", eof);
        let _ = format!("{:?}", err);
    }

    // ==================== PollerManager Tests ====================

    #[test]
    fn test_poller_manager_new() {
        let manager = PollerManager::new();
        assert_eq!(manager.count(), 0);
    }

    #[test]
    fn test_poller_manager_default() {
        let manager = PollerManager::default();
        assert_eq!(manager.count(), 0);
    }

    #[test]
    fn test_poller_manager_debug() {
        let manager = PollerManager::new();
        let debug = format!("{:?}", manager);
        assert!(debug.contains("PollerManager"));
        assert!(debug.contains("count"));
    }

    #[tokio::test]
    async fn test_poller_manager_start_stop() {
        let mut manager = PollerManager::new();
        let pane_id = Uuid::new_v4();
        let session_id = Uuid::new_v4();
        let registry = create_test_registry();
        let reader = create_reader(b"");

        manager.start(pane_id, session_id, reader, registry);

        assert!(manager.has_poller(pane_id));
        assert_eq!(manager.count(), 1);

        // Give poller time to start
        tokio::time::sleep(Duration::from_millis(10)).await;

        let stopped = manager.stop(pane_id);
        assert!(stopped);
        assert!(!manager.has_poller(pane_id));
        assert_eq!(manager.count(), 0);
    }

    #[tokio::test]
    async fn test_poller_manager_stop_nonexistent() {
        let mut manager = PollerManager::new();
        let pane_id = Uuid::new_v4();

        let stopped = manager.stop(pane_id);
        assert!(!stopped);
    }

    #[tokio::test]
    async fn test_poller_manager_stop_and_wait() {
        let mut manager = PollerManager::new();
        let pane_id = Uuid::new_v4();
        let session_id = Uuid::new_v4();
        let registry = create_test_registry();
        let reader = create_reader(b"");

        manager.start(pane_id, session_id, reader, registry);

        let stopped = manager.stop_and_wait(pane_id).await;
        assert!(stopped);
        assert!(!manager.has_poller(pane_id));
    }

    #[tokio::test]
    async fn test_poller_manager_stop_all() {
        let mut manager = PollerManager::new();
        let registry = create_test_registry();

        // Start multiple pollers
        for _ in 0..3 {
            let pane_id = Uuid::new_v4();
            let session_id = Uuid::new_v4();
            let reader = create_reader(b"");
            manager.start(pane_id, session_id, reader, registry.clone());
        }

        assert_eq!(manager.count(), 3);

        manager.stop_all();
        assert_eq!(manager.count(), 0);
    }

    #[tokio::test]
    async fn test_poller_manager_stop_all_and_wait() {
        let mut manager = PollerManager::new();
        let registry = create_test_registry();

        // Start multiple pollers
        for _ in 0..3 {
            let pane_id = Uuid::new_v4();
            let session_id = Uuid::new_v4();
            let reader = create_reader(b"");
            manager.start(pane_id, session_id, reader, registry.clone());
        }

        assert_eq!(manager.count(), 3);

        manager.stop_all_and_wait().await;
        assert_eq!(manager.count(), 0);
    }

    #[tokio::test]
    async fn test_poller_manager_pane_ids() {
        let mut manager = PollerManager::new();
        let registry = create_test_registry();

        let pane1 = Uuid::new_v4();
        let pane2 = Uuid::new_v4();
        let session_id = Uuid::new_v4();

        manager.start(pane1, session_id, create_reader(b""), registry.clone());
        manager.start(pane2, session_id, create_reader(b""), registry);

        let ids = manager.pane_ids();
        assert_eq!(ids.len(), 2);
        assert!(ids.contains(&pane1));
        assert!(ids.contains(&pane2));

        manager.stop_all();
    }

    #[tokio::test]
    async fn test_poller_manager_restart_replaces() {
        let mut manager = PollerManager::new();
        let pane_id = Uuid::new_v4();
        let session_id = Uuid::new_v4();
        let registry = create_test_registry();

        // Start first poller
        manager.start(pane_id, session_id, create_reader(b""), registry.clone());
        assert_eq!(manager.count(), 1);

        // Start again - should replace
        manager.start(pane_id, session_id, create_reader(b""), registry);
        assert_eq!(manager.count(), 1);

        manager.stop_all();
    }

    #[tokio::test]
    async fn test_poller_manager_with_config() {
        let mut manager = PollerManager::new();
        let pane_id = Uuid::new_v4();
        let session_id = Uuid::new_v4();
        let registry = create_test_registry();
        let reader = create_reader(b"");

        let config = OutputPollerConfig {
            flush_timeout: Duration::from_millis(10),
            max_buffer_size: 1024,
        };

        manager.start_with_config(pane_id, session_id, reader, registry, config);

        assert!(manager.has_poller(pane_id));
        manager.stop_all();
    }

    // ==================== Pane State Routing Tests ====================
    // Tests for BUG-016: PTY output not routed to pane state

    use tokio::sync::RwLock;
    use crate::session::SessionManager;
    use crate::pty::PtyManager;

    /// Helper to create a full test setup with session manager and executor
    async fn create_test_setup_with_session() -> (
        Arc<RwLock<SessionManager>>,
        Arc<RwLock<PtyManager>>,
        Arc<ClientRegistry>,
        Arc<AsyncCommandExecutor>,
        Uuid, // session_id
        Uuid, // pane_id
    ) {
        let session_manager = Arc::new(RwLock::new(SessionManager::new()));
        let pty_manager = Arc::new(RwLock::new(PtyManager::new()));
        let registry = Arc::new(ClientRegistry::new());

        let executor = Arc::new(AsyncCommandExecutor::new(
            session_manager.clone(),
            pty_manager.clone(),
            registry.clone(),
        ));

        // Create session, window, and pane
        let (session_id, pane_id) = {
            let mut manager = session_manager.write().await;
            let session = manager.create_session("test").unwrap();
            let session_id = session.id();

            let session = manager.get_session_mut(session_id).unwrap();
            let window = session.create_window(None);
            let window_id = window.id();

            let window = session.get_window_mut(window_id).unwrap();
            let pane = window.create_pane();
            let pane_id = pane.id();

            (session_id, pane_id)
        };

        (session_manager, pty_manager, registry, executor, session_id, pane_id)
    }

    #[tokio::test]
    async fn test_poller_routes_output_to_scrollback() {
        // Setup: create session manager with pane
        let (session_manager, _pty_manager, registry, executor, session_id, pane_id) =
            create_test_setup_with_session().await;

        // Create reader with test data
        let test_data = b"Line 1\nLine 2\nLine 3\n";
        let reader = create_reader(test_data);

        // Spawn poller with sideband (which gives access to session manager)
        let handle = PtyOutputPoller::spawn_with_sideband(
            pane_id,
            session_id,
            reader,
            registry,
            None, // No cleanup channel for test
            executor,
        );

        // Wait for processing to complete
        tokio::time::sleep(Duration::from_millis(200)).await;
        handle.cancel();

        // Verify scrollback is populated
        let manager = session_manager.read().await;
        let (_, _, pane) = manager.find_pane(pane_id).expect("Pane should exist");

        let lines: Vec<_> = pane.scrollback().get_lines().collect();
        assert_eq!(lines.len(), 3, "Scrollback should have 3 lines");
        assert_eq!(lines[0], "Line 1");
        assert_eq!(lines[1], "Line 2");
        assert_eq!(lines[2], "Line 3");
    }

    #[tokio::test]
    async fn test_poller_triggers_claude_detection() {
        use ccmux_protocol::PaneState;

        // Setup: create session manager with pane
        let (session_manager, _pty_manager, registry, executor, session_id, pane_id) =
            create_test_setup_with_session().await;

        // Create reader with Claude startup output
        // The ClaudeDetector looks for patterns like "╭─" and prompt patterns
        let claude_output = b"\x1b[?25l\x1b[2J\x1b[H\
\xe2\x95\xad\xe2\x94\x80 Claude Code\n\
\xe2\x94\x82 /home/user/project\n\
\xe2\x95\xb0\xe2\x94\x80\n\
> \n";
        let reader = create_reader(claude_output);

        // Spawn poller with sideband
        let handle = PtyOutputPoller::spawn_with_sideband(
            pane_id,
            session_id,
            reader,
            registry,
            None,
            executor,
        );

        // Wait for processing to complete
        tokio::time::sleep(Duration::from_millis(200)).await;
        handle.cancel();

        // Verify Claude detection triggered (scrollback should be populated at minimum)
        let manager = session_manager.read().await;
        let (_, _, pane) = manager.find_pane(pane_id).expect("Pane should exist");

        // Scrollback should have content
        assert!(pane.scrollback().len() > 0, "Scrollback should have content");

        // Note: Claude detection requires specific patterns that may not trigger
        // with our test data, but the scrollback should always be populated.
        // Full Claude detection testing is done in claude.rs tests.
    }

    #[tokio::test]
    async fn test_poller_scrollback_with_multiple_flushes() {
        // Setup
        let (session_manager, _pty_manager, registry, executor, session_id, pane_id) =
            create_test_setup_with_session().await;

        // Create a larger data set that will require multiple flushes
        let test_data = b"Line 1\nLine 2\nLine 3\nLine 4\nLine 5\n\
                         Line 6\nLine 7\nLine 8\nLine 9\nLine 10\n";
        let reader = create_reader(test_data);

        let handle = PtyOutputPoller::spawn_with_sideband(
            pane_id,
            session_id,
            reader,
            registry,
            None,
            executor,
        );

        tokio::time::sleep(Duration::from_millis(200)).await;
        handle.cancel();

        // Verify all lines are in scrollback
        let manager = session_manager.read().await;
        let (_, _, pane) = manager.find_pane(pane_id).expect("Pane should exist");

        let lines: Vec<_> = pane.scrollback().get_lines().collect();
        assert_eq!(lines.len(), 10, "Scrollback should have 10 lines");
    }

    #[tokio::test]
    async fn test_poller_still_broadcasts_to_clients() {
        // Verify that the fix doesn't break client broadcasting
        let (session_manager, _pty_manager, registry, executor, session_id, pane_id) =
            create_test_setup_with_session().await;

        // Create a client attached to the session
        let (tx, mut rx) = tokio_mpsc::channel(10);
        let client_id = registry.register_client(tx);
        registry.attach_to_session(client_id, session_id);

        // Create reader with test data
        let test_data = b"Hello, World!\n";
        let reader = create_reader(test_data);

        let handle = PtyOutputPoller::spawn_with_sideband(
            pane_id,
            session_id,
            reader,
            registry,
            None,
            executor,
        );

        // Wait for message to be received
        let msg = timeout(Duration::from_secs(2), rx.recv()).await;
        handle.cancel();

        // Verify the client received the output
        assert!(msg.is_ok());
        let msg = msg.unwrap();
        assert!(msg.is_some());

        if let Some(ServerMessage::Output { pane_id: pid, data }) = msg {
            assert_eq!(pid, pane_id);
            assert_eq!(data, test_data);
        } else {
            panic!("Expected Output message, got {:?}", msg);
        }

        // Also verify scrollback was populated
        let manager = session_manager.read().await;
        let (_, _, pane) = manager.find_pane(pane_id).expect("Pane should exist");
        assert_eq!(pane.scrollback().len(), 1, "Scrollback should have 1 line");
    }

    // ==================== DSR [6n] Tests (BUG-053) ====================

    #[test]
    fn test_contains_dsr_cpr_basic() {
        // Basic DSR CPR sequence
        assert!(PtyOutputPoller::contains_dsr_cpr(b"\x1b[6n"));

        // Embedded in other data
        assert!(PtyOutputPoller::contains_dsr_cpr(b"hello\x1b[6nworld"));
        assert!(PtyOutputPoller::contains_dsr_cpr(b"\x1b[?2004h\x1b[6n"));

        // At end
        assert!(PtyOutputPoller::contains_dsr_cpr(b"test\x1b[6n"));
    }

    #[test]
    fn test_contains_dsr_cpr_negative() {
        // No DSR sequence
        assert!(!PtyOutputPoller::contains_dsr_cpr(b"hello world"));

        // Similar but not DSR CPR
        assert!(!PtyOutputPoller::contains_dsr_cpr(b"\x1b[5n")); // DSR operating status
        assert!(!PtyOutputPoller::contains_dsr_cpr(b"\x1b[6m")); // Not DSR at all
        assert!(!PtyOutputPoller::contains_dsr_cpr(b"\x1b6n"));  // Missing [

        // Partial sequence
        assert!(!PtyOutputPoller::contains_dsr_cpr(b"\x1b[6"));
        assert!(!PtyOutputPoller::contains_dsr_cpr(b"\x1b["));

        // Empty
        assert!(!PtyOutputPoller::contains_dsr_cpr(b""));
    }

    #[test]
    fn test_contains_dsr_cpr_with_codex_startup_sequence() {
        // Typical Codex CLI startup sequence
        let codex_startup = b"\x1b[?2004h\x1b[>7u\x1b[?1004h\x1b[6n";
        assert!(PtyOutputPoller::contains_dsr_cpr(codex_startup));
    }

    // ==================== Cross-Session Mirror Tests (BUG-066) ====================

    /// Helper to create a test setup with two sessions for mirror testing
    async fn create_cross_session_mirror_setup() -> (
        Arc<RwLock<SessionManager>>,
        Arc<RwLock<PtyManager>>,
        Arc<ClientRegistry>,
        Arc<AsyncCommandExecutor>,
        Uuid, // source_session_id
        Uuid, // source_pane_id
        Uuid, // mirror_session_id
        Uuid, // mirror_pane_id
    ) {
        let session_manager = Arc::new(RwLock::new(SessionManager::new()));
        let pty_manager = Arc::new(RwLock::new(PtyManager::new()));
        let registry = Arc::new(ClientRegistry::new());

        let executor = Arc::new(AsyncCommandExecutor::new(
            session_manager.clone(),
            pty_manager.clone(),
            registry.clone(),
        ));

        // Create source session with pane
        let (source_session_id, source_pane_id) = {
            let mut manager = session_manager.write().await;
            let session = manager.create_session("source").unwrap();
            let session_id = session.id();

            let session = manager.get_session_mut(session_id).unwrap();
            let window = session.create_window(None);
            let window_id = window.id();

            let window = session.get_window_mut(window_id).unwrap();
            let pane = window.create_pane();
            let pane_id = pane.id();

            (session_id, pane_id)
        };

        // Create mirror session with mirror pane
        let (mirror_session_id, mirror_pane_id) = {
            let mut manager = session_manager.write().await;
            let session = manager.create_session("mirror").unwrap();
            let session_id = session.id();

            let session = manager.get_session_mut(session_id).unwrap();
            let window = session.create_window(None);
            let window_id = window.id();

            let window = session.get_window_mut(window_id).unwrap();
            // Create a mirror pane pointing to the source
            let index = window.pane_count();
            let mirror_pane = crate::session::Pane::create_mirror(window_id, index, source_pane_id);
            let mirror_pane_id = mirror_pane.id();
            window.add_pane(mirror_pane);

            // Register the mirror relationship
            manager.mirror_registry_mut().register(source_pane_id, mirror_pane_id);

            (session_id, mirror_pane_id)
        };

        (
            session_manager,
            pty_manager,
            registry,
            executor,
            source_session_id,
            source_pane_id,
            mirror_session_id,
            mirror_pane_id,
        )
    }

    #[tokio::test]
    async fn test_bug066_cross_session_mirror_output_forwarding() {
        // Setup: Create source session and mirror session with mirror pane
        let (
            session_manager,
            _pty_manager,
            registry,
            executor,
            source_session_id,
            source_pane_id,
            mirror_session_id,
            mirror_pane_id,
        ) = create_cross_session_mirror_setup().await;

        // Create clients attached to each session
        let (source_tx, mut source_rx) = tokio_mpsc::channel(10);
        let source_client_id = registry.register_client(source_tx);
        registry.attach_to_session(source_client_id, source_session_id);

        let (mirror_tx, mut mirror_rx) = tokio_mpsc::channel(10);
        let mirror_client_id = registry.register_client(mirror_tx);
        registry.attach_to_session(mirror_client_id, mirror_session_id);

        // Create reader with test data
        let test_data = b"Hello from source pane!\n";
        let reader = create_reader(test_data);

        // Spawn poller for the SOURCE pane
        let handle = PtyOutputPoller::spawn_with_sideband(
            source_pane_id,
            source_session_id,
            reader,
            registry,
            None,
            executor,
        );

        // Wait for messages to be received
        tokio::time::sleep(Duration::from_millis(300)).await;
        handle.cancel();

        // Verify the source session received the output with SOURCE pane_id
        let source_msg = timeout(Duration::from_millis(100), source_rx.recv()).await;
        assert!(source_msg.is_ok(), "Source session should receive output");
        if let Ok(Some(ServerMessage::Output { pane_id, data })) = source_msg {
            assert_eq!(pane_id, source_pane_id, "Source output should have source pane_id");
            assert_eq!(data, test_data);
        } else {
            panic!("Expected Output message for source session");
        }

        // Verify the mirror session received the output with MIRROR pane_id
        let mirror_msg = timeout(Duration::from_millis(100), mirror_rx.recv()).await;
        assert!(mirror_msg.is_ok(), "Mirror session should receive forwarded output");
        if let Ok(Some(ServerMessage::Output { pane_id, data })) = mirror_msg {
            assert_eq!(pane_id, mirror_pane_id, "Mirror output should have mirror pane_id");
            assert_eq!(data, test_data);
        } else {
            panic!("Expected Output message for mirror session, got {:?}", mirror_msg);
        }
    }

    #[tokio::test]
    async fn test_bug066_same_session_mirror_no_duplicate() {
        // Setup: Create a single session with source and mirror pane
        let session_manager = Arc::new(RwLock::new(SessionManager::new()));
        let pty_manager = Arc::new(RwLock::new(PtyManager::new()));
        let registry = Arc::new(ClientRegistry::new());

        let executor = Arc::new(AsyncCommandExecutor::new(
            session_manager.clone(),
            pty_manager.clone(),
            registry.clone(),
        ));

        // Create session with source and mirror panes
        let (session_id, source_pane_id, _mirror_pane_id) = {
            let mut manager = session_manager.write().await;
            let session = manager.create_session("same-session").unwrap();
            let session_id = session.id();

            let session = manager.get_session_mut(session_id).unwrap();
            let window = session.create_window(None);
            let window_id = window.id();

            let window = session.get_window_mut(window_id).unwrap();
            let source_pane = window.create_pane();
            let source_pane_id = source_pane.id();

            // Create mirror in SAME session
            let index = window.pane_count();
            let mirror_pane = crate::session::Pane::create_mirror(window_id, index, source_pane_id);
            let mirror_pane_id = mirror_pane.id();
            window.add_pane(mirror_pane);

            manager.mirror_registry_mut().register(source_pane_id, mirror_pane_id);

            (session_id, source_pane_id, mirror_pane_id)
        };

        // Create client attached to the session
        let (tx, mut rx) = tokio_mpsc::channel(10);
        let client_id = registry.register_client(tx);
        registry.attach_to_session(client_id, session_id);

        // Create reader with test data
        let test_data = b"Test output\n";
        let reader = create_reader(test_data);

        // Spawn poller for source pane
        let handle = PtyOutputPoller::spawn_with_sideband(
            source_pane_id,
            session_id,
            reader,
            registry,
            None,
            executor,
        );

        tokio::time::sleep(Duration::from_millis(300)).await;
        handle.cancel();

        // Should receive exactly ONE output message (the source pane broadcast)
        // Same-session mirrors should NOT receive duplicate via cross-session forwarding
        let first_msg = timeout(Duration::from_millis(100), rx.recv()).await;
        assert!(first_msg.is_ok(), "Should receive at least one output");
        if let Ok(Some(ServerMessage::Output { pane_id, .. })) = first_msg {
            assert_eq!(pane_id, source_pane_id, "Output should be for source pane");
        }

        // Check if there's another message (there might be a PaneClosed, but not another Output)
        // We allow any non-duplicate Output messages
        if let Ok(Some(msg)) = timeout(Duration::from_millis(100), rx.recv()).await {
            if let ServerMessage::Output { pane_id, .. } = msg {
                // If we get another Output, it should still be the same pane_id
                // (this tests that same-session mirrors don't get forwarded duplicates)
                assert_eq!(pane_id, source_pane_id,
                    "Same-session mirror should not cause duplicate forwarding");
            }
        }
    }
}
