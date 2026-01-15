//! Pane-related message handlers
//!
//! Handles: CreatePane, SelectPane, ClosePane, Resize

use std::path::PathBuf;

use tracing::{debug, info, warn};
use uuid::Uuid;

use ccmux_protocol::{ErrorCode, ServerMessage, SplitDirection};

use crate::beads::{self, metadata_keys};
use crate::pty::{PtyConfig, PtyOutputPoller};

use super::{HandlerContext, HandlerResult};

impl HandlerContext {
    /// Handle CreatePane message - create a new pane in a window
    ///
    /// Note: The direction parameter is currently stored for future split implementation
    /// but all panes are created at the default position for now.
    pub async fn handle_create_pane(
        &self,
        window_id: Uuid,
        direction: SplitDirection,
    ) -> HandlerResult {
        info!(
            "CreatePane in window {} request from {} (direction: {:?})",
            window_id, self.client_id, direction
        );

        // Default terminal size for new panes
        let (cols, rows) = (80, 24);

        let (session_id, pane_id, pane_info, session_env, session_name) = {
            let mut session_manager = self.session_manager.write().await;

            // Find the session that contains this window
            let session_id = session_manager
                .find_window(window_id)
                .map(|(session, _)| session.id());

            match session_id {
                Some(session_id) => {
                    // Get the window and create the pane
                    if let Some(session) = session_manager.get_session_mut(session_id) {
                        // Capture session environment before mutable borrow of window
                        let env = session.environment().clone();
                        let session_name = session.name().to_string();
                        if let Some(window) = session.get_window_mut(window_id) {
                            let pane = window.create_pane();
                            let pane_info = pane.to_info();
                            let pane_id = pane_info.id;

                            info!(
                                "Pane created in window {} with ID {}",
                                window_id, pane_id
                            );

                            (session_id, pane_id, pane_info, env, session_name)
                        } else {
                            debug!("Window {} not found in session", window_id);
                            return HandlerContext::error(
                                ErrorCode::WindowNotFound,
                                format!("Window {} not found", window_id),
                            );
                        }
                    } else {
                        debug!("Session {} not found", session_id);
                        return HandlerContext::error(
                            ErrorCode::SessionNotFound,
                            format!("Session {} not found", session_id),
                        );
                    }
                }
                None => {
                    debug!("Window {} not found in any session", window_id);
                    return HandlerContext::error(
                        ErrorCode::WindowNotFound,
                        format!("Window {} not found", window_id),
                    );
                }
            }
        };

        // Spawn PTY for the new pane
        let beads_detection: Option<PathBuf> = {
            let mut pty_manager = self.pty_manager.write().await;

            // Use default_command from config if set, otherwise shell
            let mut pty_config = if let Some(ref cmd) = self.config.general.default_command {
                PtyConfig::command(cmd).with_size(cols, rows)
            } else {
                PtyConfig::shell().with_size(cols, rows)
            };

            // Inherit server's working directory so new panes start in the project
            // Also detect beads root if auto_detect is enabled (FEAT-057)
            let mut detected_beads: Option<PathBuf> = None;
            if let Ok(cwd) = std::env::current_dir() {
                pty_config = pty_config.with_cwd(&cwd);

                // FEAT-057: Detect beads root and configure environment
                if self.config.beads.auto_detect {
                    if let Some(detection) = beads::detect_beads_root(&cwd) {
                        info!(
                            "Beads detected for pane {}: {:?}",
                            pane_id, detection.beads_dir
                        );
                        pty_config = pty_config.with_beads_config(
                            &detection.beads_dir,
                            &self.config.beads,
                        );
                        detected_beads = Some(detection.beads_dir);
                    }
                }
            }
            pty_config = pty_config.with_ccmux_context(session_id, &session_name, window_id, pane_id);

            // Apply session environment variables
            pty_config = pty_config.with_env_map(&session_env);

            match pty_manager.spawn(pane_id, pty_config) {
                Ok(handle) => {
                    info!("PTY spawned for pane {}", pane_id);

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
                    warn!("Failed to spawn PTY for pane {}: {}", pane_id, e);
                }
            }

            detected_beads
        };

        // FEAT-057: Store beads state in pane and session metadata
        if let Some(beads_dir) = beads_detection {
            let mut session_manager = self.session_manager.write().await;

            // Store beads root in pane metadata
            if let Some(pane) = session_manager.find_pane_mut(pane_id) {
                pane.set_beads_root(Some(beads_dir.clone()));
                debug!("Beads root set on pane {}: {:?}", pane_id, beads_dir);
            }

            // Store beads state in session metadata for persistence
            if let Some(session) = session_manager.get_session_mut(session_id) {
                session.set_metadata(metadata_keys::BEADS_DETECTED, "true");
                session.set_metadata(
                    metadata_keys::BEADS_ROOT,
                    beads_dir.to_string_lossy().to_string(),
                );
                debug!("Beads metadata stored in session {}", session_id);
            }
        }

        // Broadcast to all clients attached to this session
        HandlerResult::ResponseWithBroadcast {
            response: ServerMessage::PaneCreated {
                pane: pane_info.clone(),
                direction,
            },
            session_id,
            broadcast: ServerMessage::PaneCreated { pane: pane_info, direction },
        }
    }

