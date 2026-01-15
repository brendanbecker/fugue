//! Message codec for IPC framing

use bytes::{Buf, BufMut, BytesMut};
use tokio_util::codec::{Decoder, Encoder};

use crate::messages::{ClientMessage, ServerMessage, ClientType};

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
    use std::collections::{HashMap, HashSet};
    use uuid::Uuid;

    #[test]
    fn test_client_message_roundtrip() {
        let mut codec = ClientCodec::new();
        let mut server_codec = ServerCodec::new();

        let msg = ClientMessage::Connect {
            client_id: Uuid::new_v4(),
            protocol_version: 1,
            client_type: Some(ClientType::Tui),
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
                client_type: None,
            },
            ClientMessage::ListSessions,
            ClientMessage::CreateSession {
                name: "test".to_string(),
                command: None,
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
            ClientMessage::SetViewportOffset {
                pane_id: Uuid::new_v4(),
                offset: 100,
            },
            ClientMessage::JumpToBottom {
                pane_id: Uuid::new_v4(),
            },
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
                    worktree: None,
                    tags: HashSet::new(),
                    metadata: HashMap::new(),
                }],
            },
            ServerMessage::SessionCreated {
                session: SessionInfo {
                    id: session_id,
                    name: "new".to_string(),
                    created_at: 1234567890,
                    window_count: 0,
                    attached_clients: 1,
                    worktree: None,
                    tags: HashSet::new(),
                    metadata: HashMap::new(),
                },
                should_focus: true,
            },
            ServerMessage::Attached {
                session: SessionInfo {
                    id: session_id,
                    name: "test".to_string(),
                    created_at: 1234567890,
                    window_count: 1,
                    attached_clients: 1,
                    worktree: None,
                    tags: HashSet::new(),
                    metadata: HashMap::new(),
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
                    name: None,
                    title: None,
                    cwd: Some("/home/user".to_string()),
                }],
                commit_seq: 42,
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
                should_focus: true,
            },
            ServerMessage::PaneCreated {
                pane: PaneInfo {
                    id: pane_id,
                    window_id,
                    index: 0,
                    cols: 80,
                    rows: 24,
                    state: PaneState::Normal,
                    name: None,
                    title: None,
                    cwd: None,
                },
                direction: crate::types::SplitDirection::Horizontal,
                should_focus: false,
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
                details: None,
            },
            ServerMessage::Pong,
            ServerMessage::ViewportUpdated {
                pane_id,
                state: ViewportState::pinned(50),
            },
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

    /// BUG-035: Stress test for response type consistency under heavy serialization load
    ///
    /// This test verifies that bincode serialization/deserialization preserves
    /// the correct enum discriminant after many round-trips. The bug manifested
    /// as wrong response types (e.g., list_windows returning SessionList).
    #[test]
    fn test_response_type_consistency_serialization_bug035() {
        use crate::types::*;
        use crate::messages::PaneListEntry;

        let mut codec = ServerCodec::new();
        let mut client_codec = ClientCodec::new();

        let session_id = Uuid::new_v4();
        let window_id = Uuid::new_v4();
        let pane_id = Uuid::new_v4();

        // Create the three message types that were affected by BUG-035
        let session_list = ServerMessage::SessionList {
            sessions: vec![SessionInfo {
                id: session_id,
                name: "test-session".to_string(),
                created_at: 1234567890,
                window_count: 3,
                attached_clients: 1,
                worktree: None,
                tags: HashSet::new(),
                metadata: HashMap::new(),
            }],
        };

        let window_list = ServerMessage::WindowList {
            session_name: "test-session".to_string(),
            windows: vec![
                WindowInfo {
                    id: window_id,
                    session_id,
                    name: "main".to_string(),
                    index: 0,
                    pane_count: 1,
                    active_pane_id: Some(pane_id),
                },
                WindowInfo {
                    id: Uuid::new_v4(),
                    session_id,
                    name: "window-2".to_string(),
                    index: 1,
                    pane_count: 1,
                    active_pane_id: None,
                },
            ],
        };

        let panes_list = ServerMessage::AllPanesList {
            panes: vec![PaneListEntry {
                id: pane_id,
                session_name: "test-session".to_string(),
                window_index: 0,
                window_name: "main".to_string(),
                pane_index: 0,
                cols: 80,
                rows: 24,
                name: None,
                title: Some("bash".to_string()),
                cwd: Some("/home/user".to_string()),
                state: PaneState::Normal,
                is_claude: false,
                claude_state: None,
                is_focused: true,
            }],
        };

        let mut errors: Vec<String> = Vec::new();

        // Serialize and deserialize each message type 200 times
        for i in 0..200 {
            // Test SessionList
            {
                let mut buf = BytesMut::new();
                codec.encode(session_list.clone(), &mut buf).unwrap();
                let decoded = client_codec.decode(&mut buf).unwrap().unwrap();

                match decoded {
                    ServerMessage::SessionList { .. } => {}
                    other => {
                        errors.push(format!(
                            "Iteration {}: SessionList decoded as {:?}",
                            i,
                            std::mem::discriminant(&other)
                        ));
                    }
                }
            }

            // Test WindowList
            {
                let mut buf = BytesMut::new();
                codec.encode(window_list.clone(), &mut buf).unwrap();
                let decoded = client_codec.decode(&mut buf).unwrap().unwrap();

                match decoded {
                    ServerMessage::WindowList { .. } => {}
                    other => {
                        errors.push(format!(
                            "Iteration {}: WindowList decoded as {:?}",
                            i,
                            std::mem::discriminant(&other)
                        ));
                    }
                }
            }

            // Test AllPanesList
            {
                let mut buf = BytesMut::new();
                codec.encode(panes_list.clone(), &mut buf).unwrap();
                let decoded = client_codec.decode(&mut buf).unwrap().unwrap();

                match decoded {
                    ServerMessage::AllPanesList { .. } => {}
                    other => {
                        errors.push(format!(
                            "Iteration {}: AllPanesList decoded as {:?}",
                            i,
                            std::mem::discriminant(&other)
                        ));
                    }
                }
            }
        }

        if !errors.is_empty() {
            panic!(
                "BUG-035: Serialization type corruption detected:\n{}",
                errors.join("\n")
            );
        }
    }

    /// BUG-035: Test interleaved message types to catch queue ordering issues
    #[test]
    fn test_interleaved_response_types_bug035() {
        use crate::types::*;
        use crate::messages::PaneListEntry;

        let mut codec = ServerCodec::new();
        let mut client_codec = ClientCodec::new();

        let session_id = Uuid::new_v4();
        let window_id = Uuid::new_v4();
        let pane_id = Uuid::new_v4();

        let session_list = ServerMessage::SessionList {
            sessions: vec![SessionInfo {
                id: session_id,
                name: "test".to_string(),
                created_at: 0,
                window_count: 1,
                attached_clients: 0,
                worktree: None,
                tags: HashSet::new(),
                metadata: HashMap::new(),
            }],
        };

        let window_list = ServerMessage::WindowList {
            session_name: "test".to_string(),
            windows: vec![WindowInfo {
                id: window_id,
                session_id,
                name: "main".to_string(),
                index: 0,
                pane_count: 1,
                active_pane_id: Some(pane_id),
            }],
        };

        let panes_list = ServerMessage::AllPanesList {
            panes: vec![PaneListEntry {
                id: pane_id,
                session_name: "test".to_string(),
                window_index: 0,
                window_name: "main".to_string(),
                pane_index: 0,
                cols: 80,
                rows: 24,
                name: None,
                title: None,
                cwd: None,
                state: PaneState::Normal,
                is_claude: false,
                claude_state: None,
                is_focused: false,
            }],
        };

        // Encode multiple messages into the same buffer (simulating buffered I/O)
        let mut buf = BytesMut::new();

        // Interleave the message types 100 times each
        for _ in 0..100 {
            codec.encode(session_list.clone(), &mut buf).unwrap();
            codec.encode(window_list.clone(), &mut buf).unwrap();
            codec.encode(panes_list.clone(), &mut buf).unwrap();
        }

        // Now decode and verify order
        for i in 0..100 {
            // Should get SessionList
            let decoded = client_codec.decode(&mut buf).unwrap().unwrap();
            assert!(
                matches!(decoded, ServerMessage::SessionList { .. }),
                "Iteration {}: expected SessionList, got {:?}",
                i,
                std::mem::discriminant(&decoded)
            );

            // Should get WindowList
            let decoded = client_codec.decode(&mut buf).unwrap().unwrap();
            assert!(
                matches!(decoded, ServerMessage::WindowList { .. }),
                "Iteration {}: expected WindowList, got {:?}",
                i,
                std::mem::discriminant(&decoded)
            );

            // Should get AllPanesList
            let decoded = client_codec.decode(&mut buf).unwrap().unwrap();
            assert!(
                matches!(decoded, ServerMessage::AllPanesList { .. }),
                "Iteration {}: expected AllPanesList, got {:?}",
                i,
                std::mem::discriminant(&decoded)
            );
        }

        // Buffer should be empty
        assert!(client_codec.decode(&mut buf).unwrap().is_none());
    }
}
