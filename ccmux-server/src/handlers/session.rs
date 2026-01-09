//! Session-related message handlers
//!
//! Handles: ListSessions, CreateSession, AttachSession, CreateWindow

use tracing::{debug, info, warn};
use uuid::Uuid;

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
    pub async fn handle_create_session(&self, name: String) -> HandlerResult {
        info!(
            "CreateSession '{}' request from {}",
            name, self.client_id
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

                    match pty_manager.spawn(pane_id, pty_config) {
                        Ok(handle) => {
                            info!("PTY spawned for default pane {}", pane_id);

                            // Start output poller to broadcast PTY output to clients
                            let reader = handle.clone_reader();
                            let _poller_handle = PtyOutputPoller::spawn_with_cleanup(
                                pane_id,
                                session_id,
                                reader,
                                self.registry.clone(),
                                Some(self.pane_closed_tx.clone()),
                            );
                            info!("Output poller started for pane {}", pane_id);
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

            // Collect window and pane info
            let session = session_manager.get_session(session_id).unwrap();
            let windows: Vec<_> = session.windows().map(|w| w.to_info()).collect();

            let panes: Vec<_> = session
                .windows()
                .flat_map(|w| w.panes().map(|p| p.to_info()))
                .collect();

            drop(session_manager);

            // Register in client registry
            self.registry.attach_to_session(self.client_id, session_id);

            info!(
                "Client {} attached to session {} ({} windows, {} panes)",
                self.client_id,
                session_id,
                windows.len(),
                panes.len()
            );

            HandlerResult::Response(ServerMessage::Attached {
                session: session_info,
                windows,
                panes,
            })
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

        // Collect pane IDs for PTY cleanup before removing session
        let pane_ids: Vec<Uuid> = {
            let session_manager = self.session_manager.read().await;
            if let Some(session) = session_manager.get_session(session_id) {
                session
                    .windows()
                    .flat_map(|w| w.panes().map(|p| p.id()))
                    .collect()
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

        // No direct response needed - clients get the broadcast
        HandlerResult::NoResponse
    }

    /// Handle CreateWindow message - create a new window in a session
    pub async fn handle_create_window(
        &self,
        session_id: Uuid,
        name: Option<String>,
    ) -> HandlerResult {
        info!(
            "CreateWindow in session {} request from {}",
            session_id, self.client_id
        );

        let mut session_manager = self.session_manager.write().await;

        if let Some(session) = session_manager.get_session_mut(session_id) {
            let window = session.create_window(name);
            let window_info = window.to_info();

            info!(
                "Window '{}' created in session {} with ID {}",
                window_info.name, session_id, window_info.id
            );

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
        } else {
            debug!("Session {} not found for CreateWindow", session_id);
            HandlerContext::error(
                ErrorCode::SessionNotFound,
                format!("Session {} not found", session_id),
            )
        }
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

        let (tx, _rx) = mpsc::channel(10);
        let client_id = registry.register_client(tx);

        let (pane_closed_tx, _) = mpsc::channel(10);
        HandlerContext::new(session_manager, pty_manager, registry, client_id, pane_closed_tx)
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
        let result = ctx.handle_create_session("new-session".to_string()).await;

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
        ctx.handle_create_session("test-session".to_string()).await;

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
        ctx.handle_create_session("test".to_string()).await;

        // Try to create duplicate
        let result = ctx.handle_create_session("test".to_string()).await;

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
            HandlerResult::Response(ServerMessage::Attached { session, .. }) => {
                assert_eq!(session.name, "test");
                assert_eq!(session.attached_clients, 1);
            }
            _ => panic!("Expected Attached response"),
        }

        // Verify registry was updated
        assert_eq!(
            ctx.registry.get_client_session(ctx.client_id),
            Some(session_id)
        );
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
}
