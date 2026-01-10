//! Async command executor for sideband commands
//!
//! This is an async-compatible version of CommandExecutor that works with
//! tokio::sync::RwLock for integration with SharedState.

use std::sync::Arc;

use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use ccmux_protocol::ServerMessage;

use super::commands::{ControlAction, NotifyLevel, PaneRef, SidebandCommand, SplitDirection};
use super::executor::{ExecuteError, ExecuteResult, SpawnResult};
use crate::pty::{PtyConfig, PtyManager};
use crate::registry::ClientRegistry;
use crate::session::SessionManager;

/// Async executor for sideband commands
///
/// Works with tokio::sync::RwLock for integration with async SharedState.
/// Provides the same functionality as CommandExecutor but with async methods.
pub struct AsyncCommandExecutor {
    /// Reference to the session manager (tokio RwLock)
    session_manager: Arc<RwLock<SessionManager>>,
    /// Reference to the PTY manager (tokio RwLock)
    pty_manager: Arc<RwLock<PtyManager>>,
    /// Reference to the client registry for broadcasting notifications
    registry: Arc<ClientRegistry>,
}

impl AsyncCommandExecutor {
    /// Create a new async command executor
    pub fn new(
        session_manager: Arc<RwLock<SessionManager>>,
        pty_manager: Arc<RwLock<PtyManager>>,
        registry: Arc<ClientRegistry>,
    ) -> Self {
        Self {
            session_manager,
            pty_manager,
            registry,
        }
    }

    /// Get a reference to the session manager
    ///
    /// Used by PtyOutputPoller to route output to pane state for
    /// scrollback buffer and Claude detection.
    pub fn session_manager(&self) -> &Arc<RwLock<SessionManager>> {
        &self.session_manager
    }

    /// Get a reference to the client registry
    ///
    /// Used by PtyOutputPoller to broadcast state changes.
    pub fn registry(&self) -> &Arc<ClientRegistry> {
        &self.registry
    }

    /// Execute a sideband command
    ///
    /// # Arguments
    /// * `command` - The command to execute
    /// * `source_pane` - The UUID of the pane that emitted the command
    ///
    /// Note: For spawn commands, use `execute_spawn_command` instead to get
    /// the SpawnResult with PTY reader for starting the output poller.
    pub async fn execute(
        &self,
        command: SidebandCommand,
        source_pane: Uuid,
    ) -> ExecuteResult<()> {
        debug!("Executing sideband command: {:?}", command);

        match command {
            SidebandCommand::Spawn {
                direction,
                command,
                cwd,
            } => {
                // Execute spawn and broadcast notification, but discard SpawnResult
                // The caller should use execute_spawn_command if they need the result
                let _ = self.execute_spawn_internal(source_pane, direction, command, cwd).await?;
                Ok(())
            }

            SidebandCommand::Focus { pane } => self.execute_focus(source_pane, pane).await,

            SidebandCommand::Input { pane, text } => self.execute_input(source_pane, pane, text).await,

            SidebandCommand::Scroll { pane, lines } => {
                self.execute_scroll(source_pane, pane, lines).await
            }

            SidebandCommand::Notify {
                title,
                message,
                level,
            } => self.execute_notify(title, message, level),

            SidebandCommand::Control { action, pane } => {
                self.execute_control(source_pane, Some(pane), action).await
            }
        }
    }

    /// Execute a spawn command and return the result
    ///
    /// This is the preferred method for executing spawn commands, as it returns
    /// the SpawnResult containing the PTY reader needed for the output poller.
    pub async fn execute_spawn_command(
        &self,
        source_pane: Uuid,
        direction: SplitDirection,
        command: Option<String>,
        cwd: Option<String>,
    ) -> ExecuteResult<SpawnResult> {
        self.execute_spawn_internal(source_pane, direction, command, cwd).await
    }

