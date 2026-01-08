//! Message handler trait and utilities

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
}
