//! Message handlers for client requests
//!
//! This module provides the complete message handling layer that routes incoming
//! `ClientMessage` types to appropriate handlers and responds with `ServerMessage` types.

mod connection;
mod input;
mod mcp_bridge;
mod orchestration;
mod pane;
mod session;

use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use uuid::Uuid;

use ccmux_protocol::{ClientMessage, ErrorCode, ServerMessage};

use crate::config::AppConfig;
use crate::pty::{PaneClosedNotification, PtyManager};
use crate::registry::{ClientId, ClientRegistry};
use crate::session::SessionManager;
use crate::sideband::AsyncCommandExecutor;

/// Context for message handlers
///
/// Provides access to all server state needed to handle client requests.
pub struct HandlerContext {
    /// Session manager for session/window/pane operations
    pub session_manager: Arc<RwLock<SessionManager>>,
    /// PTY manager for terminal operations
    pub pty_manager: Arc<RwLock<PtyManager>>,
    /// Client connection registry for tracking and broadcasting
    pub registry: Arc<ClientRegistry>,
    /// Application configuration
    pub config: Arc<AppConfig>,
    /// The client making this request
    pub client_id: ClientId,
    /// Channel to notify when panes close (for cleanup)
    pub pane_closed_tx: mpsc::Sender<PaneClosedNotification>,
    /// Sideband command executor for processing Claude commands
    pub command_executor: Arc<AsyncCommandExecutor>,
}

/// Result of handling a message
pub enum HandlerResult {
    /// Single response to send back to the client
    Response(ServerMessage),
    /// Response to client plus broadcast to session
    ResponseWithBroadcast {
        response: ServerMessage,
        session_id: Uuid,
        broadcast: ServerMessage,
    },
    /// Response to client followed by additional messages to the same client
    ///
    /// Used when attaching to a session to send the initial response followed
    /// by the current scrollback content for each pane.
    ResponseWithFollowUp {
        response: ServerMessage,
        follow_up: Vec<ServerMessage>,
    },
    /// No response needed (for fire-and-forget messages like Input)
    NoResponse,
}

impl HandlerContext {
    /// Create a new handler context
    pub fn new(
        session_manager: Arc<RwLock<SessionManager>>,
        pty_manager: Arc<RwLock<PtyManager>>,
        registry: Arc<ClientRegistry>,
        config: Arc<AppConfig>,
        client_id: ClientId,
        pane_closed_tx: mpsc::Sender<PaneClosedNotification>,
        command_executor: Arc<AsyncCommandExecutor>,
    ) -> Self {
        Self {
            session_manager,
            pty_manager,
            registry,
            config,
            client_id,
            pane_closed_tx,
            command_executor,
        }
    }

    /// Route a client message to the appropriate handler
    pub async fn route_message(&self, msg: ClientMessage) -> HandlerResult {
        match msg {
            // Connection handlers
            ClientMessage::Connect {
                client_id,
                protocol_version,
            } => self.handle_connect(client_id, protocol_version).await,

            ClientMessage::Ping => self.handle_ping(),

            ClientMessage::Sync => self.handle_sync().await,

            ClientMessage::Detach => self.handle_detach().await,

            // Session handlers
            ClientMessage::ListSessions => self.handle_list_sessions().await,

            ClientMessage::CreateSession { name, command } => {
                self.handle_create_session(name, command).await
            }

            ClientMessage::AttachSession { session_id } => {
                self.handle_attach_session(session_id).await
            }

            ClientMessage::CreateWindow { session_id, name } => {
                self.handle_create_window(session_id, name).await
            }

            // Pane handlers
            ClientMessage::CreatePane {
                window_id,
                direction,
            } => self.handle_create_pane(window_id, direction).await,

            ClientMessage::SelectPane { pane_id } => self.handle_select_pane(pane_id).await,

            ClientMessage::ClosePane { pane_id } => self.handle_close_pane(pane_id).await,

            ClientMessage::Resize {
                pane_id,
                cols,
                rows,
            } => self.handle_resize(pane_id, cols, rows).await,

            // Input handlers
            ClientMessage::Input { pane_id, data } => self.handle_input(pane_id, data).await,

            ClientMessage::Reply { reply } => self.handle_reply(reply).await,

            ClientMessage::SetViewportOffset { pane_id, offset } => {
                self.handle_set_viewport_offset(pane_id, offset).await
            }

            ClientMessage::JumpToBottom { pane_id } => self.handle_jump_to_bottom(pane_id).await,

            // Orchestration handlers
            ClientMessage::SendOrchestration { target, message } => {
                self.handle_send_orchestration(target, message).await
            }

            // Session destruction
            ClientMessage::DestroySession { session_id } => {
                self.handle_destroy_session(session_id).await
            }

            // MCP Bridge handlers
            ClientMessage::ListAllPanes { session_filter } => {
                self.handle_list_all_panes(session_filter).await
            }

            ClientMessage::ListWindows { session_filter } => {
                self.handle_list_windows(session_filter).await
            }

            ClientMessage::ReadPane { pane_id, lines } => {
                self.handle_read_pane(pane_id, lines).await
            }

            ClientMessage::GetPaneStatus { pane_id } => {
                self.handle_get_pane_status(pane_id).await
            }

            ClientMessage::CreatePaneWithOptions {
                session_filter,
                window_filter,
                direction,
                command,
                cwd,
            } => {
                self.handle_create_pane_with_options(
                    session_filter,
                    window_filter,
                    direction,
                    command,
                    cwd,
                )
                .await
            }

            ClientMessage::CreateSessionWithOptions { name } => {
                self.handle_create_session_with_options(name).await
            }

            ClientMessage::CreateWindowWithOptions {
                session_filter,
                name,
                command,
            } => {
                self.handle_create_window_with_options(session_filter, name, command)
                    .await
            }

            ClientMessage::RenameSession {
                session_filter,
                new_name,
            } => self.handle_rename_session(session_filter, new_name).await,
        }
    }

