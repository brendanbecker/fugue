//! MCP tool handlers
//!
//! Implements the business logic for each MCP tool.

use uuid::Uuid;

use ccmux_protocol::PaneState;

use crate::claude::{inject_session_id, is_claude_command};
use crate::pty::{PtyConfig, PtyManager};
use crate::session::SessionManager;

use super::error::McpError;

/// Tool handler context
///
/// Provides access to the session and PTY managers needed by tool handlers.
pub struct ToolContext<'a> {
    pub session_manager: &'a mut SessionManager,
    pub pty_manager: &'a mut PtyManager,
}

impl<'a> ToolContext<'a> {
    /// Create a new tool context
    pub fn new(session_manager: &'a mut SessionManager, pty_manager: &'a mut PtyManager) -> Self {
        Self {
            session_manager,
            pty_manager,
        }
    }

    /// List panes, optionally filtered by session name
    pub fn list_panes(&self, session_filter: Option<&str>) -> Result<String, McpError> {
        let sessions = self.session_manager.list_sessions();
        let mut panes = Vec::new();

        for session in sessions {
            // Apply session filter if provided
            if let Some(filter) = session_filter {
                if session.name() != filter {
                    continue;
                }
            }

            for window in session.windows() {
                for pane in window.panes() {
                    let info = serde_json::json!({
                        "id": pane.id().to_string(),
                        "session": session.name(),
                        "window": window.index(),
                        "window_name": window.name(),
                        "index": pane.index(),
                        "cols": pane.dimensions().0,
                        "rows": pane.dimensions().1,
                        "title": pane.title(),
                        "cwd": pane.cwd(),
                        "is_claude": pane.is_claude(),
                        "claude_state": pane.claude_state().map(|s| {
                            serde_json::json!({
                                "session_id": s.session_id,
                                "activity": format!("{:?}", s.activity),
                                "model": s.model,
                                "tokens_used": s.tokens_used,
                            })
                        }),
                        "state": match pane.state() {
                            PaneState::Normal => "normal",
                            PaneState::Claude(_) => "claude",
                            PaneState::Exited { .. } => "exited",
                        },
                    });
                    panes.push(info);
                }
            }
        }

        serde_json::to_string_pretty(&panes).map_err(|e| McpError::Internal(e.to_string()))
    }

    /// Read output from a pane's scrollback buffer
    pub fn read_pane(&self, pane_id: Uuid, lines: usize) -> Result<String, McpError> {
        let (_, _, pane) = self
            .session_manager
            .find_pane(pane_id)
            .ok_or_else(|| McpError::PaneNotFound(pane_id.to_string()))?;

        // Limit to reasonable number of lines
        let lines = lines.min(1000);

        // Get lines from scrollback
        let scrollback = pane.scrollback();
        let all_lines: Vec<&str> = scrollback.get_lines().collect();
        let start = all_lines.len().saturating_sub(lines);
        let output_lines: Vec<&str> = all_lines[start..].to_vec();

        Ok(output_lines.join("\n"))
    }

