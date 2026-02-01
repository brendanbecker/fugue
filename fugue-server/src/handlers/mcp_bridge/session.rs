use tracing::{info, warn, debug};
use fugue_protocol::{ErrorCode, ServerMessage};
use crate::pty::{PtyConfig, PtyOutputPoller};
use crate::handlers::{HandlerContext, HandlerResult};

impl HandlerContext {
    /// Handle CreateSessionWithOptions - create a session with full control
    pub async fn handle_create_session_with_options(
        &self,
        name: Option<String>,
        command: Option<String>,
        cwd: Option<String>,
        claude_model: Option<String>,
        claude_config: Option<serde_json::Value>,
        preset: Option<String>,
        tags: Option<Vec<String>>,
    ) -> HandlerResult {
        info!(
            "CreateSessionWithOptions request from {} (name: {:?}, command: {:?}, cwd: {:?}, model: {:?}, preset: {:?}, tags: {:?})",
            self.client_id, name, command, cwd, claude_model, preset, tags
        );

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

        // Apply tags if provided (FEAT-compat-tags)
        if let Some(tag_list) = tags {
            if let Some(session) = session_manager.get_session_mut(session_id) {
                for tag in tag_list {
                    session.add_tag(&tag);
                    debug!("Added tag '{}' to session {}", tag, session_id);
                }
            }
        }

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

        // FEAT-071: Per-pane Claude configuration
        if claude_model.is_some() || claude_config.is_some() || preset.is_some() {
            let config = &self.config;
            
            let mut final_config = serde_json::Map::new();
            
            // 1. Apply preset
            if let Some(preset_name) = &preset {
                if let Some(preset_cfg) = config.presets.get(preset_name) {
                    // Check if it's a Claude harness
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

        // Broadcast updated session list to all clients (BUG-032)
        let sessions: Vec<_> = session_manager.list_sessions().iter().map(|s| s.to_info()).collect();

        // Drop session_manager lock before spawning PTY
        drop(session_manager);

        // Spawn PTY for the default pane
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".into());
        let mut config = if let Some(ref cmd) = command {
            // Wrap user command in shell to handle arguments and shell syntax
            PtyConfig::command("sh").with_arg("-c").with_arg(cmd)
        } else {
            PtyConfig::command(&shell)
        };
        if let Some(ref cwd) = cwd {
            config = config.with_cwd(cwd);
        }
        config = config.with_fugue_context(session_id, &session_name, window_id, pane_id);

        {
            let mut pty_manager = self.pty_manager.write().await;
            match pty_manager.spawn(pane_id, config) {
                Ok(handle) => {
                    info!("PTY spawned for MCP session pane {}", pane_id);

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
                    info!("Output poller started for MCP session pane {} (sideband enabled)", pane_id);
                }
                Err(e) => {
                    warn!("Failed to spawn PTY for pane {}: {}", pane_id, e);
                }
            }
        }

        info!("Session {} created with window {} and pane {}", session_name, window_id, pane_id);

        HandlerResult::ResponseWithGlobalBroadcast {
            response: ServerMessage::SessionCreatedWithDetails {
                session_id,
                session_name,
                window_id,
                pane_id,
                should_focus: true,
            },
            broadcast: ServerMessage::SessionsChanged { sessions },
        }
    }
}
