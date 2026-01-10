//! ccmux server - Background daemon

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use futures::stream::StreamExt;
use futures::sink::SinkExt;
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::{broadcast, mpsc, Mutex, RwLock};
use tokio_util::codec::{FramedRead, FramedWrite};
use tracing::{debug, error, info, warn};

use handlers::{HandlerContext, HandlerResult};

use ccmux_protocol::{ServerCodec, ServerMessage};
use ccmux_utils::Result;

mod claude;
mod config;
mod handlers;
mod isolation;
pub mod mcp;
#[allow(dead_code)]
mod orchestration;
mod parser;
#[allow(dead_code)]
mod persistence;
mod pty;
pub mod registry;
mod reply;
mod session;
pub mod sideband;

pub use registry::{ClientId, ClientRegistry};
pub use reply::{ReplyError, ReplyHandler};

use config::AppConfig;
use persistence::{
    parse_compression_method, PersistenceConfig, PersistenceManager, RestorationResult,
    ScrollbackCapture, ScrollbackConfig, SessionRestorer, SessionSnapshot, WindowSnapshot,
};
use pty::{PaneClosedNotification, PtyManager, PtyOutputPoller};
use session::SessionManager;
use sideband::AsyncCommandExecutor;

/// Shared state for concurrent access by client handlers
///
/// This holds Arc-wrapped managers that can be safely shared across
/// async tasks without requiring the server mutex.
#[derive(Clone)]
pub struct SharedState {
    /// Session manager for session/window/pane operations
    pub session_manager: Arc<RwLock<SessionManager>>,
    /// PTY manager for terminal operations
    pub pty_manager: Arc<RwLock<PtyManager>>,
    /// Client connection registry for tracking and broadcasting
    pub registry: Arc<ClientRegistry>,
    /// Application configuration
    pub config: Arc<AppConfig>,
    /// Shutdown signal sender
    shutdown_tx: broadcast::Sender<()>,
    /// Channel for pane cleanup notifications (when PTY dies)
    pub pane_closed_tx: mpsc::Sender<PaneClosedNotification>,
    /// Sideband command executor for processing Claude commands
    pub command_executor: Arc<AsyncCommandExecutor>,
}

impl SharedState {
    /// Subscribe to shutdown signals
    pub fn subscribe_shutdown(&self) -> broadcast::Receiver<()> {
        self.shutdown_tx.subscribe()
    }
}

/// Server state container
pub struct Server {
    /// Session manager (owned, moved to shared state at startup)
    session_manager: SessionManager,
    /// PTY manager (owned, moved to shared state at startup)
    pty_manager: PtyManager,
    /// Persistence manager (optional if disabled)
    persistence: Option<PersistenceManager>,
    /// Scrollback capture config
    scrollback_config: ScrollbackConfig,
    /// Shutdown signal sender
    shutdown_tx: broadcast::Sender<()>,
    /// Active client count
    active_clients: AtomicUsize,
    /// Client connection registry (owned, moved to shared state at startup)
    client_registry: ClientRegistry,
    /// Reference to shared session manager (set after startup)
    session_manager_ref: Option<Arc<RwLock<SessionManager>>>,
    /// Reference to shared PTY manager (set after startup)
    pty_manager_ref: Option<Arc<RwLock<PtyManager>>>,
}

impl Server {
    /// Create a new server with the given configuration
    pub fn new(app_config: &AppConfig) -> Result<Self> {
        let persistence_config = &app_config.persistence;
        let (shutdown_tx, _) = broadcast::channel(1);

        let mut server = Self {
            session_manager: SessionManager::new(),
            pty_manager: PtyManager::new(),
            persistence: None,
            scrollback_config: ScrollbackConfig {
                max_lines: persistence_config.screen_snapshot_lines,
                compression: parse_compression_method(&persistence_config.compression_method),
                ..Default::default()
            },
            shutdown_tx,
            active_clients: AtomicUsize::new(0),
            client_registry: ClientRegistry::new(),
            session_manager_ref: None,
            pty_manager_ref: None,
        };

        // Initialize persistence if enabled
        if persistence_config.enabled {
            let state_dir = persistence_config
                .state_dir
                .as_ref()
                .map(PathBuf::from)
                .unwrap_or_else(|| {
                    dirs::data_dir()
                        .unwrap_or_else(|| PathBuf::from("."))
                        .join(persistence::DEFAULT_STATE_DIR)
                });

            let config = PersistenceConfig::from(persistence_config);
            let manager = PersistenceManager::new(&state_dir, config)?;
            server.persistence = Some(manager);

            info!("Persistence initialized at {}", state_dir.display());
        } else {
            info!("Persistence disabled");
        }

        Ok(server)
    }

    /// Perform recovery on startup
    ///
    /// This should be called early in the server initialization.
    pub fn recover(&mut self) -> Result<RestorationResult> {
        let Some(persistence) = &self.persistence else {
            return Ok(RestorationResult::default());
        };

        // Check if recovery is needed
        if !persistence.needs_recovery()? {
            info!("No recovery needed, starting fresh");
            return Ok(RestorationResult::default());
        }

        // Perform recovery
        let state = persistence.recover()?;

        if !state.has_sessions() {
            info!("Recovery complete, no sessions to restore");
            return Ok(RestorationResult::default());
        }

        // Restore sessions
        let restorer = SessionRestorer::new();
        let result =
            restorer.restore(&state, &mut self.session_manager, &mut self.pty_manager);

        info!("{}", result.summary());

        // Log any warnings
        for warning in &state.warnings {
            warn!("Recovery warning: {}", warning);
        }

        Ok(result)
    }

    /// Create a checkpoint of current state
    pub fn checkpoint(&mut self) -> Result<()> {
        if self.persistence.is_none() {
            return Ok(());
        }

        // Collect session snapshots first (immutable borrow)
        let sessions = self.collect_session_snapshots();

        // Then create checkpoint (mutable borrow)
        if let Some(ref mut persistence) = self.persistence {
            persistence.create_checkpoint(sessions)?;
        }

        Ok(())
    }

    /// Create a checkpoint with pre-collected snapshots
    pub fn checkpoint_with_snapshots(&mut self, sessions: Vec<SessionSnapshot>) -> Result<()> {
        if let Some(ref mut persistence) = self.persistence {
            persistence.create_checkpoint(sessions)?;
        }
        Ok(())
    }

