//! Persistence module for checkpoint and WAL-based crash recovery
//!
//! This module provides durable storage for session state using:
//! - Write-Ahead Log (WAL) for incremental state changes
//! - Periodic checkpoints for full state snapshots
//! - Recovery logic to restore state after crashes
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                     PersistenceManager                       │
//! │  ┌─────────────────┐  ┌────────────────┐  ┌──────────────┐  │
//! │  │ CheckpointMgr   │  │      WAL       │  │ RecoveryMgr  │  │
//! │  │                 │  │                │  │              │  │
//! │  │ - create()      │  │ - append()     │  │ - recover()  │  │
//! │  │ - load_latest() │  │ - read_all()   │  │              │  │
//! │  │ - validate()    │  │ - sync()       │  │              │  │
//! │  └─────────────────┘  └────────────────┘  └──────────────┘  │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Usage
//!
//! ```ignore
//! use ccmux_server::persistence::{PersistenceManager, PersistenceConfig};
//!
//! // Create persistence manager
//! let config = PersistenceConfig::default();
//! let mut manager = PersistenceManager::new("~/.ccmux/state", config)?;
//!
//! // Recover state on startup
//! let state = manager.recover()?;
//!
//! // Log state changes
//! manager.log_session_created(session_id, "work")?;
//!
//! // Create periodic checkpoints
//! manager.create_checkpoint(sessions)?;
//! ```

pub mod checkpoint;
pub mod recovery;
pub mod restoration;
pub mod scrollback;
pub mod types;
pub mod wal;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use parking_lot::Mutex;
use tracing::{info, warn};
use uuid::Uuid;

use ccmux_protocol::PaneState;
use ccmux_utils::{CcmuxError, Result};

// Re-exports for public API - allow unused during development
#[allow(unused_imports)]
pub use checkpoint::{CheckpointConfig, CheckpointManager};
#[allow(unused_imports)]
pub use recovery::{
    detect_unclean_shutdown, mark_clean_shutdown, mark_server_running, RecoveryManager,
};
#[allow(unused_imports)]
pub use restoration::{
    PaneRestorationResult, RestorationResult, SessionRestorationResult, SessionRestorer,
};
#[allow(unused_imports)]
pub use scrollback::{ScrollbackCapture, ScrollbackConfig, ScrollbackRestore};
#[allow(unused_imports)]
pub use types::{
    Checkpoint, CompressionMethod, PaneSnapshot, RecoveryState, ScrollbackSnapshot,
    SessionSnapshot, WalEntry, WindowSnapshot, CHECKPOINT_MAGIC, CHECKPOINT_VERSION,
};
#[allow(unused_imports)]
pub use wal::{Wal, WalConfig, WalReader};

/// Default state directory path
pub const DEFAULT_STATE_DIR: &str = ".ccmux/state";

/// Configuration for the persistence subsystem
#[derive(Debug, Clone)]
pub struct PersistenceConfig {
    /// Checkpoint interval in seconds
    pub checkpoint_interval_secs: u64,
    /// Maximum WAL size in MB before forced checkpoint
    pub max_wal_size_mb: u64,
    /// Number of scrollback lines to persist
    pub screen_snapshot_lines: usize,
    /// Maximum checkpoints to keep
    pub max_checkpoints: usize,
    /// Whether to sync WAL on each write
    pub sync_on_write: bool,
}

impl Default for PersistenceConfig {
    fn default() -> Self {
        Self {
            checkpoint_interval_secs: 30,
            max_wal_size_mb: 128,
            screen_snapshot_lines: 500,
            max_checkpoints: 5,
            sync_on_write: true,
        }
    }
}

impl From<&crate::config::PersistenceConfig> for PersistenceConfig {
    fn from(schema: &crate::config::PersistenceConfig) -> Self {
        Self {
            checkpoint_interval_secs: schema.checkpoint_interval_secs,
            max_wal_size_mb: schema.max_wal_size_mb,
            screen_snapshot_lines: schema.screen_snapshot_lines,
            max_checkpoints: schema.max_checkpoints,
            sync_on_write: schema.sync_on_write,
        }
    }
}