    /// Create a new pane
    ///
    /// Note: The direction parameter is included in the response for informational purposes.
    /// Actual visual split layout is handled client-side. The server creates panes without
    /// specific layout positioning.
    ///
    /// When session is provided (UUID or name), the pane is created in that session.
    /// When window is provided (UUID or name), the pane is created in that window.
    /// If not specified, uses the first available session/window.
    pub fn create_pane(
        &mut self,
        session_filter: Option<&str>,
        window_filter: Option<&str>,
        direction: Option<&str>,
        command: Option<&str>,
        cwd: Option<&str>,
    ) -> Result<String, McpError> {
        // Parse direction (included in response for client-side layout hints)
        let direction_str = match direction {
            Some("horizontal") | Some("h") => "horizontal",
            _ => "vertical",
        };

        // Resolve session: by UUID, by name, or use first available
        let session = if let Some(filter) = session_filter {
            // Try to parse as UUID first
            if let Ok(id) = uuid::Uuid::parse_str(filter) {
                self.session_manager
                    .get_session(id)
                    .ok_or_else(|| McpError::Internal(format!("Session '{}' not found", filter)))?
            } else {
                // Try by name
                self.session_manager
                    .get_session_by_name(filter)
                    .ok_or_else(|| McpError::Internal(format!("Session '{}' not found", filter)))?
            }
        } else if self.session_manager.list_sessions().is_empty() {
            self.session_manager
                .create_session("default")
                .map_err(|e| McpError::Internal(e.to_string()))?
        } else {
            self.session_manager.list_sessions()[0]
        };
        let session_id = session.id();
        let session_name = session.name().to_string();

        // Resolve window: by UUID, by name, or use first available
        let session = self
            .session_manager
            .get_session_mut(session_id)
            .ok_or_else(|| McpError::Internal("Session disappeared".into()))?;

        let window_id = if let Some(filter) = window_filter {
            // Try to parse as UUID first
            if let Ok(id) = uuid::Uuid::parse_str(filter) {
                if session.get_window(id).is_some() {
                    id
                } else {
                    return Err(McpError::Internal(format!("Window '{}' not found", filter)));
                }
            } else {
                // Try by name
                session
                    .windows()
                    .find(|w| w.name() == filter)
                    .map(|w| w.id())
                    .ok_or_else(|| McpError::Internal(format!("Window '{}' not found", filter)))?
            }
        } else {
            // Get existing window ID first to avoid borrow conflict
            let existing_id = session.windows().next().map(|w| w.id());
            match existing_id {
                Some(id) => id,
                None => session.create_window(None).id(),
            }
        };

        // Create the pane
        let window = session
            .get_window_mut(window_id)
            .ok_or_else(|| McpError::Internal("Window disappeared".into()))?;
        let pane = window.create_pane();
        let pane_id = pane.id();

        // Initialize the parser
        let pane = window
            .get_pane_mut(pane_id)
            .ok_or_else(|| McpError::Internal("Pane disappeared".into()))?;
        pane.init_parser();

        // Spawn PTY
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".into());
        let cmd = command.unwrap_or(&shell);

        // Check if this is a Claude command and inject session ID if needed
        let (actual_cmd, args, injected_session_id) = if is_claude_command(cmd, &[]) {
            let injection = inject_session_id(cmd, &[]);
            if injection.injected {
                let session_id = injection.session_id.clone().unwrap();
                tracing::info!(
                    "Injected session ID {} for Claude pane {}",
                    session_id,
                    pane_id
                );
                // Mark pane as Claude immediately with the session ID
                pane.mark_as_claude_with_session(session_id);
            }
            (cmd.to_string(), injection.args, injection.session_id)
        } else {
            (cmd.to_string(), vec![], None)
        };

        let mut config = PtyConfig::command(&actual_cmd);
        for arg in &args {
            config = config.with_arg(arg);
        }
        if let Some(cwd) = cwd {
            config = config.with_cwd(cwd);
        }

        self.pty_manager
            .spawn(pane_id, config)
            .map_err(|e| McpError::Pty(e.to_string()))?;

        let mut result = serde_json::json!({
            "pane_id": pane_id.to_string(),
            "session_id": session_id.to_string(),
            "session": session_name,
            "window_id": window_id.to_string(),
            "direction": direction_str,
            "status": "created"
        });

        // Include Claude session ID in response if injected
        if let Some(claude_session_id) = injected_session_id {
            result["claude_session_id"] = serde_json::json!(claude_session_id);
        }

        serde_json::to_string_pretty(&result).map_err(|e| McpError::Internal(e.to_string()))
    }

    /// List all sessions
    pub fn list_sessions(&self) -> Result<String, McpError> {
        let sessions = self.session_manager.list_sessions();
        let mut result = Vec::new();

        for session in sessions {
            // Count total panes across all windows
            let pane_count: usize = session.windows().map(|w| w.pane_count()).sum();

            let info = serde_json::json!({
                "id": session.id().to_string(),
                "name": session.name(),
                "window_count": session.window_count(),
                "pane_count": pane_count,
                "created_at": session.created_at_unix(),
            });
            result.push(info);
        }

        serde_json::to_string_pretty(&result).map_err(|e| McpError::Internal(e.to_string()))
    }