    /// Collect session snapshots from an external session manager reference
    pub fn collect_session_snapshots_from(
        &self,
        session_manager: &SessionManager,
    ) -> Vec<SessionSnapshot> {
        let _capture = ScrollbackCapture::new(self.scrollback_config.clone());

        session_manager
            .list_sessions()
            .iter()
            .map(|session| {
                let windows: Vec<WindowSnapshot> = session
                    .windows()
                    .map(|window| {
                        let panes = window
                            .panes()
                            .map(|pane| {
                                let (cols, rows) = pane.dimensions();
                                persistence::PaneSnapshot {
                                    id: pane.id(),
                                    window_id: window.id(),
                                    index: pane.index(),
                                    cols,
                                    rows,
                                    state: pane.state().clone(),
                                    title: pane.title().map(String::from),
                                    cwd: pane.cwd().map(String::from),
                                    created_at: pane.created_at_unix(),
                                    scrollback: None,
                                }
                            })
                            .collect();

                        WindowSnapshot {
                            id: window.id(),
                            session_id: session.id(),
                            name: window.name().to_string(),
                            index: window.index(),
                            panes,
                            active_pane_id: window.active_pane_id(),
                            created_at: window.created_at_unix(),
                        }
                    })
                    .collect();

                SessionSnapshot {
                    id: session.id(),
                    name: session.name().to_string(),
                    windows,
                    active_window_id: session.active_window_id(),
                    created_at: session.created_at_unix(),
                }
            })
            .collect()
    }

    /// Perform graceful shutdown
    pub fn shutdown(&mut self) -> Result<()> {
        info!("Server shutting down");

        // Kill all PTYs
        self.pty_manager.kill_all();

        // Clean up isolation directories for all Claude panes
        for session in self.session_manager.list_sessions() {
            for window in session.windows() {
                for pane in window.panes() {
                    if pane.is_claude() {
                        if let Err(e) = isolation::cleanup_config_dir(pane.id()) {
                            warn!(
                                "Failed to cleanup isolation dir for pane {}: {}",
                                pane.id(),
                                e
                            );
                        }
                    }
                }
            }
        }

        // Collect final state and shutdown persistence
        if let Some(mut persistence) = self.persistence.take() {
            let sessions = self.collect_session_snapshots();
            persistence.shutdown(sessions)?;
        }

        info!("Shutdown complete");
        Ok(())
    }

    /// Check if checkpoint is due
    pub fn is_checkpoint_due(&self) -> bool {
        self.persistence
            .as_ref()
            .map(|p| p.is_checkpoint_due())
            .unwrap_or(false)
    }

    /// Collect session snapshots for checkpointing
    fn collect_session_snapshots(&self) -> Vec<SessionSnapshot> {
        let _capture = ScrollbackCapture::new(self.scrollback_config.clone());

        self.session_manager
            .list_sessions()
            .iter()
            .map(|session| {
                let windows: Vec<WindowSnapshot> = session
                    .windows()
                    .map(|window| {
                        let panes = window
                            .panes()
                            .map(|pane| {
                                let (cols, rows) = pane.dimensions();
                                persistence::PaneSnapshot {
                                    id: pane.id(),
                                    window_id: window.id(),
                                    index: pane.index(),
                                    cols,
                                    rows,
                                    state: pane.state().clone(),
                                    title: pane.title().map(String::from),
                                    cwd: pane.cwd().map(String::from),
                                    created_at: pane.created_at_unix(),
                                    scrollback: None, // TODO: Get from PTY
                                }
                            })
                            .collect();

                        WindowSnapshot {
                            id: window.id(),
                            session_id: session.id(),
                            name: window.name().to_string(),
                            index: window.index(),
                            panes,
                            active_pane_id: window.active_pane_id(),
                            created_at: window.created_at_unix(),
                        }
                    })
                    .collect();

                SessionSnapshot {
                    id: session.id(),
                    name: session.name().to_string(),
                    windows,
                    active_window_id: session.active_window_id(),
                    created_at: session.created_at_unix(),
                }
            })
            .collect()
    }

    /// Get session manager reference
    pub fn session_manager(&self) -> &SessionManager {
        &self.session_manager
    }

    /// Get mutable session manager reference
    pub fn session_manager_mut(&mut self) -> &mut SessionManager {
        &mut self.session_manager
    }

    /// Get PTY manager reference
    pub fn pty_manager(&self) -> &PtyManager {
        &self.pty_manager
    }

    /// Get mutable PTY manager reference
    pub fn pty_manager_mut(&mut self) -> &mut PtyManager {
        &mut self.pty_manager
    }

    /// Get persistence manager reference
    pub fn persistence(&self) -> Option<&PersistenceManager> {
        self.persistence.as_ref()
    }

    /// Get client registry reference
    pub fn client_registry(&self) -> &ClientRegistry {
        &self.client_registry
    }

    /// Perform isolation cleanup on startup
    ///
    /// Removes orphaned isolation directories from crashed sessions.
    pub fn cleanup_isolation(&self) {
        // Collect active pane IDs from session manager
        let active_pane_ids: Vec<uuid::Uuid> = self
            .session_manager
            .list_sessions()
            .iter()
            .flat_map(|session| {
                session.windows().flat_map(|window| {
                    window.panes().map(|pane| pane.id())
                })
            })
            .collect();

        // Clean up orphaned isolation directories
        if let Err(e) = isolation::startup_cleanup(&active_pane_ids) {
            warn!("Failed to clean up isolation directories: {}", e);
        }
    }

    /// Subscribe to shutdown signals
    pub fn subscribe_shutdown(&self) -> broadcast::Receiver<()> {
        self.shutdown_tx.subscribe()
    }

    /// Signal shutdown to all listeners
    pub fn signal_shutdown(&self) {
        let _ = self.shutdown_tx.send(());
    }

    /// Increment active client count
    pub fn client_connected(&self) {
        let count = self.active_clients.fetch_add(1, Ordering::SeqCst);
        info!("Client connected, active clients: {}", count + 1);
    }

    /// Decrement active client count
    pub fn client_disconnected(&self) {
        let count = self.active_clients.fetch_sub(1, Ordering::SeqCst);
        info!("Client disconnected, active clients: {}", count - 1);
    }

    /// Get active client count
    pub fn active_client_count(&self) -> usize {
        self.active_clients.load(Ordering::SeqCst)
    }
}

// ==================== Socket Setup ====================

/// Set up the Unix socket for client connections
///
/// This function:
/// 1. Creates the runtime directory if needed
/// 2. Checks for and cleans up stale sockets from previous crashes
/// 3. Binds the UnixListener to the socket path
async fn setup_socket() -> Result<UnixListener> {
    let socket_path = ccmux_utils::socket_path();
    let runtime_dir = ccmux_utils::runtime_dir();

    // Ensure runtime directory exists
    if let Err(e) = ccmux_utils::ensure_dir(&runtime_dir) {
        return Err(ccmux_utils::CcmuxError::Io(e));
    }

    // Check for stale socket
    if socket_path.exists() {
        info!("Socket file exists at {}, checking if server is running", socket_path.display());

        // Try to connect to see if a server is already running
        match tokio::net::UnixStream::connect(&socket_path).await {
            Ok(_) => {
                // Another server is running
                return Err(ccmux_utils::CcmuxError::Internal(
                    "Another ccmux server is already running".to_string()
                ));
            }
            Err(_) => {
                // Socket exists but no server - it's stale, remove it
                info!("Removing stale socket file");
                if let Err(e) = std::fs::remove_file(&socket_path) {
                    warn!("Failed to remove stale socket: {}", e);
                }
            }
        }
    }

    // Bind the listener
    info!("Binding to socket: {}", socket_path.display());
    let listener = UnixListener::bind(&socket_path).map_err(|e| {
        error!("Failed to bind socket at {}: {}", socket_path.display(), e);
        ccmux_utils::CcmuxError::Io(e)
    })?;

    info!("Server listening on {}", socket_path.display());
    Ok(listener)
}