/// Parse compression method from string
pub fn parse_compression_method(method: &str) -> CompressionMethod {
    match method.to_lowercase().as_str() {
        "none" => CompressionMethod::None,
        "lz4" => CompressionMethod::Lz4,
        "zstd" => CompressionMethod::Zstd,
        _ => {
            warn!("Unknown compression method '{}', using lz4", method);
            CompressionMethod::Lz4
        }
    }
}

/// High-level persistence manager
///
/// This is the main entry point for the persistence subsystem.
/// It coordinates checkpointing, WAL operations, and recovery.
pub struct PersistenceManager {
    /// State directory path
    state_dir: PathBuf,
    /// Configuration
    config: PersistenceConfig,
    /// Recovery manager (owns checkpoint manager and WAL)
    recovery_manager: RecoveryManager,
    /// Last checkpoint time
    last_checkpoint: Mutex<SystemTime>,
    /// Last checkpoint sequence
    last_checkpoint_sequence: Mutex<u64>,
}

impl PersistenceManager {
    /// Create a new persistence manager
    pub fn new(state_dir: impl AsRef<Path>, config: PersistenceConfig) -> Result<Self> {
        let state_dir = state_dir.as_ref().to_path_buf();

        info!("Initializing persistence at {}", state_dir.display());

        // Create state directory
        std::fs::create_dir_all(&state_dir).map_err(|e| {
            CcmuxError::persistence(format!("Failed to create state directory: {}", e))
        })?;

        // Create recovery manager
        let checkpoint_config = CheckpointConfig {
            max_checkpoints: config.max_checkpoints,
            ..Default::default()
        };

        let wal_config = WalConfig {
            sync_on_write: config.sync_on_write,
            ..Default::default()
        };

        let recovery_manager =
            RecoveryManager::new(&state_dir, checkpoint_config, wal_config)?;

        Ok(Self {
            state_dir,
            config,
            recovery_manager,
            last_checkpoint: Mutex::new(SystemTime::now()),
            last_checkpoint_sequence: Mutex::new(0),
        })
    }

    /// Get the state directory path
    pub fn state_dir(&self) -> &Path {
        &self.state_dir
    }

    /// Perform recovery on startup
    pub fn recover(&self) -> Result<RecoveryState> {
        // Check for unclean shutdown
        let unclean = detect_unclean_shutdown(&self.state_dir)?;
        if unclean {
            warn!("Detected unclean shutdown, recovering state...");
        }

        // Mark server as running
        mark_server_running(&self.state_dir)?;

        // Perform recovery
        let state = self.recovery_manager.recover()?;

        // Update last checkpoint sequence
        *self.last_checkpoint_sequence.lock() = state.last_checkpoint_sequence;

        Ok(state)
    }

    /// Check if recovery is needed
    pub fn needs_recovery(&self) -> Result<bool> {
        self.recovery_manager.has_state_to_recover()
    }

    // ==================== WAL Operations ====================

    /// Log a session creation
    pub fn log_session_created(
        &self,
        id: Uuid,
        name: impl Into<String>,
    ) -> Result<()> {
        let entry = WalEntry::SessionCreated {
            id,
            name: name.into(),
            created_at: Self::unix_timestamp(),
        };
        self.recovery_manager.wal().append(&entry)?;
        Ok(())
    }

    /// Log a session destruction
    pub fn log_session_destroyed(&self, id: Uuid) -> Result<()> {
        let entry = WalEntry::SessionDestroyed { id };
        self.recovery_manager.wal().append(&entry)?;
        Ok(())
    }

    /// Log a session rename
    pub fn log_session_renamed(&self, id: Uuid, new_name: impl Into<String>) -> Result<()> {
        let entry = WalEntry::SessionRenamed {
            id,
            new_name: new_name.into(),
        };
        self.recovery_manager.wal().append(&entry)?;
        Ok(())
    }

    /// Log a session metadata change
    pub fn log_session_metadata_set(
        &self,
        session_id: Uuid,
        key: impl Into<String>,
        value: impl Into<String>,
    ) -> Result<()> {
        let entry = WalEntry::SessionMetadataSet {
            session_id,
            key: key.into(),
            value: value.into(),
        };
        self.recovery_manager.wal().append(&entry)?;
        Ok(())
    }

