//! Persistence types for checkpoint and WAL
//!
//! This module defines the data structures used for persistence:
//! - Snapshot types that capture session/window/pane state
//! - WAL entry types for incremental state changes
//! - Checkpoint format for full state snapshots

// Scaffolding for crash recovery feature - not all types are used yet
#![allow(dead_code)]

use fugue_protocol::PaneState;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Current checkpoint format version
///
/// Version history:
/// - 1: Initial format with ClaudeState
/// - 2: Added AgentState variant to PaneState (FEAT-084)
pub const CHECKPOINT_VERSION: u32 = 2;

/// Magic bytes for checkpoint file identification
pub const CHECKPOINT_MAGIC: [u8; 4] = *b"CCCP"; // CcmuX Checkpoint

/// A complete checkpoint of all session state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    /// Format version for compatibility
    pub version: u32,
    /// Unix timestamp when checkpoint was created
    pub timestamp: u64,
    /// Sequence number (monotonically increasing)
    pub sequence: u64,
    /// All sessions in the checkpoint
    pub sessions: Vec<SessionSnapshot>,
}

impl Checkpoint {
    /// Create a new empty checkpoint
    pub fn new(sequence: u64) -> Self {
        Self {
            version: CHECKPOINT_VERSION,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
            sequence,
            sessions: Vec::new(),
        }
    }
}

/// Snapshot of a session for persistence
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SessionSnapshot {
    /// Session unique identifier
    pub id: Uuid,
    /// Session name
    pub name: String,
    /// Windows in order
    pub windows: Vec<WindowSnapshot>,
    /// Active window ID
    pub active_window_id: Option<Uuid>,
    /// Creation timestamp (Unix seconds)
    pub created_at: u64,
    /// Arbitrary key-value metadata (backward compatible with empty default)
    #[serde(default)]
    pub metadata: HashMap<String, String>,
    /// Session environment variables (backward compatible with empty default)
    #[serde(default)]
    pub environment: HashMap<String, String>,
}

/// Snapshot of a window for persistence
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WindowSnapshot {
    /// Window unique identifier
    pub id: Uuid,
    /// Parent session ID
    pub session_id: Uuid,
    /// Window name
    pub name: String,
    /// Window index within session
    pub index: usize,
    /// Panes in order
    pub panes: Vec<PaneSnapshot>,
    /// Active pane ID
    pub active_pane_id: Option<Uuid>,
    /// Creation timestamp (Unix seconds)
    pub created_at: u64,
}

/// Snapshot of a pane for persistence
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PaneSnapshot {
    /// Pane unique identifier
    pub id: Uuid,
    /// Parent window ID
    pub window_id: Uuid,
    /// Pane index within window
    pub index: usize,
    /// Terminal columns
    pub cols: u16,
    /// Terminal rows
    pub rows: u16,
    /// Pane state
    pub state: PaneState,
    /// User-assigned name (FEAT-036)
    #[serde(default)]
    pub name: Option<String>,
    /// Terminal title
    pub title: Option<String>,
    /// Current working directory
    pub cwd: Option<String>,
    /// Creation timestamp (Unix seconds)
    pub created_at: u64,
    /// Scrollback content (compressed)
    pub scrollback: Option<ScrollbackSnapshot>,
}

/// Scrollback buffer snapshot
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ScrollbackSnapshot {
    /// Total lines in scrollback
    pub line_count: usize,
    /// Compressed scrollback content
    pub compressed_data: Vec<u8>,
    /// Compression method used
    pub compression: CompressionMethod,
}

/// Compression method for scrollback data
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum CompressionMethod {
    /// No compression
    #[default]
    None,
    /// LZ4 compression (fast)
    Lz4,
    /// Zstd compression (better ratio)
    Zstd,
}

/// Write-ahead log entry types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WalEntry {
    /// Session was created
    SessionCreated {
        id: Uuid,
        name: String,
        created_at: u64,
    },

    /// Session was destroyed
    SessionDestroyed { id: Uuid },

    /// Session was renamed
    SessionRenamed { id: Uuid, new_name: String },

    /// Window was created
    WindowCreated {
        id: Uuid,
        session_id: Uuid,
        name: String,
        index: usize,
        created_at: u64,
    },

    /// Window was destroyed
    WindowDestroyed { id: Uuid, session_id: Uuid },

    /// Window was renamed
    WindowRenamed { id: Uuid, new_name: String },

    /// Active window changed
    ActiveWindowChanged {
        session_id: Uuid,
        window_id: Option<Uuid>,
    },

    /// Pane was created
    PaneCreated {
        id: Uuid,
        window_id: Uuid,
        index: usize,
        cols: u16,
        rows: u16,
        created_at: u64,
    },

    /// Pane was destroyed
    PaneDestroyed { id: Uuid, window_id: Uuid },

    /// Pane was resized
    PaneResized { id: Uuid, cols: u16, rows: u16 },

    /// Pane state changed
    PaneStateChanged { id: Uuid, state: PaneState },

    /// Pane title changed
    PaneTitleChanged { id: Uuid, title: Option<String> },

    /// Pane working directory changed
    PaneCwdChanged { id: Uuid, cwd: Option<String> },

    /// Active pane changed
    ActivePaneChanged {
        window_id: Uuid,
        pane_id: Option<Uuid>,
    },

    /// Terminal output (for scrollback)
    PaneOutput {
        pane_id: Uuid,
        /// Compressed output data
        data: Vec<u8>,
    },

    /// Checkpoint marker (indicates a checkpoint was taken)
    CheckpointMarker {
        sequence: u64,
        timestamp: u64,
    },

    /// Session metadata changed
    SessionMetadataSet {
        session_id: Uuid,
        key: String,
        value: String,
    },

    /// Session environment variable changed
    SessionEnvironmentSet {
        session_id: Uuid,
        key: String,
        value: String,
    },
}

