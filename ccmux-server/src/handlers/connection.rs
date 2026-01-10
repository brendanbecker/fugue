//! Connection-related message handlers
//!
//! Handles: Connect, Ping, Sync, Detach

use tracing::{debug, info};
use uuid::Uuid;

use ccmux_protocol::{ErrorCode, ServerMessage, PROTOCOL_VERSION};

use super::{HandlerContext, HandlerResult};

impl HandlerContext {
    /// Handle Connect message - validate protocol version
    pub async fn handle_connect(
        &self,
        client_uuid: Uuid,
        protocol_version: u32,
    ) -> HandlerResult {
        info!(
            "Client {} (UUID: {}) connecting with protocol version {}",
            self.client_id, client_uuid, protocol_version
        );

        if protocol_version != PROTOCOL_VERSION {
            return HandlerContext::error(
                ErrorCode::ProtocolMismatch,
                format!(
                    "Protocol version mismatch: client={}, server={}",
                    protocol_version, PROTOCOL_VERSION
                ),
            );
        }

        HandlerResult::Response(ServerMessage::Connected {
            server_version: env!("CARGO_PKG_VERSION").to_string(),
            protocol_version: PROTOCOL_VERSION,
        })
    }

    /// Handle Ping message - simple heartbeat response
    pub fn handle_ping(&self) -> HandlerResult {
        debug!("Received Ping from {}, sending Pong", self.client_id);
        HandlerResult::Response(ServerMessage::Pong)
    }

    /// Handle Sync message - return full state dump
    ///
    /// If attached to a session, returns session state along with current
    /// scrollback content for all panes.
    pub async fn handle_sync(&self) -> HandlerResult {
        debug!("Sync request from {}", self.client_id);

        // Check if client is attached to a session
        let attached_session_id = self.registry.get_client_session(self.client_id);

        let session_manager = self.session_manager.read().await;

        match attached_session_id {
            Some(session_id) => {
                // Return full state for the attached session
                if let Some(session) = session_manager.get_session(session_id) {
                    let session_info = session.to_info();

                    // Collect window and pane info, along with scrollback content
                    let windows: Vec<_> = session.windows().map(|w| w.to_info()).collect();

                    let mut panes = Vec::new();
                    let mut initial_output: Vec<ServerMessage> = Vec::new();

                    for window in session.windows() {
                        for pane in window.panes() {
                            panes.push(pane.to_info());

                            // Get the current scrollback content for this pane
                            let scrollback = pane.scrollback();
                            let lines: Vec<&str> = scrollback.get_lines().collect();
                            if !lines.is_empty() {
                                let content = lines.join("\n");
                                if !content.is_empty() {
                                    initial_output.push(ServerMessage::Output {
                                        pane_id: pane.id(),
                                        data: content.into_bytes(),
                                    });
                                }
                            }
                        }
                    }

                    HandlerResult::ResponseWithFollowUp {
                        response: ServerMessage::Attached {
                            session: session_info,
                            windows,
                            panes,
                        },
                        follow_up: initial_output,
                    }
                } else {
                    // Session no longer exists - detach and return session list
                    drop(session_manager); // Release read lock before modifying registry
                    self.registry.detach_from_session(self.client_id);

                    let session_manager = self.session_manager.read().await;
                    let sessions: Vec<_> = session_manager
                        .list_sessions()
                        .iter()
                        .map(|s| s.to_info())
                        .collect();

                    HandlerResult::Response(ServerMessage::SessionList { sessions })
                }
            }
            None => {
                // Not attached - return session list
                let sessions: Vec<_> = session_manager
                    .list_sessions()
                    .iter()
                    .map(|s| s.to_info())
                    .collect();

                HandlerResult::Response(ServerMessage::SessionList { sessions })
            }
        }
    }

