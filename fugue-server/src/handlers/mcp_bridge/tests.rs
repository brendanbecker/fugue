use fugue_protocol::{ClientType, messages::ErrorDetails, ServerMessage, ErrorCode, SplitDirection};
use crate::pty::PtyManager;
use crate::registry::ClientRegistry;
use crate::session::SessionManager;
use crate::arbitration::{Arbitrator, Resource, Action};
use crate::handlers::{HandlerContext, HandlerResult};
use crate::watchdog::WatchdogManager;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use uuid::Uuid;

fn create_test_context() -> HandlerContext {
    let session_manager = Arc::new(RwLock::new(SessionManager::new()));
    let pty_manager = Arc::new(RwLock::new(PtyManager::new()));
    let registry = Arc::new(ClientRegistry::new());
    let config = Arc::new(crate::config::AppConfig::default());
    let arbitrator = Arc::new(Arbitrator::new());
    let command_executor = Arc::new(crate::sideband::AsyncCommandExecutor::new(
        Arc::clone(&session_manager),
        Arc::clone(&pty_manager),
        Arc::clone(&registry),
    ));
    let watchdog = Arc::new(WatchdogManager::new());

    let (tx, _rx) = mpsc::channel(10);
    let client_id = registry.register_client(tx);

    let (pane_closed_tx, _) = mpsc::channel(10);
    HandlerContext::new(
        session_manager,
        pty_manager,
        registry,
        config,
        client_id,
        pane_closed_tx,
        command_executor,
        arbitrator,
        None,
        watchdog,
    )
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
    let result = ctx.handle_create_session_with_options(Some("my-session".to_string()), None, None, None, None, None).await;

    match result {
        HandlerResult::ResponseWithGlobalBroadcast {
            response: ServerMessage::SessionCreatedWithDetails {
                session_name,
                .. 
            },
            broadcast: ServerMessage::SessionsChanged { sessions },
        } => {
            assert_eq!(session_name, "my-session");
            assert_eq!(sessions.len(), 1);
        }
        _ => panic!("Expected SessionCreatedWithDetails response with global broadcast"),
    }
}

#[tokio::test]
async fn test_create_session_with_auto_name() {
    let ctx = create_test_context();
    let result = ctx.handle_create_session_with_options(None, None, None, None, None, None).await;

    match result {
        HandlerResult::ResponseWithGlobalBroadcast {
            response: ServerMessage::SessionCreatedWithDetails {
                session_name,
                .. 
            },
            broadcast: ServerMessage::SessionsChanged { sessions },
        } => {
            assert!(session_name.starts_with("session-"));
            assert_eq!(sessions.len(), 1);
        }
        _ => panic!("Expected SessionCreatedWithDetails response with global broadcast"),
    }
}

#[tokio::test]
async fn test_create_window_with_options_no_sessions() {
    let ctx = create_test_context();
    let result = ctx.handle_create_window_with_options(None, None, None, None).await;

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
        .handle_create_window_with_options(None, Some("new-window".to_string()), None, None)
        .await;

    match result {
        HandlerResult::ResponseWithBroadcast {
            response: ServerMessage::WindowCreatedWithDetails {
                session_name,
                .. 
            },
            broadcast: ServerMessage::PaneCreated { pane, .. },
            ..
        } => {
            assert_eq!(session_name, "test");
            // Verify broadcast contains pane info (BUG-032)
            assert!(pane.id != Uuid::nil());
        }
        _ => panic!("Expected WindowCreatedWithDetails response with broadcast"),
    }
}