    /// List windows in a session
    pub fn list_windows(&self, session_filter: Option<&str>) -> Result<String, McpError> {
        // Find the session
        let session = if let Some(filter) = session_filter {
            // Try to parse as UUID first
            if let Ok(session_id) = uuid::Uuid::parse_str(filter) {
                self.session_manager.get_session(session_id)
            } else {
                // Try by name
                self.session_manager.get_session_by_name(filter)
            }
        } else {
            // Use first session if not specified
            self.session_manager.list_sessions().first().copied()
        };

        let session = session.ok_or_else(|| {
            McpError::Internal(
                session_filter
                    .map(|s| format!("Session '{}' not found", s))
                    .unwrap_or_else(|| "No sessions exist".into()),
            )
        })?;

        let mut windows = Vec::new();
        let active_window_id = session.active_window_id();

        for window in session.windows() {
            let info = serde_json::json!({
                "id": window.id().to_string(),
                "index": window.index(),
                "name": window.name(),
                "pane_count": window.pane_count(),
                "is_active": Some(window.id()) == active_window_id,
            });
            windows.push(info);
        }

        serde_json::to_string_pretty(&windows).map_err(|e| McpError::Internal(e.to_string()))
    }

    /// Create a new session
    ///
    /// Creates a session with a default window and pane with PTY.
    pub fn create_session(&mut self, name: Option<&str>) -> Result<String, McpError> {
        // Generate name if not provided
        let session_name = name
            .map(|n| n.to_string())
            .unwrap_or_else(|| format!("session-{}", self.session_manager.session_count()));

        // Create the session
        let session = self
            .session_manager
            .create_session(&session_name)
            .map_err(|e| McpError::Internal(e.to_string()))?;
        let session_id = session.id();

        // Create default window with pane (following BUG-003 pattern)
        let session = self
            .session_manager
            .get_session_mut(session_id)
            .ok_or_else(|| McpError::Internal("Session disappeared".into()))?;

        let window = session.create_window(None);
        let window_id = window.id();

        let window = session
            .get_window_mut(window_id)
            .ok_or_else(|| McpError::Internal("Window disappeared".into()))?;

        let pane = window.create_pane();
        let pane_id = pane.id();

        // Initialize the parser
        let pane = window
            .get_pane_mut(pane_id)
            .ok_or_else(|| McpError::Internal("Pane disappeared".into()))?;
        pane.init_parser();

        // Spawn PTY for the default pane
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".into());
        let config = PtyConfig::command(&shell);

        self.pty_manager
            .spawn(pane_id, config)
            .map_err(|e| McpError::Pty(e.to_string()))?;

        let result = serde_json::json!({
            "session_id": session_id.to_string(),
            "session_name": session_name,
            "window_id": window_id.to_string(),
            "pane_id": pane_id.to_string(),
            "status": "created"
        });

        serde_json::to_string_pretty(&result).map_err(|e| McpError::Internal(e.to_string()))
    }

