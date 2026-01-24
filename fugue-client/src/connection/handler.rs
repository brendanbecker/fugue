//! Message handler trait and utilities

// Allow unused code that's part of the public API for future features
#![allow(dead_code)]

use ccmux_protocol::{ClientMessage, ServerMessage};
use ccmux_utils::Result;
use tokio::sync::mpsc;

/// Clonable message sender
#[derive(Clone)]
pub struct MessageSender {
    tx: mpsc::Sender<ClientMessage>,
}

impl MessageSender {
    pub fn new(tx: mpsc::Sender<ClientMessage>) -> Self {
        Self { tx }
    }

    pub async fn send(&self, msg: ClientMessage) -> Result<()> {
        self.tx
            .send(msg)
            .await
            .map_err(|_| ccmux_utils::CcmuxError::ConnectionClosed)?;
        Ok(())
    }

    /// Send without waiting (fire and forget)
    pub fn send_nowait(&self, msg: ClientMessage) {
        let _ = self.tx.try_send(msg);
    }
}

/// Trait for handling incoming server messages
pub trait MessageHandler: Send {
    /// Handle a server message
    fn handle(&mut self, msg: ServerMessage);

    /// Called when connection is established
    fn on_connected(&mut self) {}

    /// Called when connection is lost
    fn on_disconnected(&mut self) {}
}

/// Simple callback-based handler
pub struct CallbackHandler<F>
where
    F: FnMut(ServerMessage) + Send,
{
    callback: F,
}

impl<F> CallbackHandler<F>
where
    F: FnMut(ServerMessage) + Send,
{
    pub fn new(callback: F) -> Self {
        Self { callback }
    }
}

impl<F> MessageHandler for CallbackHandler<F>
where
    F: FnMut(ServerMessage) + Send,
{
    fn handle(&mut self, msg: ServerMessage) {
        (self.callback)(msg);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn test_message_sender_clone() {
        let (tx, _rx) = mpsc::channel(10);
        let sender = MessageSender::new(tx);
        let _sender2 = sender.clone();
    }

    #[test]
    fn test_callback_handler() {
        let mut received = Vec::new();
        let mut handler = CallbackHandler::new(|msg| {
            received.push(format!("{:?}", msg));
        });

        handler.handle(ServerMessage::Pong);
        // Note: received is captured by closure, can't check directly
        // This just tests that the handler compiles and runs
    }

    #[tokio::test]
    async fn test_message_sender_send_success() {
        let (tx, mut rx) = mpsc::channel(10);
        let sender = MessageSender::new(tx);

        sender.send(ClientMessage::Ping).await.unwrap();

        let received = rx.recv().await.unwrap();
        assert!(matches!(received, ClientMessage::Ping));
    }

    #[tokio::test]
    async fn test_message_sender_send_channel_closed() {
        let (tx, rx) = mpsc::channel(10);
        let sender = MessageSender::new(tx);

        // Drop the receiver
        drop(rx);

        let result = sender.send(ClientMessage::Ping).await;
        assert!(result.is_err());
    }

    #[test]
    fn test_message_sender_send_nowait() {
        let (tx, mut rx) = mpsc::channel(10);
        let sender = MessageSender::new(tx);

        sender.send_nowait(ClientMessage::Ping);

        // Should receive the message
        let received = rx.try_recv().unwrap();
        assert!(matches!(received, ClientMessage::Ping));
    }

    #[test]
    fn test_message_sender_send_nowait_channel_full() {
        let (tx, _rx) = mpsc::channel(1);
        let sender = MessageSender::new(tx);

        // Fill the channel
        sender.send_nowait(ClientMessage::Ping);

        // This should silently fail (fire and forget)
        sender.send_nowait(ClientMessage::Ping);
        // No panic, just ignored
    }

    #[test]
    fn test_message_sender_send_nowait_channel_closed() {
        let (tx, rx) = mpsc::channel(10);
        let sender = MessageSender::new(tx);

        drop(rx);

        // Should silently fail
        sender.send_nowait(ClientMessage::Ping);
        // No panic
    }

    #[test]
    fn test_callback_handler_receives_messages() {
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        let mut handler = CallbackHandler::new(move |_msg| {
            counter_clone.fetch_add(1, Ordering::SeqCst);
        });

        handler.handle(ServerMessage::Pong);
        handler.handle(ServerMessage::Pong);
        handler.handle(ServerMessage::Pong);

        assert_eq!(counter.load(Ordering::SeqCst), 3);
    }

    #[test]
    fn test_callback_handler_different_message_types() {
        use ccmux_protocol::SessionInfo;
        use uuid::Uuid;

        let messages = Arc::new(std::sync::Mutex::new(Vec::new()));
        let messages_clone = messages.clone();

        let mut handler = CallbackHandler::new(move |msg| {
            messages_clone.lock().unwrap().push(format!("{:?}", msg));
        });

        handler.handle(ServerMessage::Pong);
        handler.handle(ServerMessage::SessionList {
            sessions: vec![SessionInfo {
                id: Uuid::nil(),
                name: "test".into(),
                created_at: 0,
                window_count: 1,
                attached_clients: 0,
                worktree: None,
                tags: std::collections::HashSet::new(),
                metadata: HashMap::new(),
            }],
        });

        let msgs = messages.lock().unwrap();
        assert_eq!(msgs.len(), 2);
        assert!(msgs[0].contains("Pong"));
        assert!(msgs[1].contains("SessionList"));
    }

    // Test the trait default implementations
    struct TestHandler {
        connected_called: bool,
        disconnected_called: bool,
    }

    impl MessageHandler for TestHandler {
        fn handle(&mut self, _msg: ServerMessage) {}

        fn on_connected(&mut self) {
            self.connected_called = true;
        }

        fn on_disconnected(&mut self) {
            self.disconnected_called = true;
        }
    }

    #[test]
    fn test_message_handler_on_connected() {
        let mut handler = TestHandler {
            connected_called: false,
            disconnected_called: false,
        };

        handler.on_connected();
        assert!(handler.connected_called);
        assert!(!handler.disconnected_called);
    }

    #[test]
    fn test_message_handler_on_disconnected() {
        let mut handler = TestHandler {
            connected_called: false,
            disconnected_called: false,
        };

        handler.on_disconnected();
        assert!(!handler.connected_called);
        assert!(handler.disconnected_called);
    }

    #[test]
    fn test_callback_handler_default_on_connected() {
        let mut handler = CallbackHandler::new(|_| {});
        // Default implementation should do nothing and not panic
        handler.on_connected();
    }

    #[test]
    fn test_callback_handler_default_on_disconnected() {
        let mut handler = CallbackHandler::new(|_| {});
        // Default implementation should do nothing and not panic
        handler.on_disconnected();
    }

    #[test]
    fn test_message_sender_new() {
        let (tx, _rx) = mpsc::channel(10);
        let sender = MessageSender::new(tx);
        // Verify the sender was created successfully
        drop(sender);
    }

    // Test that CallbackHandler is Send
    fn assert_send<T: Send>() {}

    #[test]
    fn test_callback_handler_is_send() {
        assert_send::<CallbackHandler<fn(ServerMessage)>>();
    }
}
