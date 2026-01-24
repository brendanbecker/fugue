use uuid::Uuid;
use fugue_protocol::{
    ClientMessage,
    ServerMessage,
    SplitDirection,
    PaneListEntry,
    OrchestrationTarget,
    OrchestrationMessage,
};
use crate::mcp::error::McpError;
use crate::mcp::protocol::ToolResult;
use super::connection::{ConnectionManager, MAX_RECONNECT_ATTEMPTS};
use crate::beads::metadata_keys as beads;
use super::health::ConnectionState;
use super::orchestration::{run_expect, ExpectAction, PipelineRunner, RunPipelineRequest};

/// Parse a UUID from arguments
pub fn parse_uuid(arguments: &serde_json::Value, field: &str) -> Result<Uuid, McpError> {
    let id_str = arguments[field]
    .as_str()
    .ok_or_else(|| McpError::InvalidParams(format!("Missing '{}' parameter", field)))?;

    Uuid::parse_str(id_str)
    .map_err(|e| McpError::InvalidParams(format!("Invalid UUID for '{}': {}", field, e)))
}

/// Format pane list for JSON output
pub fn format_pane_list(panes: &[PaneListEntry]) -> Vec<serde_json::Value> {
    panes
    .iter()
    .map(|p| {
    let state_str = match &p.state {
    fugue_protocol::PaneState::Normal => "normal",
    fugue_protocol::PaneState::Agent(state) => {
        if state.is_claude() { "claude" } else { "agent" }
    }
    fugue_protocol::PaneState::Exited { .. } => "exited",
    fugue_protocol::PaneState::Status => "status",
    };

    serde_json::json!({
    "id": p.id.to_string(),
    "session": p.session_name,
    "window": p.window_index,
    "window_name": p.window_name,
    "index": p.pane_index,
    "cols": p.cols,
    "rows": p.rows,
    "title": p.title,
    "cwd": p.cwd,
    "is_claude": p.is_claude,
    "claude_state": p.claude_state.as_ref().map(|cs| {
    serde_json::json!({
        "session_id": cs.session_id,
        "activity": format!("{:?}", cs.activity),
        "model": cs.model,
        "tokens_used": cs.tokens_used,
    })
    }),
    "state": state_str,
    })
    })
    .collect()
}

pub struct ToolHandlers<'a> {
    pub connection: &'a mut ConnectionManager,
}

impl<'a> ToolHandlers<'a> {
    pub fn new(connection: &'a mut ConnectionManager) -> Self {
    Self { connection }
    }

            // BUG-065 FIX: Use atomic send_and_recv to prevent response mismatches
            pub async fn tool_list_sessions(&mut self) -> Result<ToolResult, McpError> {
                match self.connection.send_and_recv(ClientMessage::ListSessions).await? {
                    ServerMessage::SessionList { sessions } => {
                        let result: Vec<serde_json::Value> = sessions
                            .iter()
                            .map(|s| {
                                serde_json::json!({
                                    "id": s.id.to_string(),
                                    "name": s.name,
                                    "window_count": s.window_count,
                                    "attached_clients": s.attached_clients,
                                    "created_at": s.created_at,
                                    "metadata": s.metadata,
                                })
                            })
                            .collect();
        
                        let json = serde_json::to_string_pretty(&result)
                            .map_err(|e| McpError::Internal(e.to_string()))?;
                        Ok(ToolResult::text(json))
                    }
                    ServerMessage::Error { code, message, .. } => {
                        Ok(ToolResult::error(format!("{:?}: {}", code, message)))
                    }
                    msg => Err(McpError::UnexpectedResponse(format!("{:?}", msg))),
                }
            }    // BUG-065 FIX: Use atomic send_and_recv to prevent response mismatches
    pub async fn tool_list_windows(
        &mut self,
        session_filter: Option<String>,
    ) -> Result<ToolResult, McpError> {
        match self.connection.send_and_recv(ClientMessage::ListWindows { session_filter }).await? {
            ServerMessage::WindowList {
                session_name,
                windows,
            } => {
                let result: Vec<serde_json::Value> = windows
                    .iter()
                    .map(|w| {
                        serde_json::json!({
                            "id": w.id.to_string(),
                            "index": w.index,
                            "name": w.name,
                            "pane_count": w.pane_count,
                            "active_pane_id": w.active_pane_id.map(|id| id.to_string()),
                            "session": session_name,
                        })
                    })
                    .collect();

                let json = serde_json::to_string_pretty(&result)
                    .map_err(|e| McpError::Internal(e.to_string()))?;
                Ok(ToolResult::text(json))
            }
            ServerMessage::Error { code, message, .. } => {
                Ok(ToolResult::error(format!("{:?}: {}", code, message)))
            }
            msg => Err(McpError::UnexpectedResponse(format!("{:?}", msg))),
        }
    }
    // BUG-065 FIX: Use atomic send_and_recv to prevent response mismatches
    pub async fn tool_list_panes(
    &mut self,
    session_filter: Option<String>,
    ) -> Result<ToolResult, McpError> {
    match self.connection.send_and_recv(ClientMessage::ListAllPanes { session_filter }).await? {
    ServerMessage::AllPanesList { panes } => {
    let result = format_pane_list(&panes);
    let json = serde_json::to_string_pretty(&result)
    .map_err(|e| McpError::Internal(e.to_string()))?;
    Ok(ToolResult::text(json))
    }
    ServerMessage::Error { code, message, .. } => {
    Ok(ToolResult::error(format!("{:?}: {}", code, message)))
    }
    msg => Err(McpError::UnexpectedResponse(format!("{:?}", msg))),
    }
    }

    // BUG-065 FIX: Use atomic send_and_recv to prevent response mismatches
    pub async fn tool_read_pane(
    &mut self,
    pane_id: Uuid,
    lines: usize,
    strip_escapes: bool,
    ) -> Result<ToolResult, McpError> {
    match self.connection.send_and_recv(ClientMessage::ReadPane { pane_id, lines }).await? {
    ServerMessage::PaneContent { content, .. } => {
        let output = if strip_escapes {
            let stripped = strip_ansi_escapes::strip(&content);
            String::from_utf8_lossy(&stripped).into_owned()
        } else {
            content
        };
        Ok(ToolResult::text(output))
    }
    ServerMessage::Error { code, message, .. } => {
    Ok(ToolResult::error(format!("{:?}: {}", code, message)))
    }
    msg => Err(McpError::UnexpectedResponse(format!("{:?}", msg))),
    }
    }

    // BUG-065 FIX: Use atomic send_and_recv to prevent response mismatches
    pub async fn tool_get_status(&mut self, pane_id: Uuid) -> Result<ToolResult, McpError> {
    match self.connection.send_and_recv(ClientMessage::GetPaneStatus { pane_id }).await? {
    ServerMessage::PaneStatus {
    pane_id,
    session_name,
    window_name,
    window_index,
    pane_index,
    cols,
    rows,
    title,
    cwd,
    state,
    has_pty,
    is_awaiting_input,
    is_awaiting_confirmation,
    } => {
    let state_json = match &state {
    fugue_protocol::PaneState::Normal => serde_json::json!({"type": "normal"}),
    fugue_protocol::PaneState::Agent(agent_state) => serde_json::json!({
        "type": if agent_state.is_claude() { "claude" } else { "agent" },
        "agent_type": agent_state.agent_type,
        "session_id": agent_state.session_id,
        "activity": format!("{:?}", agent_state.activity),
        "model": agent_state.get_metadata("model"),
        "tokens_used": agent_state.get_metadata("tokens_used"),
    }),
    fugue_protocol::PaneState::Exited { code } => serde_json::json!({
        "type": "exited",
        "exit_code": code,
    }),
    fugue_protocol::PaneState::Status => serde_json::json!({"type": "status"}),
    };

    let result = serde_json::json!({
    "pane_id": pane_id.to_string(),
    "session": session_name,
    "window": window_index,
    "window_name": window_name,
    "index": pane_index,
    "dimensions": {
        "cols": cols,
        "rows": rows,
    },
    "title": title,
    "cwd": cwd,
    "has_pty": has_pty,
    "state": state_json,
    "is_awaiting_input": is_awaiting_input,
    "is_awaiting_confirmation": is_awaiting_confirmation,
    });

    let json = serde_json::to_string_pretty(&result)
    .map_err(|e| McpError::Internal(e.to_string()))?;
    Ok(ToolResult::text(json))
    }
    ServerMessage::Error { code, message, .. } => {
    Ok(ToolResult::error(format!("{:?}: {}", code, message)))
    }
    msg => Err(McpError::UnexpectedResponse(format!("{:?}", msg))),
    }
    }