    /// Create a new window in a session
    ///
    /// Creates a window with a default pane and PTY.
    pub fn create_window(
        &mut self,
        session_filter: Option<&str>,
        window_name: Option<&str>,
        command: Option<&str>,
    ) -> Result<String, McpError> {
        // Find the session
        let session_id = if let Some(filter) = session_filter {
            // Try to parse as UUID first
            if let Ok(id) = uuid::Uuid::parse_str(filter) {
                if self.session_manager.get_session(id).is_some() {
                    id
                } else {
                    return Err(McpError::Internal(format!("Session '{}' not found", filter)));
                }
            } else {
                // Try by name
                self.session_manager
                    .get_session_by_name(filter)
                    .map(|s| s.id())
                    .ok_or_else(|| McpError::Internal(format!("Session '{}' not found", filter)))?
            }
        } else {
            // Use first session if not specified
            self.session_manager
                .list_sessions()
                .first()
                .map(|s| s.id())
                .ok_or_else(|| McpError::Internal("No sessions exist".into()))?
        };

        // Get session name for response
        let session_name = self
            .session_manager
            .get_session(session_id)
            .map(|s| s.name().to_string())
            .unwrap_or_default();

        // Create the window
        let session = self
            .session_manager
            .get_session_mut(session_id)
            .ok_or_else(|| McpError::Internal("Session disappeared".into()))?;

        let window = session.create_window(window_name.map(|n| n.to_string()));
        let window_id = window.id();

        let window = session
            .get_window_mut(window_id)
            .ok_or_else(|| McpError::Internal("Window disappeared".into()))?;

        // Create default pane
        let pane = window.create_pane();
        let pane_id = pane.id();

        // Initialize the parser
        let pane = window
            .get_pane_mut(pane_id)
            .ok_or_else(|| McpError::Internal("Pane disappeared".into()))?;
        pane.init_parser();

        // Spawn PTY
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".into());
        let cmd = command.unwrap_or(&shell);

        // Check if this is a Claude command and inject session ID if needed
        let (actual_cmd, args, injected_session_id) = if is_claude_command(cmd, &[]) {
            let injection = inject_session_id(cmd, &[]);
            if injection.injected {
                let session_id = injection.session_id.clone().unwrap();
                tracing::info!(
                    "Injected session ID {} for Claude pane {} in new window",
                    session_id,
                    pane_id
                );
                // Mark pane as Claude immediately with the session ID
                pane.mark_as_claude_with_session(session_id);
            }
            (cmd.to_string(), injection.args, injection.session_id)
        } else {
            (cmd.to_string(), vec![], None)
        };

        let mut config = PtyConfig::command(&actual_cmd);
        for arg in &args {
            config = config.with_arg(arg);
        }

        self.pty_manager
            .spawn(pane_id, config)
            .map_err(|e| McpError::Pty(e.to_string()))?;

        let mut result = serde_json::json!({
            "window_id": window_id.to_string(),
            "pane_id": pane_id.to_string(),
            "session": session_name,
            "status": "created"
        });

        // Include Claude session ID in response if injected
        if let Some(claude_session_id) = injected_session_id {
            result["claude_session_id"] = serde_json::json!(claude_session_id);
        }

        serde_json::to_string_pretty(&result).map_err(|e| McpError::Internal(e.to_string()))
    }