impl WalEntry {
    /// Get the sequence number for checkpoint markers
    pub fn checkpoint_sequence(&self) -> Option<u64> {
        match self {
            WalEntry::CheckpointMarker { sequence, .. } => Some(*sequence),
            _ => None,
        }
    }
}

/// WAL segment metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalSegmentHeader {
    /// Segment sequence number
    pub segment_id: u64,
    /// First entry sequence in this segment
    pub first_sequence: u64,
    /// Number of entries in segment
    pub entry_count: u64,
    /// Creation timestamp
    pub created_at: u64,
}

/// Recovery state after loading checkpoint and replaying WAL
#[derive(Debug, Clone, Default)]
pub struct RecoveryState {
    /// Sessions recovered
    pub sessions: Vec<SessionSnapshot>,
    /// Last checkpoint sequence number
    pub last_checkpoint_sequence: u64,
    /// Number of WAL entries replayed
    pub wal_entries_replayed: u64,
    /// Whether recovery was clean (vs. crash recovery)
    pub clean_shutdown: bool,
    /// Any warnings during recovery
    pub warnings: Vec<String>,
}

impl RecoveryState {
    /// Check if any sessions were recovered
    pub fn has_sessions(&self) -> bool {
        !self.sessions.is_empty()
    }

    /// Get session count
    pub fn session_count(&self) -> usize {
        self.sessions.len()
    }