    /// Handle SelectPane message - update focused pane
    ///
    /// BUG-026 FIX: Now broadcasts PaneFocused to TUI clients so they update their UI.
    /// FEAT-056: Checks user priority lock before changing focus.
    pub async fn handle_select_pane(&self, pane_id: Uuid) -> HandlerResult {
        debug!("SelectPane {} request from {}", pane_id, self.client_id);

        // Check if user priority lock is active (FEAT-056)
        if let Some((_client_id, remaining_ms)) = self.user_priority.is_any_lock_active() {
            debug!(
                "SelectPane blocked by user priority lock, retry after {}ms",
                remaining_ms
            );
            return HandlerContext::error(
                ErrorCode::UserPriorityActive,
                format!("User priority lock active, retry after {}ms", remaining_ms),
            );
        }

        let mut session_manager = self.session_manager.write().await;

        // Find the pane
        match session_manager.find_pane(pane_id) {
            Some((session, window, _pane)) => {
                let session_id = session.id();
                let window_id = window.id();

                // Set as active pane in window and active window in session
                if let Some(session) = session_manager.get_session_mut(session_id) {
                    // Also set this window as active in the session
                    session.set_active_window(window_id);

                    if let Some(window) = session.get_window_mut(window_id) {
                        if window.set_active_pane(pane_id) {
                            debug!("Pane {} selected as active", pane_id);
                        // BUG-026: Broadcast pane focus change to TUI clients
                        // BUG-036: Use global broadcast so TUIs in other sessions can switch
                        return HandlerResult::ResponseWithGlobalBroadcast {
                            response: ServerMessage::PaneFocused {
                                session_id,
                                window_id,
                                pane_id,
                            },
                            broadcast: ServerMessage::PaneFocused {
                                session_id,
                                window_id,
                                pane_id,
                            },
                        };
                        }
                    }
                }

                // Shouldn't reach here
                HandlerContext::error(ErrorCode::InternalError, "Failed to select pane")
            }
            None => {
                debug!("Pane {} not found for SelectPane", pane_id);
                HandlerContext::error(
                    ErrorCode::PaneNotFound,
                    format!("Pane {} not found", pane_id),
                )
            }
        }
    }

