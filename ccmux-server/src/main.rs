//! ccmux server - Background daemon

use std::path::PathBuf;

use tracing::{error, info, warn};

use ccmux_utils::Result;

mod claude;
mod config;
mod isolation;
pub mod mcp;
#[allow(dead_code)]
mod orchestration;
mod parser;
#[allow(dead_code)]
mod persistence;
mod pty;
mod reply;
mod session;
pub mod sideband;

pub use reply::{ReplyError, ReplyHandler};

use config::AppConfig;
use persistence::{
    parse_compression_method, PersistenceConfig, PersistenceManager, RestorationResult,
    ScrollbackCapture, ScrollbackConfig, SessionRestorer, SessionSnapshot, WindowSnapshot,
};
use pty::PtyManager;
use session::SessionManager;

/// Server state container
pub struct Server {
    /// Session manager
    session_manager: SessionManager,
    /// PTY manager
    pty_manager: PtyManager,
    /// Persistence manager (optional if disabled)
    persistence: Option<PersistenceManager>,
    /// Scrollback capture config
    scrollback_config: ScrollbackConfig,
}

impl Server {
    /// Create a new server with the given configuration
    pub fn new(app_config: &AppConfig) -> Result<Self> {
        let persistence_config = &app_config.persistence;

        let mut server = Self {
            session_manager: SessionManager::new(),
            pty_manager: PtyManager::new(),
            persistence: None,
            scrollback_config: ScrollbackConfig {
                max_lines: persistence_config.screen_snapshot_lines,
                compression: parse_compression_method(&persistence_config.compression_method),
                ..Default::default()
            },
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
}

/// Run the MCP server mode
fn run_mcp_server() -> Result<()> {
    use mcp::McpServer;

    let mut mcp_server = McpServer::new();
    mcp_server.run().map_err(|e| ccmux_utils::CcmuxError::Internal(e.to_string()))
}

/// Run the main server daemon
async fn run_daemon() -> Result<()> {
    info!("ccmux server starting");

    // Load configuration
    let app_config = AppConfig::default();

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

    // TODO: Implement main server loop
    // - Listen for client connections
    // - Handle RPC requests
    // - Periodic checkpoints: if server.is_checkpoint_due() { server.checkpoint()?; }
    // - Log state changes to WAL

    // Graceful shutdown
    server.shutdown()?;

    info!("ccmux server stopped");
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    // Check for mcp-server subcommand (don't init logging for MCP - it uses stdio)
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 && args[1] == "mcp-server" {
        return run_mcp_server();
    }

    // For daemon mode, initialize logging
    ccmux_utils::init_logging()?;

    run_daemon().await
}
