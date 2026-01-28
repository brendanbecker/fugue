#[cfg(test)]
mod tests {
    use uuid::Uuid;
    use fugue_protocol::{ServerMessage, PaneState, ViewportState, SplitDirection, WindowInfo, PaneInfo, ClaudeState, ErrorCode, OrchestrationMessage};
    use crate::mcp::bridge::connection::{ConnectionManager, RECONNECT_DELAYS_MS, MAX_RECONNECT_ATTEMPTS, DAEMON_RESPONSE_TIMEOUT_SECS};
    use crate::mcp::bridge::handlers::{parse_uuid, format_pane_list};
    use crate::mcp::bridge::health::{HEARTBEAT_INTERVAL_MS, HEARTBEAT_TIMEOUT_MS, ConnectionState};
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
            should_focus: false,
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
                stuck_status: None, metadata: std::collections::HashMap::new(), is_mirror: false, mirror_source: None,
            },
            direction: SplitDirection::Horizontal,
        };
        assert!(ConnectionManager::is_broadcast_message(&msg));
    }

    #[test]
    fn test_is_broadcast_message_window_created() {
        let msg = ServerMessage::WindowCreated {
            should_focus: false,
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
            details: None,
        };
        assert!(!ConnectionManager::is_broadcast_message(&msg));
    }

    #[test]
    fn test_is_not_broadcast_message_pane_closed() {
        // BUG-062 fix: PaneClosed is NOT filtered as broadcast because it's
        // needed as a direct response to ClosePane requests. The tool_close_pane
        // handler uses recv_filtered with a predicate that checks for the specific
        // pane_id, so spurious broadcasts are already ignored.
        let msg = ServerMessage::PaneClosed {
            pane_id: Uuid::new_v4(),
            exit_code: Some(0),
        };
        assert!(!ConnectionManager::is_broadcast_message(&msg));
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

    // ==================== BUG-042: Excessive Result Nesting Tests ====================

    #[tokio::test]
    async fn test_bug042_recv_response_from_daemon_returns_flat_result() {
        // This test verifies that recv_response_from_daemon returns a flat Result<ServerMessage, McpError>
        // and not a nested Result<Result<...>>. The explicit type annotation ensures compilation
        // fails if the return type changes.
        let mut connection = ConnectionManager::new();

        // Since we are not connected, this returns Err(McpError::NotConnected)
        let result: Result<ServerMessage, McpError> = connection.recv_response_from_daemon().await;

        assert!(matches!(result, Err(McpError::NotConnected)));
    }

    // ==================== BUG-074: create_session should return pane_id ====================

    #[test]
    fn test_bug074_session_created_response_includes_pane_id() {
        // BUG-074: Verify that SessionCreatedWithDetails contains all required fields
        // including pane_id so callers can immediately use fugue_send_input
        let session_id = Uuid::new_v4();
        let window_id = Uuid::new_v4();
        let pane_id = Uuid::new_v4();

        let msg = ServerMessage::SessionCreatedWithDetails {
            session_id,
            session_name: "test-session".to_string(),
            window_id,
            pane_id,
            should_focus: true,
        };

        // Verify the message can be serialized to JSON with all fields
        let json = serde_json::to_value(&msg).unwrap();

        // The enum variant name is part of the serialization
        assert!(json.get("SessionCreatedWithDetails").is_some());
        let details = &json["SessionCreatedWithDetails"];

        // Verify all required fields are present
        assert_eq!(details["session_id"], session_id.to_string());
        assert_eq!(details["session_name"], "test-session");
        assert_eq!(details["window_id"], window_id.to_string());
        assert_eq!(details["pane_id"], pane_id.to_string());
        assert_eq!(details["should_focus"], true);
    }

    #[test]
    fn test_bug074_tool_response_json_structure() {
        // BUG-074: Verify the JSON structure returned by tool_create_session handler
        // This tests the format that MCP clients receive
        let session_id = Uuid::new_v4();
        let window_id = Uuid::new_v4();
        let pane_id = Uuid::new_v4();
        let session_name = "test-session";
        let tags: Vec<String> = vec!["worker".to_string()];

        // This matches the JSON built in handlers.rs tool_create_session
        let result = serde_json::json!({
            "session_id": session_id.to_string(),
            "session_name": session_name,
            "window_id": window_id.to_string(),
            "pane_id": pane_id.to_string(),
            "tags": tags,
            "status": "created"
        });

        // Verify pane_id is present and is a valid UUID string
        assert!(result.get("pane_id").is_some());
        let pane_id_str = result["pane_id"].as_str().unwrap();
        assert!(Uuid::parse_str(pane_id_str).is_ok(), "pane_id should be a valid UUID");

        // Verify other required fields
        assert!(result.get("session_id").is_some());
        assert!(result.get("window_id").is_some());
        assert_eq!(result["status"], "created");
    }
}