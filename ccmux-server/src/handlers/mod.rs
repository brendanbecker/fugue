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

use ccmux_protocol::{ClientMessage, ErrorCode, ServerMessage, messages::ErrorDetails};

use crate::arbitration::{Action, Actor, Arbitrator, Resource};
use crate::config::AppConfig;
use crate::observability::Metrics;
use crate::persistence::PersistenceManager;
use crate::pty::{PaneClosedNotification, PtyManager};
use crate::registry::{ClientId, ClientRegistry};
use crate::session::{Session, SessionManager, Window};
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
    /// Human-Control Arbitrator (FEAT-079)
    pub arbitrator: Arc<Arbitrator>,
    /// Persistence manager for state logging (optional)
    pub persistence: Option<Arc<RwLock<PersistenceManager>>>,
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
    /// Response to client plus broadcast to all clients
    ResponseWithGlobalBroadcast {
        response: ServerMessage,
        broadcast: ServerMessage,
    },
    /// Broadcast to session without response to sender
    BroadcastToSession {
        session_id: Uuid,
        broadcast: ServerMessage,
    },
    /// Broadcast to all clients without response to sender
    GlobalBroadcast(ServerMessage),
    /// No response needed (for fire-and-forget messages like Input)
    NoResponse,
}

impl HandlerContext {
    /// Create a new handler context
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        session_manager: Arc<RwLock<SessionManager>>,
        pty_manager: Arc<RwLock<PtyManager>>,
        registry: Arc<ClientRegistry>,
        config: Arc<AppConfig>,
        client_id: ClientId,
        pane_closed_tx: mpsc::Sender<PaneClosedNotification>,
        command_executor: Arc<AsyncCommandExecutor>,
        arbitrator: Arc<Arbitrator>,
        persistence: Option<Arc<RwLock<PersistenceManager>>>,
    ) -> Self {
        Self {
            session_manager,
            pty_manager,
            registry,
            config,
            client_id,
            pane_closed_tx,
            command_executor,
            arbitrator,
            persistence,
        }
    }

    /// Get the actor for this context based on client type
    pub fn actor(&self) -> Actor {
        match self.registry.get_client_type(self.client_id) {
            Some(ccmux_protocol::ClientType::Mcp) => Actor::Agent,
            // Treat TUI or unknown as Human
            _ => Actor::Human(self.client_id),
        }
    }

    /// Record activity for the current actor if they are human
    pub fn record_human_activity(&self, resource: Resource, action: Action) {
        if let Actor::Human(_) = self.actor() {
            self.arbitrator.record_activity(resource, action);
        }
    }

    /// Check if the current actor is allowed to perform an action on a resource
    pub fn check_arbitration(&self, resource: Resource, action: Action) -> Result<(), HandlerResult> {
        match self.arbitrator.check_access(self.actor(), resource, action) {
            crate::arbitration::ArbitrationResult::Allowed => Ok(()),
            crate::arbitration::ArbitrationResult::Blocked { remaining_ms, reason, .. } => {
                Err(HandlerContext::error_with_details(
                    ErrorCode::UserPriorityActive,
                    format!("Blocked by {}: retry after {}ms", reason, remaining_ms),
                    ErrorDetails::HumanControl { remaining_ms },
                ))
            }
        }
    }

    /// Resolve the current session for the client (FEAT-078)
    pub fn resolve_active_session(&self, manager: &SessionManager) -> Option<Uuid> {
        self.registry
            .get_client_focus(self.client_id)
            .and_then(|f| f.active_session_id)
            .or_else(|| manager.active_session_id())
    }

    /// Resolve the current window for the client within a session (FEAT-078)
    pub fn resolve_active_window(&self, session: &Session) -> Option<Uuid> {
        self.registry
            .get_client_focus(self.client_id)
            .and_then(|f| f.active_window_id)
            .or_else(|| session.active_window_id())
    }

    /// Resolve the current pane for the client within a window (FEAT-078)
    pub fn resolve_active_pane(&self, window: &Window) -> Option<Uuid> {
        self.registry
            .get_client_focus(self.client_id)
            .and_then(|f| f.active_pane_id)
            .or_else(|| window.active_pane_id())
    }

    /// Route a message to the appropriate handler
    pub async fn route_message(&self, msg: ClientMessage) -> HandlerResult {
        // FEAT-074: Record request metric by message type
        Metrics::global().record_request(&msg.type_name());

        match msg {
            // Connection handlers
            ClientMessage::Connect {
                client_id,
                protocol_version,
                client_type,
            } => self.handle_connect(client_id, protocol_version, client_type).await,

            ClientMessage::Ping => self.handle_ping(),

            ClientMessage::Sync => self.handle_sync().await,

            ClientMessage::GetServerStatus => self.handle_get_server_status().await,

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

            ClientMessage::SelectWindow { window_id } => self.handle_select_window(window_id).await,

            ClientMessage::SelectSession { session_id } => self.handle_select_session(session_id).await,

            ClientMessage::ClosePane { pane_id } => self.handle_close_pane(pane_id).await,

            ClientMessage::Resize {
                pane_id,
                cols,
                rows,
            } => self.handle_resize(pane_id, cols, rows).await,

            // Input handlers
            ClientMessage::Input { pane_id, data } => self.handle_input(pane_id, data).await,

            ClientMessage::Paste { pane_id, data } => self.handle_paste(pane_id, data).await,

            ClientMessage::Reply { reply } => self.handle_reply(reply).await,

            ClientMessage::SetViewportOffset { pane_id, offset } => {
                self.handle_set_viewport_offset(pane_id, offset).await
            }

            ClientMessage::JumpToBottom { pane_id } => self.handle_jump_to_bottom(pane_id).await,

            ClientMessage::Redraw { pane_id } => self.handle_redraw(pane_id).await,

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
                select,
                name,
                claude_model,
                claude_config,
                preset,
            } => {
                self.handle_create_pane_with_options(
                    session_filter,
                    window_filter,
                    direction,
                    command,
                    cwd,
                    select,
                    name,
                    claude_model,
                    claude_config.map(|j| j.into_inner()),
                    preset,
                )
                .await
            }

            ClientMessage::CreateSessionWithOptions {
                name,
                command,
                cwd,
                claude_model,
                claude_config,
                preset,
            } => {
                self.handle_create_session_with_options(
                    name,
                    command,
                    cwd,
                    claude_model,
                    claude_config.map(|j| j.into_inner()),
                    preset,
                )
                .await
            }

            ClientMessage::CreateWindowWithOptions {
                session_filter,
                name,
                command,
                cwd,
            } => {
                self.handle_create_window_with_options(session_filter, name, command, cwd)
                    .await
            }

            ClientMessage::RenameSession {
                session_filter,
                new_name,
            } => self.handle_rename_session(session_filter, new_name).await,

            // FEAT-036: Pane and window rename handlers
            ClientMessage::RenamPane { pane_id, new_name } => {
                self.handle_rename_pane(pane_id, new_name).await
            }

            ClientMessage::RenameWindow { window_id, new_name } => {
                self.handle_rename_window(window_id, new_name).await
            }

            ClientMessage::SplitPane {
                pane_id,
                direction,
                ratio,
                command,
                cwd,
                select,
            } => {
                self.handle_split_pane(pane_id, direction, ratio, command, cwd, select)
                    .await
            }

            ClientMessage::ResizePaneDelta { pane_id, delta } => {
                self.handle_resize_pane_delta(pane_id, delta).await
            }

            ClientMessage::CreateLayout {
                session_filter,
                window_filter,
                layout,
            } => {
                // Convert JsonValue back to serde_json::Value for the handler
                self.handle_create_layout(session_filter, window_filter, layout.into_inner())
                    .await
            }

            ClientMessage::SetEnvironment {
                session_filter,
                key,
                value,
            } => {
                self.handle_set_environment(session_filter, key, value)
                    .await
            }

            ClientMessage::GetEnvironment {
                session_filter,
                key,
            } => self.handle_get_environment(session_filter, key).await,

            ClientMessage::SetMetadata {
                session_filter,
                key,
                value,
            } => {
                self.handle_set_metadata(session_filter, key, value)
                    .await
            }

            ClientMessage::GetMetadata {
                session_filter,
                key,
            } => self.handle_get_metadata(session_filter, key).await,

            // FEAT-048: Orchestration MCP tag handlers
            ClientMessage::SetTags {
                session_filter,
                add,
                remove,
            } => self.handle_set_tags(session_filter, add, remove).await,

            ClientMessage::GetTags { session_filter } => {
                self.handle_get_tags(session_filter).await
            }

            // User priority lock handlers (FEAT-056)
            ClientMessage::UserCommandModeEntered { timeout_ms } => {
                self.handle_user_command_mode_entered(timeout_ms)
            }

            ClientMessage::UserCommandModeExited => self.handle_user_command_mode_exited(),

            // Beads query handlers (FEAT-058)
            ClientMessage::RequestBeadsStatus { pane_id } => {
                self.handle_request_beads_status(pane_id).await
            }

            ClientMessage::RequestBeadsReadyList { pane_id } => {
                self.handle_request_beads_ready_list(pane_id).await
            }

            // Generic widget handlers (FEAT-083)
            ClientMessage::RequestWidgetUpdate { pane_id, widget_type } => {
                self.handle_request_widget_update(pane_id, widget_type).await
            }

            ClientMessage::GetEventsSince { last_commit_seq } => {
                self.handle_get_events_since(last_commit_seq).await
            }

            // Mirror pane handler (FEAT-062)
            ClientMessage::CreateMirror {
                source_pane_id,
                target_pane_id,
                direction,
            } => {
                self.handle_create_mirror(source_pane_id, target_pane_id, direction)
                    .await
            }

            // FEAT-097: Orchestration Message Receive
            ClientMessage::GetWorkerStatus { worker_id } => {
                self.handle_get_worker_status(worker_id).await
            }

            ClientMessage::PollMessages { worker_id } => {
                self.handle_poll_messages(worker_id).await
            }
        }
    }

    // ==================== User Priority Lock Handlers (FEAT-056) ====================
 
    /// Handle user command mode entered (prefix key pressed)
    fn handle_user_command_mode_entered(&self, timeout_ms: u32) -> HandlerResult {
        self.arbitrator.set_lock(self.client_id, timeout_ms, "Prefix Key".to_string());
        HandlerResult::NoResponse
    }
 
    /// Handle user command mode exited (command completed/cancelled/timed out)
    fn handle_user_command_mode_exited(&self) -> HandlerResult {
        self.arbitrator.release_lock(self.client_id);
        HandlerResult::NoResponse
    }

    // ==================== Beads Query Handlers (FEAT-058) ====================

    /// Handle request for beads status (daemon availability, ready count)
    async fn handle_request_beads_status(&self, pane_id: Uuid) -> HandlerResult {
        use crate::beads::BeadsClient;
        use ccmux_protocol::types::BeadsStatus;
        use std::path::PathBuf;

        // Get the pane's working directory
        let cwd: PathBuf = {
            let session_mgr = self.session_manager.read().await;
            session_mgr
                .find_pane(pane_id)
                .and_then(|(_, _, pane)| pane.cwd().map(PathBuf::from))
                .unwrap_or_else(|| std::env::current_dir().unwrap_or_default())
        };

        // Get config for timeout
        let timeout_ms = self.config.beads.query.socket_timeout;

        // Try to create a beads client and get status
        let status = if let Some(client) = BeadsClient::new(&cwd, timeout_ms) {
            client.get_status(Some(10)).await // Limit to 10 for status bar
        } else {
            BeadsStatus::unavailable()
        };

        HandlerResult::Response(ServerMessage::BeadsStatusUpdate { pane_id, status })
    }

    /// Handle request for full beads ready list (for panel display)
    async fn handle_request_beads_ready_list(&self, pane_id: Uuid) -> HandlerResult {
        use crate::beads::BeadsClient;
        use std::path::PathBuf;

        // Get the pane's working directory
        let cwd: PathBuf = {
            let session_mgr = self.session_manager.read().await;
            session_mgr
                .find_pane(pane_id)
                .and_then(|(_, _, pane)| pane.cwd().map(PathBuf::from))
                .unwrap_or_else(|| std::env::current_dir().unwrap_or_default())
        };

        // Get config for timeout
        let timeout_ms = self.config.beads.query.socket_timeout;

        // Try to create a beads client and get tasks
        let tasks = if let Some(client) = BeadsClient::new(&cwd, timeout_ms) {
            client.query_ready(Some(100)).await.unwrap_or_default() // Limit to 100 for panel
        } else {
            Vec::new()
        };

        HandlerResult::Response(ServerMessage::BeadsReadyList { pane_id, tasks })
    }

    // ==================== Generic Widget Handlers (FEAT-083) ====================

    /// Handle generic widget update request
    ///
    /// Delegates to the appropriate handler based on widget_type:
    /// - "beads.status" -> returns BeadsStatus as WidgetUpdate
    /// - "beads.ready_list" -> returns BeadsReadyList as WidgetUpdate
    /// - Unknown types -> returns error
    async fn handle_request_widget_update(&self, pane_id: Uuid, widget_type: String) -> HandlerResult {
        use ccmux_protocol::types::{BeadsStatus, WidgetUpdate};

        match widget_type.as_str() {
            "beads.status" => {
                // Delegate to existing beads handler and convert to WidgetUpdate
                use crate::beads::BeadsClient;
                use std::path::PathBuf;

                let cwd: PathBuf = {
                    let session_mgr = self.session_manager.read().await;
                    session_mgr
                        .find_pane(pane_id)
                        .and_then(|(_, _, pane)| pane.cwd().map(PathBuf::from))
                        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default())
                };

                let timeout_ms = self.config.beads.query.socket_timeout;

                let status = if let Some(client) = BeadsClient::new(&cwd, timeout_ms) {
                    client.get_status(Some(10)).await
                } else {
                    BeadsStatus::unavailable()
                };

                // Convert BeadsStatus to WidgetUpdate
                let update: WidgetUpdate = status.into();

                HandlerResult::Response(ServerMessage::WidgetUpdate { pane_id, update })
            }

            "beads.ready_list" => {
                // Delegate to existing beads handler and convert to WidgetUpdate
                use crate::beads::BeadsClient;
                use ccmux_protocol::types::Widget;
                use std::path::PathBuf;

                let cwd: PathBuf = {
                    let session_mgr = self.session_manager.read().await;
                    session_mgr
                        .find_pane(pane_id)
                        .and_then(|(_, _, pane)| pane.cwd().map(PathBuf::from))
                        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default())
                };

                let timeout_ms = self.config.beads.query.socket_timeout;

                let tasks = if let Some(client) = BeadsClient::new(&cwd, timeout_ms) {
                    client.query_ready(Some(100)).await.unwrap_or_default()
                } else {
                    Vec::new()
                };

                // Convert tasks to widgets
                let widgets: Vec<Widget> = tasks.into_iter().map(Widget::from).collect();

                let update = WidgetUpdate::new(
                    "beads.ready_list",
                    serde_json::json!({"count": widgets.len()}),
                )
                .with_widgets(widgets);

                HandlerResult::Response(ServerMessage::WidgetUpdate { pane_id, update })
            }

            _ => {
                // Unknown widget type
                HandlerContext::error(
                    ErrorCode::InvalidOperation,
                    format!("Unknown widget type: '{}'", widget_type),
                )
            }
        }
    }

    /// Handle GetServerStatus message (FEAT-074)
    pub async fn handle_get_server_status(&self) -> HandlerResult {
        let (commit_seq, replay_range, wal_healthy, checkpoint_healthy) =
            if let Some(ref p) = self.persistence {
                let p = p.read().await;
                (
                    p.commit_seq(),
                    p.replay_range(),
                    p.is_wal_healthy(),
                    p.is_checkpoint_healthy(),
                )
            } else {
                (0, (0, 0), true, true)
            };

        let client_count = self.registry.client_count();
        let session_count = self.session_manager.read().await.session_count();
        let human_control_active = self.arbitrator.is_any_lock_active().is_some();

        HandlerResult::Response(ServerMessage::ServerStatus {
            commit_seq,
            client_count,
            session_count,
            replay_range,
            wal_healthy,
            checkpoint_healthy,
            human_control_active,
        })
    }

    /// Create an error response
    pub fn error(code: ErrorCode, message: impl Into<String>) -> HandlerResult {
        // FEAT-074: Record error metric
        Metrics::global().record_error(&format!("{:?}", code));
        HandlerResult::Response(ServerMessage::Error {
            code,
            message: message.into(),
            details: None,
        })
    }

    /// Create an error response with details
    pub fn error_with_details(
        code: ErrorCode,
        message: impl Into<String>,
        details: ccmux_protocol::messages::ErrorDetails,
    ) -> HandlerResult {
        // FEAT-074: Record error metric
        Metrics::global().record_error(&format!("{:?}", code));
        HandlerResult::Response(ServerMessage::Error {
            code,
            message: message.into(),
            details: Some(details),
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
    use ccmux_protocol::ClientType;

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
        let arbitrator = Arc::new(Arbitrator::new());

        // Register a test client
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
        )
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
                client_type: ClientType::Tui,
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
                client_type: ClientType::Tui,
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
            HandlerResult::Response(ServerMessage::Error { code, message, details }) => {
                assert_eq!(code, ErrorCode::SessionNotFound);
                assert_eq!(message, "Session not found");
                assert!(details.is_none());
            }
            _ => panic!("Expected Error response"),
        }
    }
}