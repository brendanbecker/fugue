//! Simple synchronous client for ccmux protocol
//!
//! This client is designed for one-shot command execution, unlike the
//! interactive client in ccmux-client which handles streaming updates.

use std::path::PathBuf;
use std::time::Duration;

use futures::{SinkExt, StreamExt};
use tokio::net::UnixStream;
use tokio::time::timeout;
use tokio_util::codec::Framed;
use uuid::Uuid;

use ccmux_protocol::{ClientCodec, ClientMessage, ServerMessage, PROTOCOL_VERSION};
use ccmux_utils::{socket_path, CcmuxError, Result};

/// Timeout for server responses
const RESPONSE_TIMEOUT: Duration = Duration::from_secs(10);

/// Simple client for one-shot commands
pub struct Client {
    framed: Framed<UnixStream, ClientCodec>,
    client_id: Uuid,
}

impl Client {
    /// Connect to the ccmux server
    pub async fn connect() -> Result<Self> {
        Self::connect_to(socket_path()).await
    }

    /// Connect to a specific socket path
    pub async fn connect_to(path: PathBuf) -> Result<Self> {
        // Check if socket exists
        if !path.exists() {
            return Err(CcmuxError::ServerNotRunning { path });
        }

        // Connect to Unix socket
        let stream = UnixStream::connect(&path)
            .await
            .map_err(|e| CcmuxError::Connection(format!("Failed to connect: {}", e)))?;

        // Create framed transport with codec
        let framed = Framed::new(stream, ClientCodec::new());
        let client_id = Uuid::new_v4();

        let mut client = Self { framed, client_id };

        // Send initial connect message
        client.handshake().await?;

        Ok(client)
    }

    /// Perform initial handshake
    async fn handshake(&mut self) -> Result<()> {
        let connect_msg = ClientMessage::Connect {
            client_id: self.client_id,
            protocol_version: PROTOCOL_VERSION,
        };

        self.framed
            .send(connect_msg)
            .await
            .map_err(|e| CcmuxError::Connection(format!("Failed to send handshake: {}", e)))?;

        // Wait for Connected response
        match self.recv().await? {
            ServerMessage::Connected { .. } => Ok(()),
            ServerMessage::Error { code, message } => {
                Err(CcmuxError::Protocol(format!("{:?}: {}", code, message)))
            }
            other => Err(CcmuxError::Protocol(format!(
                "Unexpected response to Connect: {:?}",
                std::mem::discriminant(&other)
            ))),
        }
    }

    /// Send a message to the server
    pub async fn send(&mut self, msg: ClientMessage) -> Result<()> {
        self.framed
            .send(msg)
            .await
            .map_err(|e| CcmuxError::Connection(format!("Failed to send: {}", e)))
    }

    /// Receive a message from the server with timeout
    pub async fn recv(&mut self) -> Result<ServerMessage> {
        match timeout(RESPONSE_TIMEOUT, self.framed.next()).await {
            Ok(Some(Ok(msg))) => Ok(msg),
            Ok(Some(Err(e))) => Err(CcmuxError::Connection(format!("Failed to receive: {}", e))),
            Ok(None) => Err(CcmuxError::ConnectionClosed),
            Err(_) => Err(CcmuxError::Connection("Response timeout".to_string())),
        }
    }

    /// Send a message and wait for a response
    pub async fn request(&mut self, msg: ClientMessage) -> Result<ServerMessage> {
        self.send(msg).await?;
        self.recv().await
    }
}
