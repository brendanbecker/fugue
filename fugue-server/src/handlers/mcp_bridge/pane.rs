use tracing::{info, warn, debug};
use uuid::Uuid;
use ccmux_protocol::{ErrorCode, PaneListEntry, ServerMessage, SplitDirection};
use crate::pty::{PtyConfig, PtyOutputPoller};
use crate::arbitration::{Action, Resource};
use crate::handlers::{HandlerContext, HandlerResult};

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

        // FEAT-078: Get client's focus state
        let client_focus = self.registry.get_client_focus(self.client_id);

        let mut panes = Vec::new();

        for session in session_manager.list_sessions() {
            // Apply session filter if provided
            if let Some(ref filter) = session_filter {
                // Try to match by UUID or name
                if session.id().to_string() != *filter && session.name() != filter {
                    continue;
                }
            }

            // Get the global active window for this session as a fallback
            let active_window_id = session.active_window_id();

            for window in session.windows() {
                // Get the global active pane in this window as a fallback
                let active_pane_id = window.active_pane_id();
                // A pane is focused if it's the active pane in the active window
                let is_active_window = Some(window.id()) == active_window_id;

                for pane in window.panes() {
                    let claude_state = pane.claude_state();

                    // FEAT-078: Pane is focused if it's the active pane for THIS client
                    let is_focused = if let Some(ref focus) = client_focus {
                        focus.active_session_id == Some(session.id())
                            && focus.active_window_id == Some(window.id())
                            && focus.active_pane_id == Some(pane.id())
                    } else {
                        // Fallback to global focus if client has no specific focus yet
                        is_active_window && Some(pane.id()) == active_pane_id
                    };

                    panes.push(PaneListEntry {
                        id: pane.id(),
                        session_name: session.name().to_string(),
                        window_index: window.index(),
                        window_name: window.name().to_string(),
                        pane_index: pane.index(),
                        cols: pane.dimensions().0,
                        rows: pane.dimensions().1,
                        name: pane.name().map(|s| s.to_string()),
                        title: pane.title().map(|s| s.to_string()),
                        cwd: pane.cwd().map(|s| s.to_string()),
                        state: pane.state().clone(),
                        is_claude: pane.is_claude(),
                        claude_state,
                        is_focused,
                    });
                }
            }
        }

        debug!("Found {} panes", panes.len());
        HandlerResult::Response(ServerMessage::AllPanesList { panes })
    }

    /// Handle GetPaneStatus - get detailed status of a pane
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
    #[allow(clippy::too_many_arguments)]
    pub async fn handle_create_pane_with_options(
        &self,
        session_filter: Option<String>,
        window_filter: Option<String>,
        direction: SplitDirection,
        command: Option<String>,
        cwd: Option<String>,
        select: bool,
        name: Option<String>,
        claude_model: Option<String>,
        claude_config: Option<serde_json::Value>,
        preset: Option<String>,
    ) -> HandlerResult {
        debug!(
            client_id = %self.client_id,
            session_filter = ?session_filter,
            window_filter = ?window_filter,
            direction = ?direction,
            command = ?command,
            cwd = ?cwd,
            select = select,
            name = ?name,
            model = ?claude_model,
            preset = ?preset,
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
            // Use client's active session, global active session, or create one (FEAT-078)
            match self.resolve_active_session(&session_manager) {
                Some(id) => id,
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
            // Use client's focused window or first window (FEAT-078)
            match self.resolve_active_window(session) {
                Some(id) => id,
                None => {
                    // Check first, then create to avoid borrow conflict
                    let existing_window_id = session.windows().next().map(|w| w.id());
                    match existing_window_id {
                        Some(id) => id,
                        None => session.create_window(None).id(),
                    }
                }
            }
        };

        // FEAT-079: Arbitrate layout access
        if let Err(blocked) = self.check_arbitration(Resource::Window(window_id), Action::Layout) {
            return blocked;
        }
        self.record_human_activity(Resource::Window(window_id), Action::Layout);

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

        // Initialize the parser and set the name (FEAT-036)
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
        if let Some(ref pane_name) = name {
            pane.set_name(Some(pane_name.clone()));
        }

        // Determine harness command if not provided explicitly
        let mut final_command = command.clone();
        let mut harness_args: Vec<String> = Vec::new();
        let mut harness_env: std::collections::HashMap<String, String> = std::collections::HashMap::new();

        // FEAT-071: Per-pane Claude configuration / FEAT-105: Universal Agent Presets
        if claude_model.is_some() || claude_config.is_some() || preset.is_some() {
            let config = &self.config;
            
            let mut final_config = serde_json::Map::new();
            
            // 1. Apply preset
            if let Some(preset_name) = &preset {
                if let Some(preset_cfg) = config.presets.get(preset_name) {
                    // Determine command based on harness type
                    match &preset_cfg.config {
                        crate::config::HarnessConfig::Shell(shell_cfg) => {
                            if final_command.is_none() {
                                final_command = shell_cfg.command.clone();
                            }
                            if let Some(args) = &shell_cfg.args {
                                harness_args.extend(args.clone());
                            }
                            if let Some(env) = &shell_cfg.env {
                                harness_env.extend(env.clone());
                            }
                        },
                        crate::config::HarnessConfig::Custom(custom_cfg) => {
                            if final_command.is_none() {
                                final_command = Some(custom_cfg.command.clone());
                            }
                            if let Some(args) = &custom_cfg.args {
                                harness_args.extend(args.clone());
                            }
                            if let Some(env) = &custom_cfg.env {
                                harness_env.extend(env.clone());
                            }
                        },
                        crate::config::HarnessConfig::Claude(_) => {
                            if final_command.is_none() {
                                final_command = Some("claude".to_string());
                            }
                        },
                        crate::config::HarnessConfig::Gemini(_) => {
                            if final_command.is_none() {
                                final_command = Some("gemini".to_string());
                            }
                        },
                        crate::config::HarnessConfig::Codex(_) => {
                            if final_command.is_none() {
                                final_command = Some("codex".to_string());
                            }
                        },
                    }

                    // Check if it's a Claude harness for config extraction
                    if preset_cfg.harness == "claude" {
                        if let crate::config::HarnessConfig::Claude(claude_cfg) = &preset_cfg.config {
                            if let Some(m) = &claude_cfg.model {
                                final_config.insert("model".to_string(), serde_json::json!(m));
                            }
                            if let Some(c) = claude_cfg.context_limit {
                                final_config.insert("context_limit".to_string(), serde_json::json!(c));
                            }
                            // Map other fields
                            if let Some(sp) = &claude_cfg.system_prompt {
                                final_config.insert("system_prompt".to_string(), serde_json::json!(sp));
                            }
                            if let Some(dsp) = &claude_cfg.dangerously_skip_permissions {
                                final_config.insert("dangerously_skip_permissions".to_string(), serde_json::json!(dsp));
                            }
                            if let Some(at) = &claude_cfg.allowed_tools {
                                final_config.insert("allowed_tools".to_string(), serde_json::json!(at));
                            }
                        }
                    }
                    debug!("Applied preset '{}' to pane {}", preset_name, pane_id);
                } else {
                    warn!("Preset '{}' not found for pane {}", preset_name, pane_id);
                }
            }
            
            // 2. Apply explicit config
            if let Some(serde_json::Value::Object(map)) = &claude_config {
                for (k, v) in map {
                    final_config.insert(k.clone(), v.clone());
                }
            }
            
            // 3. Apply model override
            if let Some(m) = &claude_model {
                final_config.insert("model".to_string(), serde_json::json!(m));
            }
            
            // Write to isolation directory
            match crate::isolation::ensure_config_dir(pane_id) {
                Ok(path) => {
                    let config_file = path.join(".claude.json");
                    let json_content = serde_json::Value::Object(final_config);
                    
                    match std::fs::File::create(&config_file) {
                        Ok(mut file) => {
                            if let Err(e) = serde_json::to_writer_pretty(&mut file, &json_content) {
                                warn!("Failed to write Claude config for pane {}: {}", pane_id, e);
                            } else {
                                info!("Wrote custom Claude config for pane {} to {:?}", pane_id, config_file);
                            }
                        },
                        Err(e) => warn!("Failed to create Claude config file for pane {}: {}", pane_id, e),
                    }
                },
                Err(e) => warn!("Failed to ensure isolation dir for pane {}: {}", pane_id, e),
            }
        }

        // If select is true, focus the new pane (set as active pane in window and window as active)
        if select {
            window.set_active_pane(pane_id);
            // Also update client focus (FEAT-078)
            self.registry.update_client_focus(self.client_id, Some(session_id), Some(window_id), Some(pane_id));
            debug!(pane_id = %pane_id, "Pane focused after creation (select=true)");
        }

        // Capture session environment before dropping lock
        let session_env = session_manager
            .get_session(session_id)
            .map(|s| s.environment().clone())
            .unwrap_or_default();

        // Drop session_manager lock before spawning PTY
        // Note: If select is true, we also need to set the window as active
        let select_window_id = if select { Some(window_id) } else { None };
        std::mem::drop(session_manager);

        // Re-acquire lock to set active window if needed
        if let Some(wid) = select_window_id {
            let mut sm = self.session_manager.write().await;
            if let Some(session) = sm.get_session_mut(session_id) {
                session.set_active_window(wid);
            }
            std::mem::drop(sm);
        }

        // Spawn PTY
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".into());
        let mut config = if let Some(ref cmd) = final_command {
            // Use derived command
            let mut cfg = if harness_args.is_empty() && !cmd.contains(' ') {
                 // Simple command, run directly or via shell? 
                 // If we have harness_args, we assume cmd is the executable.
                 // If we don't, we assume cmd might be a shell string.
                 // But wait, existing logic wrapped in `sh -c`.
                 PtyConfig::command("sh").with_arg("-c").with_arg(cmd)
            } else {
                 // If we have explicit harness args, treat cmd as executable
                 // Or if it's from preset, we should respect it.
                 // Let's stick to existing behavior for user command (sh -c), but for harness derived command we might want direct execution?
                 // The safe bet is sticking to existing behavior unless we know better.
                 // BUT `sh -c` messes up argument passing if we have extra args.
                 
                 if !harness_args.is_empty() {
                     let mut c = PtyConfig::command(cmd);
                     for arg in &harness_args {
                         c = c.with_arg(arg);
                     }
                     c
                 } else {
                     PtyConfig::command("sh").with_arg("-c").with_arg(cmd)
                 }
            };
            cfg
        } else {
            PtyConfig::command(&shell)
        };
        // Apply harness environment
        for (k, v) in harness_env {
            config = config.with_env(k, v);
        }

        // BUG-050: Apply explicit cwd, or inherit from parent pane
        if let Some(ref cwd) = cwd {
            config = config.with_cwd(cwd);
        } else if let Some(ref inherited) = inherited_cwd {
            config = config.with_cwd(inherited);
        }
        config = config.with_ccmux_context(session_id, &session_name, window_id, pane_id);
        // Apply session environment variables
        config = config.with_env_map(&session_env);

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
                should_focus: select,
            },
            session_id,
            broadcast: ServerMessage::PaneCreated {
                pane: pane_info, 
                direction,
                should_focus: false, // Don't steal focus from TUI users
            },
        }
    }

    /// Handle SplitPane - split an existing pane
    pub async fn handle_split_pane(
        &self,
        pane_id: Uuid,
        direction: SplitDirection,
        ratio: f32,
        command: Option<String>,
        cwd: Option<String>,
        select: bool,
    ) -> HandlerResult {
        info!(
            "SplitPane request from {} (pane: {}, direction: {:?}, ratio: {})",
            self.client_id, pane_id, direction, ratio
        );

        // Validate ratio
        let ratio = ratio.clamp(0.1, 0.9);

        let session_manager = self.session_manager.read().await;

        // Find the pane to split
        let (session, window, source_pane) = match session_manager.find_pane(pane_id) {
            Some(found) => found,
            None => {
                return HandlerContext::error(
                    ErrorCode::PaneNotFound,
                    format!("Pane {} not found", pane_id),
                );
            }
        };

        let session_id = session.id();
        let session_name = session.name().to_string();
        let window_id = window.id();

        // BUG-050: Capture source pane's cwd for inheritance
        let inherited_cwd = if cwd.is_none() {
            source_pane.cwd().map(String::from)
        } else {
            None
        };

        // FEAT-079: Arbitrate layout access
        if let Err(blocked) = self.check_arbitration(Resource::Window(window_id), Action::Layout) {
            return blocked;
        }
        self.record_human_activity(Resource::Window(window_id), Action::Layout);

        // Drop read lock before taking write lock
        drop(session_manager);

        // Create the new pane
        let mut session_manager = self.session_manager.write().await;
        let session = match session_manager.get_session_mut(session_id) {
            Some(s) => s,
            None => {
                return HandlerContext::error(ErrorCode::SessionNotFound, "Session disappeared");
            }
        };

        let window = match session.get_window_mut(window_id) {
            Some(w) => w,
            None => {
                return HandlerContext::error(ErrorCode::WindowNotFound, "Window disappeared");
            }
        };

        let new_pane = window.create_pane();
        let new_pane_id = new_pane.id();
        let new_pane_info = new_pane.to_info();

        // Initialize the parser for the new pane
        let pane = match window.get_pane_mut(new_pane_id) {
            Some(p) => p,
            None => {
                return HandlerContext::error(ErrorCode::InternalError, "Pane disappeared");
            }
        };
        pane.init_parser();

        // If select is true, focus the new pane
        if select {
            window.set_active_pane(new_pane_id);
            // Also update client focus (FEAT-078)
            self.registry.update_client_focus(self.client_id, Some(session_id), Some(window_id), Some(new_pane_id));
        }

        // Capture session environment before dropping lock
        let session_env = session_manager
            .get_session(session_id)
            .map(|s| s.environment().clone())
            .unwrap_or_default();

        // Drop lock before spawning PTY
        drop(session_manager);

        // Spawn PTY for the new pane
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".into());
        let mut config = if let Some(ref cmd) = command {
            PtyConfig::command("sh").with_arg("-c").with_arg(cmd)
        } else {
            PtyConfig::command(&shell)
        };
        // BUG-050: Apply explicit cwd, or inherit from source pane
        if let Some(ref cwd) = cwd {
            config = config.with_cwd(cwd);
        } else if let Some(ref inherited) = inherited_cwd {
            config = config.with_cwd(inherited);
        }
        config = config.with_ccmux_context(session_id, &session_name, window_id, new_pane_id);
        // Apply session environment variables
        config = config.with_env_map(&session_env);

        {
            let mut pty_manager = self.pty_manager.write().await;
            match pty_manager.spawn(new_pane_id, config) {
                Ok(handle) => {
                    info!("PTY spawned for split pane {}", new_pane_id);

                    let reader = handle.clone_reader();
                    let _poller_handle = PtyOutputPoller::spawn_with_sideband(
                        new_pane_id,
                        session_id,
                        reader,
                        self.registry.clone(),
                        Some(self.pane_closed_tx.clone()),
                        self.command_executor.clone(),
                    );
                }
                Err(e) => {
                    warn!("Failed to spawn PTY for pane {}: {}", new_pane_id, e);
                }
            }
        }

        let direction_str = match direction {
            SplitDirection::Horizontal => "horizontal",
            SplitDirection::Vertical => "vertical",
        };

        info!(
            "Pane {} split into new pane {} (direction: {}, ratio: {})",
            pane_id, new_pane_id, direction_str, ratio
        );

        // Return response to MCP client and broadcast to TUI clients (BUG-032)
        HandlerResult::ResponseWithBroadcast {
            response: ServerMessage::PaneSplit {
                new_pane_id,
                original_pane_id: pane_id,
                session_id,
                session_name,
                window_id,
                direction: direction_str.to_string(),
                should_focus: select,
            },
            session_id,
            broadcast: ServerMessage::PaneCreated {
                pane: new_pane_info,
                direction,
                should_focus: false,
            },
        }
    }

    /// Handle ResizePaneDelta - resize a pane by delta fraction
    pub async fn handle_resize_pane_delta(
        &self,
        pane_id: Uuid,
        delta: f32,
    ) -> HandlerResult {
        info!(
            "ResizePaneDelta request from {} (pane: {}, delta: {})",
            self.client_id, pane_id, delta
        );

        // Validate delta
        let delta = delta.clamp(-0.5, 0.5);

        let session_manager = self.session_manager.read().await;

        // Find the pane and capture session_id for broadcast (BUG-032)
        let (session, _, pane) = match session_manager.find_pane(pane_id) {
            Some(found) => found,
            None => {
                return HandlerContext::error(
                    ErrorCode::PaneNotFound,
                    format!("Pane {} not found", pane_id),
                );
            }
        };

        let session_id = session.id();
        let (current_cols, current_rows) = pane.dimensions();
        let window_id = pane.window_id();

        // FEAT-079: Arbitrate layout access
        if let Err(blocked) = self.check_arbitration(Resource::Window(window_id), Action::Layout) {
            return blocked;
        }
        self.record_human_activity(Resource::Window(window_id), Action::Layout);

        // Drop read lock before taking write lock
        drop(session_manager);

        // Calculate new dimensions based on delta
        // Delta is a fraction: positive grows, negative shrinks
        // We'll apply delta to both dimensions proportionally
        let scale = 1.0 + delta;
        let new_cols = ((current_cols as f32) * scale).clamp(10.0, 500.0) as u16;
        let new_rows = ((current_rows as f32) * scale).clamp(5.0, 200.0) as u16;

        // Update pane dimensions
        let mut session_manager = self.session_manager.write().await;
        if let Some(pane) = session_manager.find_pane_mut(pane_id) {
            pane.resize(new_cols, new_rows);
        }
        drop(session_manager);

        // Resize PTY if exists
        {
            let pty_manager = self.pty_manager.read().await;
            if let Some(handle) = pty_manager.get(pane_id) {
                if let Err(e) = handle.resize(new_cols, new_rows) {
                    warn!("Failed to resize PTY for pane {}: {}", pane_id, e);
                }
            }
        }

        info!(
            "Pane {} resized from {}x{} to {}x{} (delta: {})",
            pane_id, current_cols, current_rows, new_cols, new_rows, delta
        );

        // Return response to MCP client and broadcast to TUI clients (BUG-032)
        HandlerResult::ResponseWithBroadcast {
            response: ServerMessage::PaneResized {
                pane_id,
                new_cols,
                new_rows,
            },
            session_id,
            broadcast: ServerMessage::PaneResized {
                pane_id,
                new_cols,
                new_rows,
            },
        }
    }
}