    /// Log a window creation
    pub fn log_window_created(
        &self,
        id: Uuid,
        session_id: Uuid,
        name: impl Into<String>,
        index: usize,
    ) -> Result<()> {
        let entry = WalEntry::WindowCreated {
            id,
            session_id,
            name: name.into(),
            index,
            created_at: Self::unix_timestamp(),
        };
        self.recovery_manager.wal().append(&entry)?;
        Ok(())
    }

    /// Log a window destruction
    pub fn log_window_destroyed(&self, id: Uuid, session_id: Uuid) -> Result<()> {
        let entry = WalEntry::WindowDestroyed { id, session_id };
        self.recovery_manager.wal().append(&entry)?;
        Ok(())
    }

    /// Log a window rename
    pub fn log_window_renamed(&self, id: Uuid, new_name: impl Into<String>) -> Result<()> {
        let entry = WalEntry::WindowRenamed {
            id,
            new_name: new_name.into(),
        };
        self.recovery_manager.wal().append(&entry)?;
        Ok(())
    }

    /// Log active window change
    pub fn log_active_window_changed(
        &self,
        session_id: Uuid,
        window_id: Option<Uuid>,
    ) -> Result<()> {
        let entry = WalEntry::ActiveWindowChanged {
            session_id,
            window_id,
        };
        self.recovery_manager.wal().append(&entry)?;
        Ok(())
    }

    /// Log a pane creation
    pub fn log_pane_created(
        &self,
        id: Uuid,
        window_id: Uuid,
        index: usize,
        cols: u16,
        rows: u16,
    ) -> Result<()> {
        let entry = WalEntry::PaneCreated {
            id,
            window_id,
            index,
            cols,
            rows,
            created_at: Self::unix_timestamp(),
        };
        self.recovery_manager.wal().append(&entry)?;
        Ok(())
    }

    /// Log a pane destruction
    pub fn log_pane_destroyed(&self, id: Uuid, window_id: Uuid) -> Result<()> {
        let entry = WalEntry::PaneDestroyed { id, window_id };
        self.recovery_manager.wal().append(&entry)?;
        Ok(())
    }

    /// Log a pane resize
    pub fn log_pane_resized(&self, id: Uuid, cols: u16, rows: u16) -> Result<()> {
        let entry = WalEntry::PaneResized { id, cols, rows };
        self.recovery_manager.wal().append(&entry)?;
        Ok(())
    }

    /// Log a pane state change
    pub fn log_pane_state_changed(&self, id: Uuid, state: PaneState) -> Result<()> {
        let entry = WalEntry::PaneStateChanged { id, state };
        self.recovery_manager.wal().append(&entry)?;
        Ok(())
    }

    /// Log a pane title change
    pub fn log_pane_title_changed(&self, id: Uuid, title: Option<String>) -> Result<()> {
        let entry = WalEntry::PaneTitleChanged { id, title };
        self.recovery_manager.wal().append(&entry)?;
        Ok(())
    }

    /// Log a pane working directory change
    pub fn log_pane_cwd_changed(&self, id: Uuid, cwd: Option<String>) -> Result<()> {
        let entry = WalEntry::PaneCwdChanged { id, cwd };
        self.recovery_manager.wal().append(&entry)?;
        Ok(())
    }

    /// Log active pane change
    pub fn log_active_pane_changed(
        &self,
        window_id: Uuid,
        pane_id: Option<Uuid>,
    ) -> Result<()> {
        let entry = WalEntry::ActivePaneChanged { window_id, pane_id };
        self.recovery_manager.wal().append(&entry)?;
        Ok(())
    }

    /// Log pane output (for scrollback persistence)
    pub fn log_pane_output(&self, pane_id: Uuid, data: Vec<u8>) -> Result<()> {
        let entry = WalEntry::PaneOutput { pane_id, data };
        self.recovery_manager.wal().append(&entry)?;
        Ok(())
    }

    // ==================== Checkpoint Operations ====================

