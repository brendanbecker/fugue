//! Orchestration-related message handlers
//!
//! Handles: SendOrchestration, GetWorkerStatus, PollMessages

use tracing::{debug, info};

use fugue_protocol::{ErrorCode, OrchestrationMessage, OrchestrationTarget, ServerMessage};

use super::{HandlerContext, HandlerResult};

impl HandlerContext {
    /// Handle SendOrchestration message - route to appropriate targets
    pub async fn handle_send_orchestration(
        &self,
        target: OrchestrationTarget,
        message: OrchestrationMessage,
    ) -> HandlerResult {
        info!(
            "SendOrchestration to {:?} from {}",
            target, self.client_id
        );

        // Get the sender's session ID
        let sender_session_id = match self.registry.get_client_session(self.client_id) {
            Some(id) => id,
            None => {
                debug!("Client {} not attached to any session", self.client_id);
                return HandlerContext::error(
                    ErrorCode::InvalidOperation,
                    "Must be attached to a session to send orchestration messages",
                );
            }
        };

        // Get session manager for reading/writing session info
        // Need write lock to push to inboxes (FEAT-097)
        let mut session_manager = self.session_manager.write().await;

        // FEAT-097: Capture status updates and store them in the session
        if message.msg_type == "status.update" {
            if let Some(session) = session_manager.get_session_mut(sender_session_id) {
                session.set_status(message.payload().clone());
            }
        }

        // Build the message to send to recipients
        let outbound_message = ServerMessage::OrchestrationReceived {
            from_session_id: sender_session_id,
            message: message.clone(),
        };

        // Route based on target
        let delivered_count = match target {
            OrchestrationTarget::Tagged(ref tag) => {
                // Find sessions with the specified tag and send to their clients + inbox
                let mut total_delivered = 0;
                
                // We need to collect IDs first to avoid borrowing issues while iterating
                let target_ids: Vec<uuid::Uuid> = session_manager.list_sessions()
                    .iter()
                    .filter(|s| s.id() != sender_session_id && s.has_tag(tag))
                    .map(|s| s.id())
                    .collect();

                if target_ids.is_empty() {
                    debug!("No sessions found with tag '{}'", tag);
                }

                for session_id in target_ids {
                    // Push to inbox (FEAT-097)
                    if let Some(session) = session_manager.get_session_mut(session_id) {
                        session.push_message(sender_session_id, message.clone());
                    }

                    // Broadcast to connected clients
                    let _count = self
                        .registry
                        .broadcast_to_session(session_id, outbound_message.clone())
                        .await;
                    
                    // Count is 1 if either clients received it OR it was put in inbox
                    // But for consistency with previous behavior, we track actual deliveries + 1 for inbox
                    // Actually, let's count "sessions reached"
                    total_delivered += 1;
                }

                total_delivered
            }

            OrchestrationTarget::Session(session_id) => {
                // Send to specific session
                if session_id == sender_session_id {
                    debug!("Sender targeting own session, skipping");
                    0
                } else if let Some(session) = session_manager.get_session_mut(session_id) {
                    // Push to inbox (FEAT-097)
                    session.push_message(sender_session_id, message.clone());

                    // Broadcast to connected clients
                    self.registry
                        .broadcast_to_session(session_id, outbound_message)
                        .await;
                    
                    1
                } else {
                    debug!("Target session {} not found", session_id);
                    return HandlerContext::error(
                        ErrorCode::SessionNotFound,
                        format!("Target session {} not found", session_id),
                    );
                }
            }

            OrchestrationTarget::Broadcast => {
                // Broadcast to all sessions except sender
                let mut total_delivered = 0;

                let target_ids: Vec<uuid::Uuid> = session_manager.list_sessions()
                    .iter()
                    .map(|s| s.id())
                    .filter(|id| *id != sender_session_id)
                    .collect();

                for session_id in target_ids {
                    // Push to inbox (FEAT-097)
                    if let Some(session) = session_manager.get_session_mut(session_id) {
                        session.push_message(sender_session_id, message.clone());
                    }

                    // Broadcast to connected clients
                    self.registry
                        .broadcast_to_session(session_id, outbound_message.clone())
                        .await;
                    
                    total_delivered += 1;
                }

                total_delivered
            }

            OrchestrationTarget::Worktree(worktree_path) => {
                // Find sessions associated with the given worktree path
                let mut total_delivered = 0;

                let target_ids: Vec<uuid::Uuid> = session_manager.list_sessions()
                    .iter()
                    .filter(|s| s.id() != sender_session_id)
                    .filter(|s| s.worktree().is_some_and(|w| w.path == worktree_path))
                    .map(|s| s.id())
                    .collect();

                for session_id in target_ids {
                    // Push to inbox (FEAT-097)
                    if let Some(session) = session_manager.get_session_mut(session_id) {
                        session.push_message(sender_session_id, message.clone());
                    }

                    // Broadcast to connected clients
                    self.registry
                        .broadcast_to_session(session_id, outbound_message.clone())
                        .await;
                    
                    total_delivered += 1;
                }

                total_delivered
            }
        };

        info!("Orchestration message delivered to {} sessions", delivered_count);

        if delivered_count == 0 {
            // No recipients but this isn't necessarily an error - could be no other sessions
            debug!("No recipients for orchestration message");
        }

        HandlerResult::Response(ServerMessage::OrchestrationDelivered { delivered_count })
    }

