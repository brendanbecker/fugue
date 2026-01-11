//! Session-related message handlers
//!
//! Handles: ListSessions, CreateSession, AttachSession, CreateWindow, RenameSession

use tracing::{debug, info, warn};
use uuid::Uuid;
use ccmux_utils::CcmuxError;

use crate::pty::{PtyConfig, PtyOutputPoller};

use ccmux_protocol::{ErrorCode, ServerMessage};

use super::{HandlerContext, HandlerResult};

impl HandlerContext {
    /// Handle ListSessions message - return all available sessions
    pub async fn handle_list_sessions(&self) -> HandlerResult {
        debug!("ListSessions request from {}", self.client_id);

        let session_manager = self.session_manager.read().await;
        let sessions: Vec<_> = session_manager
            .list_sessions()
            .iter()
            .map(|s| s.to_info())
            .collect();

        HandlerResult::Response(ServerMessage::SessionList { sessions })
    }

    /// Handle CreateSession message - create a new session
    ///
    /// Creates a session with a default window and pane, including PTY spawn.
    /// This ensures new sessions are immediately usable with a shell.
    ///
    /// If `command` is provided, it overrides `default_command` from config.
    pub async fn handle_create_session(
        &self,
        name: String,
        command: Option<String>,
    ) -> HandlerResult {
        info!(
            "CreateSession '{}' request from {} (command: {:?})",
            name, self.client_id, command
        );

        let mut session_manager = self.session_manager.write().await;

        match session_manager.create_session(&name) {
            Ok(session) => {
                let session_id = session.id();
                info!("Session '{}' created with ID {}", name, session_id);

                // Get mutable session reference to create window and pane
                let session = session_manager.get_session_mut(session_id).unwrap();

                // Create default window (named "0" for first window)
                let window = session.create_window(None);
                let window_id = window.id();
                info!("Default window created with ID {}", window_id);

                // Create default pane in the window
                let window = session.get_window_mut(window_id).unwrap();
                let pane = window.create_pane();
                let pane_id = pane.id();
                let (cols, rows) = pane.dimensions();
                info!("Default pane created with ID {}", pane_id);

                // Initialize the vt100 parser for terminal emulation
                let pane = window.get_pane_mut(pane_id).unwrap();
                pane.init_parser();

                // Get session info before releasing lock
                let session_info = session_manager.get_session(session_id).unwrap().to_info();
                drop(session_manager);

                // Spawn PTY for the default pane
                {
                    let mut pty_manager = self.pty_manager.write().await;

                    // Priority: CLI command > config default_command > shell
                    let mut pty_config = if let Some(ref cmd) = command {
                        // CLI argument provided - parse command string with args
                        PtyConfig::from_command_string(cmd).with_size(cols, rows)
                    } else if let Some(ref cmd) = self.config.general.default_command {
                        // Use default_command from config
                        PtyConfig::from_command_string(cmd).with_size(cols, rows)
                    } else {
                        // Fall back to shell
                        PtyConfig::shell().with_size(cols, rows)
                    };

                    // Inherit server's working directory so new panes start in the project
                    if let Ok(cwd) = std::env::current_dir() {
                        pty_config = pty_config.with_cwd(cwd);
                    }
                    pty_config = pty_config.with_ccmux_context(session_id, &name, window_id, pane_id);

                    match pty_manager.spawn(pane_id, pty_config) {
                        Ok(handle) => {
                            info!("PTY spawned for default pane {}", pane_id);

                            // Start output poller with sideband parsing enabled
                            let reader = handle.clone_reader();
                            let _poller_handle = PtyOutputPoller::spawn_with_sideband(
                                pane_id,
                                session_id,
                                reader,
                                self.registry.clone(),
                                Some(self.pane_closed_tx.clone()),
                                self.command_executor.clone(),
                            );
                            info!("Output poller started for pane {} (sideband enabled)", pane_id);
                        }
                        Err(e) => {
                            // Log error but don't fail session creation
                            // User can manually create a pane if PTY spawn fails
                            warn!("Failed to spawn PTY for default pane: {}", e);
                        }
                    }
                }

                HandlerResult::Response(ServerMessage::SessionCreated {
                    session: session_info,
                })
            }
            Err(e) => {
                debug!("Failed to create session '{}': {}", name, e);
                HandlerContext::error(
                    ErrorCode::InvalidOperation,
                    format!("Failed to create session: {}", e),
                )
            }
        }
    }

