//! Reply message handling
//!
//! This module handles the routing of reply messages from orchestrators
//! to worker panes that are awaiting input.

use fugue_protocol::{ErrorCode, PaneTarget, ReplyMessage, ReplyResult, ServerMessage};
use uuid::Uuid;

use crate::pty::PtyManager;
use crate::session::{Pane, SessionManager};

/// Result of attempting to deliver a reply
pub type ReplyDeliveryResult = Result<ReplyResult, ReplyError>;

/// Error that can occur when delivering a reply
#[derive(Debug, Clone)]
pub enum ReplyError {
    /// Target pane was not found
    PaneNotFound { target: String },
    /// Target pane is not awaiting input
    NotAwaitingInput { pane_id: Uuid },
    /// Failed to write to PTY
    WriteError { pane_id: Uuid, message: String },
}

impl ReplyError {
    /// Convert to protocol error code
    pub fn error_code(&self) -> ErrorCode {
        match self {
            ReplyError::PaneNotFound { .. } => ErrorCode::PaneNotFound,
            ReplyError::NotAwaitingInput { .. } => ErrorCode::NotAwaitingInput,
            ReplyError::WriteError { .. } => ErrorCode::InternalError,
        }
    }

    /// Convert to error message string
    pub fn message(&self) -> String {
        match self {
            ReplyError::PaneNotFound { target } => {
                format!("Pane '{}' not found", target)
            }
            ReplyError::NotAwaitingInput { pane_id } => {
                format!("Pane {} is not awaiting input", pane_id)
            }
            ReplyError::WriteError { pane_id, message } => {
                format!("Failed to write to pane {}: {}", pane_id, message)
            }
        }
    }

    /// Convert to ServerMessage::Error
    pub fn to_server_message(&self) -> ServerMessage {
        ServerMessage::Error {
            code: self.error_code(),
            message: self.message(),
            details: None,
        }
    }
}

impl std::fmt::Display for ReplyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message())
    }
}

impl std::error::Error for ReplyError {}

/// Handler for reply messages
pub struct ReplyHandler<'a> {
    session_manager: &'a mut SessionManager,
    pty_manager: &'a PtyManager,
}

impl<'a> ReplyHandler<'a> {
    /// Create a new reply handler
    pub fn new(session_manager: &'a mut SessionManager, pty_manager: &'a PtyManager) -> Self {
        Self {
            session_manager,
            pty_manager,
        }
    }

    /// Handle a reply message
    ///
    /// This method:
    /// 1. Resolves the target pane (by ID or name)
    /// 2. Validates the pane is awaiting input
    /// 3. Writes the message content to the PTY stdin
    /// 4. Returns success or appropriate error
    pub fn handle(&mut self, reply: ReplyMessage) -> ReplyDeliveryResult {
        // Resolve the target pane
        let (pane_id, pane) = self.resolve_target(&reply.target)?;

        // Check if pane is awaiting input
        if !pane.is_awaiting_input() {
            return Err(ReplyError::NotAwaitingInput { pane_id });
        }

        // Write to the PTY
        let bytes_written = self.write_to_pty(pane_id, &reply.content)?;

        tracing::info!(
            pane_id = %pane_id,
            bytes = bytes_written,
            "Reply delivered successfully"
        );

        Ok(ReplyResult {
            pane_id,
            bytes_written,
        })
    }

    /// Resolve a PaneTarget to a pane ID and reference
    fn resolve_target(&self, target: &PaneTarget) -> Result<(Uuid, &Pane), ReplyError> {
        match target {
            PaneTarget::Id(id) => {
                if let Some((_, _, pane)) = self.session_manager.find_pane(*id) {
                    Ok((*id, pane))
                } else {
                    Err(ReplyError::PaneNotFound {
                        target: id.to_string(),
                    })
                }
            }
            PaneTarget::Name(name) => {
                if let Some((_, _, pane)) = self.session_manager.find_pane_by_name(name) {
                    Ok((pane.id(), pane))
                } else {
                    Err(ReplyError::PaneNotFound {
                        target: name.clone(),
                    })
                }
            }
        }
    }

    /// Write message content to a PTY's stdin
    fn write_to_pty(&self, pane_id: Uuid, content: &str) -> Result<usize, ReplyError> {
        let handle = self.pty_manager.get(pane_id).ok_or_else(|| {
            ReplyError::WriteError {
                pane_id,
                message: "PTY handle not found".to_string(),
            }
        })?;

        // Write the content followed by newline (simulates pressing Enter)
        let content_with_newline = format!("{}\n", content);
        let bytes = content_with_newline.as_bytes();

        handle.write_all(bytes).map_err(|e| ReplyError::WriteError {
            pane_id,
            message: e.to_string(),
        })?;

        Ok(bytes.len())
    }
}

