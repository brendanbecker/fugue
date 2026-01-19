//! Pane-related message handlers
//!
//! Handles: CreatePane, SelectPane, ClosePane, Resize

use std::path::PathBuf;

use tracing::{debug, info, warn};
use uuid::Uuid;

use ccmux_protocol::{ErrorCode, ServerMessage, SplitDirection, messages::ErrorDetails};

use crate::arbitration::{Action, Resource};
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

        // FEAT-079: Check arbitration before creating pane (layout change)
        if let Err(blocked) = self.check_arbitration(Resource::Window(window_id), Action::Layout) {
            return blocked;
        }

        // FEAT-079: Record human activity if this is a human actor
        self.record_human_activity(Resource::Window(window_id), Action::Layout);

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

        // Log to persistence
        let mut commit_seq = 0;
        if let Some(persistence_lock) = &self.persistence {
            let persistence = persistence_lock.read().await;
            if let Ok(seq) = persistence.log_pane_created(
                pane_id,
                pane_info.window_id,
                pane_info.index,
                pane_info.cols,
                pane_info.rows,
            ) {
                commit_seq = seq;
                persistence.push_replay(seq, ServerMessage::PaneCreated {
                    pane: pane_info.clone(),
                    direction,
                    should_focus: false, // Default for replay
                });
            }
        }

        let response_msg = ServerMessage::PaneCreated {
            pane: pane_info.clone(),
            direction,
            should_focus: true, // Requester focuses new pane
        };
        let broadcast_msg = ServerMessage::PaneCreated {
            pane: pane_info,
            direction,
            should_focus: false, // Others don't focus
        };

        let (response, broadcast) = if commit_seq > 0 {
            (
                ServerMessage::Sequenced {
                    seq: commit_seq,
                    inner: Box::new(response_msg),
                },
                ServerMessage::Sequenced {
                    seq: commit_seq,
                    inner: Box::new(broadcast_msg),
                },
            )
        } else {
            (response_msg, broadcast_msg)
        };

        // Broadcast to all clients attached to this session
        HandlerResult::ResponseWithBroadcast {
            response,
            session_id,
            broadcast,
        }
    }

    /// Handle SelectPane message - update focused pane
    ///
    /// FEAT-078: Updates per-client focus state instead of global.
    /// FEAT-056: Checks user priority lock before changing focus.
    pub async fn handle_select_pane(&self, pane_id: Uuid) -> HandlerResult {
        debug!("SelectPane {} request from {}", pane_id, self.client_id);

        // Check if user priority lock is active (FEAT-056)
        if let Some((_client_id, remaining_ms)) = self.arbitrator.is_any_lock_active() {
            debug!(
                "SelectPane blocked by user priority lock, retry after {}ms",
                remaining_ms
            );
            return HandlerContext::error_with_details(
                ErrorCode::UserPriorityActive,
                format!("User priority lock active, retry after {}ms", remaining_ms),
                ErrorDetails::HumanControl { remaining_ms },
            );
        }

        let session_manager = self.session_manager.read().await;

        // Find the pane
        match session_manager.find_pane(pane_id) {
            Some((session, window, _pane)) => {
                let session_id = session.id();
                let window_id = window.id();

                // Update client focus
                self.registry.update_client_focus(self.client_id, Some(session_id), Some(window_id), Some(pane_id));
                
                debug!("Client {} selected pane {}", self.client_id, pane_id);
                
                // Send confirmation ONLY to the requesting client
                HandlerResult::Response(ServerMessage::PaneFocused {
                    session_id,
                    window_id,
                    pane_id,
                })
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

    /// Handle ClosePane message - kill PTY and cleanup
    pub async fn handle_close_pane(&self, pane_id: Uuid) -> HandlerResult {
        info!("ClosePane {} request from {}", pane_id, self.client_id);

        // FEAT-079: Check arbitration before closing pane (kill action)
        if let Err(blocked) = self.check_arbitration(Resource::Pane(pane_id), Action::Kill) {
            return blocked;
        }

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

        // FEAT-079: Record human activity if this is a human actor
        self.record_human_activity(Resource::Window(window_id), Action::Layout);

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

        // FEAT-079: Check arbitration before resizing
        if let Err(blocked) = self.check_arbitration(Resource::Pane(pane_id), Action::Layout) {
            return blocked;
        }

        // FEAT-079: Record human activity if this is a human actor
        self.record_human_activity(Resource::Pane(pane_id), Action::Layout);

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

                // Find session ID for broadcast
                let session_id = session_manager.find_pane(pane_id)
                    .map(|(s, _, _)| s.id());

                if let Some(session_id) = session_id {
                    HandlerResult::BroadcastToSession {
                        session_id,
                        broadcast: ServerMessage::PaneResized {
                            pane_id,
                            new_cols: cols,
                            new_rows: rows,
                        },
                    }
                } else {
                    HandlerResult::NoResponse
                }
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

    /// Handle screen redraw request
    pub async fn handle_redraw(&self, pane_id: Option<Uuid>) -> HandlerResult {
        let session_manager = self.session_manager.read().await;
        let pty_manager = self.pty_manager.read().await;

        if let Some(pid) = pane_id {
            // Redraw specific pane
            if let Some(pane) = session_manager.find_pane(pid) {
                if let Some(handle) = pty_manager.get(pid) {
                    let (cols, rows) = pane.2.dimensions();
                    // Calling resize with same dimensions triggers SIGWINCH in most PTYs
                    let _ = handle.resize(cols, rows);
                }
            }
        } else {
            // Redraw all panes in the session this client is attached to
            if let Some(session_id) = self.registry.get_client_session(self.client_id) {
                if let Some(session) = session_manager.get_session(session_id) {
                    for window in session.windows() {
                        for pane in window.panes() {
                            if let Some(handle) = pty_manager.get(pane.id()) {
                                let (cols, rows) = pane.dimensions();
                                let _ = handle.resize(cols, rows);
                            }
                        }
                    }
                }
            }
        }

        HandlerResult::NoResponse
    }

    // ==================== Mirror Pane Handler (FEAT-062) ====================

    /// Handle CreateMirror message - create a read-only mirror of another pane
    ///
    /// BUG-063: The mirror pane is created in the CALLER's attached session,
    /// not the source pane's session. This enables cross-session "plate spinning"
    /// visibility where an orchestrator can mirror worker panes into their own session.
    pub async fn handle_create_mirror(
        &self,
        source_pane_id: Uuid,
        _target_pane_id: Option<Uuid>,
        direction: Option<SplitDirection>,
    ) -> HandlerResult {
        info!(
            "CreateMirror request from {} for source pane {}",
            self.client_id, source_pane_id
        );

        let direction = direction.unwrap_or(SplitDirection::Vertical);

        // BUG-063: Get the caller's attached session - this is where the mirror will be created
        let attached_session_id = match self.registry.get_client_session(self.client_id) {
            Some(id) => id,
            None => {
                return HandlerContext::error(
                    ErrorCode::InvalidOperation,
                    "Must be attached to a session to create mirror panes. Call ccmux_attach_session first.".to_string(),
                );
            }
        };

        // Verify the source pane exists
        {
            let session_manager = self.session_manager.read().await;
            if session_manager.find_pane(source_pane_id).is_none() {
                return HandlerContext::error(
                    ErrorCode::PaneNotFound,
                    format!("Source pane '{}' not found", source_pane_id),
                );
            }
        }

        // Create the mirror pane in the CALLER's attached session (not the source pane's session)
        let (mirror_pane_info, target_session_id, target_window_id, target_session_name) = {
            let mut session_manager = self.session_manager.write().await;

            let session = match session_manager.get_session_mut(attached_session_id) {
                Some(s) => s,
                None => {
                    return HandlerContext::error(
                        ErrorCode::SessionNotFound,
                        "Attached session not found".to_string(),
                    );
                }
            };

            let session_name = session.name().to_string();
            let session_id = session.id();

            // Get the active window ID or first window ID in the attached session
            let window_id = session.active_window_id()
                .or_else(|| session.window_ids().first().copied())
                .ok_or_else(|| {
                    HandlerContext::error(
                        ErrorCode::WindowNotFound,
                        "No window available in attached session for mirror creation".to_string(),
                    )
                });

            let window_id = match window_id {
                Ok(id) => id,
                Err(result) => return result,
            };

            let window = match session.get_window_mut(window_id) {
                Some(w) => w,
                None => {
                    return HandlerContext::error(
                        ErrorCode::WindowNotFound,
                        "Window not found in attached session".to_string(),
                    );
                }
            };

            // Create a mirror pane (special pane type that doesn't have a PTY)
            let index = window.pane_count();
            let mirror_pane = crate::session::Pane::create_mirror(window_id, index, source_pane_id);
            let mirror_info = mirror_pane.to_info();

            // Add the pane to the window
            window.add_pane(mirror_pane);

            (mirror_info, session_id, window_id, session_name)
        };

        // Register the mirror relationship
        {
            let mut session_manager = self.session_manager.write().await;
            session_manager.mirror_registry_mut().register(source_pane_id, mirror_pane_info.id);
        }

        info!(
            "Mirror pane {} created in session {} for source pane {}",
            mirror_pane_info.id, target_session_name, source_pane_id
        );

        // Send response to requesting client AND broadcast to the TARGET session
        // Using RespondWithBroadcast ensures the MCP bridge receives the response
        // (BroadcastToSession only broadcasts to others, not the sender - BUG-059)
        let response = ServerMessage::MirrorCreated {
            mirror_pane: mirror_pane_info.clone(),
            source_pane_id,
            session_id: target_session_id,
            session_name: target_session_name,
            window_id: target_window_id,
            direction,
            should_focus: false,
        };

        HandlerResult::ResponseWithBroadcast {
            response: response.clone(),
            session_id: target_session_id,
            broadcast: response,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pty::PtyManager;
    use crate::registry::ClientRegistry;
    use crate::session::SessionManager;
    use crate::arbitration::Arbitrator;
    use std::sync::Arc;
    use tokio::sync::{mpsc, RwLock};

    fn create_test_context() -> HandlerContext {
        let session_manager = Arc::new(RwLock::new(SessionManager::new()));
        let pty_manager = Arc::new(RwLock::new(PtyManager::new()));
        let registry = Arc::new(ClientRegistry::new());
        let config = Arc::new(crate::config::AppConfig::default());
        let arbitrator = Arc::new(Arbitrator::new());
        let command_executor = Arc::new(crate::sideband::AsyncCommandExecutor::new(
            Arc::clone(&session_manager),
            Arc::clone(&pty_manager),
            Arc::clone(&registry),
        ));

        let (tx, _rx) = mpsc::channel(10);
        let client_id = registry.register_client(tx);

        let (pane_closed_tx, _) = mpsc::channel(10);
        HandlerContext::new(
            session_manager,
            pty_manager,
            registry,
            config,
            client_id,
            pane_closed_tx,
            command_executor,
            arbitrator,
            None,
        )
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
                response: ServerMessage::PaneCreated { pane, direction, .. },
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
        let (_pane1_id, pane2_id) = {
            let mut session_manager = ctx.session_manager.write().await;
            let session = session_manager.get_session_mut(session_id).unwrap();
            let window = session.get_window_mut(window_id).unwrap();
            let pane1_id = window.create_pane().id();
            let pane2_id = window.create_pane().id();
            (pane1_id, pane2_id)
        };

        // Select second pane
        let result = ctx.handle_select_pane(pane2_id).await;

        // FEAT-078: Returns Response with PaneFocused
        match result {
            HandlerResult::Response(ServerMessage::PaneFocused { pane_id, .. }) => {
                assert_eq!(pane_id, pane2_id);
            }
            _ => panic!("Expected Response with PaneFocused"),
        }

        // Verify focus was updated in registry
        let focus = ctx.registry.get_client_focus(ctx.client_id).unwrap();
        assert_eq!(focus.active_pane_id, Some(pane2_id));
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
            HandlerResult::BroadcastToSession { session_id: broadcast_session, broadcast } => {
                assert_eq!(broadcast_session, session_id);
                match broadcast {
                    ServerMessage::PaneResized { pane_id: id, new_cols, new_rows } => {
                        assert_eq!(id, pane_id);
                        assert_eq!(new_cols, 120);
                        assert_eq!(new_rows, 40);
                    }
                    _ => panic!("Expected PaneResized broadcast"),
                }
            }
            _ => panic!("Expected BroadcastToSession response"),
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
    async fn test_select_pane_blocked_by_arbitrator() {
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
        ctx.arbitrator.set_lock(other_client_id, 5000, "Test Lock".to_string());

        // Try to select pane - should be blocked
        let result = ctx.handle_select_pane(pane_id).await;

        match result {
            HandlerResult::Response(ServerMessage::Error { code, message, .. }) => {
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
            HandlerResult::Response(ServerMessage::PaneFocused { pane_id: focused_id, .. }) => {
                assert_eq!(focused_id, pane_id);
            }
            _ => panic!("Expected PaneFocused response"),
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
        ctx.arbitrator.set_lock(other_client_id, 5000, "Test Lock".to_string());
        ctx.arbitrator.release_lock(other_client_id);

        // Should succeed after lock released
        let result = ctx.handle_select_pane(pane_id).await;

        match result {
            HandlerResult::Response(ServerMessage::PaneFocused { pane_id: focused_id, .. }) => {
                assert_eq!(focused_id, pane_id);
            }
            _ => panic!("Expected PaneFocused response"),
        }
    }
}