/// Clean up socket file on shutdown
fn cleanup_socket() {
    let socket_path = ccmux_utils::socket_path();
    if socket_path.exists() {
        if let Err(e) = std::fs::remove_file(&socket_path) {
            warn!("Failed to remove socket file: {}", e);
        } else {
            info!("Removed socket file: {}", socket_path.display());
        }
    }
}

// ==================== Accept Loop ====================

/// Run the main accept loop for client connections
async fn run_accept_loop(listener: UnixListener, shared_state: SharedState) {
    // Get shutdown receiver before entering loop
    let mut shutdown_rx = shared_state.subscribe_shutdown();

    loop {
        tokio::select! {
            // Accept new connections
            result = listener.accept() => {
                match result {
                    Ok((stream, _addr)) => {
                        debug!("New client connection accepted");
                        let state_clone = shared_state.clone();
                        tokio::spawn(async move {
                            handle_client(stream, state_clone).await;
                        });
                    }
                    Err(e) => {
                        error!("Accept error: {}", e);
                        // Continue accepting - transient errors shouldn't stop the server
                    }
                }
            }

            // Handle shutdown signal
            _ = shutdown_rx.recv() => {
                info!("Shutdown signal received, stopping accept loop");
                break;
            }
        }
    }
}

// ==================== Client Handler ====================

/// Handle a single client connection
async fn handle_client(stream: UnixStream, shared_state: SharedState) {
    // Create a channel for receiving messages (broadcasts from registry)
    let (tx, mut rx) = mpsc::channel::<ServerMessage>(32);

    // Register client with the registry
    let client_id = shared_state.registry.register_client(tx);
    info!("Client {} connected", client_id);

    // Split stream for reading and writing
    let (reader, writer) = stream.into_split();
    let mut framed_reader = FramedRead::new(reader, ServerCodec::new());
    let mut framed_writer = FramedWrite::new(writer, ServerCodec::new());

    // Get shutdown receiver
    let mut shutdown_rx = shared_state.subscribe_shutdown();

    // Create handler context for this client
    let handler_ctx = HandlerContext::new(
        Arc::clone(&shared_state.session_manager),
        Arc::clone(&shared_state.pty_manager),
        Arc::clone(&shared_state.registry),
        Arc::clone(&shared_state.config),
        client_id,
        shared_state.pane_closed_tx.clone(),
        Arc::clone(&shared_state.command_executor),
    );

    // Message pump loop
    loop {
        tokio::select! {
            // Read messages from client
            result = framed_reader.next() => {
                match result {
                    Some(Ok(msg)) => {
                        debug!("Received message from {}: {:?}", client_id, msg);

                        // Route message through handlers
                        let handler_result = handler_ctx.route_message(msg).await;

                        // Process handler result
                        match handler_result {
                            HandlerResult::Response(response) => {
                                if let Err(e) = framed_writer.send(response).await {
                                    error!("Failed to send response to {}: {}", client_id, e);
                                    break;
                                }
                            }
                            HandlerResult::ResponseWithBroadcast {
                                response,
                                session_id,
                                broadcast,
                            } => {
                                debug!(
                                    client_id = %client_id,
                                    session_id = %session_id,
                                    response_type = ?std::mem::discriminant(&response),
                                    broadcast_type = ?std::mem::discriminant(&broadcast),
                                    "Received ResponseWithBroadcast from handler"
                                );

                                // Send response to this client
                                if let Err(e) = framed_writer.send(response).await {
                                    error!("Failed to send response to {}: {}", client_id, e);
                                    break;
                                }

                                // Broadcast to other clients in the session
                                debug!(
                                    session_id = %session_id,
                                    except_client = %client_id,
                                    "About to broadcast to session"
                                );
                                let broadcast_count = shared_state
                                    .registry
                                    .broadcast_to_session_except(session_id, client_id, broadcast)
                                    .await;
                                info!(
                                    session_id = %session_id,
                                    clients_notified = broadcast_count,
                                    "Broadcast complete"
                                );
                            }
                            HandlerResult::ResponseWithFollowUp {
                                response,
                                follow_up,
                            } => {
                                // Send the main response first
                                if let Err(e) = framed_writer.send(response).await {
                                    error!("Failed to send response to {}: {}", client_id, e);
                                    break;
                                }

                                // Send follow-up messages (e.g., initial scrollback on attach)
                                for msg in follow_up {
                                    if let Err(e) = framed_writer.send(msg).await {
                                        error!("Failed to send follow-up message to {}: {}", client_id, e);
                                        break;
                                    }
                                }
                            }
                            HandlerResult::NoResponse => {
                                // No response needed (e.g., Input message)
                            }
                        }
                    }
                    Some(Err(e)) => {
                        error!("Client {} read error: {}", client_id, e);
                        break;
                    }
                    None => {
                        // Client disconnected (EOF)
                        debug!("Client {} disconnected (EOF)", client_id);
                        break;
                    }
                }
            }

            // Handle messages from registry (broadcasts from other clients)
            Some(msg) = rx.recv() => {
                debug!(
                    client_id = %client_id,
                    message_type = ?std::mem::discriminant(&msg),
                    "Client handler received broadcast from channel"
                );
                if let Err(e) = framed_writer.send(msg).await {
                    error!("Failed to send broadcast to {}: {}", client_id, e);
                    break;
                }
                debug!(
                    client_id = %client_id,
                    "Broadcast written to socket successfully"
                );
            }

            // Handle shutdown signal
            _ = shutdown_rx.recv() => {
                debug!("Client {} handler received shutdown signal", client_id);
                break;
            }
        }
    }

    // Clean up: detach from session if attached
    if let Some(session_id) = shared_state.registry.get_client_session(client_id) {
        // Decrement attached client count in session
        let mut session_manager = shared_state.session_manager.write().await;
        if let Some(session) = session_manager.get_session_mut(session_id) {
            session.detach_client();
        }
    }

    // Unregister client from registry
    shared_state.registry.unregister_client(client_id);
    info!("Client {} disconnected", client_id);
}

/// Run the MCP server mode (standalone, legacy)
fn run_mcp_server() -> Result<()> {
    use mcp::McpServer;

    let mut mcp_server = McpServer::new();
    mcp_server.run().map_err(|e| ccmux_utils::CcmuxError::Internal(e.to_string()))
}

/// Run the MCP bridge mode (connects to daemon)
async fn run_mcp_bridge() -> Result<()> {
    use mcp::McpBridge;

    let mut bridge = McpBridge::new();
    bridge.run().await.map_err(|e| ccmux_utils::CcmuxError::Internal(e.to_string()))
}

