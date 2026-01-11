//! Input-related message handlers
//!
//! Handles: Input, Reply, SetViewportOffset, JumpToBottom

use tracing::{debug, warn};
use uuid::Uuid;

use ccmux_protocol::{ErrorCode, ReplyMessage, ServerMessage};

use super::{HandlerContext, HandlerResult};
use crate::reply::ReplyHandler;

impl HandlerContext {
    /// Handle Input message - write data to PTY
    pub async fn handle_input(&self, pane_id: Uuid, data: Vec<u8>) -> HandlerResult {
        // Don't log data contents for privacy
        debug!(
            "Input for pane {} ({} bytes) from {}",
            pane_id,
            data.len(),
            self.client_id
        );

        // Check pane exists
        {
            let session_manager = self.session_manager.read().await;
            if session_manager.find_pane(pane_id).is_none() {
                return HandlerContext::error(
                    ErrorCode::PaneNotFound,
                    format!("Pane {} not found", pane_id),
                );
            }
        }

        // Write to PTY
        let pty_manager = self.pty_manager.read().await;
        if let Some(handle) = pty_manager.get(pane_id) {
            if let Err(e) = handle.write_all(&data) {
                warn!("Failed to write to PTY for pane {}: {}", pane_id, e);
                return HandlerContext::error(
                    ErrorCode::InternalError,
                    format!("Failed to write to PTY: {}", e),
                );
            }
            // No response for successful input
            HandlerResult::NoResponse
        } else {
            // No PTY for this pane - might be in a state that doesn't have one
            debug!("No PTY handle for pane {}", pane_id);
            HandlerContext::error(
                ErrorCode::InternalError,
                format!("No PTY handle for pane {}", pane_id),
            )
        }
    }

    /// Handle Reply message - forward to reply mechanism
    pub async fn handle_reply(&self, reply: ReplyMessage) -> HandlerResult {
        debug!(
            "Reply to {:?} ({} bytes) from {}",
            reply.target,
            reply.content.len(),
            self.client_id
        );

        // Use the ReplyHandler from reply.rs
        let mut session_manager = self.session_manager.write().await;
        let pty_manager = self.pty_manager.read().await;

        let mut handler = ReplyHandler::new(&mut session_manager, &pty_manager);
        let result = handler.handle(reply);

        match result {
            Ok(reply_result) => {
                debug!(
                    "Reply delivered to pane {} ({} bytes)",
                    reply_result.pane_id, reply_result.bytes_written
                );
                HandlerResult::Response(ServerMessage::ReplyDelivered {
                    result: reply_result,
                })
            }
            Err(error) => {
                debug!("Reply failed: {}", error);
                HandlerResult::Response(error.to_server_message())
            }
        }
    }

    /// Handle SetViewportOffset message - update pane viewport
    pub async fn handle_set_viewport_offset(
        &self,
        pane_id: Uuid,
        offset: usize,
    ) -> HandlerResult {
        debug!(
            "SetViewportOffset for pane {} to {} from {}",
            pane_id, offset, self.client_id
        );

        let mut session_manager = self.session_manager.write().await;

        match session_manager.find_pane_mut(pane_id) {
            Some(pane) => {
                // Get mutable access to scrollback and update viewport
                let scrollback = pane.scrollback_mut();
                scrollback.set_viewport_offset(offset);
                debug!("Viewport offset for pane {} set to {}", pane_id, offset);
                HandlerResult::NoResponse
            }
            None => {
                debug!("Pane {} not found for SetViewportOffset", pane_id);
                HandlerContext::error(
                    ErrorCode::PaneNotFound,
                    format!("Pane {} not found", pane_id),
                )
            }
        }
    }

