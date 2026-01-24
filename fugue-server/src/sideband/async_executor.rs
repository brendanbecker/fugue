//! Async command executor for sideband commands
//!
//! This is an async-compatible version of CommandExecutor that works with
//! tokio::sync::RwLock for integration with SharedState.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use serde::Deserialize;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use fugue_protocol::{MailPriority, ServerMessage};

use super::commands::{ControlAction, NotifyLevel, PaneRef, SidebandCommand, SplitDirection};
use super::executor::{ExecuteError, ExecuteResult, SpawnResult};
use crate::pty::{PtyConfig, PtyManager};
use crate::registry::ClientRegistry;
use crate::session::SessionManager;

/// Configuration for spawn limits to prevent runaway pane creation
#[derive(Debug, Clone)]
pub struct SpawnLimits {
    /// Maximum spawn chain depth (pane spawning pane spawning pane...)
    /// Default: 5
    pub max_spawn_depth: usize,
    /// Maximum total panes per session
    /// Default: 50
    pub max_panes_per_session: usize,
}

impl Default for SpawnLimits {
    fn default() -> Self {
        Self {
            max_spawn_depth: 5,
            max_panes_per_session: 50,
        }
    }
}

/// Sideband spawn configuration payload
#[derive(Debug, Deserialize)]
struct SpawnConfig {
    #[serde(default)]
    env: HashMap<String, String>,
    #[serde(default)]
    timeout_secs: Option<u64>,
    #[serde(default)]
    sandbox: bool,
}

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
    /// Spawn limits configuration
    spawn_limits: SpawnLimits,
    /// Current total sideband-spawned panes (for rate limiting)
    sideband_spawn_count: AtomicUsize,
}

impl AsyncCommandExecutor {
    /// Create a new async command executor with default spawn limits
    pub fn new(
        session_manager: Arc<RwLock<SessionManager>>,
        pty_manager: Arc<RwLock<PtyManager>>,
        registry: Arc<ClientRegistry>,
    ) -> Self {
        Self::with_limits(session_manager, pty_manager, registry, SpawnLimits::default())
    }

    /// Create a new async command executor with custom spawn limits
    pub fn with_limits(
        session_manager: Arc<RwLock<SessionManager>>,
        pty_manager: Arc<RwLock<PtyManager>>,
        registry: Arc<ClientRegistry>,
        spawn_limits: SpawnLimits,
    ) -> Self {
        Self {
            session_manager,
            pty_manager,
            registry,
            spawn_limits,
            sideband_spawn_count: AtomicUsize::new(0),
        }
    }