    /// Handle SelectWindow message - update active window in session
    ///
    /// BUG-026 FIX: Now broadcasts WindowFocused to TUI clients so they update their UI.
    /// FEAT-056: Checks user priority lock before changing focus.
    pub async fn handle_select_window(&self, window_id: Uuid) -> HandlerResult {
        debug!("SelectWindow {} request from {}", window_id, self.client_id);

        // Check if user priority lock is active (FEAT-056)
        if let Some((_client_id, remaining_ms)) = self.user_priority.is_any_lock_active() {
            debug!(
                "SelectWindow blocked by user priority lock, retry after {}ms",
                remaining_ms
            );
            return HandlerContext::error(
                ErrorCode::UserPriorityActive,
                format!("User priority lock active, retry after {}ms", remaining_ms),
            );
        }

        let mut session_manager = self.session_manager.write().await;

        // Find the session containing this window
        let session_id = session_manager
            .list_sessions()
            .iter()
            .find_map(|s| s.windows().find(|w| w.id() == window_id).map(|_| s.id()));

        match session_id {
            Some(session_id) => {
                if let Some(session) = session_manager.get_session_mut(session_id) {
                    session.set_active_window(window_id);
                    debug!("Window {} selected as active", window_id);
                    // BUG-026: Broadcast window focus change to TUI clients
                    // BUG-036: Use global broadcast so TUIs in other sessions can switch
                    return HandlerResult::ResponseWithGlobalBroadcast {
                        response: ServerMessage::WindowFocused {
                            session_id,
                            window_id,
                        },
                        broadcast: ServerMessage::WindowFocused {
                            session_id,
                            window_id,
                        },
                    };
                }
                HandlerContext::error(ErrorCode::InternalError, "Session disappeared")
            }
            None => {
                debug!("Window {} not found for SelectWindow", window_id);
                HandlerContext::error(
                    ErrorCode::WindowNotFound,
                    format!("Window {} not found", window_id),
                )
            }
        }
    }

    /// Handle SelectSession message - update active session
    ///
    /// BUG-026 FIX: Now broadcasts SessionFocused to TUI clients so they update their UI.
    /// FEAT-056: Checks user priority lock before changing focus.
    pub async fn handle_select_session(&self, session_id: Uuid) -> HandlerResult {
        debug!("SelectSession {} request from {}", session_id, self.client_id);

        // Check if user priority lock is active (FEAT-056)
        if let Some((_client_id, remaining_ms)) = self.user_priority.is_any_lock_active() {
            debug!(
                "SelectSession blocked by user priority lock, retry after {}ms",
                remaining_ms
            );
            return HandlerContext::error(
                ErrorCode::UserPriorityActive,
                format!("User priority lock active, retry after {}ms", remaining_ms),
            );
        }

        let mut session_manager = self.session_manager.write().await;

        // Verify session exists
        if session_manager.get_session(session_id).is_none() {
            debug!("Session {} not found for SelectSession", session_id);
            return HandlerContext::error(
                ErrorCode::SessionNotFound,
                format!("Session {} not found", session_id),
            );
        }

        // Set as active session
        session_manager.set_active_session(session_id);
        debug!("Session {} selected as active", session_id);
        // BUG-026: Broadcast session focus change to TUI clients
        // BUG-036: Use global broadcast so TUIs in other sessions can switch
        HandlerResult::ResponseWithGlobalBroadcast {
            response: ServerMessage::SessionFocused { session_id },
            broadcast: ServerMessage::SessionFocused { session_id },
        }
    }

