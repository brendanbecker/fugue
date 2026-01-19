//! Event handling for the application
//!
//! Combines terminal input events with server messages into a unified event stream.

// Allow unused code that's part of the public API for future features
#![allow(dead_code)]

use std::time::Duration;

use crossterm::event::{Event as CrosstermEvent, EventStream, KeyEvent, MouseEvent};
use futures::StreamExt;
use tokio::sync::mpsc;

use ccmux_protocol::ServerMessage;
use ccmux_utils::Result;

/// Application events combining input and server messages
#[derive(Debug)]
pub enum AppEvent {
    /// Terminal input event
    Input(InputEvent),
    /// Server message received
    Server(Box<ServerMessage>),
    /// Terminal resize
    Resize { cols: u16, rows: u16 },
    /// Tick for animations and periodic updates
    Tick,
}

/// Input events from terminal
#[derive(Debug, Clone)]
pub enum InputEvent {
    /// Key press
    Key(KeyEvent),
    /// Mouse event
    Mouse(MouseEvent),
    /// Focus gained
    FocusGained,
    /// Focus lost
    FocusLost,
    /// Paste event
    Paste(String),
}

/// Event handler that combines input polling with server message receiving
pub struct EventHandler {
    /// Sender for app events
    tx: mpsc::UnboundedSender<AppEvent>,
    /// Receiver for app events
    rx: mpsc::UnboundedReceiver<AppEvent>,
    /// Tick rate for animations
    tick_rate: Duration,
}

impl EventHandler {
    /// Create a new event handler
    pub fn new(tick_rate: Duration) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        Self { tx, rx, tick_rate }
    }

    /// Get a sender clone for forwarding server messages
    pub fn sender(&self) -> mpsc::UnboundedSender<AppEvent> {
        self.tx.clone()
    }

    /// Start polling for terminal events in a background task
    pub fn start_input_polling(&self) {
        let tx = self.tx.clone();
        let tick_rate = self.tick_rate;

        tokio::spawn(async move {
            let mut reader = EventStream::new();

            loop {
                // Use tokio timeout to get tick events at regular intervals
                let event_result = tokio::time::timeout(tick_rate, reader.next()).await;

                match event_result {
                    Ok(Some(Ok(event))) => {
                        // Got an event
                        let app_event = match event {
                            CrosstermEvent::Key(key) => Some(AppEvent::Input(InputEvent::Key(key))),
                            CrosstermEvent::Mouse(mouse) => {
                                Some(AppEvent::Input(InputEvent::Mouse(mouse)))
                            }
                            CrosstermEvent::Resize(cols, rows) => {
                                Some(AppEvent::Resize { cols, rows })
                            }
                            CrosstermEvent::FocusGained => {
                                Some(AppEvent::Input(InputEvent::FocusGained))
                            }
                            CrosstermEvent::FocusLost => {
                                Some(AppEvent::Input(InputEvent::FocusLost))
                            }
                            CrosstermEvent::Paste(text) => {
                                Some(AppEvent::Input(InputEvent::Paste(text)))
                            }
                        };

                        if let Some(evt) = app_event {
                            if tx.send(evt).is_err() {
                                tracing::debug!("Event channel closed, stopping input polling");
                                break;
                            }
                        }
                    }
                    Ok(Some(Err(e))) => {
                        tracing::error!("Error reading terminal event: {}", e);
                        break;
                    }
                    Ok(None) => {
                        // Stream ended
                        tracing::debug!("Event stream ended");
                        break;
                    }
                    Err(_) => {
                        // Timeout - send tick
                        if tx.send(AppEvent::Tick).is_err() {
                            tracing::debug!("Event channel closed, stopping input polling");
                            break;
                        }
                    }
                }
            }
        });
    }

    /// Receive next event
    pub async fn next(&mut self) -> Option<AppEvent> {
        self.rx.recv().await
    }

    /// Try to receive without blocking
    pub fn try_next(&mut self) -> Option<AppEvent> {
        self.rx.try_recv().ok()
    }

    /// Forward a server message as an event
    pub fn send_server_message(&self, msg: ServerMessage) -> Result<()> {
        self.tx
            .send(AppEvent::Server(Box::new(msg)))
            .map_err(|_| ccmux_utils::CcmuxError::connection("Event channel closed"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_handler_creation() {
        let handler = EventHandler::new(Duration::from_millis(100));
        let _sender = handler.sender();
    }

    #[tokio::test]
    async fn test_event_send_receive() {
        let mut handler = EventHandler::new(Duration::from_millis(100));
        let sender = handler.sender();

        sender.send(AppEvent::Tick).unwrap();

        let event = handler.try_next();
        assert!(matches!(event, Some(AppEvent::Tick)));
    }

    #[tokio::test]
    async fn test_server_message_forwarding() {
        let mut handler = EventHandler::new(Duration::from_millis(100));

        handler
            .send_server_message(ServerMessage::Pong)
            .unwrap();

        let event = handler.try_next();
        match event {
            Some(AppEvent::Server(msg)) => assert_eq!(*msg, ServerMessage::Pong),
            _ => panic!("Expected Server message"),
        }
    }
}