/// Run the main server daemon
async fn run_daemon() -> Result<()> {
    info!("ccmux server starting");

    // Load configuration from file or use defaults
    let app_config = config::ConfigLoader::load().unwrap_or_else(|e| {
        warn!("Failed to load config, using defaults: {}", e);
        AppConfig::default()
    });

    if let Some(ref cmd) = app_config.general.default_command {
        info!("Default command for new sessions: {}", cmd);
    }

    // Create server
    let mut server = Server::new(&app_config)?;

    // Perform recovery
    match server.recover() {
        Ok(result) => {
            if result.total_panes > 0 {
                info!("{}", result.summary());
            }
        }
        Err(e) => {
            error!("Recovery failed: {}", e);
            // Continue anyway - start fresh
        }
    }

    // Clean up orphaned isolation directories
    server.cleanup_isolation();

    // Set up Unix socket
    let listener = setup_socket().await?;

    // Create shared state for client handlers
    // Extract managers into Arc<RwLock<>> for concurrent access
    let (shutdown_tx, _) = broadcast::channel(1);
    let (pane_closed_tx, pane_closed_rx) = mpsc::channel::<PaneClosedNotification>(100);

    // Create the managers first so we can reference them for the executor
    let session_manager = Arc::new(RwLock::new(std::mem::replace(
        &mut server.session_manager,
        SessionManager::new(),
    )));
    let pty_manager = Arc::new(RwLock::new(std::mem::replace(
        &mut server.pty_manager,
        PtyManager::new(),
    )));
    let registry = Arc::new(std::mem::replace(
        &mut server.client_registry,
        ClientRegistry::new(),
    ));

    // Create the sideband command executor
    let command_executor = Arc::new(AsyncCommandExecutor::new(
        Arc::clone(&session_manager),
        Arc::clone(&pty_manager),
        Arc::clone(&registry),
    ));

    let shared_state = SharedState {
        session_manager,
        pty_manager,
        registry,
        config: Arc::new(app_config),
        shutdown_tx: shutdown_tx.clone(),
        pane_closed_tx,
        command_executor,
    };

    // Store references back in server for persistence operations
    server.session_manager_ref = Some(Arc::clone(&shared_state.session_manager));
    server.pty_manager_ref = Some(Arc::clone(&shared_state.pty_manager));
    server.shutdown_tx = shutdown_tx;

    // Start output pollers for any restored panes with PTYs
    {
        let session_manager = shared_state.session_manager.read().await;
        let pty_manager = shared_state.pty_manager.read().await;

        for session in session_manager.list_sessions() {
            let session_id = session.id();
            for window in session.windows() {
                for pane in window.panes() {
                    let pane_id = pane.id();
                    // Check if this pane has a PTY (restored panes with PTYs)
                    if let Some(handle) = pty_manager.get(pane_id) {
                        let reader = handle.clone_reader();
                        // Start poller with sideband parsing enabled
                        let _poller = PtyOutputPoller::spawn_with_sideband(
                            pane_id,
                            session_id,
                            reader,
                            shared_state.registry.clone(),
                            Some(shared_state.pane_closed_tx.clone()),
                            shared_state.command_executor.clone(),
                        );
                        info!("Started output poller for restored pane {} (sideband enabled)", pane_id);
                    }
                }
            }
        }
    }

    // Wrap server in Arc<Mutex<>> for checkpoint/persistence access
    let server = Arc::new(Mutex::new(server));

    // Spawn accept loop
    let shared_state_for_accept = shared_state.clone();
    let accept_handle = tokio::spawn(async move {
        run_accept_loop(listener, shared_state_for_accept).await;
    });

    // Spawn checkpoint task
    let server_for_checkpoint = Arc::clone(&server);
    let shared_state_for_checkpoint = shared_state.clone();
    let checkpoint_handle = tokio::spawn(async move {
        run_checkpoint_loop(server_for_checkpoint, shared_state_for_checkpoint).await;
    });

    // Spawn pane cleanup task (handles cleanup when PTY processes die)
    let shared_state_for_cleanup = shared_state.clone();
    let cleanup_handle = tokio::spawn(async move {
        run_pane_cleanup_loop(pane_closed_rx, shared_state_for_cleanup).await;
    });

    // Wait for shutdown signal (SIGTERM or SIGINT)
    info!("Server ready, waiting for shutdown signal (Ctrl+C)");
    wait_for_shutdown_signal().await;

    // Signal shutdown to all tasks
    info!("Initiating graceful shutdown...");
    let _ = shared_state.shutdown_tx.send(());

    // Wait for accept loop to finish (with timeout)
    let shutdown_timeout = tokio::time::Duration::from_secs(5);
    if tokio::time::timeout(shutdown_timeout, accept_handle).await.is_err() {
        warn!("Accept loop did not shut down in time");
    }

    // Cancel background tasks
    checkpoint_handle.abort();
    cleanup_handle.abort();

    // Wait briefly for clients to disconnect
    let client_timeout = tokio::time::Duration::from_secs(2);
    let start = std::time::Instant::now();
    loop {
        let count = shared_state.registry.client_count();
        if count == 0 {
            break;
        }
        if start.elapsed() > client_timeout {
            warn!("{} clients still connected at shutdown", count);
            break;
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }

    // Perform server shutdown with shared state
    {
        let mut server_guard = server.lock().await;

        // Kill all PTYs
        let mut pty_manager = shared_state.pty_manager.write().await;
        pty_manager.kill_all();

        // Clean up isolation directories for all Claude panes
        let session_manager = shared_state.session_manager.read().await;
        for session in session_manager.list_sessions() {
            for window in session.windows() {
                for pane in window.panes() {
                    if pane.is_claude() {
                        if let Err(e) = isolation::cleanup_config_dir(pane.id()) {
                            warn!(
                                "Failed to cleanup isolation dir for pane {}: {}",
                                pane.id(),
                                e
                            );
                        }
                    }
                }
            }
        }

        // Collect final state and shutdown persistence
        if server_guard.persistence.is_some() {
            let snapshots = server_guard.collect_session_snapshots_from(&session_manager);
            drop(session_manager);

            if let Some(mut persistence) = server_guard.persistence.take() {
                if let Err(e) = persistence.shutdown(snapshots) {
                    error!("Persistence shutdown failed: {}", e);
                }
            }
        }

        info!("Shutdown complete");
    }

    // Clean up socket file
    cleanup_socket();

    info!("ccmux server stopped");
    Ok(())
}

/// Wait for SIGTERM or SIGINT signal
async fn wait_for_shutdown_signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};

        let mut sigterm = signal(SignalKind::terminate())
            .expect("Failed to register SIGTERM handler");
        let mut sigint = signal(SignalKind::interrupt())
            .expect("Failed to register SIGINT handler");

        tokio::select! {
            _ = sigterm.recv() => {
                info!("Received SIGTERM");
            }
            _ = sigint.recv() => {
                info!("Received SIGINT");
            }
        }
    }

    #[cfg(not(unix))]
    {
        // Fallback for non-Unix systems
        tokio::signal::ctrl_c().await.expect("Failed to listen for Ctrl+C");
        info!("Received Ctrl+C");
    }
}