    /// Create a checkpoint with the given sessions
    pub fn create_checkpoint(&mut self, sessions: Vec<SessionSnapshot>) -> Result<PathBuf> {
        let sequence = self.recovery_manager.checkpoint_manager_mut().sequence() + 1;

        // Write checkpoint
        let path = self.recovery_manager.checkpoint_manager_mut().create(sessions)?;

        // Write checkpoint marker to WAL
        let marker = WalEntry::CheckpointMarker {
            sequence,
            timestamp: Self::unix_timestamp(),
        };
        self.recovery_manager.wal().append(&marker)?;

        // Update tracking
        *self.last_checkpoint.lock() = SystemTime::now();
        *self.last_checkpoint_sequence.lock() = sequence;

        info!("Created checkpoint {} at {}", sequence, path.display());

        Ok(path)
    }

    /// Check if a checkpoint is due based on interval
    pub fn is_checkpoint_due(&self) -> bool {
        let last = *self.last_checkpoint.lock();
        let elapsed = SystemTime::now()
            .duration_since(last)
            .unwrap_or(Duration::ZERO);

        elapsed.as_secs() >= self.config.checkpoint_interval_secs
    }

    /// Check if a checkpoint is needed due to WAL size
    pub fn needs_checkpoint_for_wal_size(&self) -> bool {
        let wal_size = self.recovery_manager.wal().approximate_size();
        let max_size = self.config.max_wal_size_mb * 1024 * 1024;
        wal_size >= max_size
    }

    /// Get the last checkpoint sequence
    pub fn last_checkpoint_sequence(&self) -> u64 {
        *self.last_checkpoint_sequence.lock()
    }

    // ==================== Shutdown ====================

    /// Mark clean shutdown and create final checkpoint
    pub fn shutdown(&mut self, sessions: Vec<SessionSnapshot>) -> Result<()> {
        info!("Persistence shutdown initiated");

        // Create final checkpoint
        if !sessions.is_empty() {
            self.create_checkpoint(sessions)?;
        }

        // Mark clean shutdown
        mark_clean_shutdown(&self.state_dir)?;

        info!("Persistence shutdown complete");
        Ok(())
    }

    /// Finalize the persistence manager, ensuring all data is durably written
    ///
    /// This consumes the manager and properly shuts down the WAL.
    /// Use this when you need to ensure all data is persisted before
    /// the manager is dropped (e.g., in tests that reopen the WAL).
    pub fn finalize(self) -> Result<()> {
        self.recovery_manager.shutdown()
    }

    // ==================== Utilities ====================

    /// Get current Unix timestamp
    fn unix_timestamp() -> u64 {
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    }

    /// Get configuration
    pub fn config(&self) -> &PersistenceConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_manager() -> (TempDir, PersistenceManager) {
        let temp_dir = TempDir::new().unwrap();
        let manager = PersistenceManager::new(
            temp_dir.path().join("state"),
            PersistenceConfig::default(),
        )
        .unwrap();
        (temp_dir, manager)
    }

    #[test]
    fn test_persistence_manager_new() {
        let (_temp_dir, manager) = create_test_manager();
        assert!(!manager.is_checkpoint_due());
    }

    #[test]
    fn test_persistence_recover_empty() {
        let (_temp_dir, manager) = create_test_manager();

        let state = manager.recover().unwrap();
        assert!(!state.has_sessions());
        assert!(state.clean_shutdown);
    }