    /// Handle Detach message - unregister from current session
    pub async fn handle_detach(&self) -> HandlerResult {
        info!("Detach request from {}", self.client_id);

        // Get attached session to update client count
        if let Some(session_id) = self.registry.get_client_session(self.client_id) {
            // Decrement attached client count in session
            let mut session_manager = self.session_manager.write().await;
            if let Some(session) = session_manager.get_session_mut(session_id) {
                session.detach_client();
            }
        }

        // Detach from session in registry
        let was_attached = self.registry.detach_from_session(self.client_id);

        if was_attached {
            debug!(
                "Client {} detached from session successfully",
                self.client_id
            );
        } else {
            debug!("Client {} was not attached to any session", self.client_id);
        }

        HandlerResult::Response(ServerMessage::SessionList {
            sessions: {
                let session_manager = self.session_manager.read().await;
                session_manager
                    .list_sessions()
                    .iter()
                    .map(|s| s.to_info())
                    .collect()
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pty::PtyManager;
    use crate::registry::ClientRegistry;
    use crate::session::SessionManager;
    use std::sync::Arc;
    use tokio::sync::{mpsc, RwLock};

    fn create_test_context() -> HandlerContext {
        let session_manager = Arc::new(RwLock::new(SessionManager::new()));
        let pty_manager = Arc::new(RwLock::new(PtyManager::new()));
        let registry = Arc::new(ClientRegistry::new());
        let config = Arc::new(crate::config::AppConfig::default());
        let command_executor = Arc::new(crate::sideband::AsyncCommandExecutor::new(
            Arc::clone(&session_manager),
            Arc::clone(&pty_manager),
            Arc::clone(&registry),
        ));

        let (tx, _rx) = mpsc::channel(10);
        let client_id = registry.register_client(tx);

        let (pane_closed_tx, _) = mpsc::channel(10);
        HandlerContext::new(session_manager, pty_manager, registry, config, client_id, pane_closed_tx, command_executor)
    }

    #[tokio::test]
    async fn test_handle_connect_success() {
        let ctx = create_test_context();
        let result = ctx
            .handle_connect(Uuid::new_v4(), PROTOCOL_VERSION)
            .await;

        match result {
            HandlerResult::Response(ServerMessage::Connected {
                protocol_version, ..
            }) => {
                assert_eq!(protocol_version, PROTOCOL_VERSION);
            }
            _ => panic!("Expected Connected response"),
        }
    }

    #[tokio::test]
    async fn test_handle_connect_version_mismatch() {
        let ctx = create_test_context();
        let result = ctx.handle_connect(Uuid::new_v4(), 9999).await;

        match result {
            HandlerResult::Response(ServerMessage::Error { code, message }) => {
                assert_eq!(code, ErrorCode::ProtocolMismatch);
                assert!(message.contains("9999"));
            }
            _ => panic!("Expected Error response"),
        }
    }

    #[test]
    fn test_handle_ping() {
        let ctx = create_test_context();
        let result = ctx.handle_ping();

        match result {
            HandlerResult::Response(ServerMessage::Pong) => {}
            _ => panic!("Expected Pong response"),
        }
    }

    #[tokio::test]
    async fn test_handle_sync_not_attached() {
        let ctx = create_test_context();
        let result = ctx.handle_sync().await;

        match result {
            HandlerResult::Response(ServerMessage::SessionList { sessions }) => {
                assert!(sessions.is_empty());
            }
            _ => panic!("Expected SessionList response"),
        }
    }

    #[tokio::test]
    async fn test_handle_sync_attached() {
        let ctx = create_test_context();

        // Create and attach to a session
        let session_id = {
            let mut session_manager = ctx.session_manager.write().await;
            let session = session_manager.create_session("test").unwrap();
            session.id()
        };

        ctx.registry.attach_to_session(ctx.client_id, session_id);

        let result = ctx.handle_sync().await;

        match result {
            HandlerResult::ResponseWithFollowUp {
                response: ServerMessage::Attached { session, .. },
                follow_up,
            } => {
                assert_eq!(session.name, "test");
                // Fresh session has no scrollback yet
                assert!(follow_up.is_empty());
            }
            _ => panic!("Expected Attached response with follow_up"),
        }
    }

    #[tokio::test]
    async fn test_handle_detach_not_attached() {
        let ctx = create_test_context();
        let result = ctx.handle_detach().await;

        match result {
            HandlerResult::Response(ServerMessage::SessionList { .. }) => {}
            _ => panic!("Expected SessionList response"),
        }
    }

    #[tokio::test]
    async fn test_handle_detach_attached() {
        let ctx = create_test_context();

        // Create and attach to a session
        let session_id = {
            let mut session_manager = ctx.session_manager.write().await;
            let session = session_manager.create_session("test").unwrap();
            let id = session.id();

            // Also attach client in session
            let session = session_manager.get_session_mut(id).unwrap();
            session.attach_client();
            id
        };

        ctx.registry.attach_to_session(ctx.client_id, session_id);

        let result = ctx.handle_detach().await;

        // Should no longer be attached
        assert!(ctx.registry.get_client_session(ctx.client_id).is_none());

        // Session client count should be decremented
        let session_manager = ctx.session_manager.read().await;
        let session = session_manager.get_session(session_id).unwrap();
        assert_eq!(session.attached_clients(), 0);

        match result {
            HandlerResult::Response(ServerMessage::SessionList { .. }) => {}
            _ => panic!("Expected SessionList response"),
        }
    }
}
