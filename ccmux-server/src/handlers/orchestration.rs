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
            OrchestrationTarget::Orchestrator => {
                // Find the orchestrator session and send to its clients
                let orchestrator_session = session_manager
                    .list_sessions()
                    .iter()
                    .find(|s| s.is_orchestrator())
                    .map(|s| s.id());

                match orchestrator_session {
                    Some(session_id) => {
                        if session_id == sender_session_id {
                            debug!("Orchestrator sending to itself, skipping");
                            0
                        } else {
                            self.registry
                                .broadcast_to_session(session_id, outbound_message)
                                .await
                        }
                    }
                    None => {
                        debug!("No orchestrator session found");
                        0
                    }
                }
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
    use ccmux_protocol::WorkerStatus;
    use std::sync::Arc;
    use tokio::sync::{mpsc, RwLock};
    use uuid::Uuid;

    fn create_test_context() -> HandlerContext {
        let session_manager = Arc::new(RwLock::new(SessionManager::new()));
        let pty_manager = Arc::new(RwLock::new(PtyManager::new()));
        let registry = Arc::new(ClientRegistry::new());

        let (tx, _rx) = mpsc::channel(10);
        let client_id = registry.register_client(tx);

        HandlerContext::new(session_manager, pty_manager, registry, client_id)
    }

    fn create_test_message() -> OrchestrationMessage {
        OrchestrationMessage::StatusUpdate {
            session_id: Uuid::new_v4(),
            status: WorkerStatus::Idle,
            message: Some("Ready".to_string()),
        }
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
    async fn test_send_orchestration_to_orchestrator_not_found() {
        let ctx = create_test_context();

        // Create and attach to a session (not marked as orchestrator)
        let session_id = {
            let mut session_manager = ctx.session_manager.write().await;
            session_manager.create_session("worker").unwrap().id()
        };
        ctx.registry.attach_to_session(ctx.client_id, session_id);

        let message = create_test_message();
        let result = ctx
            .handle_send_orchestration(OrchestrationTarget::Orchestrator, message)
            .await;

        match result {
            HandlerResult::Response(ServerMessage::OrchestrationDelivered { delivered_count }) => {
                // No orchestrator session exists
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
}
