//! Simple synchronous client for fugue protocol
//!
//! This client is designed for one-shot command execution, unlike the
//! interactive client in fugue-client which handles streaming updates.

use std::path::PathBuf;
use std::time::Duration;

use futures::{SinkExt, StreamExt};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::{TcpStream, UnixStream};
use tokio::time::timeout;
use tokio_util::codec::Framed;
use url::Url;
use uuid::Uuid;

use fugue_protocol::{ClientCodec, ClientMessage, ClientType, ServerMessage, PROTOCOL_VERSION};
use fugue_utils::{socket_path, CcmuxError, Result};

/// Timeout for server responses
const RESPONSE_TIMEOUT: Duration = Duration::from_secs(10);

/// Trait alias for streams that can be used with Framed
pub trait StreamTrait: AsyncRead + AsyncWrite + Unpin + Send {}
impl<T: AsyncRead + AsyncWrite + Unpin + Send> StreamTrait for T {}

/// Simple client for one-shot commands
pub struct Client {
    framed: Framed<Box<dyn StreamTrait>, ClientCodec>,
    client_id: Uuid,
}

impl Client {
    /// Connect to the fugue server using default or provided address
    pub async fn connect(addr: Option<String>) -> Result<Self> {
        let addr = addr.unwrap_or_else(|| {
            format!("unix://{}", socket_path().to_string_lossy())
        });
        Self::connect_to_addr(&addr).await
    }

    /// Connect to a specific address URL
    pub async fn connect_to_addr(addr: &str) -> Result<Self> {
        let stream: Box<dyn StreamTrait> = if addr.starts_with("tcp://") {
            let url = Url::parse(addr).map_err(|e| {
                CcmuxError::Connection(format!("Invalid TCP URL '{}': {}", addr, e))
            })?;
            
            let host = url.host_str().ok_or_else(|| {
                CcmuxError::Connection("Missing host in TCP URL".into())
            })?;
            let port = url.port().ok_or_else(|| {
                CcmuxError::Connection("Missing port in TCP URL".into())
            })?;
            
            let tcp_addr = format!("{}:{}", host, port);
            let tcp_stream = TcpStream::connect(&tcp_addr).await.map_err(|e| {
                CcmuxError::Connection(format!("Failed to connect to {}: {}", tcp_addr, e))
            })?;
            
            Box::new(tcp_stream)
        } else {
            // Assume Unix socket
            let path_str = if addr.starts_with("unix://") {
                let url = Url::parse(addr).map_err(|e| {
                    CcmuxError::Connection(format!("Invalid Unix URL: {}", e))
                })?;
                url.path().to_string()
            } else {
                addr.to_string()
            };
            
            let path = PathBuf::from(path_str);
            if !path.exists() {
                return Err(CcmuxError::ServerNotRunning { path });
            }

            let unix_stream = UnixStream::connect(&path)
                .await
                .map_err(|e| CcmuxError::Connection(format!("Failed to connect to {}: {}", path.display(), e)))?;
            
            Box::new(unix_stream)
        };

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
            client_type: ClientType::Compat,
        };

        self.framed
            .send(connect_msg)
            .await
            .map_err(|e| CcmuxError::Connection(format!("Failed to send handshake: {}", e)))?;

        // Wait for Connected response
        match self.recv().await? {
            ServerMessage::Connected { .. } => Ok(()),
            ServerMessage::Error { code, message, .. } => {
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