    /// Send input to a pane
    ///
    /// If `submit` is true, appends a carriage return (\r) to press Enter after the input.
    pub fn send_input(&self, pane_id: Uuid, input: &str, submit: bool) -> Result<String, McpError> {
        // Verify pane exists
        let _ = self
            .session_manager
            .find_pane(pane_id)
            .ok_or_else(|| McpError::PaneNotFound(pane_id.to_string()))?;

        // Get PTY handle
        let handle = self
            .pty_manager
            .get(pane_id)
            .ok_or_else(|| McpError::Pty(format!("No PTY for pane {}", pane_id)))?;

        // Write input to PTY
        handle
            .write_all(input.as_bytes())
            .map_err(|e| McpError::Pty(e.to_string()))?;

        // If submit is true, send carriage return to press Enter
        if submit {
            handle
                .write_all(b"\r")
                .map_err(|e| McpError::Pty(e.to_string()))?;
        }

        Ok(r#"{"status": "sent"}"#.into())
    }

    /// Get detailed status of a pane
    pub fn get_status(&self, pane_id: Uuid) -> Result<String, McpError> {
        let (session, window, pane) = self
            .session_manager
            .find_pane(pane_id)
            .ok_or_else(|| McpError::PaneNotFound(pane_id.to_string()))?;

        let has_pty = self.pty_manager.contains(pane_id);

        let status = serde_json::json!({
            "pane_id": pane_id.to_string(),
            "session": session.name(),
            "window": window.index(),
            "window_name": window.name(),
            "index": pane.index(),
            "dimensions": {
                "cols": pane.dimensions().0,
                "rows": pane.dimensions().1,
            },
            "title": pane.title(),
            "cwd": pane.cwd(),
            "has_pty": has_pty,
            "state": match pane.state() {
                PaneState::Normal => serde_json::json!({"type": "normal"}),
                PaneState::Claude(state) => serde_json::json!({
                    "type": "claude",
                    "session_id": state.session_id,
                    "activity": format!("{:?}", state.activity),
                    "model": state.model,
                    "tokens_used": state.tokens_used,
                }),
                PaneState::Exited { code } => serde_json::json!({
                    "type": "exited",
                    "exit_code": code,
                }),
            },
            "is_awaiting_input": pane.is_awaiting_input(),
            "is_awaiting_confirmation": pane.is_awaiting_confirmation(),
        });

        serde_json::to_string_pretty(&status).map_err(|e| McpError::Internal(e.to_string()))
    }

    /// Close a pane
    pub fn close_pane(&mut self, pane_id: Uuid) -> Result<String, McpError> {
        // Verify pane exists
        let (session, window, _) = self
            .session_manager
            .find_pane(pane_id)
            .ok_or_else(|| McpError::PaneNotFound(pane_id.to_string()))?;

        let session_id = session.id();
        let window_id = window.id();

        // Kill PTY if it exists
        if let Some(handle) = self.pty_manager.remove(pane_id) {
            let _ = handle.kill();
        }

        // Remove pane from session
        let session = self
            .session_manager
            .get_session_mut(session_id)
            .ok_or_else(|| McpError::Internal("Session disappeared".into()))?;
        let window = session
            .get_window_mut(window_id)
            .ok_or_else(|| McpError::Internal("Window disappeared".into()))?;
        window.remove_pane(pane_id);

        let result = serde_json::json!({
            "pane_id": pane_id.to_string(),
            "status": "closed"
        });

        serde_json::to_string_pretty(&result).map_err(|e| McpError::Internal(e.to_string()))
    }

    /// Focus a pane
    pub fn focus_pane(&mut self, pane_id: Uuid) -> Result<String, McpError> {
        // Find the pane and its window
        let (session, window, _) = self
            .session_manager
            .find_pane(pane_id)
            .ok_or_else(|| McpError::PaneNotFound(pane_id.to_string()))?;

        let session_id = session.id();
        let window_id = window.id();

        // Set as active pane
        let session = self
            .session_manager
            .get_session_mut(session_id)
            .ok_or_else(|| McpError::Internal("Session disappeared".into()))?;
        let window = session
            .get_window_mut(window_id)
            .ok_or_else(|| McpError::Internal("Window disappeared".into()))?;
        window.set_active_pane(pane_id);

        // Also set as active window
        session.set_active_window(window_id);

        let result = serde_json::json!({
            "pane_id": pane_id.to_string(),
            "status": "focused"
        });

        serde_json::to_string_pretty(&result).map_err(|e| McpError::Internal(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_context<'a>(
        session_manager: &'a mut SessionManager,
        pty_manager: &'a mut PtyManager,
    ) -> ToolContext<'a> {
        ToolContext::new(session_manager, pty_manager)
    }

    #[test]
    fn test_list_panes_empty() {
        let mut session_manager = SessionManager::new();
        let mut pty_manager = PtyManager::new();
        let ctx = create_test_context(&mut session_manager, &mut pty_manager);

        let result = ctx.list_panes(None).unwrap();
        assert_eq!(result, "[]");
    }

    #[test]
    fn test_list_panes_with_session() {
        let mut session_manager = SessionManager::new();
        let mut pty_manager = PtyManager::new();

        // Create a session with a pane
        let session = session_manager.create_session("test").unwrap();
        let session_id = session.id();
        let session = session_manager.get_session_mut(session_id).unwrap();
        let window = session.create_window(None);
        let window_id = window.id();
        let window = session.get_window_mut(window_id).unwrap();
        window.create_pane();

        let ctx = create_test_context(&mut session_manager, &mut pty_manager);
        let result = ctx.list_panes(None).unwrap();

        assert!(result.contains("test"));
        assert!(result.contains("normal"));
    }

    #[test]
    fn test_list_panes_with_filter() {
        let mut session_manager = SessionManager::new();
        let mut pty_manager = PtyManager::new();

        // Create two sessions
        session_manager.create_session("session1").unwrap();
        session_manager.create_session("session2").unwrap();

        let ctx = create_test_context(&mut session_manager, &mut pty_manager);

        // Filter by session1
        let result = ctx.list_panes(Some("session1")).unwrap();
        assert!(!result.contains("session2"));
    }

    #[test]
    fn test_read_pane_not_found() {
        let mut session_manager = SessionManager::new();
        let mut pty_manager = PtyManager::new();
        let ctx = create_test_context(&mut session_manager, &mut pty_manager);

        let result = ctx.read_pane(Uuid::new_v4(), 100);
        assert!(matches!(result, Err(McpError::PaneNotFound(_))));
    }

    #[test]
    fn test_send_input_pane_not_found() {
        let mut session_manager = SessionManager::new();
        let mut pty_manager = PtyManager::new();
        let ctx = create_test_context(&mut session_manager, &mut pty_manager);

        let result = ctx.send_input(Uuid::new_v4(), "hello", false);
        assert!(matches!(result, Err(McpError::PaneNotFound(_))));
    }

    #[test]
    fn test_get_status_pane_not_found() {
        let mut session_manager = SessionManager::new();
        let mut pty_manager = PtyManager::new();
        let ctx = create_test_context(&mut session_manager, &mut pty_manager);

        let result = ctx.get_status(Uuid::new_v4());
        assert!(matches!(result, Err(McpError::PaneNotFound(_))));
    }

    #[test]
    fn test_close_pane_not_found() {
        let mut session_manager = SessionManager::new();
        let mut pty_manager = PtyManager::new();
        let mut ctx = create_test_context(&mut session_manager, &mut pty_manager);

        let result = ctx.close_pane(Uuid::new_v4());
        assert!(matches!(result, Err(McpError::PaneNotFound(_))));
    }

    #[test]
    fn test_focus_pane_not_found() {
        let mut session_manager = SessionManager::new();
        let mut pty_manager = PtyManager::new();
        let mut ctx = create_test_context(&mut session_manager, &mut pty_manager);

        let result = ctx.focus_pane(Uuid::new_v4());
        assert!(matches!(result, Err(McpError::PaneNotFound(_))));
    }

    // ==================== List Sessions Tests ====================

    #[test]
    fn test_list_sessions_empty() {
        let mut session_manager = SessionManager::new();
        let mut pty_manager = PtyManager::new();
        let ctx = create_test_context(&mut session_manager, &mut pty_manager);

        let result = ctx.list_sessions().unwrap();
        assert_eq!(result, "[]");
    }

    #[test]
    fn test_list_sessions_with_sessions() {
        let mut session_manager = SessionManager::new();
        let mut pty_manager = PtyManager::new();

        session_manager.create_session("test1").unwrap();
        session_manager.create_session("test2").unwrap();

        let ctx = create_test_context(&mut session_manager, &mut pty_manager);
        let result = ctx.list_sessions().unwrap();

        assert!(result.contains("test1"));
        assert!(result.contains("test2"));
        assert!(result.contains("window_count"));
        assert!(result.contains("pane_count"));
    }

    // ==================== List Windows Tests ====================

    #[test]
    fn test_list_windows_no_sessions() {
        let mut session_manager = SessionManager::new();
        let mut pty_manager = PtyManager::new();
        let ctx = create_test_context(&mut session_manager, &mut pty_manager);

        let result = ctx.list_windows(None);
        assert!(result.is_err());
    }

    #[test]
    fn test_list_windows_with_windows() {
        let mut session_manager = SessionManager::new();
        let mut pty_manager = PtyManager::new();

        let session = session_manager.create_session("test").unwrap();
        let session_id = session.id();
        let session = session_manager.get_session_mut(session_id).unwrap();
        session.create_window(Some("win1".to_string()));
        session.create_window(Some("win2".to_string()));

        let ctx = create_test_context(&mut session_manager, &mut pty_manager);
        let result = ctx.list_windows(None).unwrap();

        assert!(result.contains("win1"));
        assert!(result.contains("win2"));
        assert!(result.contains("is_active"));
    }

    #[test]
    fn test_list_windows_by_session_name() {
        let mut session_manager = SessionManager::new();
        let mut pty_manager = PtyManager::new();

        let session = session_manager.create_session("target").unwrap();
        let session_id = session.id();
        let session = session_manager.get_session_mut(session_id).unwrap();
        session.create_window(Some("target-window".to_string()));

        session_manager.create_session("other").unwrap();

        let ctx = create_test_context(&mut session_manager, &mut pty_manager);
        let result = ctx.list_windows(Some("target")).unwrap();

        assert!(result.contains("target-window"));
    }

    #[test]
    fn test_list_windows_session_not_found() {
        let mut session_manager = SessionManager::new();
        let mut pty_manager = PtyManager::new();

        session_manager.create_session("existing").unwrap();

        let ctx = create_test_context(&mut session_manager, &mut pty_manager);
        let result = ctx.list_windows(Some("nonexistent"));

        assert!(result.is_err());
    }

    // ==================== Create Session Tests ====================

    #[test]
    fn test_create_session_with_name() {
        let mut session_manager = SessionManager::new();
        let mut pty_manager = PtyManager::new();
        let mut ctx = create_test_context(&mut session_manager, &mut pty_manager);

        let result = ctx.create_session(Some("my-session")).unwrap();

        assert!(result.contains("my-session"));
        assert!(result.contains("session_id"));
        assert!(result.contains("window_id"));
        assert!(result.contains("pane_id"));
        assert!(result.contains("\"status\": \"created\""));
    }

    #[test]
    fn test_create_session_auto_name() {
        let mut session_manager = SessionManager::new();
        let mut pty_manager = PtyManager::new();
        let mut ctx = create_test_context(&mut session_manager, &mut pty_manager);

        let result = ctx.create_session(None).unwrap();

        assert!(result.contains("session-0"));
        assert!(result.contains("\"status\": \"created\""));
    }

    #[test]
    fn test_create_session_duplicate_name() {
        let mut session_manager = SessionManager::new();
        let mut pty_manager = PtyManager::new();
        let mut ctx = create_test_context(&mut session_manager, &mut pty_manager);

        ctx.create_session(Some("duplicate")).unwrap();
        let result = ctx.create_session(Some("duplicate"));

        assert!(result.is_err());
    }

    // ==================== Create Window Tests ====================

    #[test]
    fn test_create_window_no_sessions() {
        let mut session_manager = SessionManager::new();
        let mut pty_manager = PtyManager::new();
        let mut ctx = create_test_context(&mut session_manager, &mut pty_manager);

        let result = ctx.create_window(None, None, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_create_window_in_default_session() {
        let mut session_manager = SessionManager::new();
        let mut pty_manager = PtyManager::new();

        session_manager.create_session("default").unwrap();

        let mut ctx = create_test_context(&mut session_manager, &mut pty_manager);
        let result = ctx.create_window(None, Some("new-window"), None).unwrap();

        assert!(result.contains("window_id"));
        assert!(result.contains("pane_id"));
        assert!(result.contains("default"));
        assert!(result.contains("\"status\": \"created\""));
    }

    #[test]
    fn test_create_window_in_named_session() {
        let mut session_manager = SessionManager::new();
        let mut pty_manager = PtyManager::new();

        session_manager.create_session("target").unwrap();
        session_manager.create_session("other").unwrap();

        let mut ctx = create_test_context(&mut session_manager, &mut pty_manager);
        let result = ctx.create_window(Some("target"), None, None).unwrap();

        assert!(result.contains("target"));
    }

    #[test]
    fn test_create_window_session_not_found() {
        let mut session_manager = SessionManager::new();
        let mut pty_manager = PtyManager::new();

        session_manager.create_session("existing").unwrap();

        let mut ctx = create_test_context(&mut session_manager, &mut pty_manager);
        let result = ctx.create_window(Some("nonexistent"), None, None);

        assert!(result.is_err());
    }

    // ==================== Create Pane Direction Tests ====================

    #[test]
    fn test_create_pane_includes_direction_in_response() {
        let mut session_manager = SessionManager::new();
        let mut pty_manager = PtyManager::new();

        session_manager.create_session("test").unwrap();

        let mut ctx = create_test_context(&mut session_manager, &mut pty_manager);
        let result = ctx.create_pane(None, None, Some("horizontal"), None, None).unwrap();

        assert!(result.contains("\"direction\": \"horizontal\""));
    }

    #[test]
    fn test_create_pane_default_direction() {
        let mut session_manager = SessionManager::new();
        let mut pty_manager = PtyManager::new();

        session_manager.create_session("test").unwrap();

        let mut ctx = create_test_context(&mut session_manager, &mut pty_manager);
        let result = ctx.create_pane(None, None, None, None, None).unwrap();

        assert!(result.contains("\"direction\": \"vertical\""));
    }

    #[test]
    fn test_create_pane_with_session_filter() {
        let mut session_manager = SessionManager::new();
        let mut pty_manager = PtyManager::new();

        session_manager.create_session("session1").unwrap();
        session_manager.create_session("session2").unwrap();

        let mut ctx = create_test_context(&mut session_manager, &mut pty_manager);
        let result = ctx.create_pane(Some("session2"), None, None, None, None).unwrap();

        assert!(result.contains("session2"));
        assert!(result.contains("session_id"));
    }

    #[test]
    fn test_create_pane_session_not_found() {
        let mut session_manager = SessionManager::new();
        let mut pty_manager = PtyManager::new();

        session_manager.create_session("existing").unwrap();

        let mut ctx = create_test_context(&mut session_manager, &mut pty_manager);
        let result = ctx.create_pane(Some("nonexistent"), None, None, None, None);

        assert!(result.is_err());
    }

    #[test]
    fn test_create_pane_with_window_filter() {
        let mut session_manager = SessionManager::new();
        let mut pty_manager = PtyManager::new();

        let session = session_manager.create_session("test").unwrap();
        let session_id = session.id();
        let session = session_manager.get_session_mut(session_id).unwrap();
        session.create_window(Some("window1".to_string()));
        session.create_window(Some("window2".to_string()));

        let mut ctx = create_test_context(&mut session_manager, &mut pty_manager);
        let result = ctx.create_pane(None, Some("window2"), None, None, None).unwrap();

        assert!(result.contains("pane_id"));
        assert!(result.contains("window_id"));
    }

    #[test]
    fn test_create_pane_window_not_found() {
        let mut session_manager = SessionManager::new();
        let mut pty_manager = PtyManager::new();

        session_manager.create_session("test").unwrap();

        let mut ctx = create_test_context(&mut session_manager, &mut pty_manager);
        let result = ctx.create_pane(None, Some("nonexistent"), None, None, None);

        assert!(result.is_err());
    }

    #[test]
    fn test_create_pane_response_includes_session_id() {
        let mut session_manager = SessionManager::new();
        let mut pty_manager = PtyManager::new();

        session_manager.create_session("test").unwrap();

        let mut ctx = create_test_context(&mut session_manager, &mut pty_manager);
        let result = ctx.create_pane(None, None, None, None, None).unwrap();

        assert!(result.contains("session_id"));
        assert!(result.contains("pane_id"));
        assert!(result.contains("window_id"));
    }
}
