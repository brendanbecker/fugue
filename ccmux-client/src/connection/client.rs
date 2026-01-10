//! Connection client for ccmux server

// Allow unused code that's part of the public API for future features
#![allow(dead_code)]

use std::path::PathBuf;

use futures::{SinkExt, StreamExt};
use tokio::net::UnixStream;
use tokio::sync::mpsc;
use tokio_util::codec::Framed;

use ccmux_protocol::{ClientCodec, ClientMessage, ServerMessage};
use ccmux_utils::{socket_path, CcmuxError, Result};

use super::handler::MessageSender;

/// Connection state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Reconnecting,
}

/// Client connection to ccmux server
pub struct Connection {
    /// Socket path
    socket_path: PathBuf,
    /// Current state
    state: ConnectionState,
    /// Channel for outgoing messages
    tx: mpsc::Sender<ClientMessage>,
    /// Channel for receiving messages
    rx: mpsc::Receiver<ServerMessage>,
    /// Handle to the connection task
    task_handle: Option<tokio::task::JoinHandle<()>>,
}

impl Connection {
    /// Create a new connection (not yet connected)
    pub fn new() -> Self {
        let (tx, _) = mpsc::channel(100);
        let (_, rx) = mpsc::channel(100);

        Self {
            socket_path: socket_path(),
            state: ConnectionState::Disconnected,
            tx,
            rx,
            task_handle: None,
        }
    }

    /// Create with custom socket path
    pub fn with_socket_path(path: PathBuf) -> Self {
        let mut conn = Self::new();
        conn.socket_path = path;
        conn
    }

    /// Get current connection state
    pub fn state(&self) -> ConnectionState {
        self.state
    }

    /// Connect to the server
    pub async fn connect(&mut self) -> Result<()> {
        if self.state == ConnectionState::Connected {
            return Ok(());
        }

        self.state = ConnectionState::Connecting;

        // Check if socket exists
        if !self.socket_path.exists() {
            self.state = ConnectionState::Disconnected;
            return Err(CcmuxError::ServerNotRunning {
                path: self.socket_path.clone(),
            });
        }

        // Connect to Unix socket
        let stream = UnixStream::connect(&self.socket_path).await.map_err(|e| {
            self.state = ConnectionState::Disconnected;
            CcmuxError::Connection(format!("Failed to connect: {}", e))
        })?;

        // Create framed transport with codec
        let framed = Framed::new(stream, ClientCodec::new());

        // Set up channels
        let (outgoing_tx, outgoing_rx) = mpsc::channel::<ClientMessage>(100);
        let (incoming_tx, incoming_rx) = mpsc::channel::<ServerMessage>(100);

        self.tx = outgoing_tx;
        self.rx = incoming_rx;

        // Spawn connection task
        let handle = tokio::spawn(Self::connection_task(framed, outgoing_rx, incoming_tx));
        self.task_handle = Some(handle);

        self.state = ConnectionState::Connected;
        Ok(())
    }

    /// Disconnect from server
    pub async fn disconnect(&mut self) {
        if let Some(handle) = self.task_handle.take() {
            handle.abort();
        }
        self.state = ConnectionState::Disconnected;
    }

    /// Send a message to the server
    pub async fn send(&self, msg: ClientMessage) -> Result<()> {
        if self.state != ConnectionState::Connected {
            return Err(CcmuxError::connection("Not connected"));
        }

        self.tx
            .send(msg)
            .await
            .map_err(|_| CcmuxError::ConnectionClosed)?;

        Ok(())
    }

    /// Receive next message from server (blocking)
    pub async fn recv(&mut self) -> Option<ServerMessage> {
        self.rx.recv().await
    }

    /// Try to receive without blocking
    pub fn try_recv(&mut self) -> Option<ServerMessage> {
        self.rx.try_recv().ok()
    }

    /// Get a message sender that can be cloned
    pub fn sender(&self) -> MessageSender {
        MessageSender::new(self.tx.clone())
    }

    /// Background task that handles the actual socket I/O
    async fn connection_task(
        mut framed: Framed<UnixStream, ClientCodec>,
        mut outgoing: mpsc::Receiver<ClientMessage>,
        incoming: mpsc::Sender<ServerMessage>,
    ) {
        loop {
            tokio::select! {
                // Handle outgoing messages
                Some(msg) = outgoing.recv() => {
                    if let Err(e) = framed.send(msg).await {
                        tracing::error!("Failed to send message: {}", e);
                        break;
                    }
                }

                // Handle incoming messages
                result = framed.next() => {
                    match result {
                        Some(Ok(msg)) => {
                            tracing::debug!(
                                message_type = ?std::mem::discriminant(&msg),
                                "Received message from server socket"
                            );
                            if incoming.send(msg).await.is_err() {
                                // Receiver dropped
                                tracing::debug!("Incoming channel closed, receiver dropped");
                                break;
                            }
                        }
                        Some(Err(e)) => {
                            tracing::error!("Failed to receive message: {}", e);
                            break;
                        }
                        None => {
                            // Stream ended
                            tracing::info!("Server closed connection");
                            break;
                        }
                    }
                }
            }
        }
    }
}

impl Default for Connection {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use tokio::net::UnixListener;

