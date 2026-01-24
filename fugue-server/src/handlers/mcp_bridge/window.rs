use tracing::{info, warn, debug};
use uuid::Uuid;
use fugue_protocol::{ErrorCode, ServerMessage, WindowInfo, SplitDirection};
use crate::pty::{PtyConfig, PtyOutputPoller};
use crate::handlers::{HandlerContext, HandlerResult};

impl HandlerContext {
    /// Handle ListWindows - list all windows in a session
    pub async fn handle_list_windows(
        &self,
        session_filter: Option<String>,
    ) -> HandlerResult {
        debug!("ListWindows request from {} (filter: {:?})", self.client_id, session_filter);

        let session_manager = self.session_manager.read().await;

        // Find the session
        let session = if let Some(ref filter) = session_filter {
            // Try to parse as UUID first
            if let Ok(session_id) = Uuid::parse_str(filter) {
                session_manager.get_session(session_id)
            } else {
                // Try by name
                session_manager.get_session_by_name(filter)
            }
        } else {
            // Use client's focused session, global active session, or first session (FEAT-078)
            self.resolve_active_session(&session_manager)
                .and_then(|id| session_manager.get_session(id))
                .or_else(|| session_manager.list_sessions().first().copied())
        };

        match session {
            Some(session) => {
                let session_name = session.name().to_string();
                let client_focus = self.registry.get_client_focus(self.client_id);

                let windows: Vec<WindowInfo> = session
                    .windows()
                    .map(|w| WindowInfo {
                        id: w.id(),
                        session_id: session.id(),
                        name: w.name().to_string(),
                        index: w.index(),
                        pane_count: w.pane_count(),
                        // FEAT-078: Return client-specific active pane if this is the focused window
                        active_pane_id: if let Some(ref focus) = client_focus {
                            if focus.active_window_id == Some(w.id()) {
                                focus.active_pane_id
                            } else {
                                w.active_pane_id()
                            }
                        } else {
                            w.active_pane_id()
                        },
                    })
                    .collect();

                debug!("Found {} windows in session {}", windows.len(), session_name);
                HandlerResult::Response(ServerMessage::WindowList {
                    session_name,
                    windows,
                })
            }
            None => {
                let error_msg = session_filter
                    .map(|s| format!("Session '{}' not found", s))
                    .unwrap_or_else(|| "No sessions exist".to_string());

                debug!("{}", error_msg);
                HandlerContext::error(ErrorCode::SessionNotFound, error_msg)
            }
        }
    }

    /// Handle CreateWindowWithOptions - create a window with full control
    pub async fn handle_create_window_with_options(
        &self,
        session_filter: Option<String>,
        name: Option<String>,
        command: Option<String>,
        cwd: Option<String>,
    ) -> HandlerResult {
        info!(
            "CreateWindowWithOptions request from {} (session: {:?}, name: {:?}, cwd: {:?})",
            self.client_id, session_filter, name, cwd
        );

        let mut session_manager = self.session_manager.write().await;

        // Find the session
        let session_id = if let Some(ref filter) = session_filter {
            if let Ok(id) = Uuid::parse_str(filter) {
                if session_manager.get_session(id).is_some() {
                    id
                } else {
                    return HandlerContext::error(
                        ErrorCode::SessionNotFound,
                        format!("Session '{}' not found", filter),
                    );
                }
            } else {
                match session_manager.get_session_by_name(filter) {
                    Some(s) => s.id(),
                    None => {
                        return HandlerContext::error(
                            ErrorCode::SessionNotFound,
                            format!("Session '{}' not found", filter),
                        );
                    }
                }
            }
        } else {
            // Use client's focused session or first session (FEAT-078)
            match self.resolve_active_session(&session_manager) {
                Some(id) => id,
                None => {
                    return HandlerContext::error(
                        ErrorCode::SessionNotFound,
                        "No sessions exist",
                    );
                }
            }
        };

        // Get session name for response
        let session_name = session_manager
            .get_session(session_id)
            .map(|s| s.name().to_string())
            .unwrap_or_default();

        // Create the window
        let session = match session_manager.get_session_mut(session_id) {
            Some(s) => s,
            None => {
                return HandlerContext::error(
                    ErrorCode::SessionNotFound,
                    "Session disappeared",
                );
            }
        };

        let window = session.create_window(name);
        let window_id = window.id();

        let window = match session.get_window_mut(window_id) {
            Some(w) => w,
            None => {
                return HandlerContext::error(
                    ErrorCode::InternalError,
                    "Window disappeared",
                );
            }
        };

        // Create default pane
        let pane = window.create_pane();
        let pane_id = pane.id();
        let pane_info = pane.to_info();

        // Initialize the parser
        let pane = match window.get_pane_mut(pane_id) {
            Some(p) => p,
            None => {
                return HandlerContext::error(
                    ErrorCode::InternalError,
                    "Pane disappeared",
                );
            }
        };
        pane.init_parser();

        // Broadcast WindowCreated to all clients in session (BUG-032)
        let window_info = window.to_info();

        // Capture session environment before dropping lock
        let session_env = session.environment().clone();

        // Drop session_manager lock before spawning PTY
        drop(session_manager);

        // Spawn PTY
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".into());
        let mut config = if let Some(ref cmd) = command {
            // Wrap user command in shell to handle arguments and shell syntax
            PtyConfig::command("sh").with_arg("-c").with_arg(cmd)
        } else {
            PtyConfig::command(&shell)
        };
        // BUG-050: Apply cwd if provided
        if let Some(ref cwd) = cwd {
            config = config.with_cwd(cwd);
        }
        config = config.with_fugue_context(session_id, &session_name, window_id, pane_id);
        // Apply session environment variables
        config = config.with_env_map(&session_env);

        {
            let mut pty_manager = self.pty_manager.write().await;
            match pty_manager.spawn(pane_id, config) {
                Ok(handle) => {
                    info!("PTY spawned for MCP window pane {}", pane_id);

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
                    info!("Output poller started for MCP window pane {} (sideband enabled)", pane_id);
                }
                Err(e) => {
                    warn!("Failed to spawn PTY for pane {}: {}", pane_id, e);
                }
            }
        }

        info!("Window {} created in session {} with pane {}", window_id, session_name, pane_id);

        self.registry.broadcast_to_session_except(
            session_id,
            self.client_id,
            ServerMessage::WindowCreated { should_focus: false, 
                window: window_info,
            },
        ).await;

        // Return response to MCP client and broadcast to TUI clients (BUG-032)
        HandlerResult::ResponseWithBroadcast {
            response: ServerMessage::WindowCreatedWithDetails {
                window_id,
                pane_id,
                session_name,
                should_focus: true,
            },
            session_id,
            broadcast: ServerMessage::PaneCreated {
                pane: pane_info,
                direction: SplitDirection::Vertical, // Default direction for new window pane
                should_focus: false,
            },
        }
    }
}
