//! Orchestration-related message handlers
//!
//! Handles: SendOrchestration

use tracing::{debug, info};

use ccmux_protocol::{ErrorCode, OrchestrationMessage, OrchestrationTarget, ServerMessage};

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

        // Get session manager for reading session info
        let session_manager = self.session_manager.read().await;

        // Build the message to send to recipients
        let outbound_message = ServerMessage::OrchestrationReceived {
            from_session_id: sender_session_id,
            message,
        };

        // Route based on target
        let delivered_count = match target {
            OrchestrationTarget::Tagged(ref tag) => {
                // Find sessions with the specified tag and send to their clients
                let mut total_delivered = 0;

                for session in session_manager.list_sessions() {
                    let session_id = session.id();
                    if session_id == sender_session_id {
                        continue;
                    }

                    if session.has_tag(tag) {
                        let count = self
                            .registry
                            .broadcast_to_session(session_id, outbound_message.clone())
                            .await;
                        total_delivered += count;
                    }
                }

                if total_delivered == 0 {
                    debug!("No sessions found with tag '{}'", tag);
                }

                total_delivered
            }

            OrchestrationTarget::Session(session_id) => {
                // Send to specific session
                if session_id == sender_session_id {
                    debug!("Sender targeting own session, skipping");
                    0
                } else if session_manager.get_session(session_id).is_some() {
                    self.registry
                        .broadcast_to_session(session_id, outbound_message)
                        .await
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

                for session in session_manager.list_sessions() {
                    let session_id = session.id();
                    if session_id != sender_session_id {
                        let count = self
                            .registry
                            .broadcast_to_session(session_id, outbound_message.clone())
                            .await;
                        total_delivered += count;
                    }
                }

                total_delivered
            }

            OrchestrationTarget::Worktree(worktree_path) => {
                // Find sessions associated with the given worktree path
                let mut total_delivered = 0;

                for session in session_manager.list_sessions() {
                    let session_id = session.id();
                    if session_id == sender_session_id {
                        continue;
                    }

                    // Check if session is associated with this worktree
                    if let Some(worktree) = session.worktree() {
                        if worktree.path == worktree_path {
                            let count = self
                                .registry
                                .broadcast_to_session(session_id, outbound_message.clone())
                                .await;
                            total_delivered += count;
                        }
                    }
                }

                total_delivered
            }
        };

        info!("Orchestration message delivered to {} clients", delivered_count);

        if delivered_count == 0 {
            // No recipients but this isn't necessarily an error - could be no other sessions
            debug!("No recipients for orchestration message");
        }

        HandlerResult::Response(ServerMessage::OrchestrationDelivered { delivered_count })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pty::PtyManager;
    use crate::registry::ClientRegistry;
    use crate::session::SessionManager;
    use crate::user_priority::Arbitrator;
    use serde_json::json;
    use std::sync::Arc;
    use tokio::sync::{mpsc, RwLock};
    use uuid::Uuid;

    fn create_test_context() -> HandlerContext {
        let session_manager = Arc::new(RwLock::new(SessionManager::new()));
        let pty_manager = Arc::new(RwLock::new(PtyManager::new()));
        let registry = Arc::new(ClientRegistry::new());
        let config = Arc::new(crate::config::AppConfig::default());
        let user_priority = Arc::new(Arbitrator::new());
        let command_executor = Arc::new(crate::sideband::AsyncCommandExecutor::new(
            Arc::clone(&session_manager),
            Arc::clone(&pty_manager),
            Arc::clone(&registry),
        ));

        let (tx, _rx) = mpsc::channel(10);
        let client_id = registry.register_client(tx);

        let (pane_closed_tx, _) = mpsc::channel(10);
        HandlerContext::new(session_manager, pty_manager, registry, config, client_id, pane_closed_tx, command_executor, user_priority)
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
}