    #[tokio::test]
    async fn test_connection_state_initial() {
        let conn = Connection::new();
        assert_eq!(conn.state(), ConnectionState::Disconnected);
    }

    #[tokio::test]
    async fn test_connect_no_server() {
        let mut conn = Connection::with_socket_path("/nonexistent/path.sock".into());
        let result = conn.connect().await;
        assert!(result.is_err());
        assert_eq!(conn.state(), ConnectionState::Disconnected);
    }

    #[tokio::test]
    async fn test_connect_to_server() {
        let dir = tempdir().unwrap();
        let socket_path = dir.path().join("test.sock");

        // Start a mock server
        let listener = UnixListener::bind(&socket_path).unwrap();

        // Connect client
        let mut conn = Connection::with_socket_path(socket_path);

        // Accept in background
        let accept_handle = tokio::spawn(async move { listener.accept().await.unwrap() });

        conn.connect().await.unwrap();
        assert_eq!(conn.state(), ConnectionState::Connected);

        // Clean up
        conn.disconnect().await;
        accept_handle.await.unwrap();
    }

    #[tokio::test]
    async fn test_connect_already_connected() {
        let dir = tempdir().unwrap();
        let socket_path = dir.path().join("test.sock");

        // Start a mock server
        let listener = UnixListener::bind(&socket_path).unwrap();

        // Connect client
        let mut conn = Connection::with_socket_path(socket_path);

        // Accept in background
        let accept_handle = tokio::spawn(async move { listener.accept().await.unwrap() });

        conn.connect().await.unwrap();
        assert_eq!(conn.state(), ConnectionState::Connected);

        // Connect again should be a no-op
        conn.connect().await.unwrap();
        assert_eq!(conn.state(), ConnectionState::Connected);

        // Clean up
        conn.disconnect().await;
        accept_handle.await.unwrap();
    }

    #[tokio::test]
    async fn test_send_not_connected() {
        let conn = Connection::new();
        let result = conn.send(ClientMessage::Ping).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_disconnect() {
        let dir = tempdir().unwrap();
        let socket_path = dir.path().join("test.sock");

        // Start a mock server
        let listener = UnixListener::bind(&socket_path).unwrap();

        // Connect client
        let mut conn = Connection::with_socket_path(socket_path);

        // Accept in background
        let accept_handle = tokio::spawn(async move { listener.accept().await.unwrap() });

        conn.connect().await.unwrap();
        assert_eq!(conn.state(), ConnectionState::Connected);

        conn.disconnect().await;
        assert_eq!(conn.state(), ConnectionState::Disconnected);

        accept_handle.await.unwrap();
    }

    #[test]
    fn test_connection_default() {
        let conn = Connection::default();
        assert_eq!(conn.state(), ConnectionState::Disconnected);
    }

    #[test]
    fn test_connection_state_debug() {
        assert_eq!(format!("{:?}", ConnectionState::Disconnected), "Disconnected");
        assert_eq!(format!("{:?}", ConnectionState::Connecting), "Connecting");
        assert_eq!(format!("{:?}", ConnectionState::Connected), "Connected");
        assert_eq!(format!("{:?}", ConnectionState::Reconnecting), "Reconnecting");
    }

    #[test]
    fn test_connection_state_clone() {
        let state = ConnectionState::Connected;
        let cloned = state.clone();
        assert_eq!(state, cloned);
    }

    #[test]
    fn test_connection_state_copy() {
        let state = ConnectionState::Connecting;
        let copied = state;
        assert_eq!(state, copied);
    }

    #[tokio::test]
    async fn test_try_recv_empty() {
        let mut conn = Connection::new();
        // Channel should be empty
        assert!(conn.try_recv().is_none());
    }

    #[tokio::test]
    async fn test_sender_returns_message_sender() {
        let conn = Connection::new();
        let _sender = conn.sender();
        // Just verify it compiles and returns a MessageSender
    }

    #[tokio::test]
    async fn test_with_socket_path_sets_path() {
        let path = PathBuf::from("/custom/socket.sock");
        let conn = Connection::with_socket_path(path.clone());
        assert_eq!(conn.socket_path, path);
    }

    #[tokio::test]
    async fn test_disconnect_when_not_connected() {
        let mut conn = Connection::new();
        // Should not panic
        conn.disconnect().await;
        assert_eq!(conn.state(), ConnectionState::Disconnected);
    }

    #[tokio::test]
    async fn test_state_transitions_on_failed_connect() {
        let mut conn = Connection::with_socket_path("/nonexistent/socket.sock".into());
        assert_eq!(conn.state(), ConnectionState::Disconnected);

        let _ = conn.connect().await;
        // Should return to Disconnected on failure
        assert_eq!(conn.state(), ConnectionState::Disconnected);
    }

    #[test]
    fn test_connection_state_equality() {
        assert_eq!(ConnectionState::Disconnected, ConnectionState::Disconnected);
        assert_eq!(ConnectionState::Connecting, ConnectionState::Connecting);
        assert_eq!(ConnectionState::Connected, ConnectionState::Connected);
        assert_eq!(ConnectionState::Reconnecting, ConnectionState::Reconnecting);

        assert_ne!(ConnectionState::Disconnected, ConnectionState::Connected);
        assert_ne!(ConnectionState::Connecting, ConnectionState::Reconnecting);
    }
}