    /// Handle AttachSession message - attach to an existing session
    ///
    /// After sending the Attached message with session/window/pane metadata,
    /// this also sends the current scrollback content for each pane so the
    /// client can display the existing terminal state.
    pub async fn handle_attach_session(&self, session_id: Uuid) -> HandlerResult {
        info!(
            "AttachSession {} request from {}",
            session_id, self.client_id
        );

        // First detach from any current session
        if let Some(current_session) = self.registry.get_client_session(self.client_id) {
            if current_session != session_id {
                // Decrement client count in old session
                let mut session_manager = self.session_manager.write().await;
                if let Some(session) = session_manager.get_session_mut(current_session) {
                    session.detach_client();
                }
                drop(session_manager);

                self.registry.detach_from_session(self.client_id);
            }
        }

        // Check if session exists and attach
        let mut session_manager = self.session_manager.write().await;

        if let Some(session) = session_manager.get_session_mut(session_id) {
            // Increment attached client count
            session.attach_client();

            let session_info = session.to_info();

            // Collect window and pane info, along with scrollback content for each pane
            let session = session_manager.get_session(session_id).unwrap();
            let windows: Vec<_> = session.windows().map(|w| w.to_info()).collect();

            // Collect pane info and scrollback content
            let mut panes = Vec::new();
            let mut initial_output: Vec<ServerMessage> = Vec::new();

            for window in session.windows() {
                for pane in window.panes() {
                    panes.push(pane.to_info());

                    // Get the current scrollback content for this pane
                    // This allows the client to see existing terminal content on attach
                    let scrollback = pane.scrollback();
                    let lines: Vec<&str> = scrollback.get_lines().collect();
                    if !lines.is_empty() {
                        // Join lines with newlines and send as output
                        let content = lines.join("\n");
                        let content_len = content.len();
                        let line_count = lines.len();
                        if !content.is_empty() {
                            initial_output.push(ServerMessage::Output {
                                pane_id: pane.id(),
                                data: content.into_bytes(),
                            });
                            debug!(
                                "Prepared initial scrollback for pane {} ({} lines, {} bytes)",
                                pane.id(),
                                line_count,
                                content_len
                            );
                        }
                    }
                }
            }

            // Set this as the active session for MCP commands that don't specify a session
            session_manager.set_active_session(session_id);

            drop(session_manager);

            // Register in client registry
            self.registry.attach_to_session(self.client_id, session_id);

            info!(
                "Client {} attached to session {} ({} windows, {} panes, {} panes with scrollback)",
                self.client_id,
                session_id,
                windows.len(),
                panes.len(),
                initial_output.len()
            );

            // Return response with follow-up output messages for initial scrollback
            HandlerResult::ResponseWithFollowUp {
                response: ServerMessage::Attached {
                    session: session_info,
                    windows,
                    panes,
                },
                follow_up: initial_output,
            }
        } else {
            debug!("Session {} not found", session_id);
            HandlerContext::error(
                ErrorCode::SessionNotFound,
                format!("Session {} not found", session_id),
            )
        }
    }

    /// Handle DestroySession message - destroy/kill a session
    ///
    /// Removes the session and all its windows/panes. Kills all associated PTY processes.
    /// Broadcasts updated session list to all clients.
    pub async fn handle_destroy_session(&self, session_id: Uuid) -> HandlerResult {
        info!(
            "DestroySession {} request from {}",
            session_id, self.client_id
        );

        // Collect pane IDs and session name for PTY cleanup before removing session
        let (pane_ids, session_name): (Vec<Uuid>, String) = {
            let session_manager = self.session_manager.read().await;
            if let Some(session) = session_manager.get_session(session_id) {
                let panes = session
                    .windows()
                    .flat_map(|w| w.panes().map(|p| p.id()))
                    .collect();
                (panes, session.name().to_string())
            } else {
                debug!("Session {} not found for DestroySession", session_id);
                return HandlerContext::error(
                    ErrorCode::SessionNotFound,
                    format!("Session {} not found", session_id),
                );
            }
        };

        // Kill all PTY processes for panes in this session
        {
            let mut pty_manager = self.pty_manager.write().await;
            for pane_id in &pane_ids {
                if let Some(handle) = pty_manager.remove(*pane_id) {
                    if let Err(e) = handle.kill() {
                        warn!("Failed to kill PTY for pane {}: {}", pane_id, e);
                    } else {
                        debug!("Killed PTY for pane {}", pane_id);
                    }
                }
            }
        }

        // Remove the session
        {
            let mut session_manager = self.session_manager.write().await;
            if let Some(session) = session_manager.remove_session(session_id) {
                info!(
                    "Destroyed session '{}' ({}) with {} panes",
                    session.name(),
                    session_id,
                    pane_ids.len()
                );
            }
        }

        // Detach any clients attached to this session
        self.registry.detach_session_clients(session_id);

        // Broadcast updated session list to all clients
        let sessions: Vec<_> = {
            let session_manager = self.session_manager.read().await;
            session_manager
                .list_sessions()
                .iter()
                .map(|s| s.to_info())
                .collect()
        };

        self.registry
            .broadcast_to_all(ServerMessage::SessionList { sessions });

        // Return confirmation to the requesting client (for MCP bridge)
        HandlerResult::Response(ServerMessage::SessionDestroyed {
            session_id,
            session_name,
        })
    }