    /// Resolve a pane reference to a concrete UUID
    async fn resolve_pane_ref(&self, pane_ref: PaneRef, source_pane: Uuid) -> ExecuteResult<Uuid> {
        let manager = self.session_manager.read().await;

        match pane_ref {
            PaneRef::Active => Ok(source_pane),
            PaneRef::Id(id) => {
                if manager.find_pane(id).is_some() {
                    Ok(id)
                } else {
                    Err(ExecuteError::PaneNotFound(id.to_string()))
                }
            }
            PaneRef::Index(idx) => {
                // Find the window containing the source pane, then get pane by index
                if let Some((_, window, _)) = manager.find_pane(source_pane) {
                    if let Some(pane) = window.get_pane_by_index(idx) {
                        Ok(pane.id())
                    } else {
                        Err(ExecuteError::PaneNotFound(format!("index {}", idx)))
                    }
                } else {
                    Err(ExecuteError::PaneNotFound(format!(
                        "source pane {} not found",
                        source_pane
                    )))
                }
            }
        }
    }

    /// Execute spawn command - create a new pane with PTY
    ///
    /// Creates a new pane in the same window as source_pane, spawns a PTY,
    /// and broadcasts the pane creation to connected clients.
    async fn execute_spawn_internal(
        &self,
        source_pane: Uuid,
        direction: SplitDirection,
        command: Option<String>,
        cwd: Option<String>,
    ) -> ExecuteResult<SpawnResult> {
        info!(
            "Spawn requested: direction={:?}, command={:?}, cwd={:?}",
            direction, command, cwd
        );

        // Step 1: Create the new pane in SessionManager
        let (session_id, pane_id, pane_info, pane_cwd, pane_size) = {
            let mut manager = self.session_manager.write().await;

            let (session_id, _window_id, new_pane) = manager
                .split_pane(source_pane, cwd.clone())
                .map_err(|e| ExecuteError::ExecutionFailed(e.to_string()))?;

            let pane_info = new_pane.to_info();
            let pane_id = new_pane.id();
            let pane_cwd = new_pane.cwd().map(String::from);
            let pane_size = new_pane.dimensions();

            (session_id, pane_id, pane_info, pane_cwd, pane_size)
        };

        info!(
            "Created new pane {} in session {} (direction: {:?})",
            pane_id, session_id, direction
        );

        // Step 2: Build PTY configuration
        let pty_config = if let Some(cmd) = &command {
            // Run a specific command
            // Parse command string - first word is command, rest are args
            let parts: Vec<&str> = cmd.split_whitespace().collect();
            if parts.is_empty() {
                PtyConfig::shell()
            } else {
                let mut config = PtyConfig::command(parts[0]);
                for arg in &parts[1..] {
                    config = config.with_arg(*arg);
                }
                config
            }
        } else {
            // Default to shell
            PtyConfig::shell()
        };

        // Apply cwd and size
        let pty_config = if let Some(cwd_path) = &pane_cwd {
            pty_config.with_cwd(cwd_path)
        } else {
            pty_config
        };
        let pty_config = pty_config.with_size(pane_size.0, pane_size.1);

        // Step 3: Spawn PTY for the new pane
        let pty_reader = {
            let mut pty_manager = self.pty_manager.write().await;

            pty_manager
                .spawn(pane_id, pty_config)
                .map_err(|e| ExecuteError::PtySpawnFailed(e.to_string()))?;

            // Get the reader for the output poller
            let handle = pty_manager.get(pane_id).unwrap();
            handle.clone_reader()
        };

        info!("Spawned PTY for new pane {}", pane_id);

        // Step 4: Broadcast PaneCreated to connected clients
        let msg = ServerMessage::PaneCreated {
            pane: pane_info.clone(),
            direction: direction.into(),
        };
        let delivered = self.registry.try_broadcast_to_session(session_id, msg);
        debug!(
            "Broadcast PaneCreated to {} clients in session {}",
            delivered, session_id
        );

        Ok(SpawnResult {
            session_id,
            pane_id,
            pane_info,
            pty_reader,
        })
    }