    #[test]
    fn test_persistence_log_operations() {
        let (temp_dir, manager) = create_test_manager();

        let session_id = Uuid::new_v4();
        let window_id = Uuid::new_v4();
        let pane_id = Uuid::new_v4();

        // Log various operations
        manager.log_session_created(session_id, "test").unwrap();
        manager.log_window_created(window_id, session_id, "main", 0).unwrap();
        manager.log_pane_created(pane_id, window_id, 0, 80, 24).unwrap();
        manager.log_pane_resized(pane_id, 120, 40).unwrap();
        manager.log_pane_title_changed(pane_id, Some("vim".to_string())).unwrap();
        manager.log_pane_cwd_changed(pane_id, Some("/home".to_string())).unwrap();
        manager.log_active_window_changed(session_id, Some(window_id)).unwrap();
        manager.log_active_pane_changed(window_id, Some(pane_id)).unwrap();

        // Just finalize - no checkpoint_active() since we're only using WAL recovery
        // (checkpoint_active() marks entries as processed by an external checkpoint)
        manager.finalize().unwrap();

        let manager2 = PersistenceManager::new(
            temp_dir.path().join("state"),
            PersistenceConfig::default(),
        )
        .unwrap();

        // Recover and verify
        let state = manager2.recover().unwrap();
        assert!(state.has_sessions());
        assert_eq!(state.sessions[0].name, "test");
    }

    #[test]
    fn test_persistence_checkpoint() {
        let (_temp_dir, mut manager) = create_test_manager();

        let session = SessionSnapshot {
            id: Uuid::new_v4(),
            name: "test".to_string(),
            windows: Vec::new(),
            active_window_id: None,
            created_at: 12345,
            metadata: HashMap::new(),
        };

        let path = manager.create_checkpoint(vec![session]).unwrap();
        assert!(path.exists());
        assert_eq!(manager.last_checkpoint_sequence(), 1);
    }

    #[test]
    fn test_persistence_shutdown() {
        let (temp_dir, mut manager) = create_test_manager();

        // Mark server running
        manager.recover().unwrap();

        // Verify lock file exists
        assert!(temp_dir.path().join("state/.lock").exists());

        // Shutdown
        manager.shutdown(vec![]).unwrap();

        // Verify lock file removed
        assert!(!temp_dir.path().join("state/.lock").exists());
    }

    #[test]
    fn test_persistence_config_from_schema() {
        let schema = crate::config::PersistenceConfig {
            checkpoint_interval_secs: 60,
            max_wal_size_mb: 256,
            screen_snapshot_lines: 1000,
            max_checkpoints: 10,
            sync_on_write: false,
            ..Default::default()
        };

        let config = PersistenceConfig::from(&schema);

        assert_eq!(config.checkpoint_interval_secs, 60);
        assert_eq!(config.max_wal_size_mb, 256);
        assert_eq!(config.screen_snapshot_lines, 1000);
        assert_eq!(config.max_checkpoints, 10);
        assert!(!config.sync_on_write);
    }

    #[test]
    fn test_is_checkpoint_due() {
        let (_temp_dir, manager) = create_test_manager();

        // Just created, shouldn't be due
        assert!(!manager.is_checkpoint_due());
    }

    #[test]
    fn test_needs_recovery() {
        let (temp_dir, manager) = create_test_manager();

        // Empty state - no recovery needed
        assert!(!manager.needs_recovery().unwrap());

        // Add some WAL entries
        manager.log_session_created(Uuid::new_v4(), "test").unwrap();

        // Just finalize - no checkpoint_active() since we're only using WAL recovery
        manager.finalize().unwrap();

        let manager2 = PersistenceManager::new(
            temp_dir.path().join("state"),
            PersistenceConfig::default(),
        )
        .unwrap();

        // Now recovery is needed (has persisted WAL entries)
        assert!(manager2.needs_recovery().unwrap());
    }