    /// Handle JumpToBottom message - reset viewport to follow output
    pub async fn handle_jump_to_bottom(&self, pane_id: Uuid) -> HandlerResult {
        debug!(
            "JumpToBottom for pane {} from {}",
            pane_id, self.client_id
        );

        let mut session_manager = self.session_manager.write().await;

        match session_manager.find_pane_mut(pane_id) {
            Some(pane) => {
                // Reset viewport to bottom (offset 0)
                let scrollback = pane.scrollback_mut();
                scrollback.set_viewport_offset(0);
                debug!("Viewport for pane {} jumped to bottom", pane_id);
                HandlerResult::NoResponse
            }
            None => {
                debug!("Pane {} not found for JumpToBottom", pane_id);
                HandlerContext::error(
                    ErrorCode::PaneNotFound,
                    format!("Pane {} not found", pane_id),
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pty::PtyManager;
    use crate::registry::ClientRegistry;
    use crate::session::SessionManager;
    use crate::user_priority::UserPriorityManager;
    use std::sync::Arc;
    use tokio::sync::{mpsc, RwLock};

    fn create_test_context() -> HandlerContext {
        let session_manager = Arc::new(RwLock::new(SessionManager::new()));
        let pty_manager = Arc::new(RwLock::new(PtyManager::new()));
        let registry = Arc::new(ClientRegistry::new());
        let config = Arc::new(crate::config::AppConfig::default());
        let user_priority = Arc::new(UserPriorityManager::new());
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

    async fn create_pane(ctx: &HandlerContext) -> Uuid {
        let mut session_manager = ctx.session_manager.write().await;
        let session = session_manager.create_session("test").unwrap();
        let session_id = session.id();

        let session = session_manager.get_session_mut(session_id).unwrap();
        let window = session.create_window(Some("main".to_string()));
        let window_id = window.id();

        let window = session.get_window_mut(window_id).unwrap();
        window.create_pane().id()
    }

    #[tokio::test]
    async fn test_handle_input_pane_not_found() {
        let ctx = create_test_context();
        let result = ctx.handle_input(Uuid::new_v4(), vec![0x41]).await;

        match result {
            HandlerResult::Response(ServerMessage::Error { code, .. }) => {
                assert_eq!(code, ErrorCode::PaneNotFound);
            }
            _ => panic!("Expected Error response"),
        }
    }

    #[tokio::test]
    async fn test_handle_input_no_pty() {
        let ctx = create_test_context();
        let pane_id = create_pane(&ctx).await;

        // Input should fail because no PTY was created
        let result = ctx.handle_input(pane_id, vec![0x41]).await;

        match result {
            HandlerResult::Response(ServerMessage::Error { code, .. }) => {
                assert_eq!(code, ErrorCode::InternalError);
            }
            _ => panic!("Expected Error response for no PTY"),
        }
    }

    #[tokio::test]
    async fn test_handle_reply_pane_not_found() {
        let ctx = create_test_context();
        let reply = ReplyMessage::by_id(Uuid::new_v4(), "test");
        let result = ctx.handle_reply(reply).await;

        match result {
            HandlerResult::Response(ServerMessage::Error { code, .. }) => {
                assert_eq!(code, ErrorCode::PaneNotFound);
            }
            _ => panic!("Expected Error response"),
        }
    }

    #[tokio::test]
    async fn test_handle_set_viewport_offset_success() {
        let ctx = create_test_context();
        let pane_id = create_pane(&ctx).await;

        let result = ctx.handle_set_viewport_offset(pane_id, 100).await;

        match result {
            HandlerResult::NoResponse => {}
            _ => panic!("Expected NoResponse"),
        }

        // Verify offset was set
        let session_manager = ctx.session_manager.read().await;
        let (_, _, pane) = session_manager.find_pane(pane_id).unwrap();
        assert_eq!(pane.scrollback().viewport_offset(), 100);
    }

    #[tokio::test]
    async fn test_handle_set_viewport_offset_not_found() {
        let ctx = create_test_context();
        let result = ctx.handle_set_viewport_offset(Uuid::new_v4(), 100).await;

        match result {
            HandlerResult::Response(ServerMessage::Error { code, .. }) => {
                assert_eq!(code, ErrorCode::PaneNotFound);
            }
            _ => panic!("Expected Error response"),
        }
    }

    #[tokio::test]
    async fn test_handle_jump_to_bottom_success() {
        let ctx = create_test_context();
        let pane_id = create_pane(&ctx).await;

        // First set a non-zero offset
        ctx.handle_set_viewport_offset(pane_id, 100).await;

        // Then jump to bottom
        let result = ctx.handle_jump_to_bottom(pane_id).await;

        match result {
            HandlerResult::NoResponse => {}
            _ => panic!("Expected NoResponse"),
        }

        // Verify offset is back to 0
        let session_manager = ctx.session_manager.read().await;
        let (_, _, pane) = session_manager.find_pane(pane_id).unwrap();
        assert_eq!(pane.scrollback().viewport_offset(), 0);
    }

    #[tokio::test]
    async fn test_handle_jump_to_bottom_not_found() {
        let ctx = create_test_context();
        let result = ctx.handle_jump_to_bottom(Uuid::new_v4()).await;

        match result {
            HandlerResult::Response(ServerMessage::Error { code, .. }) => {
                assert_eq!(code, ErrorCode::PaneNotFound);
            }
            _ => panic!("Expected Error response"),
        }
    }

    #[tokio::test]
    async fn test_handle_reply_by_name_not_found() {
        let ctx = create_test_context();
        let reply = ReplyMessage::by_name("nonexistent-pane", "test");
        let result = ctx.handle_reply(reply).await;

        match result {
            HandlerResult::Response(ServerMessage::Error { code, .. }) => {
                assert_eq!(code, ErrorCode::PaneNotFound);
            }
            _ => panic!("Expected Error response"),
        }
    }
}