/// Run the pane cleanup loop
///
/// This task handles cleanup when PTY processes die. When a pane's shell exits,
/// the output poller sends a notification and this loop:
/// 1. Removes the pane from session state
/// 2. Removes the window if it becomes empty
/// 3. Removes the session if it has no windows
async fn run_pane_cleanup_loop(
    mut rx: mpsc::Receiver<PaneClosedNotification>,
    shared_state: SharedState,
) {
    let mut shutdown_rx = shared_state.subscribe_shutdown();

    loop {
        tokio::select! {
            notification = rx.recv() => {
                match notification {
                    Some(PaneClosedNotification { session_id, pane_id }) => {
                        info!(
                            pane_id = %pane_id,
                            session_id = %session_id,
                            "Processing pane cleanup notification"
                        );

                        // Remove PTY if it exists
                        {
                            let mut pty_manager = shared_state.pty_manager.write().await;
                            if let Some(handle) = pty_manager.remove(pane_id) {
                                if let Err(e) = handle.kill() {
                                    warn!("Failed to kill PTY for pane {}: {}", pane_id, e);
                                }
                            }
                        }

                        // Remove pane from session and clean up empty containers
                        let mut session_manager = shared_state.session_manager.write().await;

                        // Track whether session should be removed after we release the session borrow
                        let mut should_remove_session = false;

                        if let Some(session) = session_manager.get_session_mut(session_id) {
                            // Find which window contains this pane
                            let window_id = session.windows().find_map(|w| {
                                if w.get_pane(pane_id).is_some() {
                                    Some(w.id())
                                } else {
                                    None
                                }
                            });

                            if let Some(window_id) = window_id {
                                if let Some(window) = session.get_window_mut(window_id) {
                                    if let Some(pane) = window.remove_pane(pane_id) {
                                        // Cleanup isolation directory if it was a Claude pane
                                        pane.cleanup_isolation();
                                        info!(
                                            pane_id = %pane_id,
                                            window_id = %window_id,
                                            "Pane removed from window"
                                        );
                                    }

                                    // Check if window is now empty
                                    if window.is_empty() {
                                        session.remove_window(window_id);
                                        info!(
                                            window_id = %window_id,
                                            session_id = %session_id,
                                            "Empty window removed from session"
                                        );
                                    }
                                }
                            }

                            // Check if session has no windows left
                            should_remove_session = session.window_count() == 0;
                        }

                        // Remove empty session (after releasing mutable borrow on session)
                        if should_remove_session {
                            session_manager.remove_session(session_id);
                            info!(
                                session_id = %session_id,
                                "Empty session removed"
                            );
                        }
                    }
                    None => {
                        // Channel closed, exit the loop
                        debug!("Pane cleanup channel closed");
                        break;
                    }
                }
            }
            _ = shutdown_rx.recv() => {
                debug!("Pane cleanup loop received shutdown signal");
                break;
            }
        }
    }
}