    /// Handle ClosePane message - kill PTY and cleanup
    pub async fn handle_close_pane(&self, pane_id: Uuid) -> HandlerResult {
        info!("ClosePane {} request from {}", pane_id, self.client_id);

        // First, find the pane to get session info
        let (session_id, window_id) = {
            let session_manager = self.session_manager.read().await;

            match session_manager.find_pane(pane_id) {
                Some((session, window, _pane)) => (session.id(), window.id()),
                None => {
                    debug!("Pane {} not found for ClosePane", pane_id);
                    return HandlerContext::error(
                        ErrorCode::PaneNotFound,
                        format!("Pane {} not found", pane_id),
                    );
                }
            }
        };

        // Remove PTY if exists
        {
            let mut pty_manager = self.pty_manager.write().await;
            if let Some(handle) = pty_manager.remove(pane_id) {
                if let Err(e) = handle.kill() {
                    warn!("Failed to kill PTY for pane {}: {}", pane_id, e);
                }
            }
        }

        // Remove pane from session
        let mut session_manager = self.session_manager.write().await;

        if let Some(session) = session_manager.get_session_mut(session_id) {
            if let Some(window) = session.get_window_mut(window_id) {
                if let Some(pane) = window.remove_pane(pane_id) {
                    // Cleanup isolation directory if it was a Claude pane
                    pane.cleanup_isolation();

                    info!("Pane {} closed successfully", pane_id);

                    // Broadcast to all clients attached to this session
                    return HandlerResult::ResponseWithBroadcast {
                        response: ServerMessage::PaneClosed {
                            pane_id,
                            exit_code: None,
                        },
                        session_id,
                        broadcast: ServerMessage::PaneClosed {
                            pane_id,
                            exit_code: None,
                        },
                    };
                }
            }
        }

        // Pane was removed concurrently
        HandlerContext::error(
            ErrorCode::PaneNotFound,
            format!("Pane {} not found", pane_id),
        )
    }