    /// Handle CreateWindow message - create a new window in a session
    ///
    /// Creates a window with a default pane and spawns a PTY for it.
    pub async fn handle_create_window(
        &self,
        session_id: Uuid,
        name: Option<String>,
    ) -> HandlerResult {
        info!(
            "CreateWindow in session {} request from {}",
            session_id, self.client_id
        );

        // Default terminal size for new panes
        let (cols, rows) = (80, 24);

        let (window_info, pane_id, session_name) = {
            let mut session_manager = self.session_manager.write().await;

            if let Some(session) = session_manager.get_session_mut(session_id) {
                let session_name = session.name().to_string();
                // Create window first
                let window = session.create_window(name);
                let window_info = window.to_info();
                let window_id = window_info.id;

                // Get mutable reference to create pane
                let window = session.get_window_mut(window_id).unwrap();
                let pane = window.create_pane();
                let pane_id = pane.id();

                info!(
                    "Window '{}' created in session {} with ID {} (default pane {})",
                    window_info.name, session_id, window_id, pane_id
                );

                (window_info, pane_id, session_name)
            } else {
                debug!("Session {} not found for CreateWindow", session_id);
                return HandlerContext::error(
                    ErrorCode::SessionNotFound,
                    format!("Session {} not found", session_id),
                );
            }
        };

        let window_id = window_info.id;

        // Spawn PTY for the default pane
        {
            let mut pty_manager = self.pty_manager.write().await;

            // Use default_command from config if set, otherwise shell
            let mut pty_config = if let Some(ref cmd) = self.config.general.default_command {
                PtyConfig::command(cmd).with_size(cols, rows)
            } else {
                PtyConfig::shell().with_size(cols, rows)
            };

            // Inherit server's working directory so new panes start in the project
            if let Ok(cwd) = std::env::current_dir() {
                pty_config = pty_config.with_cwd(cwd);
            }
            pty_config = pty_config.with_ccmux_context(session_id, &session_name, window_id, pane_id);

            match pty_manager.spawn(pane_id, pty_config) {
                Ok(handle) => {
                    info!("PTY spawned for default pane {}", pane_id);

                    // Start output poller with sideband parsing enabled
                    let reader = handle.clone_reader();
                    let _poller_handle = PtyOutputPoller::spawn_with_sideband(
                        pane_id,
                        session_id,
                        reader,
                        self.registry.clone(),
                        Some(self.pane_closed_tx.clone()),
                        self.command_executor.clone(),
                    );
                    info!("Output poller started for pane {} (sideband enabled)", pane_id);
                }
                Err(e) => {
                    warn!("Failed to spawn PTY for default pane: {}", e);
                }
            }
        }

        // Broadcast to all clients attached to this session
        HandlerResult::ResponseWithBroadcast {
            response: ServerMessage::WindowCreated {
                window: window_info.clone(),
            },
            session_id,
            broadcast: ServerMessage::WindowCreated {
                window: window_info,
            },
        }
    }

