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
    #[allow(deprecated)]
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

            // Get the active window for this session to determine focused pane
            let active_window_id = session.active_window_id();

            for window in session.windows() {
                // Get the active pane in this window
                let active_pane_id = window.active_pane_id();
                // A pane is focused if it's the active pane in the active window
                let is_active_window = Some(window.id()) == active_window_id;

                for pane in window.panes() {
                    // Pane is focused if it's the active pane in the active window
                    let is_focused = is_active_window && Some(pane.id()) == active_pane_id;

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
                            PaneState::Agent(state) => {
                                if state.is_claude() { "claude" } else { "agent" }
                            }
                            PaneState::Exited { .. } => "exited",
                        },
                        "is_focused": is_focused,
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
    /// If select is true, the new pane will be focused after creation.
    /// If name is provided, it will be set as the pane's user-assigned name (FEAT-036).
    pub fn create_pane(
        &mut self,
        session_filter: Option<&str>,
        window_filter: Option<&str>,
        name: Option<&str>,
        direction: Option<&str>,
        command: Option<&str>,
        cwd: Option<&str>,
        select: bool,
    ) -> Result<String, McpError> {
        // Parse direction (included in response for client-side layout hints)
        let direction_str = match direction {
            Some("horizontal") | Some("h") => "horizontal",
            _ => "vertical",
        };

        // Resolve session: by UUID, by name, or use active session (BUG-034 fix)
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
            // Use active session instead of first session
            self.session_manager
                .active_session()
                .ok_or_else(|| McpError::Internal("No sessions exist".into()))?
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

        // BUG-050: Capture active pane's cwd for inheritance before creating new pane
        let inherited_cwd = if cwd.is_none() {
            session.get_window(window_id)
                .and_then(|w| w.active_pane_id())
                .and_then(|active_id| session.get_window(window_id)?.get_pane(active_id))
                .and_then(|p| p.cwd())
                .map(String::from)
        } else {
            None
        };

        // Create the pane
        let window = session
            .get_window_mut(window_id)
            .ok_or_else(|| McpError::Internal("Window disappeared".into()))?;
        let pane = window.create_pane();
        let pane_id = pane.id();

        // If select is true, focus the new pane (before getting mutable pane ref)
        if select {
            window.set_active_pane(pane_id);
        }

        // Initialize the parser and set the name (FEAT-036)
        let pane = window
            .get_pane_mut(pane_id)
            .ok_or_else(|| McpError::Internal("Pane disappeared".into()))?;
        pane.init_parser();
        if let Some(pane_name) = name {
            pane.set_name(Some(pane_name.to_string()));
        }

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
        // BUG-050: Apply explicit cwd, or inherit from parent pane
        if let Some(cwd) = cwd {
            config = config.with_cwd(cwd);
        } else if let Some(ref inherited) = inherited_cwd {
            config = config.with_cwd(inherited);
        }
        config = config.with_ccmux_context(session_id, &session_name, window_id, pane_id);

        self.pty_manager
            .spawn(pane_id, config)
            .map_err(|e| McpError::Pty(e.to_string()))?;

        // If select is true, also set the window as active
        if select {
            let session = self
                .session_manager
                .get_session_mut(session_id)
                .ok_or_else(|| McpError::Internal("Session disappeared".into()))?;
            session.set_active_window(window_id);
        }

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
            // Use active session if not specified (BUG-034 fix)
            self.session_manager.active_session()
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
        let config = PtyConfig::command(&shell)
            .with_ccmux_context(session_id, &session_name, window_id, pane_id);

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
        cwd: Option<&str>,
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
            // Use active session if not specified (BUG-034 fix)
            self.session_manager
                .active_session_id()
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
        // BUG-050: Apply cwd if provided
        if let Some(cwd) = cwd {
            config = config.with_cwd(cwd);
        }
        config = config.with_ccmux_context(session_id, &session_name, window_id, pane_id);

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

        // Prepare data - send input and Enter key separately if submit is true
        // This avoids issues with TUI apps that expect Enter as a separate event
        // (BUG-054)
        let data = input.as_bytes();
        handle
            .write_all(data)
            .map_err(|e| McpError::Pty(e.to_string()))?;

        if submit {
            handle
                .write_all(b"\r")
                .map_err(|e| McpError::Pty(e.to_string()))?;
        }

        Ok(r#"{"status": "sent"}"#.into())
    }

    /// Send input or a special key to a pane (FEAT-093)
    ///
    /// Either `input` or `key` must be provided (but not both).
    /// - `input`: Regular text input to send
    /// - `key`: Special key name (e.g., "Escape", "Ctrl+C", "ArrowUp")
    /// - `submit`: If true and using `input`, appends carriage return
    pub fn send_input_with_key(
        &self,
        pane_id: Uuid,
        input: Option<&str>,
        key: Option<&str>,
        submit: bool,
    ) -> Result<String, McpError> {
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

        // Determine data to send based on input or key parameter
        match (input, key) {
            (Some(text), None) => {
                // Regular text input - send input and Enter key separately if submit is true
                // This avoids issues with TUI apps that expect Enter as a separate event
                // (BUG-054)
                let data = text.as_bytes();
                handle
                    .write_all(data)
                    .map_err(|e| McpError::Pty(e.to_string()))?;

                if submit {
                    handle
                        .write_all(b"\r")
                        .map_err(|e| McpError::Pty(e.to_string()))?;
                }
            }
            (None, Some(key_name)) => {
                // Special key lookup
                use super::keys::get_key_sequence;
                match get_key_sequence(key_name) {
                    Some(sequence) => {
                        handle
                            .write_all(sequence)
                            .map_err(|e| McpError::Pty(e.to_string()))?;
                    }
                    None => {
                        return Err(McpError::InvalidParams(format!(
                            "Unknown key '{}'. Supported keys include: Escape, Ctrl+C, Ctrl+D, Ctrl+Z, \
                            ArrowUp, ArrowDown, ArrowLeft, ArrowRight, F1-F12, Home, End, \
                            PageUp, PageDown, Tab, Enter, Backspace, Delete, Insert, Space. \
                            Use Ctrl+<letter> for control sequences (e.g., 'Ctrl+C').",
                            key_name
                        )));
                    }
                }
            }
            (Some(_), Some(_)) => {
                return Err(McpError::InvalidParams(
                    "Provide either 'input' or 'key', not both".into(),
                ));
            }
            (None, None) => {
                return Err(McpError::InvalidParams(
                    "Either 'input' or 'key' parameter is required".into(),
                ));
            }
        }

        Ok(r#"{"status": "sent"}"#.into())
    }

    /// Get detailed status of a pane
    #[allow(deprecated)]
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
                PaneState::Agent(state) => serde_json::json!({
                    "type": if state.is_claude() { "claude" } else { "agent" },
                    "agent_type": state.agent_type,
                    "session_id": state.session_id,
                    "activity": format!("{:?}", state.activity),
                    "model": state.get_metadata("model"),
                    "tokens_used": state.get_metadata("tokens_used"),
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

    /// Select/focus a window (make it the active window in its session)
    pub fn select_window(&mut self, window_id: Uuid) -> Result<String, McpError> {
        // Find the session containing this window
        let (session_id, _session_name) = self
            .session_manager
            .list_sessions()
            .iter()
            .find_map(|s| {
                s.windows()
                    .find(|w| w.id() == window_id)
                    .map(|_| (s.id(), s.name().to_string()))
            })
            .ok_or_else(|| McpError::WindowNotFound(format!("{}", window_id)))?;

        // Set as active window
        let session = self
            .session_manager
            .get_session_mut(session_id)
            .ok_or_else(|| McpError::Internal("Session disappeared".into()))?;
        session.set_active_window(window_id);

        let result = serde_json::json!({
            "window_id": window_id.to_string(),
            "session_id": session_id.to_string(),
            "status": "selected"
        });

        serde_json::to_string_pretty(&result).map_err(|e| McpError::Internal(e.to_string()))
    }

    /// Select/focus a session (make it the active session)
    pub fn select_session(&mut self, session_id: Uuid) -> Result<String, McpError> {
        // Verify session exists
        let session = self
            .session_manager
            .get_session(session_id)
            .ok_or_else(|| McpError::SessionNotFound(format!("{}", session_id)))?;
        let session_name = session.name().to_string();

        // Set as active session
        self.session_manager.set_active_session(session_id);

        let result = serde_json::json!({
            "session_id": session_id.to_string(),
            "session_name": session_name,
            "status": "selected"
        });

        serde_json::to_string_pretty(&result).map_err(|e| McpError::Internal(e.to_string()))
    }

    /// Rename a session
    pub fn rename_session(&mut self, session_filter: &str, new_name: &str) -> Result<String, McpError> {
        // Find session by UUID or name
        let session_id = if let Ok(id) = uuid::Uuid::parse_str(session_filter) {
            if self.session_manager.get_session(id).is_some() {
                id
            } else {
                return Err(McpError::SessionNotFound(session_filter.to_string()));
            }
        } else {
            self.session_manager
                .get_session_by_name(session_filter)
                .map(|s| s.id())
                .ok_or_else(|| McpError::SessionNotFound(session_filter.to_string()))?
        };

        // Rename the session
        self.session_manager
            .rename_session(session_id, new_name)
            .map_err(|e| McpError::Internal(e.to_string()))?;

        let result = serde_json::json!({
            "session_id": session_id.to_string(),
            "name": new_name,
            "status": "renamed"
        });

        serde_json::to_string_pretty(&result).map_err(|e| McpError::Internal(e.to_string()))
    }

    /// Split a specific pane with custom ratio
    ///
    /// Creates a new pane by splitting an existing pane. The original pane keeps
    /// the specified ratio, and the new pane gets the remaining space.
    pub fn split_pane(
        &mut self,
        pane_id: Uuid,
        direction: Option<&str>,
        ratio: Option<f64>,
        command: Option<&str>,
        cwd: Option<&str>,
        select: bool,
    ) -> Result<String, McpError> {
        // Find the pane and its session/window
        let (session, window, _pane) = self
            .session_manager
            .find_pane(pane_id)
            .ok_or_else(|| McpError::PaneNotFound(pane_id.to_string()))?;

        let session_id = session.id();
        let session_name = session.name().to_string();
        let window_id = window.id();

        // Parse direction
        let direction_str = match direction {
            Some("horizontal") | Some("h") => "horizontal",
            _ => "vertical",
        };

        // Validate and normalize ratio (default 0.5)
        let ratio = ratio.unwrap_or(0.5).clamp(0.1, 0.9) as f32;

        // Create the new pane
        let session = self
            .session_manager
            .get_session_mut(session_id)
            .ok_or_else(|| McpError::Internal("Session disappeared".into()))?;
        let window = session
            .get_window_mut(window_id)
            .ok_or_else(|| McpError::Internal("Window disappeared".into()))?;
        let new_pane = window.create_pane();
        let new_pane_id = new_pane.id();

        // If select is true, focus the new pane
        if select {
            window.set_active_pane(new_pane_id);
        }

        // Initialize the parser for the new pane
        let new_pane = window
            .get_pane_mut(new_pane_id)
            .ok_or_else(|| McpError::Internal("Pane disappeared".into()))?;
        new_pane.init_parser();

        // Spawn PTY for the new pane
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".into());
        let cmd = command.unwrap_or(&shell);

        // Check if this is a Claude command
        let (actual_cmd, args, injected_session_id) = if is_claude_command(cmd, &[]) {
            let injection = inject_session_id(cmd, &[]);
            if injection.injected {
                let session_id = injection.session_id.clone().unwrap();
                tracing::info!(
                    "Injected session ID {} for Claude pane {}",
                    session_id,
                    new_pane_id
                );
                new_pane.mark_as_claude_with_session(session_id);
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
        config = config.with_ccmux_context(session_id, &session_name, window_id, new_pane_id);

        self.pty_manager
            .spawn(new_pane_id, config)
            .map_err(|e| McpError::Pty(e.to_string()))?;

        // If select is true, also set the window as active
        if select {
            let session = self
                .session_manager
                .get_session_mut(session_id)
                .ok_or_else(|| McpError::Internal("Session disappeared".into()))?;
            session.set_active_window(window_id);
        }

        let mut result = serde_json::json!({
            "pane_id": new_pane_id.to_string(),
            "original_pane_id": pane_id.to_string(),
            "session_id": session_id.to_string(),
            "session": session_name,
            "window_id": window_id.to_string(),
            "direction": direction_str,
            "ratio": ratio,
            "status": "split"
        });

        if let Some(claude_session_id) = injected_session_id {
            result["claude_session_id"] = serde_json::json!(claude_session_id);
        }

        serde_json::to_string_pretty(&result).map_err(|e| McpError::Internal(e.to_string()))
    }

    /// Resize a pane dynamically
    ///
    /// Adjusts the size of a pane relative to its sibling. Positive delta grows
    /// the pane, negative delta shrinks it.
    pub fn resize_pane(&mut self, pane_id: Uuid, delta: f64) -> Result<String, McpError> {
        // Verify pane exists
        let _ = self
            .session_manager
            .find_pane(pane_id)
            .ok_or_else(|| McpError::PaneNotFound(pane_id.to_string()))?;

        // Validate delta bounds
        let delta = delta.clamp(-0.5, 0.5) as f32;

        // Note: The actual resize logic is handled client-side via the LayoutManager.
        // The server broadcasts the resize intent, and clients apply it to their layout.
        // For now, we acknowledge the resize request.

        let result = serde_json::json!({
            "pane_id": pane_id.to_string(),
            "delta": delta,
            "status": "resize_requested"
        });

        serde_json::to_string_pretty(&result).map_err(|e| McpError::Internal(e.to_string()))
    }

    /// Create a complex layout declaratively
    ///
    /// Parses a layout specification and creates all panes atomically.
    /// Supports nested splits with custom ratios.
    pub fn create_layout(
        &mut self,
        session_filter: Option<&str>,
        window_filter: Option<&str>,
        layout_spec: &serde_json::Value,
    ) -> Result<String, McpError> {
        // Resolve session (BUG-034 fix: use active session instead of first)
        let session = if let Some(filter) = session_filter {
            if let Ok(id) = uuid::Uuid::parse_str(filter) {
                self.session_manager
                    .get_session(id)
                    .ok_or_else(|| McpError::Internal(format!("Session '{}' not found", filter)))?
            } else {
                self.session_manager
                    .get_session_by_name(filter)
                    .ok_or_else(|| McpError::Internal(format!("Session '{}' not found", filter)))?
            }
        } else if self.session_manager.list_sessions().is_empty() {
            self.session_manager
                .create_session("default")
                .map_err(|e| McpError::Internal(e.to_string()))?
        } else {
            // Use active session instead of first session
            self.session_manager
                .active_session()
                .ok_or_else(|| McpError::Internal("No sessions exist".into()))?
        };
        let session_id = session.id();
        let session_name = session.name().to_string();

        // Resolve window
        let session = self
            .session_manager
            .get_session_mut(session_id)
            .ok_or_else(|| McpError::Internal("Session disappeared".into()))?;

        let window_id = if let Some(filter) = window_filter {
            if let Ok(id) = uuid::Uuid::parse_str(filter) {
                if session.get_window(id).is_some() {
                    id
                } else {
                    return Err(McpError::Internal(format!("Window '{}' not found", filter)));
                }
            } else {
                session
                    .windows()
                    .find(|w| w.name() == filter)
                    .map(|w| w.id())
                    .ok_or_else(|| McpError::Internal(format!("Window '{}' not found", filter)))?
            }
        } else {
            let existing_id = session.windows().next().map(|w| w.id());
            match existing_id {
                Some(id) => id,
                None => session.create_window(None).id(),
            }
        };

        // Parse the layout spec and create panes
        let created_panes = self.spawn_layout_panes(session_id, window_id, layout_spec)?;

        let result = serde_json::json!({
            "session_id": session_id.to_string(),
            "session": session_name,
            "window_id": window_id.to_string(),
            "panes": created_panes,
            "status": "created"
        });

        serde_json::to_string_pretty(&result).map_err(|e| McpError::Internal(e.to_string()))
    }

    /// Recursively spawn panes for a layout specification
    fn spawn_layout_panes(
        &mut self,
        session_id: Uuid,
        window_id: Uuid,
        spec: &serde_json::Value,
    ) -> Result<Vec<serde_json::Value>, McpError> {
        let mut created_panes = Vec::new();

        // Check if this is a pane leaf node
        if let Some(pane_spec) = spec.get("pane") {
            let pane_info = self.spawn_single_pane(session_id, window_id, pane_spec)?;
            created_panes.push(pane_info);
            return Ok(created_panes);
        }

        // Check if this is a split node
        if let (Some(direction), Some(splits)) = (spec.get("direction"), spec.get("splits")) {
            let splits = splits
                .as_array()
                .ok_or_else(|| McpError::InvalidParams("'splits' must be an array".into()))?;

            let _direction_str = direction
                .as_str()
                .ok_or_else(|| McpError::InvalidParams("'direction' must be a string".into()))?;

            // Normalize ratios
            let mut ratios: Vec<f64> = splits
                .iter()
                .map(|s| s.get("ratio").and_then(|r| r.as_f64()).unwrap_or(1.0))
                .collect();

            let sum: f64 = ratios.iter().sum();
            if sum > 0.0 {
                for r in &mut ratios {
                    *r /= sum;
                }
            }

            // Recursively create panes for each split
            for (i, split) in splits.iter().enumerate() {
                let layout = split
                    .get("layout")
                    .ok_or_else(|| McpError::InvalidParams("Each split must have a 'layout'".into()))?;

                let mut child_panes = self.spawn_layout_panes(session_id, window_id, layout)?;

                // Add ratio information to first pane in this split branch
                if let Some(first_pane) = child_panes.first_mut() {
                    if let Some(obj) = first_pane.as_object_mut() {
                        obj.insert("ratio".to_string(), serde_json::json!(ratios[i]));
                    }
                }

                created_panes.extend(child_panes);
            }

            return Ok(created_panes);
        }

        Err(McpError::InvalidParams(
            "Layout must have either 'pane' or 'direction'+'splits'".into(),
        ))
    }

    /// Spawn a single pane from a pane specification
    fn spawn_single_pane(
        &mut self,
        session_id: Uuid,
        window_id: Uuid,
        pane_spec: &serde_json::Value,
    ) -> Result<serde_json::Value, McpError> {
        let command = pane_spec.get("command").and_then(|c| c.as_str());
        let cwd = pane_spec.get("cwd").and_then(|c| c.as_str());
        let name = pane_spec.get("name").and_then(|n| n.as_str());

        // Create the pane
        let session = self
            .session_manager
            .get_session_mut(session_id)
            .ok_or_else(|| McpError::Internal("Session disappeared".into()))?;
        let session_name = session.name().to_string();
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

        // Check if this is a Claude command
        let (actual_cmd, args, injected_session_id) = if is_claude_command(cmd, &[]) {
            let injection = inject_session_id(cmd, &[]);
            if injection.injected {
                let session_id = injection.session_id.clone().unwrap();
                tracing::info!(
                    "Injected session ID {} for Claude pane {}",
                    session_id,
                    pane_id
                );
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
        config = config.with_ccmux_context(session_id, &session_name, window_id, pane_id);

        self.pty_manager
            .spawn(pane_id, config)
            .map_err(|e| McpError::Pty(e.to_string()))?;

        let mut pane_info = serde_json::json!({
            "pane_id": pane_id.to_string(),
            "command": cmd,
        });

        if let Some(name) = name {
            pane_info["name"] = serde_json::json!(name);
        }

        if let Some(claude_session_id) = injected_session_id {
            pane_info["claude_session_id"] = serde_json::json!(claude_session_id);
        }

        Ok(pane_info)
    }

    // ==================== FEAT-062: Mirror Pane Tool ====================

    /// Create a read-only mirror pane that displays another pane's output
    pub fn mirror_pane(
        &mut self,
        source_pane_id: Uuid,
        direction: Option<&str>,
    ) -> Result<String, McpError> {
        // Parse direction, defaulting to vertical
        let split_direction = match direction {
            Some("horizontal") => ccmux_protocol::SplitDirection::Horizontal,
            _ => ccmux_protocol::SplitDirection::Vertical,
        };

        // Find the source pane and extract IDs
        let (session_id, window_id, session_name) = {
            let (session, window, _pane) = self
                .session_manager
                .find_pane(source_pane_id)
                .ok_or_else(|| McpError::PaneNotFound(source_pane_id.to_string()))?;
            (session.id(), window.id(), session.name().to_string())
        };

        // Create the mirror pane
        let session = self
            .session_manager
            .get_session_mut(session_id)
            .ok_or_else(|| McpError::SessionNotFound(session_id.to_string()))?;

        let window = session
            .get_window_mut(window_id)
            .ok_or_else(|| McpError::WindowNotFound(window_id.to_string()))?;

        let index = window.pane_count();
        let mirror_pane = crate::session::Pane::create_mirror(window_id, index, source_pane_id);
        let mirror_id = mirror_pane.id();

        window.add_pane(mirror_pane);

        // Register the mirror relationship
        self.session_manager
            .mirror_registry_mut()
            .register(source_pane_id, mirror_id);

        Ok(serde_json::to_string_pretty(&serde_json::json!({
            "mirror_pane_id": mirror_id.to_string(),
            "source_pane_id": source_pane_id.to_string(),
            "session_name": session_name,
            "direction": direction.unwrap_or("vertical"),
            "status": "created"
        }))
        .map_err(|e| McpError::Internal(e.to_string()))?)
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

        let result = ctx.create_window(None, None, None, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_create_window_in_default_session() {
        let mut session_manager = SessionManager::new();
        let mut pty_manager = PtyManager::new();

        session_manager.create_session("default").unwrap();

        let mut ctx = create_test_context(&mut session_manager, &mut pty_manager);
        let result = ctx.create_window(None, Some("new-window"), None, None).unwrap();

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
        let result = ctx.create_window(Some("target"), None, None, None).unwrap();

        assert!(result.contains("target"));
    }

    #[test]
    fn test_create_window_session_not_found() {
        let mut session_manager = SessionManager::new();
        let mut pty_manager = PtyManager::new();

        session_manager.create_session("existing").unwrap();

        let mut ctx = create_test_context(&mut session_manager, &mut pty_manager);
        let result = ctx.create_window(Some("nonexistent"), None, None, None);

        assert!(result.is_err());
    }

    // ==================== BUG-034: Active Session Selection Tests ====================

    #[test]
    fn test_bug034_create_window_uses_selected_session() {
        let mut session_manager = SessionManager::new();
        let mut pty_manager = PtyManager::new();

        // Create two sessions (session1 is created first)
        let session1 = session_manager.create_session("session1").unwrap();
        let session1_id = session1.id();

        std::thread::sleep(std::time::Duration::from_millis(10));

        session_manager.create_session("session2").unwrap();

        // Select session1 as active (even though session2 is more recent)
        session_manager.set_active_session(session1_id);

        let mut ctx = create_test_context(&mut session_manager, &mut pty_manager);

        // Create window without specifying session - should use active session (session1)
        let result = ctx.create_window(None, Some("test-window"), None, None).unwrap();

        // Verify the window was created in session1, not session2
        assert!(result.contains("session1"));
        assert!(!result.contains("session2"));
    }

    #[test]
    fn test_bug034_create_pane_uses_selected_session() {
        let mut session_manager = SessionManager::new();
        let mut pty_manager = PtyManager::new();

        // Create two sessions
        let session1 = session_manager.create_session("session1").unwrap();
        let session1_id = session1.id();

        // Add a window to session1
        {
            let session1_mut = session_manager.get_session_mut(session1_id).unwrap();
            session1_mut.create_window(None);
        }

        std::thread::sleep(std::time::Duration::from_millis(10));

        {
            let session2_id = session_manager.create_session("session2").unwrap().id();
            // Add a window to session2
            let session2_mut = session_manager.get_session_mut(session2_id).unwrap();
            session2_mut.create_window(None);
        }

        // Select session1 as active
        session_manager.set_active_session(session1_id);

        let mut ctx = create_test_context(&mut session_manager, &mut pty_manager);

        // Create pane without specifying session - should use active session (session1)
        let result = ctx.create_pane(None, None, None, None, None, None, false).unwrap();

        // Verify the pane was created in session1
        assert!(result.contains("session1"));
        assert!(!result.contains("session2"));
    }

    #[test]
    fn test_bug034_select_session_then_create_window() {
        let mut session_manager = SessionManager::new();
        let mut pty_manager = PtyManager::new();

        // Create two sessions
        let session1 = session_manager.create_session("dev-qa").unwrap();
        let session1_id = session1.id();

        std::thread::sleep(std::time::Duration::from_millis(10));

        session_manager.create_session("session-0").unwrap();

        let mut ctx = create_test_context(&mut session_manager, &mut pty_manager);

        // Simulate the MCP flow: select_session then create_window
        // This is the exact scenario from BUG-034
        let _select_result = ctx.select_session(session1_id).unwrap();

        // Now create_window should use dev-qa, not session-0
        let create_result = ctx.create_window(None, Some("new-window"), None, None).unwrap();

        // Verify window was created in dev-qa
        assert!(create_result.contains("dev-qa"));
        assert!(!create_result.contains("session-0"));
    }

    // ==================== Create Pane Direction Tests ====================

    #[test]
    fn test_create_pane_includes_direction_in_response() {
        let mut session_manager = SessionManager::new();
        let mut pty_manager = PtyManager::new();

        session_manager.create_session("test").unwrap();

        let mut ctx = create_test_context(&mut session_manager, &mut pty_manager);
        let result = ctx.create_pane(None, None, None, Some("horizontal"), None, None, false).unwrap();

        assert!(result.contains("\"direction\": \"horizontal\""));
    }

    #[test]
    fn test_create_pane_default_direction() {
        let mut session_manager = SessionManager::new();
        let mut pty_manager = PtyManager::new();

        session_manager.create_session("test").unwrap();

        let mut ctx = create_test_context(&mut session_manager, &mut pty_manager);
        let result = ctx.create_pane(None, None, None, None, None, None, false).unwrap();

        assert!(result.contains("\"direction\": \"vertical\""));
    }

    #[test]
    fn test_create_pane_with_session_filter() {
        let mut session_manager = SessionManager::new();
        let mut pty_manager = PtyManager::new();

        session_manager.create_session("session1").unwrap();
        session_manager.create_session("session2").unwrap();

        let mut ctx = create_test_context(&mut session_manager, &mut pty_manager);
        let result = ctx.create_pane(Some("session2"), None, None, None, None, None, false).unwrap();

        assert!(result.contains("session2"));
        assert!(result.contains("session_id"));
    }

    #[test]
    fn test_create_pane_session_not_found() {
        let mut session_manager = SessionManager::new();
        let mut pty_manager = PtyManager::new();

        session_manager.create_session("existing").unwrap();

        let mut ctx = create_test_context(&mut session_manager, &mut pty_manager);
        let result = ctx.create_pane(Some("nonexistent"), None, None, None, None, None, false);

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
        let result = ctx.create_pane(None, Some("window2"), None, None, None, None, false).unwrap();

        assert!(result.contains("pane_id"));
        assert!(result.contains("window_id"));
    }

    #[test]
    fn test_create_pane_window_not_found() {
        let mut session_manager = SessionManager::new();
        let mut pty_manager = PtyManager::new();

        session_manager.create_session("test").unwrap();

        let mut ctx = create_test_context(&mut session_manager, &mut pty_manager);
        let result = ctx.create_pane(None, Some("nonexistent"), None, None, None, None, false);

        assert!(result.is_err());
    }

    #[test]
    fn test_create_pane_response_includes_session_id() {
        let mut session_manager = SessionManager::new();
        let mut pty_manager = PtyManager::new();

        session_manager.create_session("test").unwrap();

        let mut ctx = create_test_context(&mut session_manager, &mut pty_manager);
        let result = ctx.create_pane(None, None, None, None, None, None, false).unwrap();

        assert!(result.contains("session_id"));
        assert!(result.contains("pane_id"));
        assert!(result.contains("window_id"));
    }

    // ==================== Split Pane Tests (FEAT-045) ====================

    #[test]
    fn test_split_pane_not_found() {
        let mut session_manager = SessionManager::new();
        let mut pty_manager = PtyManager::new();
        let mut ctx = create_test_context(&mut session_manager, &mut pty_manager);

        let result = ctx.split_pane(Uuid::new_v4(), None, None, None, None, false);
        assert!(matches!(result, Err(McpError::PaneNotFound(_))));
    }

    #[test]
    fn test_split_pane_creates_new_pane() {
        let mut session_manager = SessionManager::new();
        let mut pty_manager = PtyManager::new();

        // Create a session with a pane
        let session = session_manager.create_session("test").unwrap();
        let session_id = session.id();
        let session = session_manager.get_session_mut(session_id).unwrap();
        let window = session.create_window(None);
        let window_id = window.id();
        let window = session.get_window_mut(window_id).unwrap();
        let pane = window.create_pane();
        let pane_id = pane.id();

        let mut ctx = create_test_context(&mut session_manager, &mut pty_manager);
        let result = ctx.split_pane(pane_id, Some("horizontal"), Some(0.7), None, None, false).unwrap();

        assert!(result.contains("pane_id"));
        assert!(result.contains("original_pane_id"));
        assert!(result.contains("\"direction\": \"horizontal\""));
        assert!(result.contains("\"status\": \"split\""));
    }

    #[test]
    fn test_split_pane_default_direction() {
        let mut session_manager = SessionManager::new();
        let mut pty_manager = PtyManager::new();

        let session = session_manager.create_session("test").unwrap();
        let session_id = session.id();
        let session = session_manager.get_session_mut(session_id).unwrap();
        let window = session.create_window(None);
        let window_id = window.id();
        let window = session.get_window_mut(window_id).unwrap();
        let pane = window.create_pane();
        let pane_id = pane.id();

        let mut ctx = create_test_context(&mut session_manager, &mut pty_manager);
        let result = ctx.split_pane(pane_id, None, None, None, None, false).unwrap();

        assert!(result.contains("\"direction\": \"vertical\""));
    }

    // ==================== Resize Pane Tests (FEAT-045) ====================

    #[test]
    fn test_resize_pane_not_found() {
        let mut session_manager = SessionManager::new();
        let mut pty_manager = PtyManager::new();
        let mut ctx = create_test_context(&mut session_manager, &mut pty_manager);

        let result = ctx.resize_pane(Uuid::new_v4(), 0.1);
        assert!(matches!(result, Err(McpError::PaneNotFound(_))));
    }

    #[test]
    fn test_resize_pane_success() {
        let mut session_manager = SessionManager::new();
        let mut pty_manager = PtyManager::new();

        let session = session_manager.create_session("test").unwrap();
        let session_id = session.id();
        let session = session_manager.get_session_mut(session_id).unwrap();
        let window = session.create_window(None);
        let window_id = window.id();
        let window = session.get_window_mut(window_id).unwrap();
        let pane = window.create_pane();
        let pane_id = pane.id();

        let mut ctx = create_test_context(&mut session_manager, &mut pty_manager);
        let result = ctx.resize_pane(pane_id, 0.1).unwrap();

        assert!(result.contains("pane_id"));
        assert!(result.contains("\"status\": \"resize_requested\""));
    }

    #[test]
    fn test_resize_pane_clamps_delta() {
        let mut session_manager = SessionManager::new();
        let mut pty_manager = PtyManager::new();

        let session = session_manager.create_session("test").unwrap();
        let session_id = session.id();
        let session = session_manager.get_session_mut(session_id).unwrap();
        let window = session.create_window(None);
        let window_id = window.id();
        let window = session.get_window_mut(window_id).unwrap();
        let pane = window.create_pane();
        let pane_id = pane.id();

        let mut ctx = create_test_context(&mut session_manager, &mut pty_manager);

        // Test extreme positive delta gets clamped
        let result = ctx.resize_pane(pane_id, 1.0).unwrap();
        assert!(result.contains("0.5")); // Should be clamped to 0.5

        // Test extreme negative delta gets clamped
        let result = ctx.resize_pane(pane_id, -1.0).unwrap();
        assert!(result.contains("-0.5")); // Should be clamped to -0.5
    }

    // ==================== Create Layout Tests (FEAT-045) ====================

    #[test]
    fn test_create_layout_single_pane() {
        let mut session_manager = SessionManager::new();
        let mut pty_manager = PtyManager::new();
        let mut ctx = create_test_context(&mut session_manager, &mut pty_manager);

        let layout = serde_json::json!({
            "pane": {}
        });

        let result = ctx.create_layout(None, None, &layout).unwrap();

        assert!(result.contains("\"status\": \"created\""));
        assert!(result.contains("panes"));
    }

    #[test]
    fn test_create_layout_horizontal_split() {
        let mut session_manager = SessionManager::new();
        let mut pty_manager = PtyManager::new();
        let mut ctx = create_test_context(&mut session_manager, &mut pty_manager);

        let layout = serde_json::json!({
            "direction": "horizontal",
            "splits": [
                {"ratio": 0.5, "layout": {"pane": {"command": "bash"}}},
                {"ratio": 0.5, "layout": {"pane": {"command": "bash"}}}
            ]
        });

        let result = ctx.create_layout(None, None, &layout).unwrap();

        assert!(result.contains("\"status\": \"created\""));
        // Should have created 2 panes
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        let panes = parsed["panes"].as_array().unwrap();
        assert_eq!(panes.len(), 2);
    }

    #[test]
    fn test_create_layout_nested() {
        let mut session_manager = SessionManager::new();
        let mut pty_manager = PtyManager::new();
        let mut ctx = create_test_context(&mut session_manager, &mut pty_manager);

        // Layout: 65% left, 35% right (split into top/bottom)
        let layout = serde_json::json!({
            "direction": "horizontal",
            "splits": [
                {"ratio": 0.65, "layout": {"pane": {}}},
                {"ratio": 0.35, "layout": {
                    "direction": "vertical",
                    "splits": [
                        {"ratio": 0.5, "layout": {"pane": {}}},
                        {"ratio": 0.5, "layout": {"pane": {}}}
                    ]
                }}
            ]
        });

        let result = ctx.create_layout(None, None, &layout).unwrap();

        assert!(result.contains("\"status\": \"created\""));
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        let panes = parsed["panes"].as_array().unwrap();
        assert_eq!(panes.len(), 3);
    }

    #[test]
    fn test_create_layout_invalid_spec() {
        let mut session_manager = SessionManager::new();
        let mut pty_manager = PtyManager::new();
        let mut ctx = create_test_context(&mut session_manager, &mut pty_manager);

        // Invalid: neither pane nor direction+splits
        let layout = serde_json::json!({
            "invalid": "spec"
        });

        let result = ctx.create_layout(None, None, &layout);
        assert!(matches!(result, Err(McpError::InvalidParams(_))));
    }

    #[test]
    fn test_create_layout_normalizes_ratios() {
        let mut session_manager = SessionManager::new();
        let mut pty_manager = PtyManager::new();
        let mut ctx = create_test_context(&mut session_manager, &mut pty_manager);

        // Ratios don't sum to 1.0, should be normalized
        let layout = serde_json::json!({
            "direction": "horizontal",
            "splits": [
                {"ratio": 2.0, "layout": {"pane": {}}},
                {"ratio": 1.0, "layout": {"pane": {}}}
            ]
        });

        let result = ctx.create_layout(None, None, &layout);
        assert!(result.is_ok());
    }

    // ==================== Rename Session Tests ====================

    #[test]
    fn test_rename_session_not_found() {
        let mut session_manager = SessionManager::new();
        let mut pty_manager = PtyManager::new();
        let mut ctx = create_test_context(&mut session_manager, &mut pty_manager);

        let result = ctx.rename_session("nonexistent", "newname");
        assert!(matches!(result, Err(McpError::SessionNotFound(_))));
    }

    #[test]
    fn test_rename_session_by_name() {
        let mut session_manager = SessionManager::new();
        let mut pty_manager = PtyManager::new();

        session_manager.create_session("oldname").unwrap();

        let mut ctx = create_test_context(&mut session_manager, &mut pty_manager);
        let result = ctx.rename_session("oldname", "newname").unwrap();

        assert!(result.contains("\"name\": \"newname\""));
        assert!(result.contains("\"status\": \"renamed\""));
    }
}