    #[test]
    fn test_persistence_full_lifecycle() {
        let (temp_dir, mut manager) = create_test_manager();

        // 1. Initial recovery (empty)
        let state = manager.recover().unwrap();
        assert!(!state.has_sessions());

        // 2. Create session hierarchy
        let session_id = Uuid::new_v4();
        let window_id = Uuid::new_v4();
        let pane_id = Uuid::new_v4();

        manager.log_session_created(session_id, "work").unwrap();
        manager.log_window_created(window_id, session_id, "main", 0).unwrap();
        manager.log_pane_created(pane_id, window_id, 0, 80, 24).unwrap();

        // 3. Create checkpoint
        let sessions = vec![SessionSnapshot {
            id: session_id,
            name: "work".to_string(),
            windows: vec![WindowSnapshot {
                id: window_id,
                session_id,
                name: "main".to_string(),
                index: 0,
                panes: vec![PaneSnapshot {
                    id: pane_id,
                    window_id,
                    index: 0,
                    cols: 80,
                    rows: 24,
                    state: PaneState::Normal,
                    name: None,
                    title: None,
                    cwd: None,
                    created_at: 0,
                    scrollback: None,
                }],
                active_pane_id: Some(pane_id),
                created_at: 0,
            }],
            active_window_id: Some(window_id),
            created_at: 0,
            metadata: HashMap::new(),
        }];

        manager.create_checkpoint(sessions.clone()).unwrap();

        // 4. Make more changes after checkpoint
        let pane2_id = Uuid::new_v4();
        manager.log_pane_created(pane2_id, window_id, 1, 80, 24).unwrap();

        // 5. Shutdown
        manager.shutdown(vec![]).unwrap();

        // 6. Create new manager and recover
        let manager2 = PersistenceManager::new(
            temp_dir.path().join("state"),
            PersistenceConfig::default(),
        )
        .unwrap();

        let state = manager2.recover().unwrap();

        // Should have recovered session with 2 panes
        assert!(state.has_sessions());
        assert_eq!(state.sessions[0].name, "work");
        assert_eq!(state.sessions[0].windows[0].panes.len(), 2);
    }

    #[test]
    fn test_persistence_metadata_via_checkpoint() {
        let (temp_dir, mut manager) = create_test_manager();

        // Create session with metadata
        let session_id = Uuid::new_v4();
        let window_id = Uuid::new_v4();
        let pane_id = Uuid::new_v4();

        manager.log_session_created(session_id, "test-session").unwrap();

        // Create metadata map
        let mut metadata = HashMap::new();
        metadata.insert("qa.tester".to_string(), "claude".to_string());
        metadata.insert("beads.root".to_string(), "/path/to/beads".to_string());

        let sessions = vec![SessionSnapshot {
            id: session_id,
            name: "test-session".to_string(),
            windows: vec![WindowSnapshot {
                id: window_id,
                session_id,
                name: "main".to_string(),
                index: 0,
                panes: vec![PaneSnapshot {
                    id: pane_id,
                    window_id,
                    index: 0,
                    cols: 80,
                    rows: 24,
                    state: PaneState::Normal,
                    name: None,
                    title: None,
                    cwd: None,
                    created_at: 0,
                    scrollback: None,
                }],
                active_pane_id: Some(pane_id),
                created_at: 0,
            }],
            active_window_id: Some(window_id),
            created_at: 0,
            metadata,
        }];

        manager.create_checkpoint(sessions).unwrap();
        manager.shutdown(vec![]).unwrap();

        // Recover and verify metadata is preserved
        let manager2 = PersistenceManager::new(
            temp_dir.path().join("state"),
            PersistenceConfig::default(),
        )
        .unwrap();

        let state = manager2.recover().unwrap();

        assert!(state.has_sessions());
        let session = &state.sessions[0];
        assert_eq!(session.metadata.get("qa.tester"), Some(&"claude".to_string()));
        assert_eq!(session.metadata.get("beads.root"), Some(&"/path/to/beads".to_string()));
    }

    #[test]
    fn test_persistence_metadata_via_wal() {
        let (temp_dir, manager) = create_test_manager();

        let session_id = Uuid::new_v4();

        // Log session creation and metadata set
        manager.log_session_created(session_id, "test-session").unwrap();
        manager.log_session_metadata_set(session_id, "qa.tester", "claude").unwrap();
        manager.log_session_metadata_set(session_id, "beads.root", "/path/to/beads").unwrap();

        manager.finalize().unwrap();

        // Recover and verify metadata is preserved
        let manager2 = PersistenceManager::new(
            temp_dir.path().join("state"),
            PersistenceConfig::default(),
        )
        .unwrap();

        let state = manager2.recover().unwrap();

        assert!(state.has_sessions());
        let session = &state.sessions[0];
        assert_eq!(session.metadata.get("qa.tester"), Some(&"claude".to_string()));
        assert_eq!(session.metadata.get("beads.root"), Some(&"/path/to/beads".to_string()));
    }
}
