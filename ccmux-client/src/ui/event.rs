//! Event handling for the application
//!
//! Combines terminal input events with server messages into a unified event stream.

// Allow unused code that's part of the public API for future features
#![allow(dead_code)]

use std::time::Duration;

use crossterm::event::{self, Event as CrosstermEvent, KeyEvent, MouseEvent};
use tokio::sync::mpsc;

use ccmux_protocol::ServerMessage;
use ccmux_utils::Result;

/// Application events combining input and server messages
#[derive(Debug)]
pub enum AppEvent {
    /// Terminal input event
    Input(InputEvent),
    /// Server message received
    Server(ServerMessage),
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

        std::thread::spawn(move || {
            loop {
                // Poll with timeout for tick
                if event::poll(tick_rate).unwrap_or(false) {
                    match event::read() {
                        Ok(CrosstermEvent::Key(key)) => {
                            if tx.send(AppEvent::Input(InputEvent::Key(key))).is_err() {
                                break;
                            }
                        }
                        Ok(CrosstermEvent::Mouse(mouse)) => {
                            if tx.send(AppEvent::Input(InputEvent::Mouse(mouse))).is_err() {
                                break;
                            }
                        }
                        Ok(CrosstermEvent::Resize(cols, rows)) => {
                            if tx.send(AppEvent::Resize { cols, rows }).is_err() {
                                break;
                            }
                        }
                        Ok(CrosstermEvent::FocusGained) => {
                            if tx.send(AppEvent::Input(InputEvent::FocusGained)).is_err() {
                                break;
                            }
                        }
                        Ok(CrosstermEvent::FocusLost) => {
                            if tx.send(AppEvent::Input(InputEvent::FocusLost)).is_err() {
                                break;
                            }
                        }
                        Ok(CrosstermEvent::Paste(_)) => {
                            // Handle paste events if needed in the future
                        }
                        Err(e) => {
                            tracing::error!("Error reading terminal event: {}", e);
                            break;
                        }
                    }
                } else {
                    // Timeout - send tick
                    if tx.send(AppEvent::Tick).is_err() {
                        break;
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
            .send(AppEvent::Server(msg))
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
        assert!(matches!(event, Some(AppEvent::Server(ServerMessage::Pong))));
    }
}
