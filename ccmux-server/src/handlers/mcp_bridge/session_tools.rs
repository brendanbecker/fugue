use tracing::{debug, info, warn};
use uuid::Uuid;
use ccmux_protocol::{
    ErrorCode, ServerMessage,
};

use crate::pty::{PtyConfig, PtyOutputPoller};
use crate::handlers::{HandlerContext, HandlerResult};

/// Handle CreateSessionWithOptions - create a session with full control
pub async fn create_session_with_options(
    ctx: &HandlerContext,
    name: Option<String>,
    command: Option<String>,
    cwd: Option<String>,
    claude_model: Option<String>,
    claude_config: Option<serde_json::Value>,
    preset: Option<String>,
) -> HandlerResult {
    info!(
        "CreateSessionWithOptions request from {} (name: {:?}, command: {:?}, cwd: {:?}, model: {:?}, preset: {:?})", 
        ctx.client_id, name, command, cwd, claude_model, preset
    );

    let mut session_manager = ctx.session_manager.write().await;

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

    // FEAT-071: Per-pane Claude configuration
    if claude_model.is_some() || claude_config.is_some() || preset.is_some() {
        let config = &ctx.config;
        
        let mut final_config = serde_json::Map::new();
        
        // 1. Apply preset
        if let Some(preset_name) = &preset {
            if let Some(preset_cfg) = config.presets.get(preset_name) {
                if let Some(m) = &preset_cfg.model {
                    final_config.insert("model".to_string(), serde_json::json!(m));
                }
                if let Some(c) = preset_cfg.context_limit {
                    final_config.insert("context_limit".to_string(), serde_json::json!(c));
                }
                for (k, v) in &preset_cfg.extra {
                    final_config.insert(k.clone(), v.clone());
                }
                debug!("Applied Claude preset '{}' to pane {}", preset_name, pane_id);
            } else {
                warn!("Claude preset '{}' not found for pane {}", preset_name, pane_id);
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
    config = config.with_ccmux_context(session_id, &session_name, window_id, pane_id);

    {
        let mut pty_manager = ctx.pty_manager.write().await;
        match pty_manager.spawn(pane_id, config) {
            Ok(handle) => {
                info!("PTY spawned for MCP session pane {}", pane_id);

                // Start output poller with sideband parsing enabled
                let reader = handle.clone_reader();
                let _poller_handle = PtyOutputPoller::spawn_with_sideband(
                    pane_id,
                    session_id,
                    reader,
                    ctx.registry.clone(),
                    Some(ctx.pane_closed_tx.clone()),
                    ctx.command_executor.clone(),
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

/// Handle SetEnvironment - set environment variable in a session
pub async fn set_environment(
    ctx: &HandlerContext,
    session_filter: String,
    key: String,
    value: String,
) -> HandlerResult {
    info!(
        "SetEnvironment request from {}: session={}, key={}",
        ctx.client_id, session_filter, key
    );

    let mut session_manager = ctx.session_manager.write().await;

    // Find the session by UUID or name
    let session_id = if let Ok(uuid) = Uuid::parse_str(&session_filter) {
        if session_manager.get_session(uuid).is_some() {
            uuid
        } else {
            return HandlerContext::error(
                ErrorCode::SessionNotFound,
                format!("Session {} not found", session_filter),
            );
        }
    } else {
        // Try by name
        match session_manager.get_session_by_name(&session_filter) {
            Some(session) => session.id(),
            None => {
                return HandlerContext::error(
                    ErrorCode::SessionNotFound,
                    format!("Session '{}' not found", session_filter),
                );
            }
        }
    };

    // Get the session and set the environment variable
    let session_name = if let Some(session) = session_manager.get_session_mut(session_id) {
        session.set_env(&key, &value);
        session.name().to_string()
    } else {
        return HandlerContext::error(
            ErrorCode::SessionNotFound,
            format!("Session {} not found", session_id),
        );
    };

    // Release lock before persistence call
    drop(session_manager);

    // Log to persistence (FEAT-086: environment must persist across restarts)
    if let Some(persistence_lock) = &ctx.persistence {
        let persistence = persistence_lock.read().await;
        if let Ok(seq) = persistence.log_session_environment_set(session_id, &key, &value) {
            persistence.push_replay(seq, ServerMessage::EnvironmentSet {
                session_id,
                session_name: session_name.clone(),
                key: key.clone(),
                value: value.clone(),
            });
        }
    }

    HandlerResult::Response(ServerMessage::EnvironmentSet {
        session_id,
        session_name,
        key,
        value,
    })
}

/// Handle GetEnvironment - get environment variables from a session
pub async fn get_environment(
    ctx: &HandlerContext,
    session_filter: String,
    key: Option<String>,
) -> HandlerResult {
    debug!(
        "GetEnvironment request from {}: session={}, key={:?}",
        ctx.client_id, session_filter, key
    );

    let session_manager = ctx.session_manager.read().await;

    // Find the session by UUID or name
    let session = if let Ok(uuid) = Uuid::parse_str(&session_filter) {
        session_manager.get_session(uuid)
    } else {
        session_manager.get_session_by_name(&session_filter)
    };

    let session = match session {
        Some(s) => s,
        None => {
            return HandlerContext::error(
                ErrorCode::SessionNotFound,
                format!("Session '{}' not found", session_filter),
            );
        }
    };

    let session_id = session.id();
    let session_name = session.name().to_string();

    // Get environment - either specific key or all
    let environment = if let Some(ref k) = key {
        // Get single key
        let mut env = std::collections::HashMap::new();
        if let Some(v) = session.get_env(k) {
            env.insert(k.clone(), v.clone());
        }
        env
    } else {
        // Get all
        session.environment().clone()
    };

    HandlerResult::Response(ServerMessage::EnvironmentList {
        session_id,
        session_name,
        environment,
    })
}

/// Handle SetMetadata - set metadata on a session
pub async fn set_metadata(
    ctx: &HandlerContext,
    session_filter: String,
    key: String,
    value: String,
) -> HandlerResult {
    info!(
        "SetMetadata request from {}: session={}, key={}",
        ctx.client_id, session_filter, key
    );

    let mut session_manager = ctx.session_manager.write().await;

    // Find the session by UUID or name
    let session_id = if let Ok(uuid) = Uuid::parse_str(&session_filter) {
        if session_manager.get_session(uuid).is_some() {
            uuid
        } else {
            return HandlerContext::error(
                ErrorCode::SessionNotFound,
                format!("Session {} not found", session_filter),
            );
        }
    } else {
        // Try by name
        match session_manager.get_session_by_name(&session_filter) {
            Some(session) => session.id(),
            None => {
                return HandlerContext::error(
                    ErrorCode::SessionNotFound,
                    format!("Session '{}' not found", session_filter),
                );
            }
        }
    };

    // Get the session and set the metadata
    let session_name = if let Some(session) = session_manager.get_session_mut(session_id) {
        session.set_metadata(&key, &value);
        session.name().to_string()
    } else {
        return HandlerContext::error(
            ErrorCode::SessionNotFound,
            format!("Session {} not found", session_id),
        );
    };

    // Release lock before persistence call
    drop(session_manager);

    // Log to persistence (BUG-031: metadata must persist across restarts)
    if let Some(persistence_lock) = &ctx.persistence {
        let persistence = persistence_lock.read().await;
        if let Ok(seq) = persistence.log_session_metadata_set(session_id, &key, &value) {
            persistence.push_replay(seq, ServerMessage::MetadataSet {
                session_id,
                session_name: session_name.clone(),
                key: key.clone(),
                value: value.clone(),
            });
        }
    }

    HandlerResult::Response(ServerMessage::MetadataSet {
        session_id,
        session_name,
        key,
        value,
    })
}

/// Handle GetMetadata - get metadata from a session
pub async fn get_metadata(
    ctx: &HandlerContext,
    session_filter: String,
    key: Option<String>,
) -> HandlerResult {
    debug!(
        "GetMetadata request from {}: session={}, key={:?}",
        ctx.client_id, session_filter, key
    );

    let session_manager = ctx.session_manager.read().await;

    // Find the session by UUID or name
    let session = if let Ok(uuid) = Uuid::parse_str(&session_filter) {
        session_manager.get_session(uuid)
    } else {
        session_manager.get_session_by_name(&session_filter)
    };

    let session = match session {
        Some(s) => s,
        None => {
            return HandlerContext::error(
                ErrorCode::SessionNotFound,
                format!("Session '{}' not found", session_filter),
            );
        }
    };

    let session_id = session.id();
    let session_name = session.name().to_string();

    // Get metadata - either specific key or all
    let metadata = if let Some(ref k) = key {
        // Get single key
        let mut meta = std::collections::HashMap::new();
        if let Some(v) = session.get_metadata(k) {
            meta.insert(k.clone(), v.clone());
        }
        meta
    } else {
        // Get all
        session.all_metadata().clone()
    };

    HandlerResult::Response(ServerMessage::MetadataList {
        session_id,
        session_name,
        metadata,
    })
}

// ==================== FEAT-048: Orchestration Tag Handlers ====================

/// Handle SetTags - add or remove tags on a session
pub async fn set_tags(
    ctx: &HandlerContext,
    session_filter: Option<String>,
    add: Vec<String>,
    remove: Vec<String>,
) -> HandlerResult {
    info!(
        "SetTags request from {}: session={:?}, add={:?}, remove={:?}",
        ctx.client_id, session_filter, add, remove
    );

    let mut session_manager = ctx.session_manager.write().await;

    // Find the session by UUID or name
    let session_id = if let Some(ref filter) = session_filter {
        if let Ok(uuid) = Uuid::parse_str(filter) {
            if session_manager.get_session(uuid).is_some() {
                uuid
            } else {
                return HandlerContext::error(
                    ErrorCode::SessionNotFound,
                    format!("Session {} not found", filter),
                );
            }
        } else {
            // Try by name
            match session_manager.get_session_by_name(filter) {
                Some(session) => session.id(),
                None => {
                    return HandlerContext::error(
                        ErrorCode::SessionNotFound,
                        format!("Session '{}' not found", filter),
                    );
                }
            }
        }
    } else {
        // Use active session if not specified (BUG-034 fix)
        match session_manager.active_session_id() {
            Some(id) => id,
            None => {
                return HandlerContext::error(
                    ErrorCode::SessionNotFound,
                    "No sessions exist",
                );
            }
        }
    };

    // Get the session and modify tags
    let (session_name, tags) = if let Some(session) = session_manager.get_session_mut(session_id) {
        // Add tags
        for tag in add {
            session.add_tag(tag);
        }
        // Remove tags
        for tag in &remove {
            session.remove_tag(tag);
        }
        (session.name().to_string(), session.tags().clone())
    } else {
        return HandlerContext::error(
            ErrorCode::SessionNotFound,
            format!("Session {} not found", session_id),
        );
    };

    // Broadcast updated session list to all clients (BUG-032)
    let sessions: Vec<_> = session_manager.list_sessions().iter().map(|s| s.to_info()).collect();

    HandlerResult::ResponseWithGlobalBroadcast {
        response: ServerMessage::TagsSet {
            session_id,
            session_name,
            tags,
        },
        broadcast: ServerMessage::SessionsChanged { sessions },
    }
}

/// Handle GetTags - get tags from a session
pub async fn get_tags(
    ctx: &HandlerContext,
    session_filter: Option<String>,
) -> HandlerResult {
    debug!(
        "GetTags request from {}: session={:?}",
        ctx.client_id, session_filter
    );

    let session_manager = ctx.session_manager.read().await;

    // Find the session by UUID or name
    let session = if let Some(ref filter) = session_filter {
        if let Ok(uuid) = Uuid::parse_str(filter) {
            session_manager.get_session(uuid)
        } else {
            session_manager.get_session_by_name(filter)
        }
    } else {
        // Use active session if not specified (BUG-034 fix)
        session_manager.active_session()
    };

    let session = match session {
        Some(s) => s,
        None => {
            return HandlerContext::error(
                ErrorCode::SessionNotFound,
                session_filter
                    .map(|s| format!("Session '{}' not found", s))
                    .unwrap_or_else(|| "No sessions exist".to_string()),
            );
        }
    };

    let session_id = session.id();
    let session_name = session.name().to_string();
    let tags = session.tags().clone();

    HandlerResult::Response(ServerMessage::TagsList {
        session_id,
        session_name,
        tags,
    })
}