    /// Execute focus command - focus a specific pane
    async fn execute_focus(&self, source_pane: Uuid, pane: PaneRef) -> ExecuteResult<()> {
        let target_pane = self.resolve_pane_ref(pane, source_pane).await?;

        info!("Focus requested for pane: {}", target_pane);

        // TODO: Implement pane focusing
        warn!("Focus command not yet fully implemented - pane: {}", target_pane);

        Ok(())
    }

    /// Execute input command - send input to a pane
    async fn execute_input(
        &self,
        source_pane: Uuid,
        pane: PaneRef,
        text: String,
    ) -> ExecuteResult<()> {
        let target_pane = self.resolve_pane_ref(pane, source_pane).await?;

        info!("Input requested for pane {}: {:?}", target_pane, text);

        // TODO: Implement input sending
        warn!(
            "Input command not yet fully implemented - pane: {}, text_len: {}",
            target_pane,
            text.len()
        );

        Ok(())
    }

    /// Execute scroll command - scroll pane content
    async fn execute_scroll(
        &self,
        source_pane: Uuid,
        pane: Option<PaneRef>,
        lines: i32,
    ) -> ExecuteResult<()> {
        let target_pane = match pane {
            Some(p) => self.resolve_pane_ref(p, source_pane).await?,
            None => source_pane,
        };

        info!("Scroll requested for pane {}: {} lines", target_pane, lines);

        // TODO: Implement scrolling
        warn!(
            "Scroll command not yet fully implemented - pane: {}, lines: {}",
            target_pane, lines
        );

        Ok(())
    }

    /// Execute notify command - show a notification
    fn execute_notify(
        &self,
        title: Option<String>,
        message: String,
        level: NotifyLevel,
    ) -> ExecuteResult<()> {
        // Notifications are logged and will be broadcast to clients
        let level_str = match level {
            NotifyLevel::Info => "INFO",
            NotifyLevel::Warning => "WARN",
            NotifyLevel::Error => "ERROR",
        };

        match level {
            NotifyLevel::Info => {
                info!(
                    "[SIDEBAND NOTIFY - {}] {}: {}",
                    level_str,
                    title.as_deref().unwrap_or("Notification"),
                    message
                );
            }
            NotifyLevel::Warning => {
                warn!(
                    "[SIDEBAND NOTIFY - {}] {}: {}",
                    level_str,
                    title.as_deref().unwrap_or("Warning"),
                    message
                );
            }
            NotifyLevel::Error => {
                error!(
                    "[SIDEBAND NOTIFY - {}] {}: {}",
                    level_str,
                    title.as_deref().unwrap_or("Error"),
                    message
                );
            }
        }

        Ok(())
    }