    /// Handle GetWorkerStatus message (FEAT-097)
    pub async fn handle_get_worker_status(&self, worker_id: Option<String>) -> HandlerResult {
        let session_manager = self.session_manager.read().await;

        if let Some(id_str) = worker_id {
            // Find specific worker (session)
            let session_id = if let Ok(uuid) = uuid::Uuid::parse_str(&id_str) {
                uuid
            } else {
                // Try by name
                if let Some(session) = session_manager.get_session_by_name(&id_str) {
                    session.id()
                } else {
                    return HandlerContext::error(
                        ErrorCode::SessionNotFound,
                        format!("Worker '{}' not found", id_str),
                    );
                }
            };

            if let Some(session) = session_manager.get_session(session_id) {
                let status = session.get_status().cloned().unwrap_or(serde_json::Value::Null);
                HandlerResult::Response(ServerMessage::WorkerStatus {
                    status: fugue_protocol::types::JsonValue::new(status),
                })
            } else {
                HandlerContext::error(
                    ErrorCode::SessionNotFound,
                    format!("Worker '{}' not found", id_str),
                )
            }
        } else {
            // Get all workers status
            // This returns a map of {worker_id: status}
            let mut all_statuses = serde_json::Map::new();
            for session in session_manager.list_sessions() {
                let status = session.get_status().cloned().unwrap_or(serde_json::Value::Null);
                all_statuses.insert(session.id().to_string(), status);
            }
            HandlerResult::Response(ServerMessage::WorkerStatus {
                status: fugue_protocol::types::JsonValue::new(serde_json::Value::Object(all_statuses)),
            })
        }
    }