/// Convert a ReplyDeliveryResult to a ServerMessage
#[allow(dead_code)]
pub fn result_to_server_message(result: ReplyDeliveryResult) -> ServerMessage {
    match result {
        Ok(reply_result) => ServerMessage::ReplyDelivered {
            result: reply_result,
        },
        Err(error) => error.to_server_message(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fugue_protocol::{AgentActivity, AgentState, PaneState};

    // ==================== ReplyError Tests ====================

    #[test]
    fn test_reply_error_pane_not_found() {
        let error = ReplyError::PaneNotFound {
            target: "worker-3".to_string(),
        };

        assert_eq!(error.error_code(), ErrorCode::PaneNotFound);
        assert!(error.message().contains("worker-3"));
        assert!(error.message().contains("not found"));
    }

    #[test]
    fn test_reply_error_not_awaiting_input() {
        let pane_id = Uuid::new_v4();
        let error = ReplyError::NotAwaitingInput { pane_id };

        assert_eq!(error.error_code(), ErrorCode::NotAwaitingInput);
        assert!(error.message().contains(&pane_id.to_string()));
        assert!(error.message().contains("not awaiting"));
    }

    #[test]
    fn test_reply_error_write_error() {
        let pane_id = Uuid::new_v4();
        let error = ReplyError::WriteError {
            pane_id,
            message: "connection reset".to_string(),
        };

        assert_eq!(error.error_code(), ErrorCode::InternalError);
        assert!(error.message().contains("connection reset"));
    }

    #[test]
    fn test_reply_error_display() {
        let error = ReplyError::PaneNotFound {
            target: "test".to_string(),
        };
        let display = format!("{}", error);
        assert!(display.contains("test"));
    }

    #[test]
    fn test_reply_error_to_server_message() {
        let error = ReplyError::PaneNotFound {
            target: "worker".to_string(),
        };
        let msg = error.to_server_message();

        if let ServerMessage::Error { code, message, .. } = msg {
            assert_eq!(code, ErrorCode::PaneNotFound);
            assert!(message.contains("worker"));
        } else {
            panic!("Expected Error message");
        }
    }

    #[test]
    fn test_reply_error_clone() {
        let error = ReplyError::PaneNotFound {
            target: "test".to_string(),
        };
        let cloned = error.clone();

        assert_eq!(error.message(), cloned.message());
    }

    #[test]
    fn test_reply_error_debug() {
        let error = ReplyError::PaneNotFound {
            target: "test".to_string(),
        };
        let debug = format!("{:?}", error);
        assert!(debug.contains("PaneNotFound"));
    }

    // ==================== result_to_server_message Tests ====================

    #[test]
    fn test_result_to_server_message_success() {
        let result = Ok(ReplyResult {
            pane_id: Uuid::new_v4(),
            bytes_written: 42,
        });

        let msg = result_to_server_message(result);

        if let ServerMessage::ReplyDelivered { result } = msg {
            assert_eq!(result.bytes_written, 42);
        } else {
            panic!("Expected ReplyDelivered message");
        }
    }

    #[test]
    fn test_result_to_server_message_error() {
        let result = Err(ReplyError::NotAwaitingInput {
            pane_id: Uuid::new_v4(),
        });

        let msg = result_to_server_message(result);

        assert!(matches!(msg, ServerMessage::Error { .. }));
    }

    // ==================== ReplyHandler Tests (without PTY) ====================

    #[test]
    fn test_handler_pane_not_found_by_id() {
        let mut session_manager = SessionManager::new();
        let pty_manager = PtyManager::new();

        let mut handler = ReplyHandler::new(&mut session_manager, &pty_manager);

        let reply = ReplyMessage::by_id(Uuid::new_v4(), "test");
        let result = handler.handle(reply);

        assert!(matches!(result, Err(ReplyError::PaneNotFound { .. })));
    }

    #[test]
    fn test_handler_pane_not_found_by_name() {
        let mut session_manager = SessionManager::new();
        let pty_manager = PtyManager::new();

        let mut handler = ReplyHandler::new(&mut session_manager, &pty_manager);

        let reply = ReplyMessage::by_name("nonexistent", "test");
        let result = handler.handle(reply);

        assert!(matches!(result, Err(ReplyError::PaneNotFound { .. })));
    }

    #[test]
    fn test_handler_not_awaiting_input() {
        let mut session_manager = SessionManager::new();
        let pty_manager = PtyManager::new();

        // Create a session with a pane
        let session = session_manager.create_session("test").unwrap();
        let session_id = session.id();

        let session = session_manager.get_session_mut(session_id).unwrap();
        let window = session.create_window(None);
        let window_id = window.id();

        let window = session.get_window_mut(window_id).unwrap();
        let pane = window.create_pane();
        let pane_id = pane.id();

        // Pane is in Normal state (not Claude), so not awaiting input
        let mut handler = ReplyHandler::new(&mut session_manager, &pty_manager);
        let reply = ReplyMessage::by_id(pane_id, "test");
        let result = handler.handle(reply);

        assert!(matches!(result, Err(ReplyError::NotAwaitingInput { .. })));
    }

    #[test]
    fn test_handler_with_claude_thinking_state() {
        let mut session_manager = SessionManager::new();
        let pty_manager = PtyManager::new();

        // Create a session with a Claude pane in Thinking state
        let session = session_manager.create_session("test").unwrap();
        let session_id = session.id();

        let session = session_manager.get_session_mut(session_id).unwrap();
        let window = session.create_window(None);
        let window_id = window.id();

        let window = session.get_window_mut(window_id).unwrap();
        let pane = window.create_pane();
        let pane_id = pane.id();

        // Set pane to Claude Thinking state (NOT awaiting input)
        let pane_mut = window.get_pane_mut(pane_id).unwrap();
        pane_mut.set_state(PaneState::Agent(
            AgentState::new("claude").with_activity(AgentActivity::Processing),
        ));

        let mut handler = ReplyHandler::new(&mut session_manager, &pty_manager);
        let reply = ReplyMessage::by_id(pane_id, "test");
        let result = handler.handle(reply);

        assert!(matches!(result, Err(ReplyError::NotAwaitingInput { .. })));
    }

    #[test]
    fn test_handler_with_claude_awaiting_confirmation_no_pty() {
        let mut session_manager = SessionManager::new();
        let pty_manager = PtyManager::new();

        // Create a session with a Claude pane in AwaitingConfirmation state
        let session = session_manager.create_session("test").unwrap();
        let session_id = session.id();

        let session = session_manager.get_session_mut(session_id).unwrap();
        let window = session.create_window(None);
        let window_id = window.id();

        let window = session.get_window_mut(window_id).unwrap();
        let pane = window.create_pane();
        let pane_id = pane.id();

        // Set pane to Claude AwaitingConfirmation state
        let pane_mut = window.get_pane_mut(pane_id).unwrap();
        pane_mut.set_state(PaneState::Agent(
            AgentState::new("claude").with_activity(AgentActivity::AwaitingConfirmation),
        ));

        let mut handler = ReplyHandler::new(&mut session_manager, &pty_manager);
        let reply = ReplyMessage::by_id(pane_id, "yes");
        let result = handler.handle(reply);

        // Should fail because no PTY handle exists
        assert!(matches!(result, Err(ReplyError::WriteError { .. })));
    }

    #[test]
    fn test_handler_resolve_by_name() {
        let mut session_manager = SessionManager::new();
        let pty_manager = PtyManager::new();

        // Create a session with a named pane
        let session = session_manager.create_session("test").unwrap();
        let session_id = session.id();

        let session = session_manager.get_session_mut(session_id).unwrap();
        let window = session.create_window(None);
        let window_id = window.id();

        let window = session.get_window_mut(window_id).unwrap();
        let pane = window.create_pane();
        let pane_id = pane.id();

        // Set title and Claude state
        let pane_mut = window.get_pane_mut(pane_id).unwrap();
        pane_mut.set_title(Some("worker-3".to_string()));
        pane_mut.set_state(PaneState::Agent(
            AgentState::new("claude").with_activity(AgentActivity::AwaitingConfirmation),
        ));

        let mut handler = ReplyHandler::new(&mut session_manager, &pty_manager);
        let reply = ReplyMessage::by_name("worker-3", "yes");
        let result = handler.handle(reply);

        // Should fail at PTY write but confirms name resolution worked
        if let Err(ReplyError::WriteError { pane_id: err_pane_id, .. }) = result {
            assert_eq!(err_pane_id, pane_id);
        } else {
            panic!("Expected WriteError");
        }
    }
}
