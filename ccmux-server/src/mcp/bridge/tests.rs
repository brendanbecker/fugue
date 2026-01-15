#[cfg(test)]
mod tests {
    use uuid::Uuid;
    use ccmux_protocol::{ServerMessage, PaneListEntry, PaneState, ViewportState, SplitDirection, WindowInfo, PaneInfo, ClaudeState, ErrorCode, OrchestrationMessage};
    use crate::mcp::bridge::connection::ConnectionManager;
    use crate::mcp::bridge::handlers::{parse_uuid, format_pane_list};
    use crate::mcp::bridge::types::{
        HEARTBEAT_INTERVAL_MS, HEARTBEAT_TIMEOUT_MS, RECONNECT_DELAYS_MS, MAX_RECONNECT_ATTEMPTS, DAEMON_RESPONSE_TIMEOUT_SECS, ConnectionState
    };
    use crate::mcp::error::McpError;
    use crate::beads::metadata_keys as beads;

    #[test]
    fn test_parse_uuid_valid() {
        let id = Uuid::new_v4();
        let args = serde_json::json!({"pane_id": id.to_string()});

        let result = parse_uuid(&args, "pane_id").unwrap();
        assert_eq!(result, id);
    }

    #[test]
    fn test_parse_uuid_missing() {
        let args = serde_json::json!({});
        let result = parse_uuid(&args, "pane_id");

        assert!(matches!(result, Err(McpError::InvalidParams(_))));
    }

    #[test]
    fn test_parse_uuid_invalid() {
        let args = serde_json::json!({"pane_id": "not-a-uuid"});
        let result = parse_uuid(&args, "pane_id");

        assert!(matches!(result, Err(McpError::InvalidParams(_))));
    }

    #[test]
    fn test_format_pane_list_empty() {
        let panes = vec![];
        let result = format_pane_list(&panes);
        assert!(result.is_empty());
    }

    // ==================== BUG-027 Fix Tests ====================

    #[test]
    fn test_is_broadcast_message_output() {
        let msg = ServerMessage::Output {
            pane_id: Uuid::new_v4(),
            data: vec![b'h', b'i'],
        };
        assert!(ConnectionManager::is_broadcast_message(&msg));
    }

    #[test]
    fn test_is_broadcast_message_pane_state_changed() {
        let msg = ServerMessage::PaneStateChanged {
            pane_id: Uuid::new_v4(),
            state: PaneState::Normal,
        };
        assert!(ConnectionManager::is_broadcast_message(&msg));
    }

    #[test]
    fn test_is_broadcast_message_claude_state_changed() {
        let msg = ServerMessage::ClaudeStateChanged {
            pane_id: Uuid::new_v4(),
            state: ClaudeState::default(),
        };
        assert!(ConnectionManager::is_broadcast_message(&msg));
    }

    #[test]
    fn test_is_broadcast_message_pane_created() {
        let msg = ServerMessage::PaneCreated {
            pane: PaneInfo {
                id: Uuid::new_v4(),
                window_id: Uuid::new_v4(),
                index: 0,
                cols: 80,
                rows: 24,
                state: PaneState::Normal,
                name: None,
                title: None,
                cwd: None,
            },
            direction: SplitDirection::Horizontal,
        };
        assert!(ConnectionManager::is_broadcast_message(&msg));
    }

    #[test]
    fn test_is_broadcast_message_window_created() {
        let msg = ServerMessage::WindowCreated {
            window: WindowInfo {
                id: Uuid::new_v4(),
                session_id: Uuid::new_v4(),
                name: "test".to_string(),
                index: 0,
                pane_count: 1,
                active_pane_id: None,
            },
        };
        assert!(ConnectionManager::is_broadcast_message(&msg));
    }

    #[test]
    fn test_is_broadcast_message_window_closed() {
        let msg = ServerMessage::WindowClosed {
            window_id: Uuid::new_v4(),
        };
        assert!(ConnectionManager::is_broadcast_message(&msg));
    }

    #[test]
    fn test_is_broadcast_message_session_ended() {
        let msg = ServerMessage::SessionEnded {
            session_id: Uuid::new_v4(),
        };
        assert!(ConnectionManager::is_broadcast_message(&msg));
    }

    #[test]
    fn test_is_broadcast_message_viewport_updated() {
        let msg = ServerMessage::ViewportUpdated {
            pane_id: Uuid::new_v4(),
            state: ViewportState::new(),
        };
        assert!(ConnectionManager::is_broadcast_message(&msg));
    }

    #[test]
    fn test_is_broadcast_message_orchestration_received() {
        let msg = ServerMessage::OrchestrationReceived {
            from_session_id: Uuid::new_v4(),
            message: OrchestrationMessage::new("sync.request", serde_json::json!({})),
        };
        assert!(ConnectionManager::is_broadcast_message(&msg));
    }

