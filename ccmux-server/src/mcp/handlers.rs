//! MCP tool handlers
//!
//! Implements the business logic for each MCP tool.

use uuid::Uuid;

use ccmux_protocol::{PaneState, SplitDirection};

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
    pub fn create_pane(
        &mut self,
        direction: Option<&str>,
        command: Option<&str>,
        cwd: Option<&str>,
    ) -> Result<String, McpError> {
        // Parse direction
        let _direction = match direction {
            Some("horizontal") | Some("h") => SplitDirection::Horizontal,
            _ => SplitDirection::Vertical,
        };

        // Get or create a session
        let session = if self.session_manager.list_sessions().is_empty() {
            self.session_manager
                .create_session("default")
                .map_err(|e| McpError::Internal(e.to_string()))?
        } else {
            self.session_manager.list_sessions()[0]
        };
        let session_id = session.id();

        // Get or create a window
        let session = self
            .session_manager
            .get_session_mut(session_id)
            .ok_or_else(|| McpError::Internal("Session disappeared".into()))?;

        let window_id = {
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
        let mut config = PtyConfig::command(cmd);
        if let Some(cwd) = cwd {
            config = config.with_cwd(cwd);
        }

        self.pty_manager
            .spawn(pane_id, config)
            .map_err(|e| McpError::Pty(e.to_string()))?;

        let result = serde_json::json!({
            "pane_id": pane_id.to_string(),
            "status": "created"
        });

        serde_json::to_string_pretty(&result).map_err(|e| McpError::Internal(e.to_string()))
    }

    /// Send input to a pane
    pub fn send_input(&self, pane_id: Uuid, input: &str) -> Result<String, McpError> {
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

        let result = ctx.send_input(Uuid::new_v4(), "hello");
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
}