    /// Handle RenameSession message - rename a session
    ///
    /// Resolves session by UUID or name, then renames it.
    /// Returns error if session not found or name is already in use.
    pub async fn handle_rename_session(
        &self,
        session_filter: String,
        new_name: String,
    ) -> HandlerResult {
        info!(
            "RenameSession '{}' -> '{}' request from {}",
            session_filter, new_name, self.client_id
        );

        let mut session_manager = self.session_manager.write().await;

        // Resolve session: by UUID first, then by name
        let session_id = if let Ok(id) = Uuid::parse_str(&session_filter) {
            if session_manager.get_session(id).is_some() {
                id
            } else {
                debug!("Session UUID {} not found", session_filter);
                return HandlerContext::error(
                    ErrorCode::SessionNotFound,
                    format!("Session '{}' not found", session_filter),
                );
            }
        } else {
            match session_manager.get_session_by_name(&session_filter) {
                Some(session) => session.id(),
                None => {
                    debug!("Session name '{}' not found", session_filter);
                    return HandlerContext::error(
                        ErrorCode::SessionNotFound,
                        format!("Session '{}' not found", session_filter),
                    );
                }
            }
        };

        // Perform the rename
        match session_manager.rename_session(session_id, &new_name) {
            Ok(previous_name) => {
                info!(
                    "Session {} renamed from '{}' to '{}'",
                    session_id, previous_name, new_name
                );

                HandlerResult::Response(ServerMessage::SessionRenamed {
                    session_id,
                    previous_name,
                    new_name,
                })
            }
            Err(CcmuxError::SessionExists(name)) => {
                debug!("Session name '{}' is already in use", name);
                HandlerContext::error(
                    ErrorCode::SessionNameExists,
                    format!("Session name '{}' is already in use", name),
                )
            }
            Err(e) => {
                debug!("Failed to rename session: {}", e);
                HandlerContext::error(ErrorCode::InternalError, format!("Failed to rename: {}", e))
            }
        }
    }

    /// Handle RenamPane message - rename a pane (FEAT-036)
    pub async fn handle_rename_pane(&self, pane_id: Uuid, new_name: String) -> HandlerResult {
        info!(
            "RenamPane {} -> '{}' request from {}",
            pane_id, new_name, self.client_id
        );

        let mut session_manager = self.session_manager.write().await;

        // Use find_pane_mut which searches across all sessions
        if let Some(pane) = session_manager.find_pane_mut(pane_id) {
            let previous_name = pane.name().map(String::from);
            pane.set_name(Some(new_name.clone()));

            info!(
                "Pane {} renamed from {:?} to '{}'",
                pane_id, previous_name, new_name
            );

            return HandlerResult::Response(ServerMessage::PaneRenamed {
                pane_id,
                previous_name,
                new_name,
            });
        }

        HandlerContext::error(ErrorCode::PaneNotFound, format!("Pane '{}' not found", pane_id))
    }

