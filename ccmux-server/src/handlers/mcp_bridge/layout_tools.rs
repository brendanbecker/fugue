use tracing::{info};
use uuid::Uuid;
use ccmux_protocol::{
    ErrorCode, ServerMessage, SplitDirection,
};

use crate::pty::{PtyConfig, PtyOutputPoller};
use crate::session::SessionManager;
use crate::arbitration::{Action, Resource};
use crate::handlers::{HandlerContext, HandlerResult};

/// Handle CreateLayout - create a complex layout declaratively
///
/// BUG-028 FIX: This function uses a two-phase approach to avoid deadlock:
/// 1. Phase 1 (holding session_manager lock): Create all panes, collect their configs
/// 2. Phase 2 (after releasing lock): Spawn all PTYs
///
/// The previous implementation spawned PTYs while holding the session_manager lock,
/// which could deadlock if the PTY output poller tried to execute a sideband command
/// (sideband commands require session_manager.write()).
pub async fn create_layout(
    ctx: &HandlerContext,
    session_filter: Option<String>,
    window_filter: Option<String>,
    layout: serde_json::Value,
) -> HandlerResult {
    info!(
        "CreateLayout request from {} (session: {:?}, window: {:?})",
        ctx.client_id, session_filter, window_filter
    );

    // Phase 1: Create panes while holding session_manager lock
    // Collect PTY configs for spawning after releasing lock
    // BUG-032: Also collect PaneInfo for TUI broadcast
    let (session_id, session_name, window_id, pane_ids, pane_infos, pty_configs) = {
        let mut session_manager = ctx.session_manager.write().await;

        // Find or use first session
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
            match ctx.resolve_active_session(&session_manager) {
                Some(id) => id,
                None => {
                    return HandlerContext::error(ErrorCode::SessionNotFound, "No sessions exist");
                }
            }
        };

        let session_name = session_manager
            .get_session(session_id)
            .map(|s| s.name().to_string())
            .unwrap_or_default();

        // Find or use first window
        let session = match session_manager.get_session_mut(session_id) {
            Some(s) => s,
            None => {
                return HandlerContext::error(ErrorCode::SessionNotFound, "Session disappeared");
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
            match ctx.resolve_active_window(session) {
                Some(id) => id,
                None => {
                    // Check for existing window first, then create if needed
                    let existing_id = session.windows().next().map(|w| w.id());
                    match existing_id {
                        Some(id) => id,
                        None => session.create_window(None).id(),
                    }
                }
            }
        };

        // FEAT-079: Arbitrate layout access
        if let Err(blocked) = ctx.check_arbitration(Resource::Window(window_id), Action::Layout) {
            return blocked;
        }
        ctx.record_human_activity(Resource::Window(window_id), Action::Layout);

        // Parse and create layout, collecting PTY configs for later spawning
        // BUG-032: Also collect PaneInfo for TUI broadcast
        let mut pane_ids = Vec::new();
        let mut pane_infos = Vec::new();
        let mut pty_configs = Vec::new();
        let result = create_layout_panes(
            &mut session_manager,
            session_id,
            &session_name,
            window_id,
            &layout,
            &mut pane_ids,
            &mut pane_infos,
            &mut pty_configs,
        );

        if let Err(e) = result {
            return HandlerContext::error(ErrorCode::InvalidOperation, e);
        }

        (session_id, session_name, window_id, pane_ids, pane_infos, pty_configs)
    }; // session_manager lock released here

    // Phase 2: Spawn PTYs without holding session_manager lock (BUG-028 fix)
    // This prevents deadlock if PTY output poller tries to access session_manager
    {
        let mut pty_manager = ctx.pty_manager.write().await;
        for (pane_id, config) in pty_configs {
            if let Ok(handle) = pty_manager.spawn(pane_id, config) {
                let reader = handle.clone_reader();
                let _poller_handle = PtyOutputPoller::spawn_with_sideband(
                    pane_id,
                    session_id,
                    reader,
                    ctx.registry.clone(),
                    Some(ctx.pane_closed_tx.clone()),
                    ctx.command_executor.clone(),
                );
            }
        }
    }

    info!(
        "Layout created in session {} window {} with {} panes",
        session_name, window_id, pane_ids.len()
    );

    // Broadcast PaneCreated for all new panes to notify TUI of layout changes (BUG-032)
    for pane_info in pane_infos {
        ctx.registry.broadcast_to_session_except(
            session_id,
            ctx.client_id,
            ServerMessage::PaneCreated {
                pane: pane_info,
                direction: SplitDirection::Vertical,
                should_focus: false,
            }
        ).await;
    }

    // Return response to MCP client
    HandlerResult::Response(ServerMessage::LayoutCreated {
        session_id,
        session_name,
        window_id,
        pane_ids,
    })
}

/// Recursively create panes from layout specification (Phase 1 of create_layout)
///
/// This is a synchronous function that creates panes and collects PTY configs.
/// It does NOT spawn PTYs - that happens in Phase 2 after the lock is released.
/// BUG-032: Also collects PaneInfo for TUI broadcast.
fn create_layout_panes(
    session_manager: &mut SessionManager,
    session_id: Uuid,
    session_name: &str,
    window_id: Uuid,
    layout: &serde_json::Value,
    pane_ids: &mut Vec<Uuid>,
    pane_infos: &mut Vec<ccmux_protocol::PaneInfo>,
    pty_configs: &mut Vec<(Uuid, PtyConfig)>,
) -> Result<(), String> {
    // Check if this is a simple pane definition
    if layout.get("pane").is_some() {
        let pane_spec = &layout["pane"];
        let command = pane_spec["command"].as_str().map(String::from);
        let cwd = pane_spec["cwd"].as_str().map(String::from);

        // Create the pane
        let session = session_manager
            .get_session_mut(session_id)
            .ok_or("Session not found")?;
        let window = session
            .get_window_mut(window_id)
            .ok_or("Window not found")?;

        let pane = window.create_pane();
        let pane_id = pane.id();
        let pane_info = pane.to_info();

        // Initialize parser
        let pane = window.get_pane_mut(pane_id).ok_or("Pane disappeared")?;
        pane.init_parser();

        pane_ids.push(pane_id);
        pane_infos.push(pane_info);

        // Get session environment for PTY config
        let session_env = session_manager
            .get_session(session_id)
            .map(|s| s.environment().clone())
            .unwrap_or_default();

        // Build PTY config (will be spawned in Phase 2)
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".into());
        let mut config = if let Some(ref cmd) = command {
            PtyConfig::command("sh").with_arg("-c").with_arg(cmd)
        } else {
            PtyConfig::command(&shell)
        };
        if let Some(ref cwd) = cwd {
            config = config.with_cwd(cwd);
        }
        config = config.with_ccmux_context(session_id, session_name, window_id, pane_id);
        config = config.with_env_map(&session_env);

        // Store config for later PTY spawning (Phase 2)
        pty_configs.push((pane_id, config));

        return Ok(())
    }

    // Check if this is a split definition
    if let Some(splits) = layout.get("splits").and_then(|s| s.as_array()) {
        for split in splits {
            let nested_layout = split.get("layout").ok_or_else(|| {
                "Each split must have a 'layout' field".to_string()
            })?;
            // Recursively create panes for nested layouts
            create_layout_panes(
                session_manager,
                session_id,
                session_name,
                window_id,
                nested_layout,
                pane_ids,
                pane_infos,
                pty_configs,
            )?;
        }
        return Ok(())
    }

    Err("Invalid layout specification: must contain 'pane' or 'splits'".to_string())
}