    /// Handle Resize message - resize PTY dimensions
    pub async fn handle_resize(&self, pane_id: Uuid, cols: u16, rows: u16) -> HandlerResult {
        debug!(
            "Resize pane {} to {}x{} request from {}",
            pane_id, cols, rows, self.client_id
        );

        // Resize PTY if exists
        {
            let pty_manager = self.pty_manager.read().await;
            if let Some(handle) = pty_manager.get(pane_id) {
                if let Err(e) = handle.resize(cols, rows) {
                    warn!("Failed to resize PTY for pane {}: {}", pane_id, e);
                    // Continue to update pane dimensions anyway
                }
            }
        }

        // Update pane dimensions
        let mut session_manager = self.session_manager.write().await;

        match session_manager.find_pane_mut(pane_id) {
            Some(pane) => {
                pane.resize(cols, rows);
                debug!("Pane {} resized to {}x{}", pane_id, cols, rows);
                HandlerResult::NoResponse
            }
            None => {
                debug!("Pane {} not found for Resize", pane_id);
                HandlerContext::error(
                    ErrorCode::PaneNotFound,
                    format!("Pane {} not found", pane_id),
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pty::PtyManager;
    use crate::registry::ClientRegistry;
    use crate::session::SessionManager;
    use crate::user_priority::UserPriorityManager;
    use std::sync::Arc;
    use tokio::sync::{mpsc, RwLock};

    fn create_test_context() -> HandlerContext {
        let session_manager = Arc::new(RwLock::new(SessionManager::new()));
        let pty_manager = Arc::new(RwLock::new(PtyManager::new()));
        let registry = Arc::new(ClientRegistry::new());
        let config = Arc::new(crate::config::AppConfig::default());
        let user_priority = Arc::new(UserPriorityManager::new());
        let command_executor = Arc::new(crate::sideband::AsyncCommandExecutor::new(
            Arc::clone(&session_manager),
            Arc::clone(&pty_manager),
            Arc::clone(&registry),
        ));

        let (tx, _rx) = mpsc::channel(10);
        let client_id = registry.register_client(tx);

        let (pane_closed_tx, _) = mpsc::channel(10);
        HandlerContext::new(session_manager, pty_manager, registry, config, client_id, pane_closed_tx, command_executor, user_priority)
    }

    async fn create_session_with_window(ctx: &HandlerContext) -> (Uuid, Uuid) {
        let mut session_manager = ctx.session_manager.write().await;
        let session = session_manager.create_session("test").unwrap();
        let session_id = session.id();

        let session = session_manager.get_session_mut(session_id).unwrap();
        let window = session.create_window(Some("main".to_string()));
        let window_id = window.id();

        (session_id, window_id)
    }

    #[tokio::test]
    async fn test_handle_create_pane_success() {
        let ctx = create_test_context();
        let (_session_id, window_id) = create_session_with_window(&ctx).await;

        let result = ctx
            .handle_create_pane(window_id, SplitDirection::Horizontal)
            .await;

        match result {
            HandlerResult::ResponseWithBroadcast {
                response: ServerMessage::PaneCreated { pane, direction },
                ..
            } => {
                assert_eq!(pane.window_id, window_id);
                assert_eq!(pane.index, 0);
                assert_eq!(direction, SplitDirection::Horizontal);
            }
            _ => panic!("Expected PaneCreated response with broadcast"),
        }
    }

    #[tokio::test]
    async fn test_handle_create_pane_with_vertical_direction() {
        let ctx = create_test_context();
        let (_session_id, window_id) = create_session_with_window(&ctx).await;

        let result = ctx
            .handle_create_pane(window_id, SplitDirection::Vertical)
            .await;

        match result {
            HandlerResult::ResponseWithBroadcast {
                response: ServerMessage::PaneCreated { direction, .. },
                ..
            } => {
                assert_eq!(direction, SplitDirection::Vertical);
            }
            _ => panic!("Expected PaneCreated response"),
        }
    }

    #[tokio::test]
    async fn test_handle_create_pane_window_not_found() {
        let ctx = create_test_context();
        let result = ctx
            .handle_create_pane(Uuid::new_v4(), SplitDirection::Horizontal)
            .await;

        match result {
            HandlerResult::Response(ServerMessage::Error { code, .. }) => {
                assert_eq!(code, ErrorCode::WindowNotFound);
            }
            _ => panic!("Expected Error response"),
        }
    }

    #[tokio::test]
    async fn test_handle_select_pane_success() {
        let ctx = create_test_context();
        let (session_id, window_id) = create_session_with_window(&ctx).await;

        // Create two panes
        let (pane1_id, pane2_id) = {
            let mut session_manager = ctx.session_manager.write().await;
            let session = session_manager.get_session_mut(session_id).unwrap();
            let window = session.get_window_mut(window_id).unwrap();
            let pane1_id = window.create_pane().id();
            let pane2_id = window.create_pane().id();
            (pane1_id, pane2_id)
        };

        // Select second pane
        let result = ctx.handle_select_pane(pane2_id).await;

        // BUG-026: Now returns ResponseWithBroadcast with PaneFocused
        // BUG-036: Use ResponseWithGlobalBroadcast for global focus notification
        match result {
            HandlerResult::ResponseWithGlobalBroadcast {
                response: ServerMessage::PaneFocused { pane_id, .. },
                ..
            } => {
                assert_eq!(pane_id, pane2_id);
            }
            _ => panic!("Expected ResponseWithGlobalBroadcast with PaneFocused"),
        }

        // Verify pane2 is now active
        let session_manager = ctx.session_manager.read().await;
        let (_, window) = session_manager.find_window(window_id).unwrap();
        assert_eq!(window.active_pane_id(), Some(pane2_id));
        assert_ne!(window.active_pane_id(), Some(pane1_id));
    }

    #[tokio::test]
    async fn test_handle_select_pane_not_found() {
        let ctx = create_test_context();
        let result = ctx.handle_select_pane(Uuid::new_v4()).await;

        match result {
            HandlerResult::Response(ServerMessage::Error { code, .. }) => {
                assert_eq!(code, ErrorCode::PaneNotFound);
            }
            _ => panic!("Expected Error response"),
        }
    }

    #[tokio::test]
    async fn test_handle_close_pane_success() {
        let ctx = create_test_context();
        let (session_id, window_id) = create_session_with_window(&ctx).await;

        // Create a pane
        let pane_id = {
            let mut session_manager = ctx.session_manager.write().await;
            let session = session_manager.get_session_mut(session_id).unwrap();
            let window = session.get_window_mut(window_id).unwrap();
            window.create_pane().id()
        };

        let result = ctx.handle_close_pane(pane_id).await;

        match result {
            HandlerResult::ResponseWithBroadcast {
                response:
                    ServerMessage::PaneClosed {
                        pane_id: closed_id,
                        exit_code,
                    },
                session_id: broadcast_session,
                ..
            } => {
                assert_eq!(closed_id, pane_id);
                assert_eq!(broadcast_session, session_id);
                assert_eq!(exit_code, None);
            }
            _ => panic!("Expected PaneClosed response with broadcast"),
        }

        // Verify pane was removed
        let session_manager = ctx.session_manager.read().await;
        let (_, window) = session_manager.find_window(window_id).unwrap();
        assert!(window.get_pane(pane_id).is_none());
    }

    #[tokio::test]
    async fn test_handle_close_pane_not_found() {
        let ctx = create_test_context();
        let result = ctx.handle_close_pane(Uuid::new_v4()).await;

        match result {
            HandlerResult::Response(ServerMessage::Error { code, .. }) => {
                assert_eq!(code, ErrorCode::PaneNotFound);
            }
            _ => panic!("Expected Error response"),
        }
    }

    #[tokio::test]
    async fn test_handle_resize_success() {
        let ctx = create_test_context();
        let (session_id, window_id) = create_session_with_window(&ctx).await;

        // Create a pane
        let pane_id = {
            let mut session_manager = ctx.session_manager.write().await;
            let session = session_manager.get_session_mut(session_id).unwrap();
            let window = session.get_window_mut(window_id).unwrap();
            window.create_pane().id()
        };

        let result = ctx.handle_resize(pane_id, 120, 40).await;

        match result {
            HandlerResult::NoResponse => {}
            _ => panic!("Expected NoResponse"),
        }

        // Verify dimensions were updated
        let session_manager = ctx.session_manager.read().await;
        let (_, _, pane) = session_manager.find_pane(pane_id).unwrap();
        assert_eq!(pane.dimensions(), (120, 40));
    }

    #[tokio::test]
    async fn test_handle_resize_not_found() {
        let ctx = create_test_context();
        let result = ctx.handle_resize(Uuid::new_v4(), 120, 40).await;

        match result {
            HandlerResult::Response(ServerMessage::Error { code, .. }) => {
                assert_eq!(code, ErrorCode::PaneNotFound);
            }
            _ => panic!("Expected Error response"),
        }
    }

    #[tokio::test]
    async fn test_handle_create_multiple_panes() {
        let ctx = create_test_context();
        let (_session_id, window_id) = create_session_with_window(&ctx).await;

        // Create three panes
        ctx.handle_create_pane(window_id, SplitDirection::Horizontal)
            .await;
        ctx.handle_create_pane(window_id, SplitDirection::Vertical)
            .await;
        ctx.handle_create_pane(window_id, SplitDirection::Horizontal)
            .await;

        // Verify all panes exist
        let session_manager = ctx.session_manager.read().await;
        let (_, window) = session_manager.find_window(window_id).unwrap();
        assert_eq!(window.pane_count(), 3);
    }

    // ==================== FEAT-056 User Priority Lockout Tests ====================

    #[tokio::test]
    async fn test_select_pane_blocked_by_user_priority() {
        let ctx = create_test_context();
        let (session_id, window_id) = create_session_with_window(&ctx).await;

        // Create a pane
        let pane_id = {
            let mut session_manager = ctx.session_manager.write().await;
            let session = session_manager.get_session_mut(session_id).unwrap();
            let window = session.get_window_mut(window_id).unwrap();
            window.create_pane().id()
        };

        // Activate user priority lock from a different client
        let other_client_id = crate::registry::ClientId::new(999);
        ctx.user_priority.set_lock(other_client_id, 5000);

        // Try to select pane - should be blocked
        let result = ctx.handle_select_pane(pane_id).await;

        match result {
            HandlerResult::Response(ServerMessage::Error { code, message }) => {
                assert_eq!(code, ErrorCode::UserPriorityActive);
                assert!(message.contains("retry after"));
            }
            _ => panic!("Expected UserPriorityActive error"),
        }
    }

    #[tokio::test]
    async fn test_select_window_blocked_by_user_priority() {
        let ctx = create_test_context();
        let (_session_id, window_id) = create_session_with_window(&ctx).await;

        // Activate user priority lock
        let other_client_id = crate::registry::ClientId::new(999);
        ctx.user_priority.set_lock(other_client_id, 5000);

        // Try to select window - should be blocked
        let result = ctx.handle_select_window(window_id).await;

        match result {
            HandlerResult::Response(ServerMessage::Error { code, message }) => {
                assert_eq!(code, ErrorCode::UserPriorityActive);
                assert!(message.contains("retry after"));
            }
            _ => panic!("Expected UserPriorityActive error"),
        }
    }

    #[tokio::test]
    async fn test_select_session_blocked_by_user_priority() {
        let ctx = create_test_context();
        let (session_id, _window_id) = create_session_with_window(&ctx).await;

        // Activate user priority lock
        let other_client_id = crate::registry::ClientId::new(999);
        ctx.user_priority.set_lock(other_client_id, 5000);

        // Try to select session - should be blocked
        let result = ctx.handle_select_session(session_id).await;

        match result {
            HandlerResult::Response(ServerMessage::Error { code, message }) => {
                assert_eq!(code, ErrorCode::UserPriorityActive);
                assert!(message.contains("retry after"));
            }
            _ => panic!("Expected UserPriorityActive error"),
        }
    }

    #[tokio::test]
    async fn test_select_pane_allowed_when_no_lock() {
        let ctx = create_test_context();
        let (session_id, window_id) = create_session_with_window(&ctx).await;

        // Create a pane
        let pane_id = {
            let mut session_manager = ctx.session_manager.write().await;
            let session = session_manager.get_session_mut(session_id).unwrap();
            let window = session.get_window_mut(window_id).unwrap();
            window.create_pane().id()
        };

        // No lock active - should succeed
        let result = ctx.handle_select_pane(pane_id).await;

        match result {
            HandlerResult::ResponseWithGlobalBroadcast {
                response: ServerMessage::PaneFocused { pane_id: focused_id, .. },
                ..
            } => {
                assert_eq!(focused_id, pane_id);
            }
            _ => panic!("Expected ResponseWithGlobalBroadcast with PaneFocused"),
        }
    }

    #[tokio::test]
    async fn test_select_pane_allowed_after_lock_released() {
        let ctx = create_test_context();
        let (session_id, window_id) = create_session_with_window(&ctx).await;

        // Create a pane
        let pane_id = {
            let mut session_manager = ctx.session_manager.write().await;
            let session = session_manager.get_session_mut(session_id).unwrap();
            let window = session.get_window_mut(window_id).unwrap();
            window.create_pane().id()
        };

        // Activate then release lock
        let other_client_id = crate::registry::ClientId::new(999);
        ctx.user_priority.set_lock(other_client_id, 5000);
        ctx.user_priority.release_lock(other_client_id);

        // Should succeed after lock released
        let result = ctx.handle_select_pane(pane_id).await;

        match result {
            HandlerResult::ResponseWithGlobalBroadcast {
                response: ServerMessage::PaneFocused { pane_id: focused_id, .. },
                ..
            } => {
                assert_eq!(focused_id, pane_id);
            }
            _ => panic!("Expected ResponseWithGlobalBroadcast with PaneFocused"),
        }
    }
}