    /// Add a warning message
    pub fn add_warning(&mut self, warning: impl Into<String>) {
        self.warnings.push(warning.into());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fugue_protocol::AgentState;

    #[test]
    fn test_checkpoint_new() {
        let checkpoint = Checkpoint::new(42);

        assert_eq!(checkpoint.version, CHECKPOINT_VERSION);
        assert_eq!(checkpoint.sequence, 42);
        assert!(checkpoint.sessions.is_empty());
        assert!(checkpoint.timestamp > 0);
    }

    #[test]
    fn test_checkpoint_serde() {
        let mut checkpoint = Checkpoint::new(1);
        checkpoint.sessions.push(SessionSnapshot {
            id: Uuid::new_v4(),
            name: "test".to_string(),
            windows: Vec::new(),
            active_window_id: None,
            created_at: 12345,
            metadata: HashMap::new(),
            environment: HashMap::new(),
        });

        let serialized = bincode::serialize(&checkpoint).unwrap();
        let deserialized: Checkpoint = bincode::deserialize(&serialized).unwrap();

        assert_eq!(deserialized.version, checkpoint.version);
        assert_eq!(deserialized.sequence, checkpoint.sequence);
        assert_eq!(deserialized.sessions.len(), 1);
        assert_eq!(deserialized.sessions[0].name, "test");
    }

    #[test]
    fn test_session_snapshot_serde() {
        let snapshot = SessionSnapshot {
            id: Uuid::new_v4(),
            name: "work".to_string(),
            windows: vec![WindowSnapshot {
                id: Uuid::new_v4(),
                session_id: Uuid::new_v4(),
                name: "main".to_string(),
                index: 0,
                panes: Vec::new(),
                active_pane_id: None,
                created_at: 1000,
            }],
            active_window_id: None,
            created_at: 1000,
            metadata: HashMap::new(),
            environment: HashMap::new(),
        };

        let serialized = bincode::serialize(&snapshot).unwrap();
        let deserialized: SessionSnapshot = bincode::deserialize(&serialized).unwrap();

        assert_eq!(snapshot, deserialized);
    }

    #[test]
    fn test_pane_snapshot_with_state() {
        let snapshot = PaneSnapshot {
            id: Uuid::new_v4(),
            window_id: Uuid::new_v4(),
            index: 0,
            cols: 80,
            rows: 24,
            state: PaneState::Agent(AgentState::new("claude")),
            name: None,
            title: Some("vim".to_string()),
            cwd: Some("/home/user".to_string()),
            created_at: 1000,
            scrollback: Some(ScrollbackSnapshot {
                line_count: 100,
                compressed_data: vec![1, 2, 3],
                compression: CompressionMethod::None,
            }),
        };

        let serialized = bincode::serialize(&snapshot).unwrap();
        let deserialized: PaneSnapshot = bincode::deserialize(&serialized).unwrap();

        assert_eq!(snapshot, deserialized);
    }

    #[test]
    fn test_wal_entry_session_created() {
        let entry = WalEntry::SessionCreated {
            id: Uuid::new_v4(),
            name: "test".to_string(),
            created_at: 12345,
        };

        let serialized = bincode::serialize(&entry).unwrap();
        let deserialized: WalEntry = bincode::deserialize(&serialized).unwrap();

        if let WalEntry::SessionCreated { name, created_at, .. } = deserialized {
            assert_eq!(name, "test");
            assert_eq!(created_at, 12345);
        } else {
            panic!("Expected SessionCreated variant");
        }
    }

    #[test]
    fn test_wal_entry_pane_state_changed() {
        let entry = WalEntry::PaneStateChanged {
            id: Uuid::new_v4(),
            state: PaneState::Exited { code: Some(0) },
        };

        let serialized = bincode::serialize(&entry).unwrap();
        let deserialized: WalEntry = bincode::deserialize(&serialized).unwrap();

        if let WalEntry::PaneStateChanged { state, .. } = deserialized {
            assert!(matches!(state, PaneState::Exited { code: Some(0) }));
        } else {
            panic!("Expected PaneStateChanged variant");
        }
    }

    #[test]
    fn test_wal_entry_checkpoint_marker() {
        let entry = WalEntry::CheckpointMarker {
            sequence: 100,
            timestamp: 12345,
        };

        assert_eq!(entry.checkpoint_sequence(), Some(100));

        let entry = WalEntry::SessionCreated {
            id: Uuid::new_v4(),
            name: "test".to_string(),
            created_at: 0,
        };
        assert_eq!(entry.checkpoint_sequence(), None);
    }

    #[test]
    fn test_recovery_state_default() {
        let state = RecoveryState::default();

        assert!(!state.has_sessions());
        assert_eq!(state.session_count(), 0);
        assert!(state.warnings.is_empty());
    }

    #[test]
    fn test_recovery_state_with_sessions() {
        let mut state = RecoveryState::default();
        state.sessions.push(SessionSnapshot {
            id: Uuid::new_v4(),
            name: "test".to_string(),
            windows: Vec::new(),
            active_window_id: None,
            created_at: 0,
            metadata: HashMap::new(),
            environment: HashMap::new(),
        });

        assert!(state.has_sessions());
        assert_eq!(state.session_count(), 1);
    }

    #[test]
    fn test_recovery_state_warnings() {
        let mut state = RecoveryState::default();
        state.add_warning("Warning 1");
        state.add_warning("Warning 2".to_string());

        assert_eq!(state.warnings.len(), 2);
        assert_eq!(state.warnings[0], "Warning 1");
        assert_eq!(state.warnings[1], "Warning 2");
    }

    #[test]
    fn test_compression_method_default() {
        assert_eq!(CompressionMethod::default(), CompressionMethod::None);
    }

    #[test]
    fn test_all_wal_entries_serde() {
        let entries: Vec<WalEntry> = vec![
            WalEntry::SessionCreated {
                id: Uuid::new_v4(),
                name: "test".to_string(),
                created_at: 0,
            },
            WalEntry::SessionDestroyed { id: Uuid::new_v4() },
            WalEntry::SessionRenamed {
                id: Uuid::new_v4(),
                new_name: "new".to_string(),
            },
            WalEntry::WindowCreated {
                id: Uuid::new_v4(),
                session_id: Uuid::new_v4(),
                name: "win".to_string(),
                index: 0,
                created_at: 0,
            },
            WalEntry::WindowDestroyed {
                id: Uuid::new_v4(),
                session_id: Uuid::new_v4(),
            },
            WalEntry::WindowRenamed {
                id: Uuid::new_v4(),
                new_name: "new".to_string(),
            },
            WalEntry::ActiveWindowChanged {
                session_id: Uuid::new_v4(),
                window_id: Some(Uuid::new_v4()),
            },
            WalEntry::PaneCreated {
                id: Uuid::new_v4(),
                window_id: Uuid::new_v4(),
                index: 0,
                cols: 80,
                rows: 24,
                created_at: 0,
            },
            WalEntry::PaneDestroyed {
                id: Uuid::new_v4(),
                window_id: Uuid::new_v4(),
            },
            WalEntry::PaneResized {
                id: Uuid::new_v4(),
                cols: 120,
                rows: 40,
            },
            WalEntry::PaneStateChanged {
                id: Uuid::new_v4(),
                state: PaneState::Normal,
            },
            WalEntry::PaneTitleChanged {
                id: Uuid::new_v4(),
                title: Some("title".to_string()),
            },
            WalEntry::PaneCwdChanged {
                id: Uuid::new_v4(),
                cwd: Some("/tmp".to_string()),
            },
            WalEntry::ActivePaneChanged {
                window_id: Uuid::new_v4(),
                pane_id: Some(Uuid::new_v4()),
            },
            WalEntry::PaneOutput {
                pane_id: Uuid::new_v4(),
                data: vec![1, 2, 3],
            },
            WalEntry::CheckpointMarker {
                sequence: 1,
                timestamp: 12345,
            },
        ];

        for entry in entries {
            let serialized = bincode::serialize(&entry).unwrap();
            let _deserialized: WalEntry = bincode::deserialize(&serialized).unwrap();
        }
    }
}