    /// Create an error response
    pub fn error(code: ErrorCode, message: impl Into<String>) -> HandlerResult {
        HandlerResult::Response(ServerMessage::Error {
            code,
            message: message.into(),
        })
    }
}

impl From<ServerMessage> for HandlerResult {
    fn from(msg: ServerMessage) -> Self {
        HandlerResult::Response(msg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ccmux_protocol::PROTOCOL_VERSION;
    use tokio::sync::mpsc;

    fn create_test_context() -> HandlerContext {
        let session_manager = Arc::new(RwLock::new(SessionManager::new()));
        let pty_manager = Arc::new(RwLock::new(PtyManager::new()));
        let registry = Arc::new(ClientRegistry::new());
        let config = Arc::new(crate::config::AppConfig::default());
        let command_executor = Arc::new(AsyncCommandExecutor::new(
            Arc::clone(&session_manager),
            Arc::clone(&pty_manager),
            Arc::clone(&registry),
        ));

        // Register a test client
        let (tx, _rx) = mpsc::channel(10);
        let client_id = registry.register_client(tx);

        // Create cleanup channel (receiver is dropped in tests)
        let (pane_closed_tx, _pane_closed_rx) = mpsc::channel(10);

        HandlerContext::new(session_manager, pty_manager, registry, config, client_id, pane_closed_tx, command_executor)
    }

    #[tokio::test]
    async fn test_route_ping() {
        let ctx = create_test_context();
        let result = ctx.route_message(ClientMessage::Ping).await;

        match result {
            HandlerResult::Response(ServerMessage::Pong) => {}
            _ => panic!("Expected Pong response"),
        }
    }

    #[tokio::test]
    async fn test_route_connect() {
        let ctx = create_test_context();
        let result = ctx
            .route_message(ClientMessage::Connect {
                client_id: Uuid::new_v4(),
                protocol_version: PROTOCOL_VERSION,
            })
            .await;

        match result {
            HandlerResult::Response(ServerMessage::Connected { .. }) => {}
            _ => panic!("Expected Connected response"),
        }
    }

    #[tokio::test]
    async fn test_route_connect_version_mismatch() {
        let ctx = create_test_context();
        let result = ctx
            .route_message(ClientMessage::Connect {
                client_id: Uuid::new_v4(),
                protocol_version: 9999,
            })
            .await;

        match result {
            HandlerResult::Response(ServerMessage::Error {
                code: ErrorCode::ProtocolMismatch,
                ..
            }) => {}
            _ => panic!("Expected ProtocolMismatch error"),
        }
    }

    #[tokio::test]
    async fn test_route_list_sessions() {
        let ctx = create_test_context();
        let result = ctx.route_message(ClientMessage::ListSessions).await;

        match result {
            HandlerResult::Response(ServerMessage::SessionList { .. }) => {}
            _ => panic!("Expected SessionList response"),
        }
    }

    #[tokio::test]
    async fn test_error_helper() {
        let result = HandlerContext::error(ErrorCode::SessionNotFound, "Session not found");

        match result {
            HandlerResult::Response(ServerMessage::Error { code, message }) => {
                assert_eq!(code, ErrorCode::SessionNotFound);
                assert_eq!(message, "Session not found");
            }
            _ => panic!("Expected Error response"),
        }
    }
}