    /// Execute control command - pane control actions
    async fn execute_control(
        &self,
        source_pane: Uuid,
        pane: Option<PaneRef>,
        action: ControlAction,
    ) -> ExecuteResult<()> {
        let target_pane = match pane {
            Some(p) => self.resolve_pane_ref(p, source_pane).await?,
            None => source_pane,
        };

        match action {
            ControlAction::Resize { cols, rows } => {
                info!(
                    "Resize requested for pane {}: {}x{}",
                    target_pane, cols, rows
                );

                // Update pane dimensions in session manager
                let mut manager = self.session_manager.write().await;
                if let Some(pane) = manager.find_pane_mut(target_pane) {
                    pane.resize(cols, rows);
                    info!("Pane {} resized to {}x{}", target_pane, cols, rows);
                } else {
                    return Err(ExecuteError::PaneNotFound(target_pane.to_string()));
                }
                drop(manager);

                // Also resize the PTY
                let pty_manager = self.pty_manager.read().await;
                if let Some(handle) = pty_manager.get(target_pane) {
                    if let Err(e) = handle.resize(cols, rows) {
                        warn!("Failed to resize PTY for pane {}: {}", target_pane, e);
                    }
                }
            }

            ControlAction::Pin => {
                info!("Pin requested for pane: {}", target_pane);
                warn!("Pin command not yet implemented");
            }

            ControlAction::Unpin => {
                info!("Unpin requested for pane: {}", target_pane);
                warn!("Unpin command not yet implemented");
            }

            ControlAction::Close => {
                info!("Close requested for pane: {}", target_pane);
                warn!("Close command not yet implemented");
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pty::PtyManager;
    use crate::registry::ClientRegistry;
    use crate::session::SessionManager;

    fn create_test_executor() -> (AsyncCommandExecutor, Arc<RwLock<SessionManager>>) {
        let manager = Arc::new(RwLock::new(SessionManager::new()));
        let pty_manager = Arc::new(RwLock::new(PtyManager::new()));
        let registry = Arc::new(ClientRegistry::new());
        let executor = AsyncCommandExecutor::new(
            Arc::clone(&manager),
            pty_manager,
            registry,
        );
        (executor, manager)
    }

    #[tokio::test]
    async fn test_notify_info() {
        let (executor, _) = create_test_executor();

        let result = executor.execute(
            SidebandCommand::Notify {
                title: Some("Test".to_string()),
                message: "Hello".to_string(),
                level: NotifyLevel::Info,
            },
            Uuid::new_v4(),
        ).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_notify_warning() {
        let (executor, _) = create_test_executor();

        let result = executor.execute(
            SidebandCommand::Notify {
                title: None,
                message: "Warning message".to_string(),
                level: NotifyLevel::Warning,
            },
            Uuid::new_v4(),
        ).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_notify_error() {
        let (executor, _) = create_test_executor();

        let result = executor.execute(
            SidebandCommand::Notify {
                title: Some("Error".to_string()),
                message: "Something went wrong".to_string(),
                level: NotifyLevel::Error,
            },
            Uuid::new_v4(),
        ).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_focus_nonexistent_pane() {
        let (executor, _) = create_test_executor();

        let result = executor.execute(
            SidebandCommand::Focus {
                pane: PaneRef::Id(Uuid::new_v4()),
            },
            Uuid::new_v4(),
        ).await;

        assert!(matches!(result, Err(ExecuteError::PaneNotFound(_))));
    }

    #[tokio::test]
    async fn test_input_nonexistent_pane() {
        let (executor, _) = create_test_executor();

        let result = executor.execute(
            SidebandCommand::Input {
                pane: PaneRef::Id(Uuid::new_v4()),
                text: "test input".to_string(),
            },
            Uuid::new_v4(),
        ).await;

        assert!(matches!(result, Err(ExecuteError::PaneNotFound(_))));
    }

    #[tokio::test]
    async fn test_scroll_with_source_pane() {
        let (executor, manager) = create_test_executor();

        // Create a session with a pane
        let pane_id = {
            let mut mgr = manager.write().await;
            let session = mgr.create_session("test").unwrap();
            let session_id = session.id();

            let session = mgr.get_session_mut(session_id).unwrap();
            let window = session.create_window(None);
            let window_id = window.id();

            let window = session.get_window_mut(window_id).unwrap();
            let pane = window.create_pane();
            pane.id()
        };

        // Scroll with no explicit pane (uses source pane)
        let result = executor.execute(
            SidebandCommand::Scroll {
                pane: None,
                lines: -10,
            },
            pane_id,
        ).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_control_pin_unpin() {
        let (executor, manager) = create_test_executor();

        // Create a session with a pane
        let pane_id = {
            let mut mgr = manager.write().await;
            let session = mgr.create_session("test").unwrap();
            let session_id = session.id();

            let session = mgr.get_session_mut(session_id).unwrap();
            let window = session.create_window(None);
            let window_id = window.id();

            let window = session.get_window_mut(window_id).unwrap();
            let pane = window.create_pane();
            pane.id()
        };

        // Test pin
        let result = executor.execute(
            SidebandCommand::Control {
                action: ControlAction::Pin,
                pane: PaneRef::Active,
            },
            pane_id,
        ).await;
        assert!(result.is_ok());

        // Test unpin
        let result = executor.execute(
            SidebandCommand::Control {
                action: ControlAction::Unpin,
                pane: PaneRef::Active,
            },
            pane_id,
        ).await;
        assert!(result.is_ok());
    }
}