    #[test]
    fn test_is_not_broadcast_session_list() {
        let msg = ServerMessage::SessionList { sessions: vec![] };
        assert!(!ConnectionManager::is_broadcast_message(&msg));
    }

    #[test]
    fn test_is_not_broadcast_pane_content() {
        let msg = ServerMessage::PaneContent {
            pane_id: Uuid::new_v4(),
            content: "test".to_string(),
        };
        assert!(!ConnectionManager::is_broadcast_message(&msg));
    }

    #[test]
    fn test_is_not_broadcast_error() {
        let msg = ServerMessage::Error {
            code: ErrorCode::PaneNotFound,
            message: "Pane not found".to_string(),
        };
        assert!(!ConnectionManager::is_broadcast_message(&msg));
    }

    #[test]
    fn test_is_broadcast_message_pane_closed() {
        // BUG-035 fix: PaneClosed is now filtered as a broadcast
        let msg = ServerMessage::PaneClosed {
            pane_id: Uuid::new_v4(),
            exit_code: Some(0),
        };
        assert!(ConnectionManager::is_broadcast_message(&msg));
    }

    #[test]
    fn test_is_not_broadcast_connected() {
        let msg = ServerMessage::Connected {
            server_version: "1.0.0".to_string(),
            protocol_version: 1,
        };
        assert!(!ConnectionManager::is_broadcast_message(&msg));
    }

    // ==================== FEAT-060: Connection State Tests ====================

    #[test]
    fn test_connection_state_enum_equality() {
        assert_eq!(ConnectionState::Connected, ConnectionState::Connected);
        assert_eq!(ConnectionState::Disconnected, ConnectionState::Disconnected);
        assert_eq!(
            ConnectionState::Reconnecting { attempt: 1 },
            ConnectionState::Reconnecting { attempt: 1 }
        );
        assert_ne!(
            ConnectionState::Reconnecting { attempt: 1 },
            ConnectionState::Reconnecting { attempt: 2 }
        );
        assert_ne!(ConnectionState::Connected, ConnectionState::Disconnected);
    }

    #[test]
    fn test_reconnect_delays_exponential() {
        assert_eq!(RECONNECT_DELAYS_MS, &[100, 200, 400, 800, 1600]);
        for i in 1..RECONNECT_DELAYS_MS.len() {
            assert_eq!(RECONNECT_DELAYS_MS[i], RECONNECT_DELAYS_MS[i - 1] * 2);
        }
    }

    #[test]
    fn test_heartbeat_constants() {
        assert_eq!(HEARTBEAT_INTERVAL_MS, 1000);
        assert_eq!(HEARTBEAT_TIMEOUT_MS, 2000);
    }

    #[test]
    fn test_max_reconnect_attempts() {
        assert_eq!(MAX_RECONNECT_ATTEMPTS, 5);
    }

    #[test]
    fn test_daemon_response_timeout_constant() {
        assert_eq!(DAEMON_RESPONSE_TIMEOUT_SECS, 25);
    }

    #[test]
    fn test_is_broadcast_message_pong() {
        let msg = ServerMessage::Pong;
        assert!(ConnectionManager::is_broadcast_message(&msg));
    }

    #[test]
    fn test_is_broadcast_message_session_focused() {
        let msg = ServerMessage::SessionFocused {
            session_id: Uuid::new_v4(),
        };
        assert!(ConnectionManager::is_broadcast_message(&msg));
    }

    #[test]
    fn test_is_broadcast_message_window_focused() {
        let msg = ServerMessage::WindowFocused {
            session_id: Uuid::new_v4(),
            window_id: Uuid::new_v4(),
        };
        assert!(ConnectionManager::is_broadcast_message(&msg));
    }

    #[test]
    fn test_is_broadcast_message_pane_focused() {
        let msg = ServerMessage::PaneFocused {
            session_id: Uuid::new_v4(),
            window_id: Uuid::new_v4(),
            pane_id: Uuid::new_v4(),
        };
        assert!(ConnectionManager::is_broadcast_message(&msg));
    }

    #[test]
    fn test_beads_metadata_key_constants() {
        assert_eq!(beads::CURRENT_ISSUE, "beads.current_issue");
        assert_eq!(beads::ASSIGNED_AT, "beads.assigned_at");
        assert_eq!(beads::ISSUE_HISTORY, "beads.issue_history");
    }

    // ==================== BUG-033: Layout String Parsing Tests ====================

    #[test]
    fn test_layout_string_parsing_bug033() {
        let layout_object = serde_json::json!({"pane": {}});
        assert!(layout_object.get("pane").is_some());

        let layout_string = serde_json::Value::String(r#"{"pane": {}}"#.to_string());
        assert!(layout_string.get("pane").is_none());

        let parsed = match &layout_string {
            serde_json::Value::String(s) => serde_json::from_str::<serde_json::Value>(s).unwrap(),
            other => other.clone(),
        };
        assert!(parsed.get("pane").is_some());
    }
}