    // BUG-065 FIX: Use atomic send_and_recv to prevent response mismatches
    // FEAT-106: Added tags parameter for session creation
    pub async fn tool_create_session(
    &mut self,
    name: Option<String>,
    command: Option<String>,
    cwd: Option<String>,
    claude_model: Option<String>,
    claude_config: Option<serde_json::Value>,
    preset: Option<String>,
    tags: Vec<String>,
    ) -> Result<ToolResult, McpError> {
    match self.connection.send_and_recv(ClientMessage::CreateSessionWithOptions {
    name,
    command,
    cwd,
    claude_model,
    claude_config: claude_config.map(Into::into),
    preset,
    }).await? {
            ServerMessage::SessionCreatedWithDetails {
                session_id,
                session_name,
                window_id,
                pane_id,
                ..
            } => {
    // FEAT-106: Apply tags if provided
    let applied_tags = if !tags.is_empty() {
        match self.connection.send_and_recv(ClientMessage::SetTags {
            session_filter: Some(session_id.to_string()),
            add: tags,
            remove: vec![],
        }).await {
            Ok(ServerMessage::TagsSet { tags: set_tags, .. }) => {
                set_tags.into_iter().collect::<Vec<_>>()
            }
            Ok(_) | Err(_) => {
                // Tags failed to apply, but session was created - return empty tags
                vec![]
            }
        }
    } else {
        vec![]
    };

    let result = serde_json::json!({
    "session_id": session_id.to_string(),
    "session_name": session_name,
    "window_id": window_id.to_string(),
    "pane_id": pane_id.to_string(),
    "tags": applied_tags,
    "status": "created"
    });

    let json = serde_json::to_string_pretty(&result)
    .map_err(|e| McpError::Internal(e.to_string()))?;
    Ok(ToolResult::text(json))
    }
    ServerMessage::Error { code, message, .. } => {
    Ok(ToolResult::error(format!("{:?}: {}", code, message)))
    }
    msg => Err(McpError::UnexpectedResponse(format!("{:?}", msg))),
    }
    }

    // BUG-065 FIX: Use atomic send_and_recv to prevent response mismatches
    pub async fn tool_attach_session(&mut self, session_id: Uuid) -> Result<ToolResult, McpError> {
        match self.connection.send_and_recv(ClientMessage::AttachSession { session_id }).await? {
            ServerMessage::Attached { session, .. } => {
                let result = serde_json::json!({
                    "success": true,
                    "session_id": session.id.to_string(),
                    "session_name": session.name,
                    "status": "attached"
                });

                let json = serde_json::to_string_pretty(&result)
                    .map_err(|e| McpError::Internal(e.to_string()))?;
                Ok(ToolResult::text(json))
            }
            ServerMessage::Error { code, message, .. } => {
                Ok(ToolResult::error(format!("{:?}: {}", code, message)))
            }
            msg => Err(McpError::UnexpectedResponse(format!("{:?}", msg))),
        }
    }

    // BUG-065 FIX: Use atomic send_and_recv to prevent response mismatches
    pub async fn tool_create_window(
    &mut self,
    session_filter: Option<String>,
    name: Option<String>,
    command: Option<String>,
    cwd: Option<String>,
    ) -> Result<ToolResult, McpError> {
    match self.connection.send_and_recv(ClientMessage::CreateWindowWithOptions {
    session_filter,
    name,
    command,
    cwd,
    }).await? {
            ServerMessage::WindowCreatedWithDetails {
                window_id,
                pane_id,
                session_name,
                ..
            } => {
    let result = serde_json::json!({
    "window_id": window_id.to_string(),
    "pane_id": pane_id.to_string(),
    "session": session_name,
    "status": "created"
    });

    let json = serde_json::to_string_pretty(&result)
    .map_err(|e| McpError::Internal(e.to_string()))?;
    Ok(ToolResult::text(json))
    }
    ServerMessage::Error { code, message, .. } => {
    Ok(ToolResult::error(format!("{:?}: {}", code, message)))
    }
    msg => Err(McpError::UnexpectedResponse(format!("{:?}", msg))),
    }
    }

    // BUG-065 FIX: Use atomic send_and_recv to prevent response mismatches
    #[allow(clippy::too_many_arguments)]
    pub async fn tool_create_pane(
    &mut self,
    session: Option<String>,
    window: Option<String>,
    name: Option<String>,
    direction: Option<String>,
    command: Option<String>,
    cwd: Option<String>,
    select: bool,
    claude_model: Option<String>,
    claude_config: Option<serde_json::Value>,
    preset: Option<String>,
    ) -> Result<ToolResult, McpError> {
    let split_direction = match direction.as_deref() {
    Some("horizontal") | Some("h") => SplitDirection::Horizontal,
    _ => SplitDirection::Vertical,
    };

    let user_direction = match direction.as_deref() {
    Some("horizontal") | Some("h") => "horizontal",
    _ => "vertical",
    };

    match self.connection.send_and_recv(ClientMessage::CreatePaneWithOptions {
    session_filter: session,
    window_filter: window,
    direction: split_direction,
    command,
    cwd,
    select,
    name,
    claude_model,
    claude_config: claude_config.map(Into::into),
    preset,
    }).await? {
            ServerMessage::PaneCreatedWithDetails {
                pane_id,
                session_id,
                session_name,
                window_id,
                direction: _,
                ..
            } => {
    let result = serde_json::json!({
    "pane_id": pane_id.to_string(),
    "session_id": session_id.to_string(),
    "session": session_name,
    "window_id": window_id.to_string(),
    "direction": user_direction,
    "status": "created"
    });

    let json = serde_json::to_string_pretty(&result)
    .map_err(|e| McpError::Internal(e.to_string()))?;
    Ok(ToolResult::text(json))
    }
    ServerMessage::Error { code, message, .. } => {
    Ok(ToolResult::error(format!("{:?}: {}", code, message)))
    }
    msg => Err(McpError::UnexpectedResponse(format!("{:?}", msg))),
    }
    }