    /// Get the current spawn limits
    pub fn spawn_limits(&self) -> &SpawnLimits {
        &self.spawn_limits
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

    /// Get a reference to the PTY manager
    ///
    /// Used by PtyOutputPoller to write DSR responses back to the PTY.
    pub fn pty_manager(&self) -> &Arc<RwLock<PtyManager>> {
        &self.pty_manager
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
                config,
            } => {
                // Execute spawn and broadcast notification, but discard SpawnResult
                // The caller should use execute_spawn_command if they need the result
                let _ = self.execute_spawn_internal(source_pane, direction, command, cwd, config).await?;
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

            SidebandCommand::Mail { summary, priority } => {
                self.execute_mail(source_pane, summary, priority).await
            }

            SidebandCommand::Control { action, pane } => {
                self.execute_control(source_pane, Some(pane), action).await
            }

            SidebandCommand::AdvertiseCapabilities { capabilities } => {
                self.execute_advertise_capabilities(source_pane, capabilities).await
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
        config: Option<String>,
    ) -> ExecuteResult<SpawnResult> {
        self.execute_spawn_internal(source_pane, direction, command, cwd, config).await
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
    ///
    /// Respects spawn limits to prevent runaway pane creation.
    async fn execute_spawn_internal(
        &self,
        source_pane: Uuid,
        direction: SplitDirection,
        command: Option<String>,
        cwd: Option<String>,
        config: Option<String>,
    ) -> ExecuteResult<SpawnResult> {
        info!(
            "Spawn requested: direction={:?}, command={:?}, cwd={:?}, config_len={:?}",
            direction, command, cwd, config.as_ref().map(|s| s.len())
        );

        // Check spawn limits before proceeding
        let current_spawn_count = self.sideband_spawn_count.load(Ordering::SeqCst);
        if current_spawn_count >= self.spawn_limits.max_panes_per_session {
            warn!(
                "Spawn limit reached: {} sideband-spawned panes (max: {})",
                current_spawn_count, self.spawn_limits.max_panes_per_session
            );
            return Err(ExecuteError::ExecutionFailed(format!(
                "Spawn limit reached: maximum {} sideband-spawned panes allowed",
                self.spawn_limits.max_panes_per_session
            )));
        }

        // Parse configuration if present
        let spawn_config: Option<SpawnConfig> = if let Some(config_json) = &config {
            match serde_json::from_str(config_json) {
                Ok(c) => Some(c),
                Err(e) => {
                    warn!("Failed to parse spawn config: {}", e);
                    return Err(ExecuteError::ExecutionFailed(format!(
                        "Invalid spawn config: {}", e
                    )));
                }
            }
        } else {
            None
        };

        // Step 1: Create the new pane in SessionManager
        let (session_id, window_id, pane_id, pane_info, pane_cwd, pane_size, session_name) = {
            let mut manager = self.session_manager.write().await;

            let (session_id, window_id, new_pane) = manager
                .split_pane(source_pane, cwd.clone())
                .map_err(|e| ExecuteError::ExecutionFailed(e.to_string()))?;

            // Extract pane info before borrowing manager again
            let pane_info = new_pane.to_info();
            let pane_id = new_pane.id();
            let pane_cwd = new_pane.cwd().map(String::from);
            let pane_size = new_pane.dimensions();

            // Now we can safely get session name
            let session_name = manager
                .get_session(session_id)
                .map(|s| s.name().to_string())
                .unwrap_or_default();

            (session_id, window_id, pane_id, pane_info, pane_cwd, pane_size, session_name)
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
        let mut pty_config = if let Some(cwd_path) = &pane_cwd {
            pty_config.with_cwd(cwd_path)
        } else {
            pty_config
        };

        // Apply environment variables from config
        if let Some(cfg) = &spawn_config {
            if !cfg.env.is_empty() {
                debug!("Applying {} environment variables from config", cfg.env.len());
                pty_config = pty_config.with_env_map(&cfg.env);
            }
            
            // Note: timeout_secs handling is pending (FEAT-080 task 4)
            if let Some(timeout) = cfg.timeout_secs {
                info!("Scheduling auto-kill for pane {} after {} seconds", pane_id, timeout);
                let session_manager = self.session_manager.clone();
                let pty_manager = self.pty_manager.clone();
                let registry = self.registry.clone();
                
                tokio::spawn(async move {
                    tokio::time::sleep(tokio::time::Duration::from_secs(timeout)).await;
                    info!("Timeout reached for pane {}, closing...", pane_id);
                    
                    // 1. Find session info
                    let location = {
                        let manager = session_manager.read().await;
                        manager.find_pane(pane_id).map(|(s, w, _)| (s.id(), w.id()))
                    };
                    
                    if let Some((session_id, window_id)) = location {
                        // 2. Remove PTY
                        {
                            let mut pty_mgr = pty_manager.write().await;
                            if let Some(handle) = pty_mgr.remove(pane_id) {
                                if let Err(e) = handle.kill() {
                                    warn!("Failed to kill PTY for pane {}: {}", pane_id, e);
                                }
                            }
                        }
                        
                        // 3. Remove pane from session
                        {
                            let mut session_mgr = session_manager.write().await;
                            if let Some(session) = session_mgr.get_session_mut(session_id) {
                                if let Some(window) = session.get_window_mut(window_id) {
                                    if let Some(pane) = window.remove_pane(pane_id) {
                                        // 4. Cleanup isolation
                                        pane.cleanup_isolation();
                                        info!("Pane {} closed successfully due to timeout", pane_id);
                                        
                                        // 5. Broadcast PaneClosed
                                        let msg = ServerMessage::PaneClosed {
                                            pane_id,
                                            exit_code: None,
                                        };
                                        registry.broadcast_to_session(session_id, msg).await;
                                    }
                                }
                            }
                        }
                    } else {
                        warn!("Pane {} not found for timeout closure (may have been closed already)", pane_id);
                    }
                });
            }

            // Apply sandbox if requested (FEAT-081)
            if cfg.sandbox {
                match std::env::current_exe() {
                    Ok(exe_path) => {
                        let sandbox_path = exe_path.parent().unwrap().join("fugue-sandbox");
                        if sandbox_path.exists() {
                            info!("Applying sandbox wrapper: {:?}", sandbox_path);
                            
                            // Reconstruct command: sandbox <cmd> <args...>
                            let original_cmd = pty_config.command.clone();
                            let mut new_args = vec![original_cmd];
                            new_args.extend(pty_config.args.clone());
                            
                            pty_config.command = sandbox_path.to_string_lossy().to_string();
                            pty_config.args = new_args;
                        } else {
                            warn!("Sandbox requested but fugue-sandbox binary not found at {:?}", sandbox_path);
                            return Err(ExecuteError::ExecutionFailed("Sandbox requested but fugue-sandbox binary not found".to_string()));
                        }
                    }
                    Err(e) => {
                        return Err(ExecuteError::ExecutionFailed(format!("Failed to locate fugue-sandbox: {}", e)));
                    }
                }
            }
        }

        let pty_config = pty_config
            .with_size(pane_size.0, pane_size.1)
            .with_fugue_context(session_id, &session_name, window_id, pane_id);

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
            should_focus: false,
        };
        let delivered = self.registry.try_broadcast_to_session(session_id, msg);
        debug!(
            "Broadcast PaneCreated to {} clients in session {}",
            delivered, session_id
        );

        // Increment spawn count for rate limiting
        let new_count = self.sideband_spawn_count.fetch_add(1, Ordering::SeqCst) + 1;
        info!(
            "Sideband spawn successful (total sideband-spawned: {}/{})",
            new_count, self.spawn_limits.max_panes_per_session
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

    /// Execute mail command - send worker summary to dashboard
    async fn execute_mail(
        &self,
        source_pane: Uuid,
        summary: String,
        priority: MailPriority,
    ) -> ExecuteResult<()> {
        info!(
            "Mail from pane {}: [{:?}] {}",
            source_pane, priority, summary
        );

        // Find session ID for broadcast
        let session_id = {
            let manager = self.session_manager.read().await;
            manager.find_pane(source_pane).map(|(s, _, _)| s.id())
        };

        if let Some(session_id) = session_id {
            let msg = ServerMessage::MailReceived {
                pane_id: source_pane,
                priority,
                summary,
            };
            self.registry.try_broadcast_to_session(session_id, msg);
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

    /// Execute advertise capabilities command
    async fn execute_advertise_capabilities(
        &self,
        source_pane: Uuid,
        json: String,
    ) -> ExecuteResult<()> {
        info!("AdvertiseCapabilities requested for pane: {}", source_pane);

        // Parse JSON
        let capabilities: serde_json::Value = serde_json::from_str(&json)
            .map_err(|e| ExecuteError::ExecutionFailed(format!("Invalid capabilities JSON: {}", e)))?;

        let mut manager = self.session_manager.write().await;
        if let Some(pane) = manager.find_pane_mut(source_pane) {
            // Update metadata
            if let Some(obj) = capabilities.as_object() {
                for (key, value) in obj {
                    let val_str = match value {
                        serde_json::Value::String(s) => s.clone(),
                        serde_json::Value::Bool(b) => b.to_string(),
                        serde_json::Value::Number(n) => n.to_string(),
                        _ => value.to_string(),
                    };
                    pane.set_metadata(format!("capability.{}", key), val_str);
                }
            }
            Ok(())
        } else {
            Err(ExecuteError::PaneNotFound(source_pane.to_string()))
        }
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
    async fn test_execute_mail() {
        let (executor, _) = create_test_executor();

        let result = executor.execute(
            SidebandCommand::Mail {
                summary: "Task complete".to_string(),
                priority: MailPriority::Info,
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

    #[test]
    fn test_spawn_config_deserialization() {
        let json = r#"{"env": {"FOO": "bar"}, "timeout_secs": 60, "sandbox": true}"#;
        let config: SpawnConfig = serde_json::from_str(json).unwrap();
        
        assert_eq!(config.env.get("FOO").map(|s| s.as_str()), Some("bar"));
        assert_eq!(config.timeout_secs, Some(60));
        assert_eq!(config.sandbox, true);
    }

    #[test]
    fn test_spawn_config_default() {
        let json = r#"{}"#;
        let config: SpawnConfig = serde_json::from_str(json).unwrap();
        
        assert!(config.env.is_empty());
        assert_eq!(config.timeout_secs, None);
        assert_eq!(config.sandbox, false);
    }
}