    /// Handle RenameWindow message - rename a window (FEAT-036)
    pub async fn handle_rename_window(&self, window_id: Uuid, new_name: String) -> HandlerResult {
        info!(
            "RenameWindow {} -> '{}' request from {}",
            window_id, new_name, self.client_id
        );

        let mut session_manager = self.session_manager.write().await;

        // First find the window to get session_id (immutable), then get mutable access
        let session_id = session_manager.find_window(window_id).map(|(s, _)| s.id());

        if let Some(session_id) = session_id {
            if let Some(session) = session_manager.get_session_mut(session_id) {
                if let Some(window) = session.get_window_mut(window_id) {
                    let previous_name = window.name().to_string();
                    window.set_name(new_name.clone());

                    info!(
                        "Window {} renamed from '{}' to '{}'",
                        window_id, previous_name, new_name
                    );

                    return HandlerResult::Response(ServerMessage::WindowRenamed {
                        window_id,
                        previous_name,
                        new_name,
                    });
                }
            }
        }

        HandlerContext::error(
            ErrorCode::WindowNotFound,
            format!("Window '{}' not found", window_id),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pty::PtyManager;
    use crate::registry::ClientRegistry;
    use crate::session::SessionManager;
    use std::sync::Arc;
    use tokio::sync::{mpsc, RwLock};

    fn create_test_context() -> HandlerContext {
        let session_manager = Arc::new(RwLock::new(SessionManager::new()));
        let pty_manager = Arc::new(RwLock::new(PtyManager::new()));
        let registry = Arc::new(ClientRegistry::new());
        let config = Arc::new(crate::config::AppConfig::default());
        let command_executor = Arc::new(crate::sideband::AsyncCommandExecutor::new(
            Arc::clone(&session_manager),
            Arc::clone(&pty_manager),
            Arc::clone(&registry),
        ));

        let (tx, _rx) = mpsc::channel(10);
        let client_id = registry.register_client(tx);

        let (pane_closed_tx, _) = mpsc::channel(10);
        HandlerContext::new(session_manager, pty_manager, registry, config, client_id, pane_closed_tx, command_executor)
    }

    #[tokio::test]
    async fn test_handle_list_sessions_empty() {
        let ctx = create_test_context();
        let result = ctx.handle_list_sessions().await;

        match result {
            HandlerResult::Response(ServerMessage::SessionList { sessions }) => {
                assert!(sessions.is_empty());
            }
            _ => panic!("Expected SessionList response"),
        }
    }

    #[tokio::test]
    async fn test_handle_list_sessions_with_sessions() {
        let ctx = create_test_context();

        // Create some sessions
        {
            let mut session_manager = ctx.session_manager.write().await;
            session_manager.create_session("session1").unwrap();
            session_manager.create_session("session2").unwrap();
        }

        let result = ctx.handle_list_sessions().await;

        match result {
            HandlerResult::Response(ServerMessage::SessionList { sessions }) => {
                assert_eq!(sessions.len(), 2);
            }
            _ => panic!("Expected SessionList response"),
        }
    }

    #[tokio::test]
    async fn test_handle_create_session_success() {
        let ctx = create_test_context();
        let result = ctx.handle_create_session("new-session".to_string(), None).await;

        match result {
            HandlerResult::Response(ServerMessage::SessionCreated { session }) => {
                assert_eq!(session.name, "new-session");
                // Session should now have 1 window with 1 pane
                assert_eq!(session.window_count, 1);
            }
            _ => panic!("Expected SessionCreated response"),
        }

        // Verify window and pane were created
        let session_manager = ctx.session_manager.read().await;
        let session = session_manager.get_session_by_name("new-session").unwrap();
        assert_eq!(session.window_count(), 1);

        let window = session.windows().next().unwrap();
        assert_eq!(window.pane_count(), 1);

        let pane = window.panes().next().unwrap();
        assert!(pane.has_parser()); // Parser should be initialized
    }

    #[tokio::test]
    async fn test_handle_create_session_spawns_pty() {
        let ctx = create_test_context();
        ctx.handle_create_session("test-session".to_string(), None).await;

        // Find the pane ID
        let pane_id = {
            let session_manager = ctx.session_manager.read().await;
            let session = session_manager.get_session_by_name("test-session").unwrap();
            let window = session.windows().next().unwrap();
            let pane = window.panes().next().unwrap();
            pane.id()
        };

        // Verify PTY was spawned
        let pty_manager = ctx.pty_manager.read().await;
        assert!(
            pty_manager.contains(pane_id),
            "PTY should be spawned for default pane"
        );
    }

    #[tokio::test]
    async fn test_handle_create_session_duplicate() {
        let ctx = create_test_context();

        // Create first session
        ctx.handle_create_session("test".to_string(), None).await;

        // Try to create duplicate
        let result = ctx.handle_create_session("test".to_string(), None).await;

        match result {
            HandlerResult::Response(ServerMessage::Error { code, .. }) => {
                assert_eq!(code, ErrorCode::InvalidOperation);
            }
            _ => panic!("Expected Error response"),
        }
    }

    #[tokio::test]
    async fn test_handle_attach_session_success() {
        let ctx = create_test_context();

        // Create a session
        let session_id = {
            let mut session_manager = ctx.session_manager.write().await;
            let session = session_manager.create_session("test").unwrap();
            session.id()
        };

        let result = ctx.handle_attach_session(session_id).await;

        match result {
            HandlerResult::ResponseWithFollowUp {
                response: ServerMessage::Attached { session, .. },
                follow_up,
            } => {
                assert_eq!(session.name, "test");
                assert_eq!(session.attached_clients, 1);
                // Fresh session has no scrollback yet
                assert!(follow_up.is_empty());
            }
            _ => panic!("Expected Attached response with follow_up"),
        }

        // Verify registry was updated
        assert_eq!(
            ctx.registry.get_client_session(ctx.client_id),
            Some(session_id)
        );
    }

    #[tokio::test]
    async fn test_handle_attach_session_sends_scrollback() {
        let ctx = create_test_context();

        // Create a session with a pane that has scrollback
        let session_id = {
            let mut session_manager = ctx.session_manager.write().await;
            let session = session_manager.create_session("test").unwrap();
            let session_id = session.id();

            // Create a window and pane
            let session = session_manager.get_session_mut(session_id).unwrap();
            let window = session.create_window(None);
            let window_id = window.id();

            let session = session_manager.get_session_mut(session_id).unwrap();
            let window = session.get_window_mut(window_id).unwrap();
            let pane = window.create_pane();
            let pane_id = pane.id();

            // Add some scrollback content via session manager
            let pane = session_manager.find_pane_mut(pane_id).unwrap();
            pane.push_output(b"Hello, World!\nThis is a test.\n");

            session_id
        };

        let result = ctx.handle_attach_session(session_id).await;

        match result {
            HandlerResult::ResponseWithFollowUp {
                response: ServerMessage::Attached { session, panes, .. },
                follow_up,
            } => {
                assert_eq!(session.name, "test");
                assert_eq!(panes.len(), 1);

                // Should have follow-up Output messages with scrollback
                assert_eq!(follow_up.len(), 1);
                match &follow_up[0] {
                    ServerMessage::Output { pane_id, data } => {
                        assert_eq!(*pane_id, panes[0].id);
                        let content = String::from_utf8_lossy(data);
                        assert!(content.contains("Hello, World!"));
                        assert!(content.contains("This is a test."));
                    }
                    _ => panic!("Expected Output message in follow_up"),
                }
            }
            _ => panic!("Expected Attached response with follow_up"),
        }
    }

    #[tokio::test]
    async fn test_handle_attach_session_not_found() {
        let ctx = create_test_context();
        let result = ctx.handle_attach_session(Uuid::new_v4()).await;

        match result {
            HandlerResult::Response(ServerMessage::Error { code, .. }) => {
                assert_eq!(code, ErrorCode::SessionNotFound);
            }
            _ => panic!("Expected Error response"),
        }
    }

    #[tokio::test]
    async fn test_handle_attach_session_switches_sessions() {
        let ctx = create_test_context();

        // Create two sessions
        let (session1_id, session2_id) = {
            let mut session_manager = ctx.session_manager.write().await;
            let s1_id = session_manager.create_session("session1").unwrap().id();
            let s2_id = session_manager.create_session("session2").unwrap().id();
            (s1_id, s2_id)
        };

        // Attach to first session
        ctx.handle_attach_session(session1_id).await;
        assert_eq!(
            ctx.registry.get_client_session(ctx.client_id),
            Some(session1_id)
        );

        // Attach to second session
        ctx.handle_attach_session(session2_id).await;
        assert_eq!(
            ctx.registry.get_client_session(ctx.client_id),
            Some(session2_id)
        );

        // First session should have client count decremented
        let session_manager = ctx.session_manager.read().await;
        let session1 = session_manager.get_session(session1_id).unwrap();
        assert_eq!(session1.attached_clients(), 0);

        let session2 = session_manager.get_session(session2_id).unwrap();
        assert_eq!(session2.attached_clients(), 1);
    }

    #[tokio::test]
    async fn test_handle_create_window_success() {
        let ctx = create_test_context();

        // Create a session
        let session_id = {
            let mut session_manager = ctx.session_manager.write().await;
            let session = session_manager.create_session("test").unwrap();
            session.id()
        };

        let result = ctx
            .handle_create_window(session_id, Some("main".to_string()))
            .await;

        match result {
            HandlerResult::ResponseWithBroadcast {
                response: ServerMessage::WindowCreated { window },
                session_id: broadcast_session,
                ..
            } => {
                assert_eq!(window.name, "main");
                assert_eq!(window.session_id, session_id);
                assert_eq!(broadcast_session, session_id);
            }
            _ => panic!("Expected WindowCreated response with broadcast"),
        }
    }

    #[tokio::test]
    async fn test_handle_create_window_session_not_found() {
        let ctx = create_test_context();
        let result = ctx
            .handle_create_window(Uuid::new_v4(), Some("main".to_string()))
            .await;

        match result {
            HandlerResult::Response(ServerMessage::Error { code, .. }) => {
                assert_eq!(code, ErrorCode::SessionNotFound);
            }
            _ => panic!("Expected Error response"),
        }
    }

    #[tokio::test]
    async fn test_handle_create_window_auto_name() {
        let ctx = create_test_context();

        // Create a session
        let session_id = {
            let mut session_manager = ctx.session_manager.write().await;
            let session = session_manager.create_session("test").unwrap();
            session.id()
        };

        let result = ctx.handle_create_window(session_id, None).await;

        match result {
            HandlerResult::ResponseWithBroadcast {
                response: ServerMessage::WindowCreated { window },
                ..
            } => {
                // Auto-generated name should be the index
                assert_eq!(window.name, "0");
            }
            _ => panic!("Expected WindowCreated response"),
        }
    }

    // ==================== Rename Session Tests ====================

    #[tokio::test]
    async fn test_handle_rename_session_by_name() {
        let ctx = create_test_context();

        // Create a session
        {
            let mut session_manager = ctx.session_manager.write().await;
            session_manager.create_session("original").unwrap();
        }

        let result = ctx
            .handle_rename_session("original".to_string(), "renamed".to_string())
            .await;

        match result {
            HandlerResult::Response(ServerMessage::SessionRenamed {
                previous_name,
                new_name,
                ..
            }) => {
                assert_eq!(previous_name, "original");
                assert_eq!(new_name, "renamed");
            }
            _ => panic!("Expected SessionRenamed response"),
        }

        // Verify the session was actually renamed
        let session_manager = ctx.session_manager.read().await;
        assert!(session_manager.get_session_by_name("original").is_none());
        assert!(session_manager.get_session_by_name("renamed").is_some());
    }

    #[tokio::test]
    async fn test_handle_rename_session_by_uuid() {
        let ctx = create_test_context();

        // Create a session and get its ID
        let session_id = {
            let mut session_manager = ctx.session_manager.write().await;
            session_manager.create_session("test").unwrap().id()
        };

        let result = ctx
            .handle_rename_session(session_id.to_string(), "new-name".to_string())
            .await;

        match result {
            HandlerResult::Response(ServerMessage::SessionRenamed {
                session_id: resp_id,
                previous_name,
                new_name,
            }) => {
                assert_eq!(resp_id, session_id);
                assert_eq!(previous_name, "test");
                assert_eq!(new_name, "new-name");
            }
            _ => panic!("Expected SessionRenamed response"),
        }
    }

    #[tokio::test]
    async fn test_handle_rename_session_not_found() {
        let ctx = create_test_context();

        let result = ctx
            .handle_rename_session("nonexistent".to_string(), "new-name".to_string())
            .await;

        match result {
            HandlerResult::Response(ServerMessage::Error { code, .. }) => {
                assert_eq!(code, ErrorCode::SessionNotFound);
            }
            _ => panic!("Expected Error response"),
        }
    }

    #[tokio::test]
    async fn test_handle_rename_session_uuid_not_found() {
        let ctx = create_test_context();

        let fake_uuid = Uuid::new_v4();
        let result = ctx
            .handle_rename_session(fake_uuid.to_string(), "new-name".to_string())
            .await;

        match result {
            HandlerResult::Response(ServerMessage::Error { code, .. }) => {
                assert_eq!(code, ErrorCode::SessionNotFound);
            }
            _ => panic!("Expected Error response"),
        }
    }

    #[tokio::test]
    async fn test_handle_rename_session_duplicate_name() {
        let ctx = create_test_context();

        // Create two sessions
        {
            let mut session_manager = ctx.session_manager.write().await;
            session_manager.create_session("session1").unwrap();
            session_manager.create_session("session2").unwrap();
        }

        // Try to rename session2 to session1's name
        let result = ctx
            .handle_rename_session("session2".to_string(), "session1".to_string())
            .await;

        match result {
            HandlerResult::Response(ServerMessage::Error { code, message }) => {
                assert_eq!(code, ErrorCode::SessionNameExists);
                assert!(message.contains("session1"));
            }
            _ => panic!("Expected Error response"),
        }
    }

    #[tokio::test]
    async fn test_handle_rename_session_same_name() {
        let ctx = create_test_context();

        // Create a session
        {
            let mut session_manager = ctx.session_manager.write().await;
            session_manager.create_session("same").unwrap();
        }

        // Rename to same name should succeed as no-op
        let result = ctx
            .handle_rename_session("same".to_string(), "same".to_string())
            .await;

        match result {
            HandlerResult::Response(ServerMessage::SessionRenamed {
                previous_name,
                new_name,
                ..
            }) => {
                assert_eq!(previous_name, "same");
                assert_eq!(new_name, "same");
            }
            _ => panic!("Expected SessionRenamed response"),
        }
    }
}
