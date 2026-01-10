//! MCP Bridge message handlers
//!
//! Handlers for messages used by the MCP bridge to query and manipulate
//! sessions, windows, and panes without being attached to a specific session.

use tracing::{debug, info, warn};
use uuid::Uuid;

use ccmux_protocol::{
    ErrorCode, PaneListEntry, PaneState, ServerMessage, SplitDirection, WindowInfo,
};

use crate::pty::{PtyConfig, PtyOutputPoller};

use super::{HandlerContext, HandlerResult};

impl HandlerContext {
    /// Handle ListAllPanes - list all panes across all sessions
    ///
    /// This is used by the MCP bridge to give Claude visibility into all panes
    /// without needing to attach to a specific session.
    pub async fn handle_list_all_panes(
        &self,
        session_filter: Option<String>,
    ) -> HandlerResult {
        debug!("ListAllPanes request from {} (filter: {:?})", self.client_id, session_filter);

        let session_manager = self.session_manager.read().await;

        let mut panes = Vec::new();

        for session in session_manager.list_sessions() {
            // Apply session filter if provided
            if let Some(ref filter) = session_filter {
                // Try to match by UUID or name
                if session.id().to_string() != *filter && session.name() != filter {
                    continue;
                }
            }

            for window in session.windows() {
                for pane in window.panes() {
                    let claude_state = match pane.state() {
                        PaneState::Claude(cs) => Some(cs.clone()),
                        _ => None,
                    };

                    panes.push(PaneListEntry {
                        id: pane.id(),
                        session_name: session.name().to_string(),
                        window_index: window.index(),
                        window_name: window.name().to_string(),
                        pane_index: pane.index(),
                        cols: pane.dimensions().0,
                        rows: pane.dimensions().1,
                        title: pane.title().map(|s| s.to_string()),
                        cwd: pane.cwd().map(|s| s.to_string()),
                        state: pane.state().clone(),
                        is_claude: pane.is_claude(),
                        claude_state,
                    });
                }
            }
        }

        debug!("Found {} panes", panes.len());
        HandlerResult::Response(ServerMessage::AllPanesList { panes })
    }

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
            // Use first session if not specified
            session_manager.list_sessions().first().copied()
        };

        match session {
            Some(session) => {
                let session_name = session.name().to_string();

                let windows: Vec<WindowInfo> = session
                    .windows()
                    .map(|w| WindowInfo {
                        id: w.id(),
                        session_id: session.id(),
                        name: w.name().to_string(),
                        index: w.index(),
                        pane_count: w.pane_count(),
                        active_pane_id: w.active_pane_id(),
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

    /// Handle ReadPane - read scrollback from a pane
    pub async fn handle_read_pane(
        &self,
        pane_id: Uuid,
        lines: usize,
    ) -> HandlerResult {
        debug!("ReadPane {} request from {} (lines: {})", pane_id, self.client_id, lines);

        let session_manager = self.session_manager.read().await;

        match session_manager.find_pane(pane_id) {
            Some((_, _, pane)) => {
                // Limit to reasonable number of lines
                let lines = lines.min(1000);

                // Get lines from scrollback
                let scrollback = pane.scrollback();
                let all_lines: Vec<&str> = scrollback.get_lines().collect();
                let start = all_lines.len().saturating_sub(lines);
                let content = all_lines[start..].join("\n");

                debug!("Read {} lines from pane {}", all_lines.len().min(lines), pane_id);
                HandlerResult::Response(ServerMessage::PaneContent {
                    pane_id,
                    content,
                })
            }
            None => {
                debug!("Pane {} not found for ReadPane", pane_id);
                HandlerContext::error(
                    ErrorCode::PaneNotFound,
                    format!("Pane {} not found", pane_id),
                )
            }
        }
    }

    /// Handle GetPaneStatus - get detailed pane status
    pub async fn handle_get_pane_status(&self, pane_id: Uuid) -> HandlerResult {
        debug!("GetPaneStatus {} request from {}", pane_id, self.client_id);

        let session_manager = self.session_manager.read().await;
        let pty_manager = self.pty_manager.read().await;

        match session_manager.find_pane(pane_id) {
            Some((session, window, pane)) => {
                let has_pty = pty_manager.contains(pane_id);

                HandlerResult::Response(ServerMessage::PaneStatus {
                    pane_id,
                    session_name: session.name().to_string(),
                    window_name: window.name().to_string(),
                    window_index: window.index(),
                    pane_index: pane.index(),
                    cols: pane.dimensions().0,
                    rows: pane.dimensions().1,
                    title: pane.title().map(|s| s.to_string()),
                    cwd: pane.cwd().map(|s| s.to_string()),
                    state: pane.state().clone(),
                    has_pty,
                    is_awaiting_input: pane.is_awaiting_input(),
                    is_awaiting_confirmation: pane.is_awaiting_confirmation(),
                })
            }
            None => {
                debug!("Pane {} not found for GetPaneStatus", pane_id);
                HandlerContext::error(
                    ErrorCode::PaneNotFound,
                    format!("Pane {} not found", pane_id),
                )
            }
        }
    }

    /// Handle CreatePaneWithOptions - create a pane with full control
    pub async fn handle_create_pane_with_options(
        &self,
        session_filter: Option<String>,
        window_filter: Option<String>,
        direction: SplitDirection,
        command: Option<String>,
        cwd: Option<String>,
    ) -> HandlerResult {
        debug!(
            client_id = %self.client_id,
            session_filter = ?session_filter,
            window_filter = ?window_filter,
            direction = ?direction,
            command = ?command,
            cwd = ?cwd,
            "handle_create_pane_with_options called"
        );
        info!(
            "CreatePaneWithOptions request from {} (session: {:?}, window: {:?}, direction: {:?})",
            self.client_id, session_filter, window_filter, direction
        );

        let mut session_manager = self.session_manager.write().await;

        // Find or create session
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
            // Use first session or create one
            match session_manager.list_sessions().first() {
                Some(s) => s.id(),
                None => {
                    match session_manager.create_session("default") {
                        Ok(s) => s.id(),
                        Err(e) => {
                            return HandlerContext::error(
                                ErrorCode::InternalError,
                                format!("Failed to create default session: {}", e),
                            );
                        }
                    }
                }
            }
        };

        let session_name = session_manager
            .get_session(session_id)
            .map(|s| s.name().to_string())
            .unwrap_or_default();

        // Find or create window
        let session = match session_manager.get_session_mut(session_id) {
            Some(s) => s,
            None => {
                return HandlerContext::error(
                    ErrorCode::SessionNotFound,
                    "Session disappeared",
                );
            }
        };

        let window_id = if let Some(ref filter) = window_filter {
            if let Ok(id) = Uuid::parse_str(filter) {
                if session.get_window(id).is_some() {
                    id
                } else {
                    return HandlerContext::error(
                        ErrorCode::WindowNotFound,
                        format!("Window '{}' not found", filter),
                    );
                }
            } else {
                // Try by name - find first matching window
                match session.windows().find(|w| w.name() == filter) {
                    Some(w) => w.id(),
                    None => {
                        return HandlerContext::error(
                            ErrorCode::WindowNotFound,
                            format!("Window '{}' not found", filter),
                        );
                    }
                }
            }
        } else {
            // Use first window or create one
            // Check first, then create to avoid borrow conflict
            let existing_window_id = session.windows().next().map(|w| w.id());
            match existing_window_id {
                Some(id) => id,
                None => session.create_window(None).id(),
            }
        };

        // Create the pane
        let window = match session.get_window_mut(window_id) {
            Some(w) => w,
            None => {
                return HandlerContext::error(
                    ErrorCode::WindowNotFound,
                    "Window disappeared",
                );
            }
        };

        let pane = window.create_pane();
        let pane_info = pane.to_info();
        let pane_id = pane_info.id;

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

        // Drop session_manager lock before spawning PTY
        drop(session_manager);

        // Spawn PTY
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".into());
        let cmd = command.as_deref().unwrap_or(&shell);
        let mut config = PtyConfig::command(cmd);
        if let Some(ref cwd) = cwd {
            config = config.with_cwd(cwd);
        }

        {
            let mut pty_manager = self.pty_manager.write().await;
            match pty_manager.spawn(pane_id, config) {
                Ok(handle) => {
                    info!("PTY spawned for MCP pane {}", pane_id);

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
                    info!("Output poller started for MCP pane {} (sideband enabled)", pane_id);
                }
                Err(e) => {
                    warn!("Failed to spawn PTY for pane {}: {}", pane_id, e);
                    // Continue anyway - pane exists but without PTY
                }
            }
        }

        let direction_str = match direction {
            SplitDirection::Horizontal => "horizontal",
            SplitDirection::Vertical => "vertical",
        };

        info!(
            pane_id = %pane_id,
            session_id = %session_id,
            session_name = %session_name,
            window_id = %window_id,
            "Pane created in session"
        );

        // Return detailed response to MCP client and broadcast to TUI clients
        debug!(
            pane_id = %pane_id,
            session_id = %session_id,
            broadcast_type = "PaneCreated",
            "Returning ResponseWithBroadcast for pane creation - TUI clients in session should receive PaneCreated"
        );
        HandlerResult::ResponseWithBroadcast {
            response: ServerMessage::PaneCreatedWithDetails {
                pane_id,
                session_id,
                session_name,
                window_id,
                direction: direction_str.to_string(),
            },
            session_id,
            broadcast: ServerMessage::PaneCreated { pane: pane_info },
        }
    }

    /// Handle CreateSessionWithOptions - create a session with full control
    pub async fn handle_create_session_with_options(
        &self,
        name: Option<String>,
    ) -> HandlerResult {
        info!("CreateSessionWithOptions request from {} (name: {:?})", self.client_id, name);

        let mut session_manager = self.session_manager.write().await;

        // Generate name if not provided
        let session_name = name.unwrap_or_else(|| {
            format!("session-{}", session_manager.session_count())
        });

        // Create the session
        let session = match session_manager.create_session(&session_name) {
            Ok(s) => s,
            Err(e) => {
                return HandlerContext::error(
                    ErrorCode::InternalError,
                    format!("Failed to create session: {}", e),
                );
            }
        };
        let session_id = session.id();

        // Create default window with pane
        let session = match session_manager.get_session_mut(session_id) {
            Some(s) => s,
            None => {
                return HandlerContext::error(
                    ErrorCode::InternalError,
                    "Session disappeared",
                );
            }
        };

        let window = session.create_window(None);
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

        let pane = window.create_pane();
        let pane_id = pane.id();

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

        // Drop session_manager lock before spawning PTY
        drop(session_manager);

        // Spawn PTY for the default pane
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".into());
        let config = PtyConfig::command(&shell);

        {
            let mut pty_manager = self.pty_manager.write().await;
            if let Err(e) = pty_manager.spawn(pane_id, config) {
                warn!("Failed to spawn PTY for pane {}: {}", pane_id, e);
            }
        }

        info!("Session {} created with window {} and pane {}", session_name, window_id, pane_id);

        HandlerResult::Response(ServerMessage::SessionCreatedWithDetails {
            session_id,
            session_name,
            window_id,
            pane_id,
        })
    }

    /// Handle CreateWindowWithOptions - create a window with full control
    pub async fn handle_create_window_with_options(
        &self,
        session_filter: Option<String>,
        name: Option<String>,
        command: Option<String>,
    ) -> HandlerResult {
        info!(
            "CreateWindowWithOptions request from {} (session: {:?}, name: {:?})",
            self.client_id, session_filter, name
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
            // Use first session
            match session_manager.list_sessions().first() {
                Some(s) => s.id(),
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

        // Drop session_manager lock before spawning PTY
        drop(session_manager);

        // Spawn PTY
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".into());
        let cmd = command.as_deref().unwrap_or(&shell);
        let config = PtyConfig::command(cmd);

        {
            let mut pty_manager = self.pty_manager.write().await;
            if let Err(e) = pty_manager.spawn(pane_id, config) {
                warn!("Failed to spawn PTY for pane {}: {}", pane_id, e);
            }
        }

        info!("Window {} created in session {} with pane {}", window_id, session_name, pane_id);

        HandlerResult::Response(ServerMessage::WindowCreatedWithDetails {
            window_id,
            pane_id,
            session_name,
        })
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

    async fn create_session_with_pane(ctx: &HandlerContext) -> (Uuid, Uuid, Uuid) {
        let mut session_manager = ctx.session_manager.write().await;
        let session = session_manager.create_session("test").unwrap();
        let session_id = session.id();

        let session = session_manager.get_session_mut(session_id).unwrap();
        let window = session.create_window(Some("main".to_string()));
        let window_id = window.id();

        let window = session.get_window_mut(window_id).unwrap();
        let pane_id = window.create_pane().id();

        (session_id, window_id, pane_id)
    }

    #[tokio::test]
    async fn test_list_all_panes_empty() {
        let ctx = create_test_context();
        let result = ctx.handle_list_all_panes(None).await;

        match result {
            HandlerResult::Response(ServerMessage::AllPanesList { panes }) => {
                assert!(panes.is_empty());
            }
            _ => panic!("Expected AllPanesList response"),
        }
    }

    #[tokio::test]
    async fn test_list_all_panes_with_panes() {
        let ctx = create_test_context();
        let (_session_id, _window_id, pane_id) = create_session_with_pane(&ctx).await;

        let result = ctx.handle_list_all_panes(None).await;

        match result {
            HandlerResult::Response(ServerMessage::AllPanesList { panes }) => {
                assert_eq!(panes.len(), 1);
                assert_eq!(panes[0].id, pane_id);
                assert_eq!(panes[0].session_name, "test");
            }
            _ => panic!("Expected AllPanesList response"),
        }
    }

    #[tokio::test]
    async fn test_list_all_panes_with_session_filter() {
        let ctx = create_test_context();
        create_session_with_pane(&ctx).await;

        // Create another session
        {
            let mut session_manager = ctx.session_manager.write().await;
            let session = session_manager.create_session("other").unwrap();
            let session_id = session.id();
            let session = session_manager.get_session_mut(session_id).unwrap();
            let window_id = session.create_window(None).id();
            let window = session.get_window_mut(window_id).unwrap();
            window.create_pane();
        }

        let result = ctx.handle_list_all_panes(Some("test".to_string())).await;

        match result {
            HandlerResult::Response(ServerMessage::AllPanesList { panes }) => {
                assert_eq!(panes.len(), 1);
                assert_eq!(panes[0].session_name, "test");
            }
            _ => panic!("Expected AllPanesList response"),
        }
    }

    #[tokio::test]
    async fn test_list_windows_no_sessions() {
        let ctx = create_test_context();
        let result = ctx.handle_list_windows(None).await;

        match result {
            HandlerResult::Response(ServerMessage::Error { code, .. }) => {
                assert_eq!(code, ErrorCode::SessionNotFound);
            }
            _ => panic!("Expected Error response"),
        }
    }

    #[tokio::test]
    async fn test_list_windows_success() {
        let ctx = create_test_context();
        let (_session_id, _window_id, _pane_id) = create_session_with_pane(&ctx).await;

        let result = ctx.handle_list_windows(None).await;

        match result {
            HandlerResult::Response(ServerMessage::WindowList { session_name, windows }) => {
                assert_eq!(session_name, "test");
                assert_eq!(windows.len(), 1);
                assert_eq!(windows[0].name, "main");
            }
            _ => panic!("Expected WindowList response"),
        }
    }

    #[tokio::test]
    async fn test_read_pane_not_found() {
        let ctx = create_test_context();
        let result = ctx.handle_read_pane(Uuid::new_v4(), 100).await;

        match result {
            HandlerResult::Response(ServerMessage::Error { code, .. }) => {
                assert_eq!(code, ErrorCode::PaneNotFound);
            }
            _ => panic!("Expected Error response"),
        }
    }

    #[tokio::test]
    async fn test_read_pane_success() {
        let ctx = create_test_context();
        let (_session_id, _window_id, pane_id) = create_session_with_pane(&ctx).await;

        let result = ctx.handle_read_pane(pane_id, 100).await;

        match result {
            HandlerResult::Response(ServerMessage::PaneContent { pane_id: id, content }) => {
                assert_eq!(id, pane_id);
                // Content should be empty for a new pane
                assert!(content.is_empty());
            }
            _ => panic!("Expected PaneContent response"),
        }
    }

    #[tokio::test]
    async fn test_get_pane_status_not_found() {
        let ctx = create_test_context();
        let result = ctx.handle_get_pane_status(Uuid::new_v4()).await;

        match result {
            HandlerResult::Response(ServerMessage::Error { code, .. }) => {
                assert_eq!(code, ErrorCode::PaneNotFound);
            }
            _ => panic!("Expected Error response"),
        }
    }

    #[tokio::test]
    async fn test_get_pane_status_success() {
        let ctx = create_test_context();
        let (_session_id, _window_id, pane_id) = create_session_with_pane(&ctx).await;

        let result = ctx.handle_get_pane_status(pane_id).await;

        match result {
            HandlerResult::Response(ServerMessage::PaneStatus {
                pane_id: id,
                session_name,
                window_name,
                ..
            }) => {
                assert_eq!(id, pane_id);
                assert_eq!(session_name, "test");
                assert_eq!(window_name, "main");
            }
            _ => panic!("Expected PaneStatus response"),
        }
    }

    #[tokio::test]
    async fn test_create_session_with_options() {
        let ctx = create_test_context();
        let result = ctx.handle_create_session_with_options(Some("my-session".to_string())).await;

        match result {
            HandlerResult::Response(ServerMessage::SessionCreatedWithDetails {
                session_name,
                ..
            }) => {
                assert_eq!(session_name, "my-session");
            }
            _ => panic!("Expected SessionCreatedWithDetails response"),
        }
    }

    #[tokio::test]
    async fn test_create_session_with_auto_name() {
        let ctx = create_test_context();
        let result = ctx.handle_create_session_with_options(None).await;

        match result {
            HandlerResult::Response(ServerMessage::SessionCreatedWithDetails {
                session_name,
                ..
            }) => {
                assert!(session_name.starts_with("session-"));
            }
            _ => panic!("Expected SessionCreatedWithDetails response"),
        }
    }

    #[tokio::test]
    async fn test_create_window_with_options_no_sessions() {
        let ctx = create_test_context();
        let result = ctx.handle_create_window_with_options(None, None, None).await;

        match result {
            HandlerResult::Response(ServerMessage::Error { code, .. }) => {
                assert_eq!(code, ErrorCode::SessionNotFound);
            }
            _ => panic!("Expected Error response"),
        }
    }

    #[tokio::test]
    async fn test_create_window_with_options_success() {
        let ctx = create_test_context();
        create_session_with_pane(&ctx).await;

        let result = ctx
            .handle_create_window_with_options(None, Some("new-window".to_string()), None)
            .await;

        match result {
            HandlerResult::Response(ServerMessage::WindowCreatedWithDetails {
                session_name,
                ..
            }) => {
                assert_eq!(session_name, "test");
            }
            _ => panic!("Expected WindowCreatedWithDetails response"),
        }
    }

    #[tokio::test]
    async fn test_create_pane_with_options_creates_session() {
        let ctx = create_test_context();

        // No sessions exist, should create default
        let result = ctx
            .handle_create_pane_with_options(None, None, SplitDirection::Vertical, None, None)
            .await;

        match result {
            HandlerResult::ResponseWithBroadcast {
                response: ServerMessage::PaneCreatedWithDetails {
                    session_name,
                    direction,
                    ..
                },
                broadcast: ServerMessage::PaneCreated { pane },
                ..
            } => {
                assert_eq!(session_name, "default");
                assert_eq!(direction, "vertical");
                // Verify broadcast contains pane info
                assert!(pane.id != Uuid::nil());
            }
            _ => panic!("Expected PaneCreatedWithDetails response with broadcast"),
        }
    }

    // ==================== MCP-to-TUI Broadcast Integration Tests (BUG-010) ====================

    /// Test that MCP pane creation broadcasts to TUI clients
    ///
    /// This is an integration test for BUG-010: verifies the full path from
    /// MCP creating a pane to TUI receiving the PaneCreated broadcast.
    #[tokio::test]
    async fn test_mcp_pane_creation_broadcasts_to_tui() {
        // Create shared infrastructure (simulating what main.rs does)
        let session_manager = Arc::new(RwLock::new(SessionManager::new()));
        let pty_manager = Arc::new(RwLock::new(PtyManager::new()));
        let registry = Arc::new(ClientRegistry::new());
        let config = Arc::new(crate::config::AppConfig::default());
        let command_executor = Arc::new(crate::sideband::AsyncCommandExecutor::new(
            Arc::clone(&session_manager),
            Arc::clone(&pty_manager),
            Arc::clone(&registry),
        ));
        let (pane_closed_tx, _) = mpsc::channel(10);

        // Create a session that both TUI and MCP will use
        let session_id = {
            let mut sm = session_manager.write().await;
            let session = sm.create_session("test-session").unwrap();
            let session_id = session.id();

            // Add a window with a pane (simulating initial session state)
            let session = sm.get_session_mut(session_id).unwrap();
            let window = session.create_window(Some("main".to_string()));
            let window_id = window.id();
            let window = session.get_window_mut(window_id).unwrap();
            window.create_pane();

            session_id
        };

        // Register TUI client AND attach it to the session
        let (tui_tx, mut tui_rx) = mpsc::channel(10);
        let tui_client_id = registry.register_client(tui_tx);
        registry.attach_to_session(tui_client_id, session_id);

        // Register MCP client (NOT attached to any session - this is the key difference)
        let (mcp_tx, _mcp_rx) = mpsc::channel(10);
        let mcp_client_id = registry.register_client(mcp_tx);
        // Note: MCP client is NOT attached to any session

        // Verify initial state
        assert_eq!(registry.session_client_count(session_id), 1, "Only TUI should be attached");
        assert!(registry.get_client_session(mcp_client_id).is_none(), "MCP should not be attached");

        // Create handler context for MCP client
        let mcp_ctx = HandlerContext::new(
            Arc::clone(&session_manager),
            Arc::clone(&pty_manager),
            Arc::clone(&registry),
            Arc::clone(&config),
            mcp_client_id,
            pane_closed_tx,
            Arc::clone(&command_executor),
        );

        // MCP creates a pane (uses first session since no filter provided)
        let result = mcp_ctx
            .handle_create_pane_with_options(None, None, SplitDirection::Vertical, None, None)
            .await;

        // Extract the broadcast info from the result
        let (broadcast_session_id, broadcast_msg) = match result {
            HandlerResult::ResponseWithBroadcast {
                session_id: sid,
                broadcast,
                ..
            } => (sid, broadcast),
            _ => panic!("Expected ResponseWithBroadcast"),
        };

        // Verify the session_id matches the session TUI is attached to
        assert_eq!(
            broadcast_session_id, session_id,
            "Broadcast should target the session TUI is attached to"
        );

        // Simulate what main.rs does: call broadcast_to_session_except
        let broadcast_count = registry
            .broadcast_to_session_except(broadcast_session_id, mcp_client_id, broadcast_msg)
            .await;

        // Verify broadcast succeeded
        assert_eq!(broadcast_count, 1, "Should have broadcast to 1 client (TUI)");

        // Verify TUI received the PaneCreated message
        let received = tui_rx.try_recv();
        assert!(received.is_ok(), "TUI should have received the broadcast");

        match received.unwrap() {
            ServerMessage::PaneCreated { pane } => {
                // The new pane should have a valid ID
                assert_ne!(pane.id, Uuid::nil());
            }
            msg => panic!("Expected PaneCreated, got {:?}", msg),
        }
    }

    /// Test that broadcast fails when TUI is attached to a different session
    ///
    /// This tests Hypothesis 1 from BUG-010: session ID mismatch
    #[tokio::test]
    async fn test_mcp_broadcast_fails_with_session_mismatch() {
        let session_manager = Arc::new(RwLock::new(SessionManager::new()));
        let pty_manager = Arc::new(RwLock::new(PtyManager::new()));
        let registry = Arc::new(ClientRegistry::new());
        let config = Arc::new(crate::config::AppConfig::default());
        let command_executor = Arc::new(crate::sideband::AsyncCommandExecutor::new(
            Arc::clone(&session_manager),
            Arc::clone(&pty_manager),
            Arc::clone(&registry),
        ));
        let (pane_closed_tx, _) = mpsc::channel(10);

        // Create TWO sessions
        let (session_a_id, session_b_id) = {
            let mut sm = session_manager.write().await;

            // Session A (the one MCP will target)
            let session_a = sm.create_session("session-a").unwrap();
            let session_a_id = session_a.id();
            let session_a = sm.get_session_mut(session_a_id).unwrap();
            let window = session_a.create_window(None);
            let window_id = window.id();
            let window = session_a.get_window_mut(window_id).unwrap();
            window.create_pane();

            // Session B (where TUI is attached)
            let session_b = sm.create_session("session-b").unwrap();
            let session_b_id = session_b.id();
            let session_b = sm.get_session_mut(session_b_id).unwrap();
            let window = session_b.create_window(None);
            let window_id = window.id();
            let window = session_b.get_window_mut(window_id).unwrap();
            window.create_pane();

            (session_a_id, session_b_id)
        };

        // TUI is attached to session B
        let (tui_tx, mut tui_rx) = mpsc::channel(10);
        let tui_client_id = registry.register_client(tui_tx);
        registry.attach_to_session(tui_client_id, session_b_id);

        // MCP client (not attached)
        let (mcp_tx, _mcp_rx) = mpsc::channel(10);
        let mcp_client_id = registry.register_client(mcp_tx);

        // Create MCP handler context
        let mcp_ctx = HandlerContext::new(
            Arc::clone(&session_manager),
            Arc::clone(&pty_manager),
            Arc::clone(&registry),
            Arc::clone(&config),
            mcp_client_id,
            pane_closed_tx,
            Arc::clone(&command_executor),
        );

        // MCP creates a pane, explicitly targeting session A
        let result = mcp_ctx
            .handle_create_pane_with_options(
                Some(session_a_id.to_string()),
                None,
                SplitDirection::Vertical,
                None,
                None,
            )
            .await;

        // Extract broadcast info
        let (broadcast_session_id, broadcast_msg) = match result {
            HandlerResult::ResponseWithBroadcast {
                session_id: sid,
                broadcast,
                ..
            } => (sid, broadcast),
            _ => panic!("Expected ResponseWithBroadcast"),
        };

        // The broadcast targets session A
        assert_eq!(broadcast_session_id, session_a_id);

        // Broadcast to session A (where TUI is NOT attached)
        let broadcast_count = registry
            .broadcast_to_session_except(broadcast_session_id, mcp_client_id, broadcast_msg)
            .await;

        // No clients attached to session A, so broadcast count should be 0
        assert_eq!(
            broadcast_count, 0,
            "No clients attached to session A, so broadcast should reach 0 clients"
        );

        // TUI should NOT receive anything
        assert!(
            tui_rx.try_recv().is_err(),
            "TUI (attached to session B) should NOT receive broadcast for session A"
        );
    }
}