/// Run the periodic checkpoint loop
async fn run_checkpoint_loop(server: Arc<Mutex<Server>>, shared_state: SharedState) {
    let mut shutdown_rx = shared_state.subscribe_shutdown();

    let checkpoint_interval = tokio::time::Duration::from_secs(30);

    loop {
        tokio::select! {
            _ = tokio::time::sleep(checkpoint_interval) => {
                let mut server_guard = server.lock().await;
                if server_guard.is_checkpoint_due() {
                    // Collect snapshots from shared state
                    let session_manager = shared_state.session_manager.read().await;
                    let snapshots = server_guard.collect_session_snapshots_from(&session_manager);
                    drop(session_manager);

                    if let Err(e) = server_guard.checkpoint_with_snapshots(snapshots) {
                        error!("Checkpoint failed: {}", e);
                    }
                }
            }
            _ = shutdown_rx.recv() => {
                debug!("Checkpoint loop received shutdown signal");
                break;
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Check for subcommands (don't init logging for MCP modes - they use stdio)
    let args: Vec<String> = std::env::args().collect();

    if args.len() > 1 {
        match args[1].as_str() {
            "mcp-server" => {
                // Legacy standalone MCP server (has its own session state)
                return run_mcp_server();
            }
            "mcp-bridge" => {
                // MCP bridge mode - connects to daemon (recommended)
                return run_mcp_bridge().await;
            }
            "--help" | "-h" => {
                print_help();
                return Ok(());
            }
            "--version" | "-V" => {
                println!("ccmux-server {}", env!("CARGO_PKG_VERSION"));
                return Ok(());
            }
            arg => {
                eprintln!("Unknown subcommand: {}", arg);
                eprintln!("Run with --help for usage information");
                return Err(ccmux_utils::CcmuxError::Internal(format!(
                    "Unknown subcommand: {}",
                    arg
                )));
            }
        }
    }

    // For daemon mode, initialize logging
    ccmux_utils::init_logging()?;

    run_daemon().await
}

/// Print help information
fn print_help() {
    println!(
        r#"ccmux-server - Background daemon for ccmux terminal multiplexer

USAGE:
    ccmux-server [SUBCOMMAND]

SUBCOMMANDS:
    (none)          Run as daemon (default)
    mcp-bridge      Run as MCP bridge for Claude Code (recommended)
                    Connects to the running daemon, sharing sessions with TUI
    mcp-server      Run as standalone MCP server (legacy)
                    Has its own session state, separate from daemon

OPTIONS:
    -h, --help      Print this help information
    -V, --version   Print version information

EXAMPLES:
    # Start the daemon
    ccmux-server

    # Run MCP bridge for Claude Code integration
    ccmux-server mcp-bridge

    # Claude Code MCP configuration (~/.config/claude/claude_desktop_config.json):
    {{
      "mcpServers": {{
        "ccmux": {{
          "command": "ccmux-server",
          "args": ["mcp-bridge"]
        }}
      }}
    }}
"#
    );
}

// ==================== Tests ====================

#[cfg(test)]
mod tests {
    use super::*;
    use ccmux_protocol::{ClientMessage, PROTOCOL_VERSION};
    use tempfile::TempDir;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    /// Create a test server with temporary paths
    fn create_test_server() -> Server {
        let app_config = config::AppConfig::default();
        Server::new(&app_config).expect("Failed to create test server")
    }

    /// Create a SharedState for testing
    fn create_test_shared_state() -> SharedState {
        let (shutdown_tx, _) = broadcast::channel(1);
        let (pane_closed_tx, _) = mpsc::channel(100);
        let session_manager = Arc::new(RwLock::new(SessionManager::new()));
        let pty_manager = Arc::new(RwLock::new(PtyManager::new()));
        let registry = Arc::new(ClientRegistry::new());
        let command_executor = Arc::new(AsyncCommandExecutor::new(
            Arc::clone(&session_manager),
            Arc::clone(&pty_manager),
            Arc::clone(&registry),
        ));
        SharedState {
            session_manager,
            pty_manager,
            registry,
            config: Arc::new(config::AppConfig::default()),
            shutdown_tx,
            pane_closed_tx,
            command_executor,
        }
    }

    // ==================== Socket Setup Tests ====================

    #[tokio::test]
    async fn test_socket_binding() {
        let temp_dir = TempDir::new().unwrap();
        let socket_path = temp_dir.path().join("test.sock");

        // Bind listener
        let listener = UnixListener::bind(&socket_path).unwrap();
        assert!(socket_path.exists());

        // Clean up
        drop(listener);
        std::fs::remove_file(&socket_path).ok();
    }

    #[tokio::test]
    async fn test_stale_socket_detection() {
        let temp_dir = TempDir::new().unwrap();
        let socket_path = temp_dir.path().join("test.sock");

        // Create a stale socket file (not a real socket)
        std::fs::write(&socket_path, "stale").unwrap();
        assert!(socket_path.exists());

        // Try to connect - should fail since it's not a real socket
        let result = tokio::net::UnixStream::connect(&socket_path).await;
        assert!(result.is_err());

        // Bind should succeed after removing stale socket
        std::fs::remove_file(&socket_path).unwrap();
        let listener = UnixListener::bind(&socket_path).unwrap();
        assert!(socket_path.exists());

        drop(listener);
    }

    // ==================== Client Connection Tests ====================

    #[tokio::test]
    async fn test_client_connect_disconnect() {
        let temp_dir = TempDir::new().unwrap();
        let socket_path = temp_dir.path().join("test.sock");

        // Start listener
        let listener = UnixListener::bind(&socket_path).unwrap();

        // Connect client
        let connect_handle = tokio::spawn({
            let socket_path = socket_path.clone();
            async move {
                tokio::net::UnixStream::connect(&socket_path).await
            }
        });

        // Accept connection
        let (server_stream, _) = listener.accept().await.unwrap();
        let client_stream = connect_handle.await.unwrap().unwrap();

        // Both sides should be connected
        drop(client_stream);
        drop(server_stream);
    }

    #[tokio::test]
    async fn test_multiple_clients() {
        let temp_dir = TempDir::new().unwrap();
        let socket_path = temp_dir.path().join("test.sock");

        let listener = UnixListener::bind(&socket_path).unwrap();
        let num_clients = 5;

        // Spawn clients
        let client_handles: Vec<_> = (0..num_clients)
            .map(|_| {
                let socket_path = socket_path.clone();
                tokio::spawn(async move {
                    tokio::net::UnixStream::connect(&socket_path).await
                })
            })
            .collect();

        // Accept all connections
        for _ in 0..num_clients {
            let result = listener.accept().await;
            assert!(result.is_ok());
        }

        // Verify all clients connected
        for handle in client_handles {
            let result = handle.await.unwrap();
            assert!(result.is_ok());
        }
    }

    // ==================== Message Protocol Tests ====================

    #[tokio::test]
    async fn test_ping_pong() {
        let temp_dir = TempDir::new().unwrap();
        let socket_path = temp_dir.path().join("test.sock");

        let listener = UnixListener::bind(&socket_path).unwrap();
        let shared_state = create_test_shared_state();

        // Connect client
        let client_handle = tokio::spawn({
            let socket_path = socket_path.clone();
            async move {
                tokio::net::UnixStream::connect(&socket_path).await.unwrap()
            }
        });

        // Accept and handle connection
        let (server_stream, _) = listener.accept().await.unwrap();
        let mut client_stream = client_handle.await.unwrap();

        // Start server-side handler
        let state_clone = shared_state.clone();
        let server_handle = tokio::spawn(async move {
            handle_client(server_stream, state_clone).await;
        });

        // Send Ping from client
        let mut client_codec = ccmux_protocol::ClientCodec::new();
        let mut buf = bytes::BytesMut::new();
        tokio_util::codec::Encoder::encode(&mut client_codec, ClientMessage::Ping, &mut buf).unwrap();
        client_stream.write_all(&buf).await.unwrap();

        // Read response
        let mut response_buf = vec![0u8; 1024];
        let n = client_stream.read(&mut response_buf).await.unwrap();
        assert!(n > 0, "Should receive response");

        // Decode response
        let mut response_bytes = bytes::BytesMut::from(&response_buf[..n]);
        let response: ServerMessage = tokio_util::codec::Decoder::decode(&mut client_codec, &mut response_bytes)
            .unwrap()
            .unwrap();
        assert_eq!(response, ServerMessage::Pong);

        // Clean up
        drop(client_stream);
        server_handle.await.ok();
    }

    #[tokio::test]
    async fn test_connect_message() {
        let temp_dir = TempDir::new().unwrap();
        let socket_path = temp_dir.path().join("test.sock");

        let listener = UnixListener::bind(&socket_path).unwrap();
        let shared_state = create_test_shared_state();

        // Connect client
        let client_handle = tokio::spawn({
            let socket_path = socket_path.clone();
            async move {
                tokio::net::UnixStream::connect(&socket_path).await.unwrap()
            }
        });

        let (server_stream, _) = listener.accept().await.unwrap();
        let mut client_stream = client_handle.await.unwrap();

        let state_clone = shared_state.clone();
        let server_handle = tokio::spawn(async move {
            handle_client(server_stream, state_clone).await;
        });

        // Send Connect message
        let mut client_codec = ccmux_protocol::ClientCodec::new();
        let mut buf = bytes::BytesMut::new();
        let connect_msg = ClientMessage::Connect {
            client_id: uuid::Uuid::new_v4(),
            protocol_version: PROTOCOL_VERSION,
        };
        tokio_util::codec::Encoder::encode(&mut client_codec, connect_msg, &mut buf).unwrap();
        client_stream.write_all(&buf).await.unwrap();

        // Read response
        let mut response_buf = vec![0u8; 1024];
        let n = client_stream.read(&mut response_buf).await.unwrap();
        assert!(n > 0);

        // Decode response
        let mut response_bytes = bytes::BytesMut::from(&response_buf[..n]);
        let response: ServerMessage = tokio_util::codec::Decoder::decode(&mut client_codec, &mut response_bytes)
            .unwrap()
            .unwrap();

        match response {
            ServerMessage::Connected { protocol_version, .. } => {
                assert_eq!(protocol_version, PROTOCOL_VERSION);
            }
            _ => panic!("Expected Connected response, got {:?}", response),
        }

        drop(client_stream);
        server_handle.await.ok();
    }

    #[tokio::test]
    async fn test_protocol_version_mismatch() {
        let temp_dir = TempDir::new().unwrap();
        let socket_path = temp_dir.path().join("test.sock");

        let listener = UnixListener::bind(&socket_path).unwrap();
        let shared_state = create_test_shared_state();

        let client_handle = tokio::spawn({
            let socket_path = socket_path.clone();
            async move {
                tokio::net::UnixStream::connect(&socket_path).await.unwrap()
            }
        });

        let (server_stream, _) = listener.accept().await.unwrap();
        let mut client_stream = client_handle.await.unwrap();

        let state_clone = shared_state.clone();
        let server_handle = tokio::spawn(async move {
            handle_client(server_stream, state_clone).await;
        });

        // Send Connect with wrong protocol version
        let mut client_codec = ccmux_protocol::ClientCodec::new();
        let mut buf = bytes::BytesMut::new();
        let connect_msg = ClientMessage::Connect {
            client_id: uuid::Uuid::new_v4(),
            protocol_version: 9999, // Invalid version
        };
        tokio_util::codec::Encoder::encode(&mut client_codec, connect_msg, &mut buf).unwrap();
        client_stream.write_all(&buf).await.unwrap();

        // Read response
        let mut response_buf = vec![0u8; 1024];
        let n = client_stream.read(&mut response_buf).await.unwrap();
        assert!(n > 0);

        // Decode response
        let mut response_bytes = bytes::BytesMut::from(&response_buf[..n]);
        let response: ServerMessage = tokio_util::codec::Decoder::decode(&mut client_codec, &mut response_bytes)
            .unwrap()
            .unwrap();

        match response {
            ServerMessage::Error { code, .. } => {
                assert_eq!(code, ccmux_protocol::ErrorCode::ProtocolMismatch);
            }
            _ => panic!("Expected ProtocolMismatch error, got {:?}", response),
        }

        drop(client_stream);
        server_handle.await.ok();
    }

    // ==================== Server Shutdown Tests ====================

    #[tokio::test]
    async fn test_server_shutdown_signal() {
        let server = create_test_server();

        // Subscribe to shutdown
        let mut rx = server.subscribe_shutdown();

        // Signal shutdown
        server.signal_shutdown();

        // Should receive signal
        let result = tokio::time::timeout(
            std::time::Duration::from_millis(100),
            rx.recv()
        ).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_client_count_tracking() {
        let server = create_test_server();

        assert_eq!(server.active_client_count(), 0);

        server.client_connected();
        assert_eq!(server.active_client_count(), 1);

        server.client_connected();
        assert_eq!(server.active_client_count(), 2);

        server.client_disconnected();
        assert_eq!(server.active_client_count(), 1);

        server.client_disconnected();
        assert_eq!(server.active_client_count(), 0);
    }

    // ==================== Accept Loop Tests ====================

    #[tokio::test]
    async fn test_accept_loop_shutdown() {
        let temp_dir = TempDir::new().unwrap();
        let socket_path = temp_dir.path().join("test.sock");

        let listener = UnixListener::bind(&socket_path).unwrap();
        let shared_state = create_test_shared_state();

        // Start accept loop
        let state_clone = shared_state.clone();
        let accept_handle = tokio::spawn(async move {
            run_accept_loop(listener, state_clone).await;
        });

        // Give it time to start
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        // Signal shutdown via the broadcast channel
        let _ = shared_state.shutdown_tx.send(());

        // Accept loop should exit
        let result = tokio::time::timeout(
            std::time::Duration::from_secs(1),
            accept_handle
        ).await;
        assert!(result.is_ok(), "Accept loop should exit on shutdown");
    }

    #[tokio::test]
    async fn test_client_handler_shutdown() {
        let temp_dir = TempDir::new().unwrap();
        let socket_path = temp_dir.path().join("test.sock");

        let listener = UnixListener::bind(&socket_path).unwrap();
        let shared_state = create_test_shared_state();

        // Connect client
        let client_handle = tokio::spawn({
            let socket_path = socket_path.clone();
            async move {
                tokio::net::UnixStream::connect(&socket_path).await.unwrap()
            }
        });

        let (server_stream, _) = listener.accept().await.unwrap();
        let _client_stream = client_handle.await.unwrap();

        // Start handler
        let state_clone = shared_state.clone();
        let handler_handle = tokio::spawn(async move {
            handle_client(server_stream, state_clone).await;
        });

        // Give it time to start
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        // Signal shutdown via the broadcast channel
        let _ = shared_state.shutdown_tx.send(());

        // Handler should exit
        let result = tokio::time::timeout(
            std::time::Duration::from_secs(1),
            handler_handle
        ).await;
        assert!(result.is_ok(), "Client handler should exit on shutdown");
    }

    // ==================== Message Handling Tests ====================

    fn create_test_handler_context() -> HandlerContext {
        let shared_state = create_test_shared_state();

        // Register a test client
        let (tx, _rx) = mpsc::channel(10);
        let client_id = shared_state.registry.register_client(tx);

        HandlerContext::new(
            shared_state.session_manager,
            shared_state.pty_manager,
            shared_state.registry,
            shared_state.config,
            client_id,
            shared_state.pane_closed_tx,
            shared_state.command_executor,
        )
    }

    #[tokio::test]
    async fn test_route_message_ping() {
        let ctx = create_test_handler_context();
        let result = ctx.route_message(ccmux_protocol::ClientMessage::Ping).await;

        match result {
            HandlerResult::Response(ServerMessage::Pong) => {}
            _ => panic!("Expected Pong response"),
        }
    }

    #[tokio::test]
    async fn test_route_message_list_sessions() {
        let ctx = create_test_handler_context();
        let result = ctx.route_message(ccmux_protocol::ClientMessage::ListSessions).await;

        match result {
            HandlerResult::Response(ServerMessage::SessionList { sessions }) => {
                assert!(sessions.is_empty());
            }
            _ => panic!("Expected SessionList response"),
        }
    }

    #[tokio::test]
    async fn test_route_message_sync_not_attached() {
        let ctx = create_test_handler_context();
        let result = ctx.route_message(ccmux_protocol::ClientMessage::Sync).await;

        // When not attached to a session, Sync returns SessionList
        match result {
            HandlerResult::Response(ServerMessage::SessionList { sessions }) => {
                assert!(sessions.is_empty());
            }
            _ => panic!("Expected SessionList response"),
        }
    }

    #[tokio::test]
    async fn test_route_message_detach_not_attached() {
        let ctx = create_test_handler_context();
        let result = ctx.route_message(ccmux_protocol::ClientMessage::Detach).await;

        // Detach when not attached returns SessionList (current sessions)
        match result {
            HandlerResult::Response(ServerMessage::SessionList { sessions }) => {
                assert!(sessions.is_empty());
            }
            _ => panic!("Expected SessionList response"),
        }
    }

    // ==================== MCP-to-TUI Broadcast Integration Test (BUG-010) ====================

    /// Full integration test for MCP-to-TUI broadcast through sockets
    ///
    /// This tests the COMPLETE path including:
    /// 1. TUI client connects and attaches to session
    /// 2. MCP client connects
    /// 3. MCP creates pane (returns ResponseWithBroadcast)
    /// 4. Server broadcasts PaneCreated to TUI
    /// 5. TUI receives PaneCreated through socket
    ///
    /// This is a BUG-010 investigation test to verify the full broadcast path.
    #[tokio::test]
    async fn test_mcp_to_tui_broadcast_via_socket() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio_util::codec::{Decoder, Encoder};

        let temp_dir = TempDir::new().unwrap();
        let socket_path = temp_dir.path().join("test.sock");

        let listener = UnixListener::bind(&socket_path).unwrap();
        let shared_state = create_test_shared_state();

        // --- Connect TUI client ---
        let tui_client_handle = tokio::spawn({
            let socket_path = socket_path.clone();
            async move {
                tokio::net::UnixStream::connect(&socket_path).await.unwrap()
            }
        });

        let (tui_server_stream, _) = listener.accept().await.unwrap();
        let mut tui_client_stream = tui_client_handle.await.unwrap();

        let state_clone = shared_state.clone();
        let tui_server_handle = tokio::spawn(async move {
            handle_client(tui_server_stream, state_clone).await;
        });

        // --- TUI: Send Connect ---
        let mut tui_codec = ccmux_protocol::ClientCodec::new();
        let mut buf = bytes::BytesMut::new();
        let connect_msg = ClientMessage::Connect {
            client_id: uuid::Uuid::new_v4(),
            protocol_version: PROTOCOL_VERSION,
        };
        Encoder::encode(&mut tui_codec, connect_msg, &mut buf).unwrap();
        tui_client_stream.write_all(&buf).await.unwrap();

        // Read Connected response
        let mut response_buf = vec![0u8; 4096];
        let n = tui_client_stream.read(&mut response_buf).await.unwrap();
        let mut response_bytes = bytes::BytesMut::from(&response_buf[..n]);
        let response: ServerMessage = Decoder::decode(&mut tui_codec, &mut response_bytes)
            .unwrap()
            .unwrap();
        assert!(matches!(response, ServerMessage::Connected { .. }), "Expected Connected");

        // --- TUI: Create a session ---
        buf.clear();
        let create_session_msg = ClientMessage::CreateSession {
            name: "test-session".to_string(),
            command: None,
        };
        Encoder::encode(&mut tui_codec, create_session_msg, &mut buf).unwrap();
        tui_client_stream.write_all(&buf).await.unwrap();

        // Read SessionCreated response
        let n = tui_client_stream.read(&mut response_buf).await.unwrap();
        let mut response_bytes = bytes::BytesMut::from(&response_buf[..n]);
        let response: ServerMessage = Decoder::decode(&mut tui_codec, &mut response_bytes)
            .unwrap()
            .unwrap();
        let session_id = match response {
            ServerMessage::SessionCreated { session } => session.id,
            _ => panic!("Expected SessionCreated, got {:?}", response),
        };

        // --- TUI: Attach to session ---
        buf.clear();
        let attach_msg = ClientMessage::AttachSession { session_id };
        Encoder::encode(&mut tui_codec, attach_msg, &mut buf).unwrap();
        tui_client_stream.write_all(&buf).await.unwrap();

        // Read Attached response
        let n = tui_client_stream.read(&mut response_buf).await.unwrap();
        let mut response_bytes = bytes::BytesMut::from(&response_buf[..n]);
        let response: ServerMessage = Decoder::decode(&mut tui_codec, &mut response_bytes)
            .unwrap()
            .unwrap();
        assert!(matches!(response, ServerMessage::Attached { .. }), "Expected Attached");

        // Verify TUI is registered in session_clients
        let tui_count = shared_state.registry.session_client_count(session_id);
        assert_eq!(tui_count, 1, "TUI should be attached to session");

        // --- Connect MCP client ---
        let mcp_client_handle = tokio::spawn({
            let socket_path = socket_path.clone();
            async move {
                tokio::net::UnixStream::connect(&socket_path).await.unwrap()
            }
        });

        let (mcp_server_stream, _) = listener.accept().await.unwrap();
        let mut mcp_client_stream = mcp_client_handle.await.unwrap();

        let state_clone = shared_state.clone();
        let _mcp_server_handle = tokio::spawn(async move {
            handle_client(mcp_server_stream, state_clone).await;
        });

        // --- MCP: Send Connect ---
        let mut mcp_codec = ccmux_protocol::ClientCodec::new();
        buf.clear();
        let connect_msg = ClientMessage::Connect {
            client_id: uuid::Uuid::new_v4(),
            protocol_version: PROTOCOL_VERSION,
        };
        Encoder::encode(&mut mcp_codec, connect_msg, &mut buf).unwrap();
        mcp_client_stream.write_all(&buf).await.unwrap();

        // Read Connected response
        let n = mcp_client_stream.read(&mut response_buf).await.unwrap();
        let mut response_bytes = bytes::BytesMut::from(&response_buf[..n]);
        let response: ServerMessage = Decoder::decode(&mut mcp_codec, &mut response_bytes)
            .unwrap()
            .unwrap();
        assert!(matches!(response, ServerMessage::Connected { .. }), "Expected Connected");

        // Small delay to ensure TUI's handler is waiting in select loop
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        // --- MCP: Create pane (should broadcast to TUI) ---
        buf.clear();
        let create_pane_msg = ClientMessage::CreatePaneWithOptions {
            session_filter: None,  // Use active session
            window_filter: None,
            direction: ccmux_protocol::SplitDirection::Vertical,
            command: None,
            cwd: None,
        };
        Encoder::encode(&mut mcp_codec, create_pane_msg, &mut buf).unwrap();
        mcp_client_stream.write_all(&buf).await.unwrap();

        // MCP reads its response (PaneCreatedWithDetails)
        let n = mcp_client_stream.read(&mut response_buf).await.unwrap();
        let mut response_bytes = bytes::BytesMut::from(&response_buf[..n]);
        let mcp_response: ServerMessage = Decoder::decode(&mut mcp_codec, &mut response_bytes)
            .unwrap()
            .unwrap();
        assert!(
            matches!(mcp_response, ServerMessage::PaneCreatedWithDetails { .. }),
            "MCP should receive PaneCreatedWithDetails, got {:?}", mcp_response
        );

        // --- TUI should receive PaneCreated broadcast ---
        // Use timeout to avoid hanging forever if broadcast fails
        let tui_broadcast_result = tokio::time::timeout(
            std::time::Duration::from_secs(2),
            async {
                let n = tui_client_stream.read(&mut response_buf).await.unwrap();
                let mut response_bytes = bytes::BytesMut::from(&response_buf[..n]);
                Decoder::decode(&mut tui_codec, &mut response_bytes)
                    .unwrap()
                    .unwrap()
            }
        ).await;

        match tui_broadcast_result {
            Ok(msg) => {
                match msg {
                    ServerMessage::PaneCreated { pane, .. } => {
                        // Success! TUI received the broadcast
                        assert!(!pane.id.is_nil(), "Pane ID should be valid");
                    }
                    other => panic!(
                        "TUI received unexpected message: {:?}. Expected PaneCreated broadcast.",
                        other
                    ),
                }
            }
            Err(_) => {
                panic!(
                    "TIMEOUT: TUI did not receive PaneCreated broadcast within 2 seconds. \
                     This confirms BUG-010: MCP pane creation broadcast is not reaching TUI."
                );
            }
        }

        // Clean up
        drop(tui_client_stream);
        drop(mcp_client_stream);
        tui_server_handle.await.ok();
    }
}