    /// Handle PollMessages message (FEAT-097, BUG-069 fix)
    ///
    /// When `worker_id` is None, polls the current client's attached session.
    /// This fixes BUG-069 where users would poll the wrong session because they
    /// didn't know which session was tagged "orchestrator".
    pub async fn handle_poll_messages(&self, worker_id: Option<String>) -> HandlerResult {
        let mut session_manager = self.session_manager.write().await;

        // BUG-069 FIX: If no worker_id specified, use the client's attached session
        let session_id = match worker_id {
            Some(ref id) => {
                if let Ok(uuid) = uuid::Uuid::parse_str(id) {
                    uuid
                } else if let Some(session) = session_manager.get_session_by_name(id) {
                    session.id()
                } else {
                    return HandlerContext::error(
                        ErrorCode::SessionNotFound,
                        format!("Session '{}' not found", id),
                    );
                }
            }
            None => {
                // Use the client's attached session
                match self.registry.get_client_session(self.client_id) {
                    Some(id) => id,
                    None => {
                        return HandlerContext::error(
                            ErrorCode::InvalidOperation,
                            "Must specify worker_id or be attached to a session to poll messages",
                        );
                    }
                }
            }
        };

        if let Some(session) = session_manager.get_session_mut(session_id) {
            let session_name = session.name().to_string();
            let messages = session.poll_messages();
            debug!(
                "BUG-069: Polled {} messages from session '{}' ({})",
                messages.len(),
                session_name,
                session_id
            );
            HandlerResult::Response(ServerMessage::MessagesPolled { messages })
        } else {
            HandlerContext::error(
                ErrorCode::SessionNotFound,
                format!("Session '{}' not found", worker_id.unwrap_or_else(|| session_id.to_string())),
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pty::PtyManager;
    use crate::registry::ClientRegistry;
    use crate::session::SessionManager;
    use crate::arbitration::Arbitrator;
    use crate::watchdog::WatchdogManager;
    use serde_json::json;
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

        // Create cleanup channel (receiver is dropped in tests)
        let (pane_closed_tx, _pane_closed_rx) = mpsc::channel(10);

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

    async fn create_session(ctx: &HandlerContext) -> Uuid {
        let mut session_manager = ctx.session_manager.write().await;
        session_manager.create_session("test").unwrap().id()
    }

    fn create_test_message() -> OrchestrationMessage {
        OrchestrationMessage::new(
            "status.update",
            json!({"status": "idle", "message": "Ready"}),
        )
    }

    #[tokio::test]
    async fn test_send_orchestration_not_attached() {
        let ctx = create_test_context();
        let message = create_test_message();

        // Client is not attached to any session
        let result = ctx
            .handle_send_orchestration(OrchestrationTarget::Broadcast, message)
            .await;

        match result {
            HandlerResult::Response(ServerMessage::Error { code, .. }) => {
                assert_eq!(code, ErrorCode::InvalidOperation);
            }
            _ => panic!("Expected Error response"),
        }
    }

    #[tokio::test]
    async fn test_send_orchestration_broadcast_empty() {
        let ctx = create_test_context();

        // Create and attach to a session
        let session_id = {
            let mut session_manager = ctx.session_manager.write().await;
            session_manager.create_session("sender").unwrap().id()
        };
        ctx.registry.attach_to_session(ctx.client_id, session_id);

        let message = create_test_message();
        let result = ctx
            .handle_send_orchestration(OrchestrationTarget::Broadcast, message)
            .await;

        match result {
            HandlerResult::Response(ServerMessage::OrchestrationDelivered { delivered_count }) => {
                // No other sessions to deliver to
                assert_eq!(delivered_count, 0);
            }
            _ => panic!("Expected OrchestrationDelivered response"),
        }
    }

    #[tokio::test]
    async fn test_send_orchestration_to_specific_session_not_found() {
        let ctx = create_test_context();

        // Create and attach to a session
        let session_id = {
            let mut session_manager = ctx.session_manager.write().await;
            session_manager.create_session("sender").unwrap().id()
        };
        ctx.registry.attach_to_session(ctx.client_id, session_id);

        let message = create_test_message();
        let result = ctx
            .handle_send_orchestration(OrchestrationTarget::Session(Uuid::new_v4()), message)
            .await;

        match result {
            HandlerResult::Response(ServerMessage::Error { code, .. }) => {
                assert_eq!(code, ErrorCode::SessionNotFound);
            }
            _ => panic!("Expected Error response"),
        }
    }

    #[tokio::test]
    async fn test_send_orchestration_to_own_session() {
        let ctx = create_test_context();

        // Create and attach to a session
        let session_id = {
            let mut session_manager = ctx.session_manager.write().await;
            session_manager.create_session("sender").unwrap().id()
        };
        ctx.registry.attach_to_session(ctx.client_id, session_id);

        let message = create_test_message();
        let result = ctx
            .handle_send_orchestration(OrchestrationTarget::Session(session_id), message)
            .await;

        match result {
            HandlerResult::Response(ServerMessage::OrchestrationDelivered { delivered_count }) => {
                // Sending to own session skips
                assert_eq!(delivered_count, 0);
            }
            _ => panic!("Expected OrchestrationDelivered response"),
        }
    }

    #[tokio::test]
    async fn test_send_orchestration_to_tagged_not_found() {
        let ctx = create_test_context();

        // Create and attach to a session (without orchestrator tag)
        let session_id = {
            let mut session_manager = ctx.session_manager.write().await;
            session_manager.create_session("worker").unwrap().id()
        };
        ctx.registry.attach_to_session(ctx.client_id, session_id);

        let message = create_test_message();
        let result = ctx
            .handle_send_orchestration(
                OrchestrationTarget::Tagged("orchestrator".to_string()),
                message,
            )
            .await;

        match result {
            HandlerResult::Response(ServerMessage::OrchestrationDelivered { delivered_count }) => {
                // No session with orchestrator tag exists
                assert_eq!(delivered_count, 0);
            }
            _ => panic!("Expected OrchestrationDelivered response"),
        }
    }

    #[tokio::test]
    async fn test_send_orchestration_to_worktree_no_matches() {
        let ctx = create_test_context();

        // Create and attach to a session
        let session_id = {
            let mut session_manager = ctx.session_manager.write().await;
            session_manager.create_session("worker").unwrap().id()
        };
        ctx.registry.attach_to_session(ctx.client_id, session_id);

        let message = create_test_message();
        let result = ctx
            .handle_send_orchestration(
                OrchestrationTarget::Worktree("/path/to/worktree".to_string()),
                message,
            )
            .await;

        match result {
            HandlerResult::Response(ServerMessage::OrchestrationDelivered { delivered_count }) => {
                // No sessions with matching worktree
                assert_eq!(delivered_count, 0);
            }
            _ => panic!("Expected OrchestrationDelivered response"),
        }
    }

    #[tokio::test]
    async fn test_send_orchestration_broadcast_with_recipient() {
        let ctx = create_test_context();

        // Create sender session and attach
        let sender_session_id = {
            let mut session_manager = ctx.session_manager.write().await;
            session_manager.create_session("sender").unwrap().id()
        };
        ctx.registry.attach_to_session(ctx.client_id, sender_session_id);

        // Create recipient session with a client
        let recipient_session_id = {
            let mut session_manager = ctx.session_manager.write().await;
            session_manager.create_session("recipient").unwrap().id()
        };

        // Register a recipient client
        let (tx, mut rx) = mpsc::channel(10);
        let recipient_client_id = ctx.registry.register_client(tx);
        ctx.registry
            .attach_to_session(recipient_client_id, recipient_session_id);

        let message = create_test_message();
        let result = ctx
            .handle_send_orchestration(OrchestrationTarget::Broadcast, message.clone())
            .await;

        match result {
            HandlerResult::Response(ServerMessage::OrchestrationDelivered { delivered_count }) => {
                assert_eq!(delivered_count, 1);
            }
            _ => panic!("Expected OrchestrationDelivered response"),
        }

        // Verify recipient got the message
        let received = rx.try_recv().expect("Should have received message");
        match received {
            ServerMessage::OrchestrationReceived {
                from_session_id,
                message: received_message,
            } => {
                assert_eq!(from_session_id, sender_session_id);
                assert_eq!(received_message, message);
            }
            _ => panic!("Expected OrchestrationReceived message"),
        }
    }

    #[tokio::test]
    async fn test_send_orchestration_to_tagged_with_recipient() {
        let ctx = create_test_context();

        // Create sender session (worker) and attach
        let sender_session_id = {
            let mut session_manager = ctx.session_manager.write().await;
            let session_id = session_manager.create_session("worker").unwrap().id();
            session_manager.get_session_mut(session_id).unwrap().add_tag("worker");
            session_id
        };
        ctx.registry.attach_to_session(ctx.client_id, sender_session_id);

        // Create orchestrator session with a client
        let orchestrator_session_id = {
            let mut session_manager = ctx.session_manager.write().await;
            let session_id = session_manager.create_session("orchestrator").unwrap().id();
            session_manager.get_session_mut(session_id).unwrap().add_tag("orchestrator");
            session_id
        };

        // Register an orchestrator client
        let (tx, mut rx) = mpsc::channel(10);
        let orchestrator_client_id = ctx.registry.register_client(tx);
        ctx.registry
            .attach_to_session(orchestrator_client_id, orchestrator_session_id);

        let message = create_test_message();
        let result = ctx
            .handle_send_orchestration(
                OrchestrationTarget::Tagged("orchestrator".to_string()),
                message.clone(),
            )
            .await;

        match result {
            HandlerResult::Response(ServerMessage::OrchestrationDelivered { delivered_count }) => {
                assert_eq!(delivered_count, 1);
            }
            _ => panic!("Expected OrchestrationDelivered response"),
        }

        // Verify orchestrator got the message
        let received = rx.try_recv().expect("Should have received message");
        match received {
            ServerMessage::OrchestrationReceived {
                from_session_id,
                message: received_message,
            } => {
                assert_eq!(from_session_id, sender_session_id);
                assert_eq!(received_message, message);
            }
            _ => panic!("Expected OrchestrationReceived message"),
        }
    }

    // ==================== BUG-069: Inbox/Poll Messages Tests ====================

    #[tokio::test]
    async fn test_send_orchestration_to_tagged_stores_in_inbox() {
        let ctx = create_test_context();

        // Create sender session (watchdog) and attach
        let sender_session_id = {
            let mut session_manager = ctx.session_manager.write().await;
            let session_id = session_manager.create_session("watchdog").unwrap().id();
            session_manager.get_session_mut(session_id).unwrap().add_tag("watchdog");
            session_id
        };
        ctx.registry.attach_to_session(ctx.client_id, sender_session_id);

        // Create orchestrator session with tag (NO client attached to demonstrate inbox works independently)
        let orchestrator_session_id = {
            let mut session_manager = ctx.session_manager.write().await;
            let session_id = session_manager.create_session("orchestrator-session").unwrap().id();
            session_manager.get_session_mut(session_id).unwrap().add_tag("orchestrator");
            session_id
        };

        // Send message from watchdog to tag "orchestrator"
        let message = create_test_message();
        let send_result = ctx
            .handle_send_orchestration(
                OrchestrationTarget::Tagged("orchestrator".to_string()),
                message.clone(),
            )
            .await;

        // Verify delivery count
        match send_result {
            HandlerResult::Response(ServerMessage::OrchestrationDelivered { delivered_count }) => {
                assert_eq!(delivered_count, 1, "Should deliver to 1 orchestrator session");
            }
            _ => panic!("Expected OrchestrationDelivered response"),
        }

        // Verify message is stored in the inbox (directly check session state)
        {
            let mut session_manager = ctx.session_manager.write().await;
            let session = session_manager.get_session_mut(orchestrator_session_id).unwrap();
            let inbox_messages = session.poll_messages();

            assert_eq!(inbox_messages.len(), 1, "Should have 1 message in inbox");
            let (from_id, msg) = &inbox_messages[0];
            assert_eq!(*from_id, sender_session_id, "Message should be from sender session");
            assert_eq!(msg.msg_type, "status.update", "Message type should match");
        }
    }

    #[tokio::test]
    async fn test_poll_messages_returns_inbox_contents() {
        let ctx = create_test_context();

        // Create sender session (watchdog) and attach
        let sender_session_id = {
            let mut session_manager = ctx.session_manager.write().await;
            let session_id = session_manager.create_session("watchdog").unwrap().id();
            session_manager.get_session_mut(session_id).unwrap().add_tag("watchdog");
            session_id
        };
        ctx.registry.attach_to_session(ctx.client_id, sender_session_id);

        // Create orchestrator session with tag
        let _orchestrator_session_id = {
            let mut session_manager = ctx.session_manager.write().await;
            let session_id = session_manager.create_session("orch-session").unwrap().id();
            session_manager.get_session_mut(session_id).unwrap().add_tag("orchestrator");
            session_id
        };

        // Send message from watchdog to tag "orchestrator"
        let message = create_test_message();
        let _ = ctx
            .handle_send_orchestration(
                OrchestrationTarget::Tagged("orchestrator".to_string()),
                message.clone(),
            )
            .await;

        // Now call poll_messages via the handler (simulating what MCP client does)
        let poll_result = ctx.handle_poll_messages(Some("orch-session".to_string())).await;

        match poll_result {
            HandlerResult::Response(ServerMessage::MessagesPolled { messages }) => {
                assert_eq!(messages.len(), 1, "Should poll 1 message");
                let (from_id, msg) = &messages[0];
                assert_eq!(*from_id, sender_session_id, "Message should be from sender session");
                assert_eq!(msg.msg_type, "status.update", "Message type should match");
            }
            HandlerResult::Response(ServerMessage::Error { code, message, .. }) => {
                panic!("Got error response: {:?} - {}", code, message);
            }
            _ => panic!("Expected MessagesPolled response"),
        }

        // Verify inbox is now empty (poll clears it)
        let poll_result2 = ctx.handle_poll_messages(Some("orch-session".to_string())).await;
        match poll_result2 {
            HandlerResult::Response(ServerMessage::MessagesPolled { messages }) => {
                assert!(messages.is_empty(), "Second poll should return empty (inbox drained)");
            }
            _ => panic!("Expected MessagesPolled response"),
        }
    }

    #[tokio::test]
    async fn test_poll_messages_by_session_uuid() {
        let ctx = create_test_context();

        // Create sender session (watchdog) and attach
        let sender_session_id = {
            let mut session_manager = ctx.session_manager.write().await;
            let session_id = session_manager.create_session("watchdog").unwrap().id();
            session_manager.get_session_mut(session_id).unwrap().add_tag("watchdog");
            session_id
        };
        ctx.registry.attach_to_session(ctx.client_id, sender_session_id);

        // Create orchestrator session with tag
        let orchestrator_session_id = {
            let mut session_manager = ctx.session_manager.write().await;
            let session_id = session_manager.create_session("orch-session").unwrap().id();
            session_manager.get_session_mut(session_id).unwrap().add_tag("orchestrator");
            session_id
        };

        // Send message
        let message = create_test_message();
        let _ = ctx
            .handle_send_orchestration(
                OrchestrationTarget::Tagged("orchestrator".to_string()),
                message.clone(),
            )
            .await;

        // Poll using UUID string instead of name
        let poll_result = ctx.handle_poll_messages(Some(orchestrator_session_id.to_string())).await;

        match poll_result {
            HandlerResult::Response(ServerMessage::MessagesPolled { messages }) => {
                assert_eq!(messages.len(), 1, "Should poll 1 message by UUID");
            }
            _ => panic!("Expected MessagesPolled response"),
        }
    }

    /// BUG-069: Test the exact scenario from the bug report
    /// This verifies that polling the WRONG session returns empty (expected behavior)
    /// and polling the CORRECT session returns the message
    #[tokio::test]
    async fn test_poll_wrong_session_returns_empty() {
        let ctx = create_test_context();

        // Create sender session (watchdog) and attach
        let sender_session_id = {
            let mut session_manager = ctx.session_manager.write().await;
            let session_id = session_manager.create_session("watchdog").unwrap().id();
            session_manager.get_session_mut(session_id).unwrap().add_tag("watchdog");
            session_id
        };
        ctx.registry.attach_to_session(ctx.client_id, sender_session_id);

        // Create TWO sessions: one with tag, one without
        // session-0: NO orchestrator tag (like in the bug report scenario)
        let _session_0_id = {
            let mut session_manager = ctx.session_manager.write().await;
            let session_id = session_manager.create_session("session-0").unwrap().id();
            // Note: session-0 does NOT have orchestrator tag!
            session_id
        };

        // orchestrator-session: HAS orchestrator tag
        let _orchestrator_session_id = {
            let mut session_manager = ctx.session_manager.write().await;
            let session_id = session_manager.create_session("orchestrator-session").unwrap().id();
            session_manager.get_session_mut(session_id).unwrap().add_tag("orchestrator");
            session_id
        };

        // Send message to tag "orchestrator"
        // This goes to orchestrator-session, NOT session-0
        let message = create_test_message();
        let send_result = ctx
            .handle_send_orchestration(
                OrchestrationTarget::Tagged("orchestrator".to_string()),
                message.clone(),
            )
            .await;

        // Verify delivery (to orchestrator-session)
        match send_result {
            HandlerResult::Response(ServerMessage::OrchestrationDelivered { delivered_count }) => {
                assert_eq!(delivered_count, 1, "Should deliver to 1 session");
            }
            _ => panic!("Expected OrchestrationDelivered"),
        }

        // BUG-069 scenario: poll session-0 (which does NOT have the tag)
        // This should return EMPTY because messages went to orchestrator-session
        let poll_wrong = ctx.handle_poll_messages(Some("session-0".to_string())).await;
        match poll_wrong {
            HandlerResult::Response(ServerMessage::MessagesPolled { messages }) => {
                assert!(messages.is_empty(), "BUG-069: session-0 should have NO messages (it's not tagged)");
            }
            _ => panic!("Expected MessagesPolled response"),
        }

        // Poll the correct session (orchestrator-session)
        // This should return the message
        let poll_correct = ctx.handle_poll_messages(Some("orchestrator-session".to_string())).await;
        match poll_correct {
            HandlerResult::Response(ServerMessage::MessagesPolled { messages }) => {
                assert_eq!(messages.len(), 1, "orchestrator-session should have 1 message");
            }
            _ => panic!("Expected MessagesPolled response"),
        }
    }

    /// Test that multiple sessions with same tag each get their own copy of the message
    #[tokio::test]
    async fn test_multiple_sessions_with_same_tag() {
        let ctx = create_test_context();

        // Create sender session and attach
        let sender_session_id = {
            let mut session_manager = ctx.session_manager.write().await;
            let session_id = session_manager.create_session("sender").unwrap().id();
            session_id
        };
        ctx.registry.attach_to_session(ctx.client_id, sender_session_id);

        // Create TWO sessions both tagged "orchestrator"
        let _orch1_id = {
            let mut session_manager = ctx.session_manager.write().await;
            let session_id = session_manager.create_session("orch-1").unwrap().id();
            session_manager.get_session_mut(session_id).unwrap().add_tag("orchestrator");
            session_id
        };

        let _orch2_id = {
            let mut session_manager = ctx.session_manager.write().await;
            let session_id = session_manager.create_session("orch-2").unwrap().id();
            session_manager.get_session_mut(session_id).unwrap().add_tag("orchestrator");
            session_id
        };

        // Send to tag "orchestrator"
        let message = create_test_message();
        let send_result = ctx
            .handle_send_orchestration(
                OrchestrationTarget::Tagged("orchestrator".to_string()),
                message.clone(),
            )
            .await;

        // Should deliver to BOTH sessions
        match send_result {
            HandlerResult::Response(ServerMessage::OrchestrationDelivered { delivered_count }) => {
                assert_eq!(delivered_count, 2, "Should deliver to 2 orchestrator sessions");
            }
            _ => panic!("Expected OrchestrationDelivered"),
        }

        // Both sessions should have their own copy of the message
        let poll1 = ctx.handle_poll_messages(Some("orch-1".to_string())).await;
        match poll1 {
            HandlerResult::Response(ServerMessage::MessagesPolled { messages }) => {
                assert_eq!(messages.len(), 1, "orch-1 should have 1 message");
            }
            _ => panic!("Expected MessagesPolled"),
        }

        let poll2 = ctx.handle_poll_messages(Some("orch-2".to_string())).await;
        match poll2 {
            HandlerResult::Response(ServerMessage::MessagesPolled { messages }) => {
                assert_eq!(messages.len(), 1, "orch-2 should also have 1 message");
            }
            _ => panic!("Expected MessagesPolled"),
        }
    }

    /// BUG-069 FIX: Test polling with None worker_id uses the attached session
    #[tokio::test]
    async fn test_poll_messages_none_uses_attached_session() {
        let ctx = create_test_context();

        // Create sender session (watchdog) and attach
        let sender_session_id = {
            let mut session_manager = ctx.session_manager.write().await;
            let session_id = session_manager.create_session("watchdog").unwrap().id();
            session_manager.get_session_mut(session_id).unwrap().add_tag("watchdog");
            session_id
        };
        ctx.registry.attach_to_session(ctx.client_id, sender_session_id);

        // Create orchestrator session with tag
        let orchestrator_session_id = {
            let mut session_manager = ctx.session_manager.write().await;
            let session_id = session_manager.create_session("orch-session").unwrap().id();
            session_manager.get_session_mut(session_id).unwrap().add_tag("orchestrator");
            session_id
        };

        // Send message to orchestrator
        let message = create_test_message();
        let _ = ctx
            .handle_send_orchestration(
                OrchestrationTarget::Tagged("orchestrator".to_string()),
                message.clone(),
            )
            .await;

        // Now create a second client attached to the orchestrator session
        let (tx2, _rx2) = mpsc::channel(10);
        let client_id_2 = ctx.registry.register_client(tx2);
        ctx.registry.attach_to_session(client_id_2, orchestrator_session_id);

        // Create a new context for client_id_2
        let ctx2 = HandlerContext::new(
            Arc::clone(&ctx.session_manager),
            Arc::clone(&ctx.pty_manager),
            Arc::clone(&ctx.registry),
            Arc::clone(&ctx.config),
            client_id_2,
            ctx.pane_closed_tx.clone(),
            Arc::clone(&ctx.command_executor),
            Arc::clone(&ctx.arbitrator),
            None,
            Arc::clone(&ctx.watchdog),
        );

        // BUG-069 FIX: Poll with None - should use the attached session (orch-session)
        let poll_result = ctx2.handle_poll_messages(None).await;
        match poll_result {
            HandlerResult::Response(ServerMessage::MessagesPolled { messages }) => {
                assert_eq!(messages.len(), 1, "Should poll 1 message from attached session");
                let (from_id, _msg) = &messages[0];
                assert_eq!(*from_id, sender_session_id, "Message should be from sender session");
            }
            HandlerResult::Response(ServerMessage::Error { code, message, .. }) => {
                panic!("Got error response: {:?} - {}", code, message);
            }
            _ => panic!("Expected MessagesPolled response"),
        }
    }

    /// BUG-069 FIX: Test polling with None when not attached returns error
    #[tokio::test]
    async fn test_poll_messages_none_not_attached_returns_error() {
        let ctx = create_test_context();

        // Don't attach the client to any session

        // Poll with None - should return error since not attached
        let poll_result = ctx.handle_poll_messages(None).await;
        match poll_result {
            HandlerResult::Response(ServerMessage::Error { code, .. }) => {
                assert_eq!(code, ErrorCode::InvalidOperation);
            }
            _ => panic!("Expected Error response for unattached client polling with None"),
        }
    }
}