#[tokio::test]
async fn test_create_pane_with_options_creates_session() {
    let ctx = create_test_context();

    // No sessions exist, should create default
    let result = ctx
        .handle_create_pane_with_options(None, None, SplitDirection::Vertical, None, None, false, None, None, None, None)
        .await;

    match result {
        HandlerResult::ResponseWithBroadcast {
            response: ServerMessage::PaneCreatedWithDetails {
                session_name,
                direction,
                .. 
            },
            broadcast: ServerMessage::PaneCreated { pane, direction: broadcast_dir, .. },
            ..
        } => {
            assert_eq!(session_name, "default");
            assert_eq!(direction, "vertical");
            // Verify broadcast contains pane info and direction
            assert!(pane.id != Uuid::nil());
            assert_eq!(broadcast_dir, SplitDirection::Vertical);
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
    let arbitrator = Arc::new(Arbitrator::new());
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
    let watchdog = Arc::new(WatchdogManager::new());
    let mcp_ctx = HandlerContext::new(
        Arc::clone(&session_manager),
        Arc::clone(&pty_manager),
        Arc::clone(&registry),
        Arc::clone(&config),
        mcp_client_id,
        pane_closed_tx,
        Arc::clone(&command_executor),
        Arc::clone(&arbitrator),
        None,
        watchdog,
    );

    // MCP creates a pane (uses first session since no filter provided)
    let result = mcp_ctx
        .handle_create_pane_with_options(None, None, SplitDirection::Vertical, None, None, false, None, None, None, None)
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
        ServerMessage::PaneCreated { pane, direction: _, .. } => {
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
    let arbitrator = Arc::new(Arbitrator::new());
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
    let watchdog = Arc::new(WatchdogManager::new());
    let mcp_ctx = HandlerContext::new(
        Arc::clone(&session_manager),
        Arc::clone(&pty_manager),
        Arc::clone(&registry),
        Arc::clone(&config),
        mcp_client_id,
        pane_closed_tx,
        Arc::clone(&command_executor),
        Arc::clone(&arbitrator),
        None,
        watchdog,
    );

    // MCP creates a pane, explicitly targeting session A
    let result = mcp_ctx
        .handle_create_pane_with_options(
            Some(session_a_id.to_string()),
            None,
            SplitDirection::Vertical,
            None,
            None,
            false,
            None,
            None,
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

// ==================== BUG-032: Split Pane Broadcast Tests ====================

/// Test that MCP split_pane broadcasts PaneCreated to TUI clients
#[tokio::test]
async fn test_mcp_split_pane_broadcasts_to_tui() {
    let session_manager = Arc::new(RwLock::new(SessionManager::new()));
    let pty_manager = Arc::new(RwLock::new(PtyManager::new()));
    let registry = Arc::new(ClientRegistry::new());
    let config = Arc::new(crate::config::AppConfig::default());
    let arbitrator = Arc::new(Arbitrator::new());
    let command_executor = Arc::new(crate::sideband::AsyncCommandExecutor::new(
        Arc::clone(&session_manager),
        Arc::clone(&pty_manager),
        Arc::clone(&registry),
    ));
    let (pane_closed_tx, _) = mpsc::channel(10);

    // Create a session with a pane
    let (session_id, pane_id) = {
        let mut sm = session_manager.write().await;
        let session = sm.create_session("test-session").unwrap();
        let session_id = session.id();

        let session = sm.get_session_mut(session_id).unwrap();
        let window = session.create_window(Some("main".to_string()));
        let window_id = window.id();
        let window = session.get_window_mut(window_id).unwrap();
        let pane_id = window.create_pane().id();

        (session_id, pane_id)
    };

    // Register TUI client and attach to session
    let (tui_tx, mut tui_rx) = mpsc::channel(10);
    let tui_client_id = registry.register_client(tui_tx);
    registry.attach_to_session(tui_client_id, session_id);

    // Register MCP client (not attached)
    let (mcp_tx, _mcp_rx) = mpsc::channel(10);
    let mcp_client_id = registry.register_client(mcp_tx);

    // Create handler context for MCP client
    let watchdog = Arc::new(WatchdogManager::new());
    let mcp_ctx = HandlerContext::new(
        Arc::clone(&session_manager),
        Arc::clone(&pty_manager),
        Arc::clone(&registry),
        Arc::clone(&config),
        mcp_client_id,
        pane_closed_tx,
        Arc::clone(&command_executor),
        Arc::clone(&arbitrator),
        None,
        watchdog,
    );

    // MCP splits the pane
    let result = mcp_ctx
        .handle_split_pane(pane_id, SplitDirection::Horizontal, 0.5, None, None, false)
        .await;

    // Extract broadcast info
    let (broadcast_session_id, broadcast_msg) = match result {
        HandlerResult::ResponseWithBroadcast {
            session_id: sid,
            broadcast,
            response: ServerMessage::PaneSplit { new_pane_id, .. },
            ..
        } => {
            assert!(new_pane_id != Uuid::nil());
            (sid, broadcast)
        }
        _ => panic!("Expected ResponseWithBroadcast with PaneSplit"),
    };

    // Verify session_id matches
    assert_eq!(broadcast_session_id, session_id);

    // Broadcast to TUI
    let broadcast_count = registry
        .broadcast_to_session_except(broadcast_session_id, mcp_client_id, broadcast_msg)
        .await;

    assert_eq!(broadcast_count, 1, "Should broadcast to TUI client");

    // Verify TUI received PaneCreated
    match tui_rx.try_recv() {
        Ok(ServerMessage::PaneCreated { pane, direction, .. }) => {
            assert!(pane.id != Uuid::nil());
            assert_eq!(direction, SplitDirection::Horizontal);
        }
        msg => panic!("Expected PaneCreated, got {:?}", msg),
    }
}

/// Test that MCP resize_pane_delta broadcasts PaneResized to TUI clients
#[tokio::test]
async fn test_mcp_resize_pane_broadcasts_to_tui() {
    let session_manager = Arc::new(RwLock::new(SessionManager::new()));
    let pty_manager = Arc::new(RwLock::new(PtyManager::new()));
    let registry = Arc::new(ClientRegistry::new());
    let config = Arc::new(crate::config::AppConfig::default());
    let arbitrator = Arc::new(Arbitrator::new());
    let command_executor = Arc::new(crate::sideband::AsyncCommandExecutor::new(
        Arc::clone(&session_manager),
        Arc::clone(&pty_manager),
        Arc::clone(&registry),
    ));
    let (pane_closed_tx, _) = mpsc::channel(10);

    // Create a session with a pane
    let (session_id, pane_id) = {
        let mut sm = session_manager.write().await;
        let session = sm.create_session("test-session").unwrap();
        let session_id = session.id();

        let session = sm.get_session_mut(session_id).unwrap();
        let window = session.create_window(Some("main".to_string()));
        let window_id = window.id();
        let window = session.get_window_mut(window_id).unwrap();
        let pane_id = window.create_pane().id();

        (session_id, pane_id)
    };

    // Register TUI client and attach to session
    let (tui_tx, mut tui_rx) = mpsc::channel(10);
    let tui_client_id = registry.register_client(tui_tx);
    registry.attach_to_session(tui_client_id, session_id);

    // Register MCP client (not attached)
    let (mcp_tx, _mcp_rx) = mpsc::channel(10);
    let mcp_client_id = registry.register_client(mcp_tx);

    // Create handler context for MCP client
    let watchdog = Arc::new(WatchdogManager::new());
    let mcp_ctx = HandlerContext::new(
        Arc::clone(&session_manager),
        Arc::clone(&pty_manager),
        Arc::clone(&registry),
        Arc::clone(&config),
        mcp_client_id,
        pane_closed_tx,
        Arc::clone(&command_executor),
        Arc::clone(&arbitrator),
        None,
        watchdog,
    );

    // MCP resizes the pane
    let result = mcp_ctx.handle_resize_pane_delta(pane_id, 0.2).await;

    // Extract broadcast info
    let (broadcast_session_id, broadcast_msg) = match result {
        HandlerResult::ResponseWithBroadcast {
            session_id: sid,
            broadcast,
            response: ServerMessage::PaneResized { pane_id: pid, .. },
            ..
        } => {
            assert_eq!(pid, pane_id);
            (sid, broadcast)
        }
        _ => panic!("Expected ResponseWithBroadcast with PaneResized"),
    };

    // Verify session_id matches
    assert_eq!(broadcast_session_id, session_id);

    // Broadcast to TUI
    let broadcast_count = registry
        .broadcast_to_session_except(broadcast_session_id, mcp_client_id, broadcast_msg)
        .await;

    assert_eq!(broadcast_count, 1, "Should broadcast to TUI client");

    // Verify TUI received PaneResized
    match tui_rx.try_recv() {
        Ok(ServerMessage::PaneResized { pane_id: pid, new_cols, new_rows }) => {
            assert_eq!(pid, pane_id);
            // Verify dimensions changed (original is 80x24, delta is 0.2 so scale is 1.2)
            assert!(new_cols > 0);
            assert!(new_rows > 0);
        }
        msg => panic!("Expected PaneResized, got {:?}", msg),
    }
}

// ==================== FEAT-048: Orchestration Tag Tests ====================

#[tokio::test]
async fn test_set_tags_no_sessions() {
    let ctx = create_test_context();
    let result = ctx
        .handle_set_tags(None, vec!["worker".to_string()], vec![])
        .await;

    match result {
        HandlerResult::Response(ServerMessage::Error { code, .. }) => {
            assert_eq!(code, ErrorCode::SessionNotFound);
        }
        _ => panic!("Expected Error response"),
    }
}

#[tokio::test]
async fn test_set_tags_add_success() {
    let ctx = create_test_context();
    create_session_with_pane(&ctx).await;

    let result = ctx
        .handle_set_tags(None, vec!["orchestrator".to_string(), "primary".to_string()], vec![])
        .await;

    match result {
        HandlerResult::ResponseWithGlobalBroadcast {
            response: ServerMessage::TagsSet { tags, .. },
            broadcast: ServerMessage::SessionsChanged { sessions },
        } => {
            assert!(tags.contains("orchestrator"));
            assert!(tags.contains("primary"));
            assert_eq!(sessions.len(), 1);
        }
        _ => panic!("Expected TagsSet response with global broadcast"),
    }
}

#[tokio::test]
async fn test_set_tags_add_and_remove() {
    let ctx = create_test_context();
    create_session_with_pane(&ctx).await;

    // First add some tags
    ctx.handle_set_tags(None, vec!["a".to_string(), "b".to_string(), "c".to_string()], vec![])
        .await;

    // Now remove one and add another
    let result = ctx
        .handle_set_tags(None, vec!["d".to_string()], vec!["b".to_string()])
        .await;

    match result {
        HandlerResult::ResponseWithGlobalBroadcast {
            response: ServerMessage::TagsSet { tags, .. },
            broadcast: ServerMessage::SessionsChanged { sessions },
        } => {
            assert!(tags.contains("a"));
            assert!(tags.contains("c"));
            assert!(tags.contains("d"));
            assert!(!tags.contains("b"));
            assert_eq!(sessions.len(), 1);
        }
        _ => panic!("Expected TagsSet response with global broadcast"),
    }
}

#[tokio::test]
async fn test_get_tags_no_sessions() {
    let ctx = create_test_context();
    let result = ctx.handle_get_tags(None).await;

    match result {
        HandlerResult::Response(ServerMessage::Error { code, .. }) => {
            assert_eq!(code, ErrorCode::SessionNotFound);
        }
        _ => panic!("Expected Error response"),
    }
}

#[tokio::test]
async fn test_get_tags_empty() {
    let ctx = create_test_context();
    create_session_with_pane(&ctx).await;

    let result = ctx.handle_get_tags(None).await;

    match result {
        HandlerResult::Response(ServerMessage::TagsList {
            session_name,
            tags,
            .. 
        }) => {
            assert_eq!(session_name, "test");
            assert!(tags.is_empty());
        }
        _ => panic!("Expected TagsList response"),
    }
}

#[tokio::test]
async fn test_get_tags_with_tags() {
    let ctx = create_test_context();
    create_session_with_pane(&ctx).await;

    // Add some tags first
    ctx.handle_set_tags(None, vec!["worker".to_string(), "stream-a".to_string()], vec![])
        .await;

    let result = ctx.handle_get_tags(None).await;

    match result {
        HandlerResult::Response(ServerMessage::TagsList {
            session_name,
            tags,
            .. 
        }) => {
            assert_eq!(session_name, "test");
            assert!(tags.contains("worker"));
            assert!(tags.contains("stream-a"));
            assert_eq!(tags.len(), 2);
        }
        _ => panic!("Expected TagsList response"),
    }
}

#[tokio::test]
async fn test_get_tags_by_name() {
    let ctx = create_test_context();
    create_session_with_pane(&ctx).await;

    // Add tags
    ctx.handle_set_tags(Some("test".to_string()), vec!["named".to_string()], vec![])
        .await;

    let result = ctx.handle_get_tags(Some("test".to_string())).await;

    match result {
        HandlerResult::Response(ServerMessage::TagsList { tags, .. }) => {
            assert!(tags.contains("named"));
        }
        _ => panic!("Expected TagsList response"),
    }
}

// ==================== CreateLayout Tests (BUG-028) ====================

#[tokio::test]
async fn test_handle_create_layout_simple_pane() {
    let ctx = create_test_context();

    // Create a session first
    let _ = ctx.handle_create_session_with_options(Some("test".to_string()), None, None, None, None, None).await;

    let layout = serde_json::json!({
        "pane": {"name": "test-pane"}
    });

    let result = ctx.handle_create_layout(Some("test".to_string()), None, layout).await;

    match result {
        HandlerResult::Response(ServerMessage::LayoutCreated { pane_ids, .. }) => {
            assert_eq!(pane_ids.len(), 1, "Should create exactly 1 pane");
        }
        HandlerResult::Response(ServerMessage::Error { code, message, .. }) => {
            panic!("Layout creation failed: {:?} - {}", code, message);
        }
        _ => panic!("Expected LayoutCreated response"),
    }
}

#[tokio::test]
async fn test_handle_create_layout_horizontal_split() {
    let ctx = create_test_context();

    // Create a session first
    let _ = ctx.handle_create_session_with_options(Some("test".to_string()), None, None, None, None, None).await;

    let layout = serde_json::json!({
        "direction": "horizontal",
        "splits": [
            {"ratio": 0.5, "layout": {"pane": {}}},
            {"ratio": 0.5, "layout": {"pane": {}}}
        ]
    });

    let result = ctx.handle_create_layout(Some("test".to_string()), None, layout).await;

    match result {
        HandlerResult::Response(ServerMessage::LayoutCreated { pane_ids, .. }) => {
            assert_eq!(pane_ids.len(), 2, "Should create exactly 2 panes");
        }
        HandlerResult::Response(ServerMessage::Error { code, message, .. }) => {
            panic!("Layout creation failed: {:?} - {}", code, message);
        }
        _ => panic!("Expected LayoutCreated response"),
    }
}

#[tokio::test]
async fn test_handle_create_layout_nested_bug028() {
    // This is the exact reproduction case from BUG-028
    let ctx = create_test_context();

    // Create a session first
    let _ = ctx.handle_create_session_with_options(Some("test".to_string()), None, None, None, None, None).await;

    let layout = serde_json::json!({
        "direction": "horizontal",
        "splits": [
            {
                "ratio": 0.7,
                "layout": {
                    "direction": "vertical",
                    "splits": [
                        {"ratio": 0.6, "layout": {"pane": {"name": "editor"}}},
                        {"ratio": 0.4, "layout": {"pane": {"name": "sidebar"}}}
                    ]
                }
            },
            {
                "ratio": 0.3,
                "layout": {"pane": {"name": "terminal"}}
            }
        ]
    });

    let result = ctx.handle_create_layout(Some("test".to_string()), None, layout).await;

    match result {
        HandlerResult::Response(ServerMessage::LayoutCreated { pane_ids, .. }) => {
            assert_eq!(pane_ids.len(), 3, "Should create exactly 3 panes");
        }
        HandlerResult::Response(ServerMessage::Error { code, message, .. }) => {
            panic!("Layout creation failed: {:?} - {}", code, message);
        }
        _ => panic!("Expected LayoutCreated response"),
    }
}

/// BUG-035: Stress test to verify response types remain consistent under load
///
/// This test runs 100+ MCP operations in sequence and verifies that each
/// response type matches the request type. The bug manifests as wrong wrapper
/// types after many operations (e.g., list_windows returning SessionList).
#[tokio::test]
async fn test_response_type_consistency_under_load_bug035() {
    let ctx = create_test_context();

    // Create a session with multiple windows and panes to have actual data
    let _ = ctx.handle_create_session_with_options(Some("stress-test".to_string()), None, None, None, None, None).await;
    let _ = ctx.handle_create_window_with_options(Some("stress-test".to_string()), Some("window-2".to_string()), None, None).await;
    let _ = ctx.handle_create_window_with_options(Some("stress-test".to_string()), Some("window-3".to_string()), None, None).await;

    // Track expected response types for each operation
    let mut errors: Vec<String> = Vec::new();

    // Run 150 operations in sequence
    for i in 0..150 {
        // Rotate through different operations
        match i % 3 {
            0 => {
                // ListSessions should return SessionList
                let result = ctx.handle_list_sessions().await;
                match result {
                    HandlerResult::Response(ServerMessage::SessionList { .. }) => {}
                    HandlerResult::Response(ServerMessage::Error { .. }) => {}
                    other => {
                        errors.push(format!(
                            "Iteration {}: ListSessions returned {:?} instead of SessionList",
                            i, 
                            std::mem::discriminant(&other)
                        ));
                    }
                }
            }
            1 => {
                // ListWindows should return WindowList
                let result = ctx.handle_list_windows(Some("stress-test".to_string())).await;
                match result {
                    HandlerResult::Response(ServerMessage::WindowList { .. }) => {}
                    HandlerResult::Response(ServerMessage::Error { .. }) => {}
                    other => {
                        errors.push(format!(
                            "Iteration {}: ListWindows returned {:?} instead of WindowList",
                            i,
                            std::mem::discriminant(&other)
                        ));
                    }
                }
            }
            2 => {
                // ListAllPanes should return AllPanesList
                let result = ctx.handle_list_all_panes(Some("stress-test".to_string())).await;
                match result {
                    HandlerResult::Response(ServerMessage::AllPanesList { .. }) => {}
                    HandlerResult::Response(ServerMessage::Error { .. }) => {}
                    other => {
                        errors.push(format!(
                            "Iteration {}: ListAllPanes returned {:?} instead of AllPanesList",
                            i,
                            std::mem::discriminant(&other)
                        ));
                    }
                }
            }
            _ => unreachable!()
        }
    }

    // Report all errors
    if !errors.is_empty() {
        panic!(
            "BUG-035: Response type mismatch detected after {} operations:\n{}",
            150,
            errors.join("\n")
        );
    }
}

#[tokio::test]
async fn test_create_pane_blocked_by_human_activity() {
    let ctx = create_test_context();
    let (_session_id, window_id, _pane_id) = create_session_with_pane(&ctx).await;

    // Create a separate "human" client context to record activity
    let (tx, _rx) = mpsc::channel(10);
    let human_id = ctx.registry.register_client(tx);
    ctx.registry.set_client_type(human_id, ClientType::Tui);
    
    // Record activity directly on the arbitrator as if it came from the human client
    ctx.arbitrator.record_activity(Resource::Window(window_id), Action::Layout);

    // Set the test context client to MCP (Agent)
    ctx.registry.set_client_type(ctx.client_id, ClientType::Mcp);

    // Try to create pane - should be blocked
    let result = ctx
        .handle_create_pane_with_options(None, None, SplitDirection::Vertical, None, None, false, None, None, None, None)
        .await;

    match result {
        HandlerResult::Response(ServerMessage::Error { code, message, details }) => {
            assert_eq!(code, ErrorCode::UserPriorityActive);
            assert!(message.contains("Human layout active"));
            match details {
                Some(ErrorDetails::HumanControl { remaining_ms }) => {
                    assert!(remaining_ms > 0);
                }
                _ => panic!("Expected HumanControl details"),
            }
        }
        _ => panic!("Expected UserPriorityActive error, got {:?}", std::mem::discriminant(&result)),
    }
}
