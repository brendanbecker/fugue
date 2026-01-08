//! Message codec for IPC framing

use bytes::{Buf, BufMut, BytesMut};
use tokio_util::codec::{Decoder, Encoder};

use crate::messages::{ClientMessage, ServerMessage};

/// Maximum message size (16 MB)
const MAX_MESSAGE_SIZE: usize = 16 * 1024 * 1024;

/// Protocol codec error
#[derive(Debug, thiserror::Error)]
pub enum CodecError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Bincode(#[from] bincode::Error),

    #[error("Message too large: {size} bytes (max {max})")]
    MessageTooLarge { size: usize, max: usize },
}

/// Codec for ClientMessage (encoding) and ServerMessage (decoding)
/// Used by the client side
pub struct ClientCodec;

impl ClientCodec {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ClientCodec {
    fn default() -> Self {
        Self::new()
    }
}

impl Decoder for ClientCodec {
    type Item = ServerMessage;
    type Error = CodecError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        decode_message(src)
    }
}

impl Encoder<ClientMessage> for ClientCodec {
    type Error = CodecError;

    fn encode(&mut self, item: ClientMessage, dst: &mut BytesMut) -> Result<(), Self::Error> {
        encode_message(&item, dst)
    }
}

/// Codec for ServerMessage (encoding) and ClientMessage (decoding)
/// Used by the server side
pub struct ServerCodec;

impl ServerCodec {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ServerCodec {
    fn default() -> Self {
        Self::new()
    }
}

impl Decoder for ServerCodec {
    type Item = ClientMessage;
    type Error = CodecError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        decode_message(src)
    }
}

impl Encoder<ServerMessage> for ServerCodec {
    type Error = CodecError;

    fn encode(&mut self, item: ServerMessage, dst: &mut BytesMut) -> Result<(), Self::Error> {
        encode_message(&item, dst)
    }
}

/// Decode a length-prefixed message
fn decode_message<T: serde::de::DeserializeOwned>(
    src: &mut BytesMut,
) -> Result<Option<T>, CodecError> {
    // Need at least 4 bytes for length prefix
    if src.len() < 4 {
        return Ok(None);
    }

    // Peek at length without consuming
    let len = u32::from_be_bytes([src[0], src[1], src[2], src[3]]) as usize;

    // Validate message size
    if len > MAX_MESSAGE_SIZE {
        return Err(CodecError::MessageTooLarge {
            size: len,
            max: MAX_MESSAGE_SIZE,
        });
    }

    // Check if we have the full message
    if src.len() < 4 + len {
        // Reserve space for the rest of the message
        src.reserve(4 + len - src.len());
        return Ok(None);
    }

    // Consume length prefix
    src.advance(4);

    // Extract message bytes
    let data = src.split_to(len);

    // Deserialize
    let msg: T = bincode::deserialize(&data)?;
    Ok(Some(msg))
}