    /// Send input or a special key to a pane (FEAT-093)
    ///
    /// Either `input` or `key` must be provided (but not both).
    /// - `input`: Regular text input to send
    /// - `key`: Special key name (e.g., "Escape", "Ctrl+C", "ArrowUp")
    /// - `submit`: If true and using `input`, appends carriage return
    pub async fn tool_send_input(
        &mut self,
        pane_id: Uuid,
        input: Option<String>,
        key: Option<String>,
        submit: bool,
    ) -> Result<ToolResult, McpError> {
        // Determine data to send based on input or key parameter
        match (input, key) {
            (Some(text), None) => {
                // Regular text input - send input and Enter key separately if submit is true
                // This avoids issues with TUI apps that expect Enter as a separate event
                // (BUG-054)
                let data = text.as_bytes().to_vec();
                if !data.is_empty() {
                    self.connection
                        .send_to_daemon(ClientMessage::Input { pane_id, data })
                        .await?;
                }

                if submit {
                    // Larger delay to ensure TUI sees it as separate event (BUG-054)
                    // 50ms was not enough for some TUI apps (Gemini CLI, Claude Code)
                    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
                    let enter_data = b"\r".to_vec();
                    self.connection
                        .send_to_daemon(ClientMessage::Input {
                            pane_id,
                            data: enter_data,
                        })
                        .await?;
                }
            }
            (None, Some(key_name)) => {
                // Special key lookup
                use crate::mcp::keys::get_key_sequence;
                match get_key_sequence(&key_name) {
                    Some(sequence) => {
                        let data = sequence.to_vec();
                        self.connection
                            .send_to_daemon(ClientMessage::Input { pane_id, data })
                            .await?;
                    }
                    None => {
                        // Return error with helpful message listing some valid keys
                        return Ok(ToolResult::error(format!(
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
                // Both provided - error
                return Err(McpError::InvalidParams(
                    "Provide either 'input' or 'key', not both".into(),
                ));
            }
            (None, None) => {
                // Neither provided - error
                return Err(McpError::InvalidParams(
                    "Either 'input' or 'key' parameter is required".into(),
                ));
            }
        }

        Ok(ToolResult::text(r#"{"status": "sent"}"#.to_string()))
    }

    // BUG-065 FIX: Use atomic send_and_recv_filtered to prevent response mismatches
    pub async fn tool_close_pane(&mut self, pane_id: Uuid) -> Result<ToolResult, McpError> {
    match self.connection
    .send_and_recv_filtered(
        ClientMessage::ClosePane { pane_id },
        |msg| matches!(msg, ServerMessage::PaneClosed { pane_id: id, .. } if *id == pane_id),
    )
    .await?
    {
    ServerMessage::PaneClosed { pane_id: closed_id, .. } if closed_id == pane_id => {
    let result = serde_json::json!({
    "pane_id": pane_id.to_string(),
    "status": "closed"
    });

    let json = serde_json::to_string_pretty(&result)
    .map_err(|e| McpError::Internal(e.to_string()))?;
    Ok(ToolResult::text(json))
    }
    ServerMessage::Error { code, message, .. } => {
    Ok(ToolResult::error(format!("{:?}: {}", code, message)))
    }
    msg => Err(McpError::UnexpectedResponse(format!("{:?}", msg))),
    }
    }

    // BUG-065 FIX: Use atomic send_and_recv_filtered to prevent response mismatches
    pub async fn tool_focus_pane(&mut self, pane_id: Uuid) -> Result<ToolResult, McpError> {
    match self.connection
    .send_and_recv_filtered(
        ClientMessage::SelectPane { pane_id },
        |msg| matches!(msg, ServerMessage::PaneFocused { pane_id: id, .. } if *id == pane_id),
    )
    .await?
    {
    ServerMessage::PaneFocused { pane_id: focused_id, session_id, window_id } if focused_id == pane_id => {
    let result = serde_json::json!({
    "pane_id": pane_id.to_string(),
    "session_id": session_id.to_string(),
    "window_id": window_id.to_string(),
    "status": "focused"
    });

    let json = serde_json::to_string_pretty(&result)
    .map_err(|e| McpError::Internal(e.to_string()))?;
    Ok(ToolResult::text(json))
    }
    ServerMessage::Error { code, message, .. } => {
    Ok(ToolResult::error(format!("{:?}: {}", code, message)))
    }
    msg => Err(McpError::UnexpectedResponse(format!("{:?}", msg))),
    }
    }

    // BUG-065 FIX: Use atomic send_and_recv_filtered to prevent response mismatches
    pub async fn tool_select_window(&mut self, window_id: Uuid) -> Result<ToolResult, McpError> {
    match self.connection
    .send_and_recv_filtered(
        ClientMessage::SelectWindow { window_id },
        |msg| matches!(msg, ServerMessage::WindowFocused { window_id: id, .. } if *id == window_id),
    )
    .await?
    {
    ServerMessage::WindowFocused { window_id: focused_id, session_id } if focused_id == window_id => {
    let result = serde_json::json!({
    "window_id": window_id.to_string(),
    "session_id": session_id.to_string(),
    "status": "selected"
    });

    let json = serde_json::to_string_pretty(&result)
    .map_err(|e| McpError::Internal(e.to_string()))?;
    Ok(ToolResult::text(json))
    }
    ServerMessage::Error { code, message, .. } => {
    Ok(ToolResult::error(format!("{:?}: {}", code, message)))
    }
    msg => Err(McpError::UnexpectedResponse(format!("{:?}", msg))),
    }
    }

    // BUG-065 FIX: Use atomic send_and_recv_filtered to prevent response mismatches
    pub async fn tool_select_session(&mut self, session_id: Uuid) -> Result<ToolResult, McpError> {
    match self.connection
    .send_and_recv_filtered(
        ClientMessage::SelectSession { session_id },
        |msg| matches!(msg, ServerMessage::SessionFocused { session_id: id } if *id == session_id),
    )
    .await?
    {
    ServerMessage::SessionFocused { session_id: focused_id } if focused_id == session_id => {
    let result = serde_json::json!({
    "session_id": session_id.to_string(),
    "status": "selected"
    });

    let json = serde_json::to_string_pretty(&result)
    .map_err(|e| McpError::Internal(e.to_string()))?;
    Ok(ToolResult::text(json))
    }
    ServerMessage::Error { code, message, .. } => {
    Ok(ToolResult::error(format!("{:?}: {}", code, message)))
    }
    msg => Err(McpError::UnexpectedResponse(format!("{:?}", msg))),
    }
    }

    // BUG-065 FIX: Use atomic send_and_recv to prevent response mismatches
    pub async fn tool_rename_session(
    &mut self,
    session_filter: &str,
    new_name: &str,
    ) -> Result<ToolResult, McpError> {
    match self.connection.send_and_recv(ClientMessage::RenameSession {
    session_filter: session_filter.to_string(),
    new_name: new_name.to_string(),
    }).await? {
    ServerMessage::SessionRenamed {
    session_id,
    previous_name,
    new_name,
    } => {
    let result = serde_json::json!({
    "success": true,
    "session_id": session_id.to_string(),
    "previous_name": previous_name,
    "new_name": new_name
    });

    let json = serde_json::to_string_pretty(&result)
    .map_err(|e| McpError::Internal(e.to_string()))?;
    Ok(ToolResult::text(json))
    }
    ServerMessage::Error { code, message, .. } => {
    Ok(ToolResult::error(format!("{:?}: {}", code, message)))
    }
    msg => Err(McpError::UnexpectedResponse(format!("{:?}", msg))),
    }
    }

    // BUG-065 FIX: Use atomic send_and_recv to prevent response mismatches
    pub async fn tool_rename_pane(
    &mut self,
    pane_id: Uuid,
    new_name: &str,
    ) -> Result<ToolResult, McpError> {
    match self.connection.send_and_recv(ClientMessage::RenamPane {
    pane_id,
    new_name: new_name.to_string(),
    }).await? {
    ServerMessage::PaneRenamed {
    pane_id,
    previous_name,
    new_name,
    } => {
    let result = serde_json::json!({
    "success": true,
    "pane_id": pane_id.to_string(),
    "previous_name": previous_name,
    "new_name": new_name
    });

    let json = serde_json::to_string_pretty(&result)
    .map_err(|e| McpError::Internal(e.to_string()))?;
    Ok(ToolResult::text(json))
    }
    ServerMessage::Error { code, message, .. } => {
    Ok(ToolResult::error(format!("{:?}: {}", code, message)))
    }
    msg => Err(McpError::UnexpectedResponse(format!("{:?}", msg))),
    }
    }

    // BUG-065 FIX: Use atomic send_and_recv to prevent response mismatches
    pub async fn tool_rename_window(
    &mut self,
    window_id: Uuid,
    new_name: &str,
    ) -> Result<ToolResult, McpError> {
    match self.connection.send_and_recv(ClientMessage::RenameWindow {
    window_id,
    new_name: new_name.to_string(),
    }).await? {
    ServerMessage::WindowRenamed {
    window_id,
    previous_name,
    new_name,
    } => {
    let result = serde_json::json!({
    "success": true,
    "window_id": window_id.to_string(),
    "previous_name": previous_name,
    "new_name": new_name
    });

    let json = serde_json::to_string_pretty(&result)
    .map_err(|e| McpError::Internal(e.to_string()))?;
    Ok(ToolResult::text(json))
    }
    ServerMessage::Error { code, message, .. } => {
    Ok(ToolResult::error(format!("{:?}: {}", code, message)))
    }
    msg => Err(McpError::UnexpectedResponse(format!("{:?}", msg))),
    }
    }

    // BUG-065 FIX: Use atomic send_and_recv to prevent response mismatches
    pub async fn tool_split_pane(
    &mut self,
    pane_id: Uuid,
    direction: Option<String>,
    ratio: f32,
    command: Option<String>,
    cwd: Option<String>,
    select: bool,
    ) -> Result<ToolResult, McpError> {
    let split_direction = match direction.as_deref() {
    Some("horizontal") | Some("h") => SplitDirection::Horizontal,
    _ => SplitDirection::Vertical,
    };

    let user_direction = match direction.as_deref() {
    Some("horizontal") | Some("h") => "horizontal",
    _ => "vertical",
    };

    match self.connection.send_and_recv(ClientMessage::SplitPane {
    pane_id,
    direction: split_direction,
    ratio,
    command,
    cwd,
    select,
    }).await? {
            ServerMessage::PaneSplit {
                new_pane_id,
                original_pane_id,
                session_id,
                session_name,
                window_id,
                direction: _,
                ..
            } => {
    let result = serde_json::json!({
    "new_pane_id": new_pane_id.to_string(),
    "original_pane_id": original_pane_id.to_string(),
    "session_id": session_id.to_string(),
    "session": session_name,
    "window_id": window_id.to_string(),
    "direction": user_direction,
    });

    let json = serde_json::to_string_pretty(&result)
    .map_err(|e| McpError::Internal(e.to_string()))?;
    Ok(ToolResult::text(json))
    }
    ServerMessage::Error { code, message, .. } => {
    Ok(ToolResult::error(format!("{:?}: {}", code, message)))
    }
    msg => Err(McpError::UnexpectedResponse(format!("{:?}", msg))),
    }
    }

    // BUG-065 FIX: Use atomic send_and_recv to prevent response mismatches
    pub async fn tool_resize_pane(
    &mut self,
    pane_id: Uuid,
    delta: f32,
    ) -> Result<ToolResult, McpError> {
    match self.connection.send_and_recv(ClientMessage::ResizePaneDelta { pane_id, delta }).await? {
    ServerMessage::PaneResized {
    pane_id,
    new_cols,
    new_rows,
    } => {
    let result = serde_json::json!({
    "pane_id": pane_id.to_string(),
    "new_cols": new_cols,
    "new_rows": new_rows,
    });

    let json = serde_json::to_string_pretty(&result)
    .map_err(|e| McpError::Internal(e.to_string()))?;
    Ok(ToolResult::text(json))
    }
    ServerMessage::Error { code, message, .. } => {
    Ok(ToolResult::error(format!("{:?}: {}", code, message)))
    }
    msg => Err(McpError::UnexpectedResponse(format!("{:?}", msg))),
    }
    }

    // BUG-065 FIX: Use atomic send_and_recv to prevent response mismatches
    pub async fn tool_create_layout(
    &mut self,
    session: Option<String>,
    window: Option<String>,
    layout: serde_json::Value,
    ) -> Result<ToolResult, McpError> {
    match self.connection.send_and_recv(ClientMessage::CreateLayout {
    session_filter: session,
    window_filter: window,
    layout: layout.into(),
    }).await? {
    ServerMessage::LayoutCreated {
    session_id,
    session_name,
    window_id,
    pane_ids,
    } => {
    let result = serde_json::json!({
    "session_id": session_id.to_string(),
    "session": session_name,
    "window_id": window_id.to_string(),
    "pane_ids": pane_ids.iter().map(|id| id.to_string()).collect::<Vec<_>>(),
    "pane_count": pane_ids.len(),
    });

    let json = serde_json::to_string_pretty(&result)
    .map_err(|e| McpError::Internal(e.to_string()))?;
    Ok(ToolResult::text(json))
    }
    ServerMessage::Error { code, message, .. } => {
    Ok(ToolResult::error(format!("{:?}: {}", code, message)))
    }
    msg => Err(McpError::UnexpectedResponse(format!("{:?}", msg))),
    }
    }

    // BUG-065 FIX: Use atomic send_and_recv to prevent response mismatches
    pub async fn tool_kill_session(&mut self, session_filter: &str) -> Result<ToolResult, McpError> {
    let session_id = if let Ok(uuid) = Uuid::parse_str(session_filter) {
    uuid
    } else {
    match self.connection.send_and_recv(ClientMessage::ListSessions).await? {
    ServerMessage::SessionList { sessions } => {
    sessions
        .iter()
        .find(|s| s.name == session_filter)
        .map(|s| s.id)
        .ok_or_else(|| {
        McpError::InvalidParams(format!(
        "Session '{}' not found",
        session_filter
        ))
        })?
    }
    ServerMessage::Error { code, message, .. } => {
    return Ok(ToolResult::error(format!("{:?}: {}", code, message)));
    }
    msg => return Err(McpError::UnexpectedResponse(format!("{:?}", msg))),
    }
    };

    match self.connection.send_and_recv(ClientMessage::DestroySession { session_id }).await? {
    ServerMessage::SessionDestroyed {
    session_id,
    session_name,
    } => {
    let result = serde_json::json!({
    "success": true,
    "message": "Session killed",
    "session_id": session_id.to_string(),
    "session_name": session_name,
    });

    let json = serde_json::to_string_pretty(&result)
    .map_err(|e| McpError::Internal(e.to_string()))?;
    Ok(ToolResult::text(json))
    }
    ServerMessage::Error { code, message, .. } => {
    Ok(ToolResult::error(format!("{:?}: {}", code, message)))
    }
    msg => Err(McpError::UnexpectedResponse(format!("{:?}", msg))),
    }
    }

    // BUG-065 FIX: Use atomic send_and_recv to prevent response mismatches
    pub async fn tool_set_environment(
    &mut self,
    session_filter: &str,
    key: &str,
    value: &str,
    ) -> Result<ToolResult, McpError> {
    match self.connection.send_and_recv(ClientMessage::SetEnvironment {
    session_filter: session_filter.to_string(),
    key: key.to_string(),
    value: value.to_string(),
    }).await? {
    ServerMessage::EnvironmentSet {
    session_id,
    session_name,
    key,
    value,
    } => {
    let result = serde_json::json!({
    "success": true,
    "session_id": session_id.to_string(),
    "session_name": session_name,
    "key": key,
    "value": value,
    });

    let json = serde_json::to_string_pretty(&result)
    .map_err(|e| McpError::Internal(e.to_string()))?;
    Ok(ToolResult::text(json))
    }
    ServerMessage::Error { code, message, .. } => {
    Ok(ToolResult::error(format!("{:?}: {}", code, message)))
    }
    msg => Err(McpError::UnexpectedResponse(format!("{:?}", msg))),
    }
    }

    // BUG-065 FIX: Use atomic send_and_recv to prevent response mismatches
    pub async fn tool_get_environment(
    &mut self,
    session_filter: &str,
    key: Option<String>,
    ) -> Result<ToolResult, McpError> {
    match self.connection.send_and_recv(ClientMessage::GetEnvironment {
    session_filter: session_filter.to_string(),
    key,
    }).await? {
    ServerMessage::EnvironmentList {
    session_id,
    session_name,
    environment,
    } => {
    let result = serde_json::json!({
    "session_id": session_id.to_string(),
    "session_name": session_name,
    "environment": environment,
    });

    let json = serde_json::to_string_pretty(&result)
    .map_err(|e| McpError::Internal(e.to_string()))?;
    Ok(ToolResult::text(json))
    }
    ServerMessage::Error { code, message, .. } => {
    Ok(ToolResult::error(format!("{:?}: {}", code, message)))
    }
    msg => Err(McpError::UnexpectedResponse(format!("{:?}", msg))),
    }
    }

    // BUG-065 FIX: Use atomic send_and_recv to prevent response mismatches
    pub async fn tool_set_metadata(
    &mut self,
    session_filter: &str,
    key: &str,
    value: &str,
    ) -> Result<ToolResult, McpError> {
    match self.connection.send_and_recv(ClientMessage::SetMetadata {
    session_filter: session_filter.to_string(),
    key: key.to_string(),
    value: value.to_string(),
    }).await? {
    ServerMessage::MetadataSet {
    session_id,
    session_name,
    key,
    value,
    } => {
    let result = serde_json::json!({
    "success": true,
    "session_id": session_id.to_string(),
    "session_name": session_name,
    "key": key,
    "value": value,
    });

    let json = serde_json::to_string_pretty(&result)
    .map_err(|e| McpError::Internal(e.to_string()))?;
    Ok(ToolResult::text(json))
    }
    ServerMessage::Error { code, message, .. } => {
    Ok(ToolResult::error(format!("{:?}: {}", code, message)))
    }
    msg => Err(McpError::UnexpectedResponse(format!("{:?}", msg))),
    }
    }

    // BUG-065 FIX: Use atomic send_and_recv to prevent response mismatches
    pub async fn tool_get_metadata(
    &mut self,
    session_filter: &str,
    key: Option<String>,
    ) -> Result<ToolResult, McpError> {
    match self.connection.send_and_recv(ClientMessage::GetMetadata {
    session_filter: session_filter.to_string(),
    key,
    }).await? {
    ServerMessage::MetadataList {
    session_id,
    session_name,
    metadata,
    } => {
    let result = serde_json::json!({
    "session_id": session_id.to_string(),
    "session_name": session_name,
    "metadata": metadata,
    });

    let json = serde_json::to_string_pretty(&result)
    .map_err(|e| McpError::Internal(e.to_string()))?;
    Ok(ToolResult::text(json))
    }
    ServerMessage::Error { code, message, .. } => {
    Ok(ToolResult::error(format!("{:?}: {}", code, message)))
    }
    msg => Err(McpError::UnexpectedResponse(format!("{:?}", msg))),
    }
    }

    pub async fn tool_send_orchestration(
    &mut self,
    target: &serde_json::Value,
    msg_type: &str,
    payload: serde_json::Value,
    ) -> Result<ToolResult, McpError> {
    let orchestration_target = if let Some(tag) = target.get("tag").and_then(|v| v.as_str()) {
    OrchestrationTarget::Tagged(tag.to_string())
    } else if let Some(session) = target.get("session").and_then(|v| v.as_str()) {
    let session_id = Uuid::parse_str(session)
    .map_err(|e| McpError::InvalidParams(format!("Invalid session UUID: {}", e)))?;
    OrchestrationTarget::Session(session_id)
    } else if target.get("broadcast").and_then(|v| v.as_bool()).unwrap_or(false) {
    OrchestrationTarget::Broadcast
    } else if let Some(worktree) = target.get("worktree").and_then(|v| v.as_str()) {
    OrchestrationTarget::Worktree(worktree.to_string())
    } else {
    return Err(McpError::InvalidParams(
    "Invalid target: must specify 'tag', 'session', 'broadcast', or 'worktree'".into(),
    ));
    };

    let message = OrchestrationMessage::new(msg_type, payload);

    // BUG-065 FIX: Use atomic send_and_recv to prevent response mismatches
    match self.connection.send_and_recv(ClientMessage::SendOrchestration {
    target: orchestration_target,
    message,
    }).await? {
    ServerMessage::OrchestrationDelivered { delivered_count } => {
    let result = serde_json::json!({
    "success": true,
    "delivered_count": delivered_count,
    });

    let json = serde_json::to_string_pretty(&result)
    .map_err(|e| McpError::Internal(e.to_string()))?;
    Ok(ToolResult::text(json))
    }
    ServerMessage::Error { code, message, .. } => {
    Ok(ToolResult::error(format!("{:?}: {}", code, message)))
    }
    msg => Err(McpError::UnexpectedResponse(format!("{:?}", msg))),
    }
    }

    // BUG-065 FIX: Use atomic send_and_recv to prevent response mismatches
    pub async fn tool_set_tags(
    &mut self,
    session_filter: Option<String>,
    add: Vec<String>,
    remove: Vec<String>,
    ) -> Result<ToolResult, McpError> {
    match self.connection.send_and_recv(ClientMessage::SetTags {
    session_filter,
    add,
    remove,
    }).await? {
    ServerMessage::TagsSet {
    session_id,
    session_name,
    tags,
    } => {
    let result = serde_json::json!({
    "success": true,
    "session_id": session_id.to_string(),
    "session_name": session_name,
    "tags": tags.into_iter().collect::<Vec<_>>(),
    });

    let json = serde_json::to_string_pretty(&result)
    .map_err(|e| McpError::Internal(e.to_string()))?;
    Ok(ToolResult::text(json))
    }
    ServerMessage::Error { code, message, .. } => {
    Ok(ToolResult::error(format!("{:?}: {}", code, message)))
    }
    msg => Err(McpError::UnexpectedResponse(format!("{:?}", msg))),
    }
    }

    // BUG-065 FIX: Use atomic send_and_recv to prevent response mismatches
    pub async fn tool_get_tags(
    &mut self,
    session_filter: Option<String>,
    ) -> Result<ToolResult, McpError> {
    match self.connection.send_and_recv(ClientMessage::GetTags { session_filter }).await? {
    ServerMessage::TagsList {
    session_id,
    session_name,
    tags,
    } => {
    let result = serde_json::json!({
    "session_id": session_id.to_string(),
    "session_name": session_name,
    "tags": tags.into_iter().collect::<Vec<_>>(),
    });

    let json = serde_json::to_string_pretty(&result)
    .map_err(|e| McpError::Internal(e.to_string()))?;
    Ok(ToolResult::text(json))
    }
    ServerMessage::Error { code, message, .. } => {
    Ok(ToolResult::error(format!("{:?}: {}", code, message)))
    }
    msg => Err(McpError::UnexpectedResponse(format!("{:?}", msg))),
    }
    }

    // BUG-065 FIX: Use atomic send_and_recv to prevent response mismatches
    pub async fn tool_report_status(
    &mut self,
    status: &str,
    message: Option<String>,
    ) -> Result<ToolResult, McpError> {
    let current_issue_id = self.get_current_issue_id().await;

    let target = OrchestrationTarget::Tagged("orchestrator".to_string());
    let payload = serde_json::json!({
    "status": status,
    "message": message,
    "issue_id": current_issue_id,
    });
    let msg = OrchestrationMessage::new("status.update", payload);

    match self.connection.send_and_recv(ClientMessage::SendOrchestration {
    target,
    message: msg,
    }).await? {
    ServerMessage::OrchestrationDelivered { delivered_count } => {
    let result = serde_json::json!({
    "success": true,
    "delivered_count": delivered_count,
    "status": status,
    "issue_id": current_issue_id,
    });

    let json = serde_json::to_string_pretty(&result)
    .map_err(|e| McpError::Internal(e.to_string()))?;
    Ok(ToolResult::text(json))
    }
    ServerMessage::Error { code, message, .. } => {
    Ok(ToolResult::error(format!("{:?}: {}", code, message)))
    }
    msg => Err(McpError::UnexpectedResponse(format!("{:?}", msg))),
    }
    }

    // BUG-065 FIX: Use atomic send_and_recv to prevent response mismatches
    async fn get_current_issue_id(&mut self) -> Option<String> {
    let sessions = match self.connection.send_and_recv(ClientMessage::ListSessions).await {
    Ok(ServerMessage::SessionList { sessions }) => sessions,
    _ => return None,
    };

    if sessions.is_empty() {
    return None;
    }

    let session_name = &sessions[0].name;

    match self.connection.send_and_recv(ClientMessage::GetMetadata {
    session_filter: session_name.clone(),
    key: Some(beads::CURRENT_ISSUE.to_string()),
    }).await {
    Ok(ServerMessage::MetadataList { metadata, .. }) => {
    metadata
    .get(beads::CURRENT_ISSUE)
    .cloned()
    .filter(|s| !s.is_empty())
    }
    _ => None,
    }
    }

    // BUG-065 FIX: Use atomic send_and_recv to prevent response mismatches
    pub async fn tool_request_help(&mut self, context: &str) -> Result<ToolResult, McpError> {
    let target = OrchestrationTarget::Tagged("orchestrator".to_string());
    let payload = serde_json::json!({
    "context": context,
    });
    let msg = OrchestrationMessage::new("help.request", payload);

    match self.connection.send_and_recv(ClientMessage::SendOrchestration {
    target,
    message: msg,
    }).await? {
    ServerMessage::OrchestrationDelivered { delivered_count } => {
    let result = serde_json::json!({
    "success": true,
    "delivered_count": delivered_count,
    "type": "help.request",
    });

    let json = serde_json::to_string_pretty(&result)
    .map_err(|e| McpError::Internal(e.to_string()))?;
    Ok(ToolResult::text(json))
    }
    ServerMessage::Error { code, message, .. } => {
    Ok(ToolResult::error(format!("{:?}: {}", code, message)))
    }
    msg => Err(McpError::UnexpectedResponse(format!("{:?}", msg))),
    }
    }

    // BUG-065 FIX: Use atomic send_and_recv to prevent response mismatches
    pub async fn tool_broadcast(
    &mut self,
    msg_type: &str,
    payload: serde_json::Value,
    ) -> Result<ToolResult, McpError> {
    let target = OrchestrationTarget::Broadcast;
    let msg = OrchestrationMessage::new(msg_type, payload);

    match self.connection.send_and_recv(ClientMessage::SendOrchestration {
    target,
    message: msg,
    }).await? {
    ServerMessage::OrchestrationDelivered { delivered_count } => {
    let result = serde_json::json!({
    "success": true,
    "delivered_count": delivered_count,
    "type": msg_type,
    });

    let json = serde_json::to_string_pretty(&result)
    .map_err(|e| McpError::Internal(e.to_string()))?;
    Ok(ToolResult::text(json))
    }
    ServerMessage::Error { code, message, .. } => {
    Ok(ToolResult::error(format!("{:?}: {}", code, message)))
    }
    msg => Err(McpError::UnexpectedResponse(format!("{:?}", msg))),
    }
    }

    pub async fn tool_connection_status(&self) -> Result<ToolResult, McpError> {
    let state = *self.connection.connection_state.read().await;

    let result = match state {
    ConnectionState::Connected => serde_json::json!({
    "status": "connected",
    "healthy": true,
    "daemon_responsive": true
    }),
    ConnectionState::Reconnecting { attempt } => serde_json::json!({
    "status": "reconnecting",
    "healthy": false,
    "reconnect_attempt": attempt,
    "max_attempts": MAX_RECONNECT_ATTEMPTS
    }),
    ConnectionState::Disconnected => serde_json::json!({
    "status": "disconnected",
    "healthy": false,
    "recoverable": true,
    "action": "Tool calls will trigger automatic reconnection"
    }),
    };

    let json = serde_json::to_string_pretty(&result)
    .map_err(|e| McpError::Internal(e.to_string()))?;
    Ok(ToolResult::text(json))
    }

    // BUG-065 FIX: Use atomic send_and_recv to prevent response mismatches
    async fn resolve_session_for_pane(
    &mut self,
    pane_id: Option<Uuid>,
    ) -> Result<(String, Option<Uuid>), McpError> {
    match pane_id {
    Some(id) => {
    match self.connection.send_and_recv(ClientMessage::GetPaneStatus { pane_id: id }).await? {
    ServerMessage::PaneStatus {
        pane_id,
        session_name,
        ..
    } => Ok((session_name, Some(pane_id))),
    ServerMessage::Error { code, message, .. } => {
        Err(McpError::InvalidParams(format!("{:?}: {}", code, message)))
    }
    msg => Err(McpError::UnexpectedResponse(format!("{:?}", msg))),
    }
    }
    None => {
    match self.connection.send_and_recv(ClientMessage::ListSessions).await? {
    ServerMessage::SessionList { sessions } => {
        if sessions.is_empty() {
        Err(McpError::InvalidParams("No sessions available".into()))
        } else {
        Ok((sessions[0].name.clone(), None))
        }
    }
    ServerMessage::Error { code, message, .. } => {
        Err(McpError::InvalidParams(format!("{:?}: {}", code, message)))
    }
    msg => Err(McpError::UnexpectedResponse(format!("{:?}", msg))),
    }
    }
    }
    }

    // BUG-065 FIX: Use atomic send_and_recv to prevent response mismatches
    pub async fn tool_beads_assign(
    &mut self,
    issue_id: &str,
    pane_id: Option<Uuid>,
    ) -> Result<ToolResult, McpError> {
    let (session_name, resolved_pane_id) = self.resolve_session_for_pane(pane_id).await?;

    let timestamp = {
    use std::time::{SystemTime, UNIX_EPOCH};
    let duration = SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .unwrap_or_default();
    format!("{}", duration.as_secs())
    };

    match self.connection.send_and_recv(ClientMessage::SetMetadata {
    session_filter: session_name.clone(),
    key: beads::CURRENT_ISSUE.to_string(),
    value: issue_id.to_string(),
    }).await? {
    ServerMessage::MetadataSet { .. } => {}
    ServerMessage::Error { code, message, .. } => {
    return Ok(ToolResult::error(format!("{:?}: {}", code, message)));
    }
    msg => return Err(McpError::UnexpectedResponse(format!("{:?}", msg))),
    }

    match self.connection.send_and_recv(ClientMessage::SetMetadata {
    session_filter: session_name.clone(),
    key: beads::ASSIGNED_AT.to_string(),
    value: timestamp.clone(),
    }).await? {
    ServerMessage::MetadataSet {
    session_id,
    session_name,
    ..
    } => {
    let result = serde_json::json!({
    "success": true,
    "session_id": session_id.to_string(),
    "session_name": session_name,
    "pane_id": resolved_pane_id.map(|id| id.to_string()),
    "issue_id": issue_id,
    "assigned_at": timestamp,
    });

    let json = serde_json::to_string_pretty(&result)
    .map_err(|e| McpError::Internal(e.to_string()))?;
    Ok(ToolResult::text(json))
    }
    ServerMessage::Error { code, message, .. } => {
    Ok(ToolResult::error(format!("{:?}: {}", code, message)))
    }
    msg => Err(McpError::UnexpectedResponse(format!("{:?}", msg))),
    }
    }

    // BUG-065 FIX: Use atomic send_and_recv to prevent response mismatches
    pub async fn tool_beads_release(
    &mut self,
    pane_id: Option<Uuid>,
    outcome: Option<String>,
    ) -> Result<ToolResult, McpError> {
    let (session_name, resolved_pane_id) = self.resolve_session_for_pane(pane_id).await?;
    let outcome = outcome.unwrap_or_else(|| "completed".to_string());

    let current_issue = match self.connection.send_and_recv(ClientMessage::GetMetadata {
    session_filter: session_name.clone(),
    key: Some(beads::CURRENT_ISSUE.to_string()),
    }).await? {
    ServerMessage::MetadataList { metadata, .. } => {
    metadata.get(beads::CURRENT_ISSUE).cloned()
    }
    ServerMessage::Error { code, message, .. } => {
    return Ok(ToolResult::error(format!("{:?}: {}", code, message)));
    }
    msg => return Err(McpError::UnexpectedResponse(format!("{:?}", msg))),
    };

    let current_issue = match current_issue {
    Some(issue) => issue,
    None => {
    return Ok(ToolResult::error("No issue currently assigned".to_string()));
    }
    };

    let assigned_at = match self.connection.send_and_recv(ClientMessage::GetMetadata {
    session_filter: session_name.clone(),
    key: Some(beads::ASSIGNED_AT.to_string()),
    }).await? {
    ServerMessage::MetadataList { metadata, .. } => {
    metadata.get(beads::ASSIGNED_AT).cloned().unwrap_or_default()
    }
    ServerMessage::Error { .. } => String::new(),
    _ => String::new(),
    };

    let existing_history = match self.connection.send_and_recv(ClientMessage::GetMetadata {
    session_filter: session_name.clone(),
    key: Some(beads::ISSUE_HISTORY.to_string()),
    }).await? {
    ServerMessage::MetadataList { metadata, .. } => {
    metadata.get(beads::ISSUE_HISTORY).cloned()
    }
    _ => None,
    };

    let mut history: Vec<serde_json::Value> = existing_history
    .and_then(|h| serde_json::from_str(&h).ok())
    .unwrap_or_default();

    let released_at = {
    use std::time::{SystemTime, UNIX_EPOCH};
    let duration = SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .unwrap_or_default();
    format!("{}", duration.as_secs())
    };

    history.push(serde_json::json!({
    "issue_id": current_issue,
    "assigned_at": assigned_at,
    "released_at": released_at,
    "outcome": outcome,
    }));

    let history_json = serde_json::to_string(&history)
    .map_err(|e| McpError::Internal(e.to_string()))?;

    match self.connection.send_and_recv(ClientMessage::SetMetadata {
    session_filter: session_name.clone(),
    key: beads::ISSUE_HISTORY.to_string(),
    value: history_json,
    }).await? {
    ServerMessage::MetadataSet { .. } => {}
    ServerMessage::Error { code, message, .. } => {
    return Ok(ToolResult::error(format!("{:?}: {}", code, message)));
    }
    msg => return Err(McpError::UnexpectedResponse(format!("{:?}", msg))),
    }

    match self.connection.send_and_recv(ClientMessage::SetMetadata {
    session_filter: session_name.clone(),
    key: beads::CURRENT_ISSUE.to_string(),
    value: String::new(),
    }).await? {
    ServerMessage::MetadataSet { .. } => {}
    ServerMessage::Error { code, message, .. } => {
    return Ok(ToolResult::error(format!("{:?}: {}", code, message)));
    }
    msg => return Err(McpError::UnexpectedResponse(format!("{:?}", msg))),
    }

    match self.connection.send_and_recv(ClientMessage::SetMetadata {
    session_filter: session_name.clone(),
    key: beads::ASSIGNED_AT.to_string(),
    value: String::new(),
    }).await? {
    ServerMessage::MetadataSet {
    session_id,
    session_name,
    ..
    } => {
    let result = serde_json::json!({
    "success": true,
    "session_id": session_id.to_string(),
    "session_name": session_name,
    "pane_id": resolved_pane_id.map(|id| id.to_string()),
    "released_issue": current_issue,
    "outcome": outcome,
    "released_at": released_at,
    });

    let json = serde_json::to_string_pretty(&result)
    .map_err(|e| McpError::Internal(e.to_string()))?;
    Ok(ToolResult::text(json))
    }
    ServerMessage::Error { code, message, .. } => {
    Ok(ToolResult::error(format!("{:?}: {}", code, message)))
    }
    msg => Err(McpError::UnexpectedResponse(format!("{:?}", msg))),
    }
    }

    // BUG-065 FIX: Use atomic send_and_recv to prevent response mismatches
    pub async fn tool_beads_find_pane(&mut self, issue_id: &str) -> Result<ToolResult, McpError> {
    let sessions = match self.connection.send_and_recv(ClientMessage::ListSessions).await? {
    ServerMessage::SessionList { sessions } => sessions,
    ServerMessage::Error { code, message, .. } => {
    return Ok(ToolResult::error(format!("{:?}: {}", code, message)));
    }
    msg => return Err(McpError::UnexpectedResponse(format!("{:?}", msg))),
    }
    .into_iter();

    for session in sessions {
    if let ServerMessage::MetadataList {
    session_id,
    session_name,
    metadata,
    } = self.connection.send_and_recv(ClientMessage::GetMetadata {
    session_filter: session.id.to_string(),
    key: Some(beads::CURRENT_ISSUE.to_string()),
    }).await? {
    if let Some(current_issue) = metadata.get(beads::CURRENT_ISSUE) {
    if current_issue == issue_id {
        let pane_id = match self.connection.send_and_recv(ClientMessage::ListAllPanes {
        session_filter: Some(session_id.to_string()),
        }).await? {
        ServerMessage::AllPanesList { panes } => {
        panes.first().map(|p| p.id)
        }
        _ => None,
        };

        let result = serde_json::json!({
        "found": true,
        "session_id": session_id.to_string(),
        "session_name": session_name,
        "pane_id": pane_id.map(|id| id.to_string()),
        "issue_id": issue_id,
        });

        let json = serde_json::to_string_pretty(&result)
        .map_err(|e| McpError::Internal(e.to_string()))?;
        return Ok(ToolResult::text(json));
    }
    }
    }
    }

    let result = serde_json::json!({
    "found": false,
    "issue_id": issue_id,
    "message": "No pane is currently working on this issue",
    });

    let json = serde_json::to_string_pretty(&result)
    .map_err(|e| McpError::Internal(e.to_string()))?;
    Ok(ToolResult::text(json))
    }

    // BUG-065 FIX: Use atomic send_and_recv to prevent response mismatches
    pub async fn tool_beads_pane_history(
    &mut self,
    pane_id: Option<Uuid>,
    ) -> Result<ToolResult, McpError> {
    let (session_name, resolved_pane_id) = self.resolve_session_for_pane(pane_id).await?;

    match self.connection.send_and_recv(ClientMessage::GetMetadata {
    session_filter: session_name.clone(),
    key: Some(beads::ISSUE_HISTORY.to_string()),
    }).await? {
    ServerMessage::MetadataList {
    session_id,
    session_name,
    metadata,
    } => {
    let history_json = metadata.get(beads::ISSUE_HISTORY).cloned();

    let history: Vec<serde_json::Value> = history_json
    .and_then(|h| serde_json::from_str(&h).ok())
    .unwrap_or_default();

    let current_issue = match self.connection.send_and_recv(ClientMessage::GetMetadata {
    session_filter: session_id.to_string(),
    key: Some(beads::CURRENT_ISSUE.to_string()),
    }).await? {
    ServerMessage::MetadataList { metadata, .. } => {
        metadata
        .get(beads::CURRENT_ISSUE)
        .cloned()
        .filter(|s| !s.is_empty())
    }
    _ => None,
    };

    let result = serde_json::json!({
    "session_id": session_id.to_string(),
    "session_name": session_name,
    "pane_id": resolved_pane_id.map(|id| id.to_string()),
    "current_issue": current_issue,
    "history": history,
    "history_count": history.len(),
    });

    let json = serde_json::to_string_pretty(&result)
    .map_err(|e| McpError::Internal(e.to_string()))?;
    Ok(ToolResult::text(json))
    }
    ServerMessage::Error { code, message, .. } => {
    Ok(ToolResult::error(format!("{:?}: {}", code, message)))
    }
    msg => Err(McpError::UnexpectedResponse(format!("{:?}", msg))),
    }
    }

    // ==================== FEAT-062: Mirror Pane Tool ====================

    /// Create a read-only mirror pane that displays another pane's output
    pub async fn tool_mirror_pane(
        &mut self,
        source_pane_id: Uuid,
        direction: Option<&str>,
    ) -> Result<ToolResult, McpError> {
        let split_direction = match direction {
            Some("horizontal") => SplitDirection::Horizontal,
            _ => SplitDirection::Vertical,
        };

        // BUG-065 FIX: Use atomic send_and_recv_filtered to prevent response mismatches
        match self.connection
            .send_and_recv_filtered(
                ClientMessage::CreateMirror {
                    source_pane_id,
                    target_pane_id: None,
                    direction: Some(split_direction),
                },
                |msg| matches!(msg, ServerMessage::MirrorCreated { source_pane_id: src_id, .. } if *src_id == source_pane_id),
            )
            .await?
        {
            ServerMessage::MirrorCreated {
                mirror_pane,
                source_pane_id,
                session_name,
                direction,
                ..
            } => {
                let result = serde_json::json!({
                    "mirror_pane_id": mirror_pane.id.to_string(),
                    "source_pane_id": source_pane_id.to_string(),
                    "session_name": session_name,
                    "direction": format!("{:?}", direction).to_lowercase(),
                    "status": "created"
                });

                let json = serde_json::to_string_pretty(&result)
                    .map_err(|e| McpError::Internal(e.to_string()))?;
                Ok(ToolResult::text(json))
            }
            ServerMessage::Error { code, message, .. } => {
                Ok(ToolResult::error(format!("{:?}: {}", code, message)))
            }
            msg => Err(McpError::UnexpectedResponse(format!("{:?}", msg))),
        }
    }

    // ==================== FEAT-096: Expect Tool ====================

    pub async fn tool_expect(
        &mut self,
        pane_id: Uuid,
        pattern: &str,
        timeout_ms: u64,
        action: &str,
        poll_interval_ms: u64,
        lines: usize,
    ) -> Result<ToolResult, McpError> {
        let expect_action: ExpectAction = action.parse()?;
        
        run_expect(
            self.connection,
            pane_id,
            pattern,
            timeout_ms,
            expect_action,
            poll_interval_ms,
            lines,
        )
        .await
    }

    // ==================== FEAT-095: Pipeline Tool ====================

    pub async fn tool_run_pipeline(
        &mut self,
        request: RunPipelineRequest,
    ) -> Result<ToolResult, McpError> {
        let mut runner = PipelineRunner::new(self.connection);
        let response = runner.run(request).await?;

        let json = serde_json::to_string_pretty(&response)
            .map_err(|e| McpError::Internal(e.to_string()))?;
        Ok(ToolResult::text(json))
    }

    // ==================== FEAT-097: Orchestration Message Receive ====================

    // BUG-065 FIX: Use atomic send_and_recv to prevent response mismatches
    pub async fn tool_get_worker_status(
        &mut self,
        worker_id: Option<String>,
    ) -> Result<ToolResult, McpError> {
        match self.connection.send_and_recv(ClientMessage::GetWorkerStatus { worker_id }).await? {
            ServerMessage::WorkerStatus { status } => {
                let json = serde_json::to_string_pretty(&status.inner())
                    .map_err(|e| McpError::Internal(e.to_string()))?;
                Ok(ToolResult::text(json))
            }
            ServerMessage::Error { code, message, .. } => {
                Ok(ToolResult::error(format!("{:?}: {}", code, message)))
            }
            msg => Err(McpError::UnexpectedResponse(format!("{:?}", msg))),
        }
    }

    // BUG-065 FIX: Use atomic send_and_recv to prevent response mismatches
    // BUG-069 FIX: worker_id is now optional - if None, polls the attached session
    pub async fn tool_poll_messages(
        &mut self,
        worker_id: Option<String>,
    ) -> Result<ToolResult, McpError> {
        match self.connection.send_and_recv(ClientMessage::PollMessages { worker_id }).await? {
            ServerMessage::MessagesPolled { messages } => {
                let result: Vec<serde_json::Value> = messages
                    .into_iter()
                    .map(|(from_id, msg)| {
                        serde_json::json!({
                            "from_session_id": from_id.to_string(),
                            "type": msg.msg_type,
                            "payload": msg.payload.inner(),
                        })
                    })
                    .collect();

                let json = serde_json::to_string_pretty(&result)
                    .map_err(|e| McpError::Internal(e.to_string()))?;
                Ok(ToolResult::text(json))
            }
                        ServerMessage::Error { code, message, .. } => {
                            Ok(ToolResult::error(format!("{:?}: {}", code, message)))
                        }
                        msg => Err(McpError::UnexpectedResponse(format!("{:?}", msg))),
                    }
                }
            
                // BUG-065 FIX: Use atomic send_and_recv to prevent response mismatches
                pub async fn tool_create_status_pane(
                    &mut self,
                    position: Option<String>,
                    width_percent: Option<i64>,
                    show_activity_feed: bool,
                    show_output_preview: bool,
                    filter_tags: Option<Vec<String>>,
                ) -> Result<ToolResult, McpError> {
                    match self.connection.send_and_recv(ClientMessage::CreateStatusPane {
                        position,
                        width_percent,
                        show_activity_feed,
                        show_output_preview,
                        filter_tags,
                    }).await? {
                        ServerMessage::PaneCreated { pane, direction, .. } => {
                            let result = serde_json::json!({
                                "pane_id": pane.id.to_string(),
                                "window_id": pane.window_id.to_string(),
                                "direction": format!("{:?}", direction).to_lowercase(),
                                "status": "created",
                                "type": "status_pane"
                            });
            
                            let json = serde_json::to_string_pretty(&result)
                                .map_err(|e| McpError::Internal(e.to_string()))?;
                            Ok(ToolResult::text(json))
                        }
                        ServerMessage::Error { code, message, .. } => {
                            Ok(ToolResult::error(format!("{:?}: {}", code, message)))
                        }
                        msg => Err(McpError::UnexpectedResponse(format!("{:?}", msg))),
                    }
                }

    // ==================== FEAT-104: Watchdog Timer ====================
    // FEAT-114: Named/Multiple Watchdogs

    /// Start a named watchdog timer
    pub async fn tool_watchdog_start(
        &mut self,
        pane_id: Uuid,
        interval_secs: Option<u64>,
        message: Option<String>,
        name: Option<String>,
    ) -> Result<ToolResult, McpError> {
        match self.connection.send_and_recv(ClientMessage::WatchdogStart {
            pane_id,
            interval_secs: interval_secs.unwrap_or(90),
            message,
            name,
        }).await? {
            ServerMessage::WatchdogStarted {
                name,
                pane_id,
                interval_secs,
                message,
            } => {
                let result = serde_json::json!({
                    "status": "started",
                    "name": name,
                    "pane_id": pane_id.to_string(),
                    "interval_secs": interval_secs,
                    "message": message
                });

                let json = serde_json::to_string_pretty(&result)
                    .map_err(|e| McpError::Internal(e.to_string()))?;
                Ok(ToolResult::text(json))
            }
            ServerMessage::Error { code, message, .. } => {
                Ok(ToolResult::error(format!("{:?}: {}", code, message)))
            }
            msg => Err(McpError::UnexpectedResponse(format!("{:?}", msg))),
        }
    }

    /// Stop watchdog timer(s)
    /// If name is Some, stops only that watchdog. If None, stops all.
    pub async fn tool_watchdog_stop(&mut self, name: Option<String>) -> Result<ToolResult, McpError> {
        match self.connection.send_and_recv(ClientMessage::WatchdogStop { name: name.clone() }).await? {
            ServerMessage::WatchdogStopped { stopped } => {
                let result = if stopped.is_empty() {
                    serde_json::json!({
                        "status": "no_watchdogs_running",
                        "stopped": []
                    })
                } else {
                    serde_json::json!({
                        "status": "stopped",
                        "stopped": stopped
                    })
                };

                let json = serde_json::to_string_pretty(&result)
                    .map_err(|e| McpError::Internal(e.to_string()))?;
                Ok(ToolResult::text(json))
            }
            ServerMessage::Error { code, message, .. } => {
                Ok(ToolResult::error(format!("{:?}: {}", code, message)))
            }
            msg => Err(McpError::UnexpectedResponse(format!("{:?}", msg))),
        }
    }

    /// Get watchdog timer status
    /// If name is Some, returns status of that specific watchdog. If None, returns all.
    pub async fn tool_watchdog_status(&mut self, name: Option<String>) -> Result<ToolResult, McpError> {
        match self.connection.send_and_recv(ClientMessage::WatchdogStatus { name: name.clone() }).await? {
            ServerMessage::WatchdogStatusResponse { watchdogs } => {
                let result = if watchdogs.is_empty() {
                    serde_json::json!({
                        "is_running": false,
                        "watchdogs": []
                    })
                } else {
                    let watchdog_list: Vec<_> = watchdogs.iter().map(|w| {
                        serde_json::json!({
                            "name": w.name,
                            "pane_id": w.pane_id.to_string(),
                            "interval_secs": w.interval_secs,
                            "message": w.message
                        })
                    }).collect();

                    serde_json::json!({
                        "is_running": true,
                        "count": watchdogs.len(),
                        "watchdogs": watchdog_list
                    })
                };

                let json = serde_json::to_string_pretty(&result)
                    .map_err(|e| McpError::Internal(e.to_string()))?;
                Ok(ToolResult::text(json))
            }
            ServerMessage::Error { code, message, .. } => {
                Ok(ToolResult::error(format!("{:?}: {}", code, message)))
            }
            msg => Err(McpError::UnexpectedResponse(format!("{:?}", msg))),
        }
    }

    // ==================== FEAT-109: Drain Messages Tool ====================

    /// Drain stale broadcast messages from the response channel
    ///
    /// This tool clears any pending messages that may have accumulated in the
    /// response channel due to broadcasts or stale responses from prior timeouts.
    /// It returns diagnostic information about what was drained.
    pub fn tool_drain_messages(&mut self) -> Result<ToolResult, McpError> {
        let (total, type_counts) = self.connection.drain_with_diagnostics();

        let result = serde_json::json!({
            "drained_count": total,
            "message_types": type_counts,
            "status": if total > 0 { "cleared" } else { "empty" }
        });

        let json = serde_json::to_string_pretty(&result)
            .map_err(|e| McpError::Internal(e.to_string()))?;
        Ok(ToolResult::text(json))
    }
            }
            