/// Encode a length-prefixed message
fn encode_message<T: serde::Serialize>(item: &T, dst: &mut BytesMut) -> Result<(), CodecError> {
    let data = bincode::serialize(item)?;

    if data.len() > MAX_MESSAGE_SIZE {
        return Err(CodecError::MessageTooLarge {
            size: data.len(),
            max: MAX_MESSAGE_SIZE,
        });
    }

    dst.reserve(4 + data.len());
    dst.put_u32(data.len() as u32);
    dst.put_slice(&data);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_client_message_roundtrip() {
        let mut codec = ClientCodec::new();
        let mut server_codec = ServerCodec::new();

        let msg = ClientMessage::Connect {
            client_id: Uuid::new_v4(),
            protocol_version: 1,
        };

        let mut buf = BytesMut::new();
        codec.encode(msg.clone(), &mut buf).unwrap();

        let decoded = server_codec.decode(&mut buf).unwrap().unwrap();

        // Compare via debug string since ClientMessage doesn't impl PartialEq
        assert_eq!(format!("{:?}", msg), format!("{:?}", decoded));
    }

    #[test]
    fn test_server_message_roundtrip() {
        let mut codec = ServerCodec::new();
        let mut client_codec = ClientCodec::new();

        let msg = ServerMessage::Pong;

        let mut buf = BytesMut::new();
        codec.encode(msg.clone(), &mut buf).unwrap();

        let decoded = client_codec.decode(&mut buf).unwrap().unwrap();
        assert_eq!(format!("{:?}", msg), format!("{:?}", decoded));
    }

    #[test]
    fn test_partial_message() {
        let mut codec = ClientCodec::new();
        let mut server_codec = ServerCodec::new();

        let msg = ClientMessage::Ping;

        let mut buf = BytesMut::new();
        codec.encode(msg, &mut buf).unwrap();

        // Split buffer to simulate partial read
        let mut partial = buf.split_to(2);

        // Should return None for partial message
        assert!(server_codec.decode(&mut partial).unwrap().is_none());

        // Add rest of message
        partial.unsplit(buf);

        // Now should decode
        assert!(server_codec.decode(&mut partial).unwrap().is_some());
    }

    #[test]
    fn test_message_too_large_on_decode() {
        let mut codec = ServerCodec::new();
        let mut buf = BytesMut::new();

        // Write a length that exceeds MAX_MESSAGE_SIZE
        let huge_size: u32 = (MAX_MESSAGE_SIZE + 1) as u32;
        buf.put_u32(huge_size);

        let result = codec.decode(&mut buf);
        assert!(matches!(result, Err(CodecError::MessageTooLarge { .. })));
    }

    #[test]
    fn test_all_client_message_variants() {
        let mut codec = ClientCodec::new();
        let mut server_codec = ServerCodec::new();

        let messages = vec![
            ClientMessage::Connect {
                client_id: Uuid::new_v4(),
                protocol_version: 1,
            },
            ClientMessage::ListSessions,
            ClientMessage::CreateSession {
                name: "test".to_string(),
            },
            ClientMessage::AttachSession {
                session_id: Uuid::new_v4(),
            },
            ClientMessage::CreateWindow {
                session_id: Uuid::new_v4(),
                name: Some("window".to_string()),
            },
            ClientMessage::CreatePane {
                window_id: Uuid::new_v4(),
                direction: crate::types::SplitDirection::Horizontal,
            },
            ClientMessage::Input {
                pane_id: Uuid::new_v4(),
                data: vec![0x1b, 0x5b, 0x41], // Up arrow
            },
            ClientMessage::Resize {
                pane_id: Uuid::new_v4(),
                cols: 80,
                rows: 24,
            },
            ClientMessage::ClosePane {
                pane_id: Uuid::new_v4(),
            },
            ClientMessage::SelectPane {
                pane_id: Uuid::new_v4(),
            },
            ClientMessage::Detach,
            ClientMessage::Sync,
            ClientMessage::Ping,
        ];

        for msg in messages {
            let mut buf = BytesMut::new();
            codec.encode(msg.clone(), &mut buf).unwrap();
            let decoded = server_codec.decode(&mut buf).unwrap().unwrap();
            assert_eq!(format!("{:?}", msg), format!("{:?}", decoded));
        }
    }

    #[test]
    fn test_all_server_message_variants() {
        use crate::types::*;

        let mut codec = ServerCodec::new();
        let mut client_codec = ClientCodec::new();

        let session_id = Uuid::new_v4();
        let window_id = Uuid::new_v4();
        let pane_id = Uuid::new_v4();

        let messages = vec![
            ServerMessage::Connected {
                server_version: "1.0.0".to_string(),
                protocol_version: 1,
            },
            ServerMessage::SessionList {
                sessions: vec![SessionInfo {
                    id: session_id,
                    name: "test".to_string(),
                    created_at: 1234567890,
                    window_count: 1,
                    attached_clients: 0,
                }],
            },
            ServerMessage::SessionCreated {
                session: SessionInfo {
                    id: session_id,
                    name: "new".to_string(),
                    created_at: 1234567890,
                    window_count: 0,
                    attached_clients: 1,
                },
            },
            ServerMessage::Attached {
                session: SessionInfo {
                    id: session_id,
                    name: "test".to_string(),
                    created_at: 1234567890,
                    window_count: 1,
                    attached_clients: 1,
                },
                windows: vec![WindowInfo {
                    id: window_id,
                    session_id,
                    name: "main".to_string(),
                    index: 0,
                    pane_count: 1,
                    active_pane_id: Some(pane_id),
                }],
                panes: vec![PaneInfo {
                    id: pane_id,
                    window_id,
                    index: 0,
                    cols: 80,
                    rows: 24,
                    state: PaneState::Normal,
                    title: None,
                    cwd: Some("/home/user".to_string()),
                }],
            },
            ServerMessage::WindowCreated {
                window: WindowInfo {
                    id: window_id,
                    session_id,
                    name: "new window".to_string(),
                    index: 1,
                    pane_count: 0,
                    active_pane_id: None,
                },
            },
            ServerMessage::PaneCreated {
                pane: PaneInfo {
                    id: pane_id,
                    window_id,
                    index: 0,
                    cols: 80,
                    rows: 24,
                    state: PaneState::Normal,
                    title: None,
                    cwd: None,
                },
            },
            ServerMessage::Output {
                pane_id,
                data: b"Hello, World!\n".to_vec(),
            },
            ServerMessage::PaneStateChanged {
                pane_id,
                state: PaneState::Claude(ClaudeState::default()),
            },
            ServerMessage::ClaudeStateChanged {
                pane_id,
                state: ClaudeState {
                    session_id: Some("abc123".to_string()),
                    activity: ClaudeActivity::Thinking,
                    model: Some("claude-3-opus".to_string()),
                    tokens_used: Some(1500),
                },
            },
            ServerMessage::PaneClosed {
                pane_id,
                exit_code: Some(0),
            },
            ServerMessage::WindowClosed { window_id },
            ServerMessage::SessionEnded { session_id },
            ServerMessage::Error {
                code: crate::messages::ErrorCode::SessionNotFound,
                message: "Session not found".to_string(),
            },
            ServerMessage::Pong,
        ];

        for msg in messages {
            let mut buf = BytesMut::new();
            codec.encode(msg.clone(), &mut buf).unwrap();
            let decoded = client_codec.decode(&mut buf).unwrap().unwrap();
            assert_eq!(format!("{:?}", msg), format!("{:?}", decoded));
        }
    }

    #[test]
    fn test_multiple_messages_in_buffer() {
        let mut codec = ClientCodec::new();
        let mut server_codec = ServerCodec::new();

        let msg1 = ClientMessage::Ping;
        let msg2 = ClientMessage::ListSessions;
        let msg3 = ClientMessage::Sync;

        let mut buf = BytesMut::new();
        codec.encode(msg1.clone(), &mut buf).unwrap();
        codec.encode(msg2.clone(), &mut buf).unwrap();
        codec.encode(msg3.clone(), &mut buf).unwrap();

        let decoded1 = server_codec.decode(&mut buf).unwrap().unwrap();
        let decoded2 = server_codec.decode(&mut buf).unwrap().unwrap();
        let decoded3 = server_codec.decode(&mut buf).unwrap().unwrap();

        assert_eq!(format!("{:?}", msg1), format!("{:?}", decoded1));
        assert_eq!(format!("{:?}", msg2), format!("{:?}", decoded2));
        assert_eq!(format!("{:?}", msg3), format!("{:?}", decoded3));

        // Buffer should be empty now
        assert!(server_codec.decode(&mut buf).unwrap().is_none());
    }
}
