//! Checkpoint management for periodic state snapshots
//!
//! Checkpoints provide a complete snapshot of session state at a point in time.
//! They are used as the base for recovery, with WAL entries replayed on top.

use std::fs::{self, File};
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use tracing::{debug, info, warn};

use super::types::{Checkpoint, SessionSnapshot, CHECKPOINT_MAGIC, CHECKPOINT_VERSION};
use ccmux_utils::{CcmuxError, Result};

// Helper functions for specific error types
fn io_error(msg: impl Into<String>) -> CcmuxError {
    CcmuxError::persistence(msg)
}

fn validation_error(msg: impl Into<String>) -> CcmuxError {
    CcmuxError::persistence(msg)
}

fn serialization_error(msg: impl Into<String>) -> CcmuxError {
    CcmuxError::persistence(msg)
}

/// Configuration for checkpoint management
#[derive(Debug, Clone)]
pub struct CheckpointConfig {
    /// Maximum number of checkpoints to retain
    pub max_checkpoints: usize,
    /// Checkpoint file prefix
    pub file_prefix: String,
}

impl Default for CheckpointConfig {
    fn default() -> Self {
        Self {
            max_checkpoints: 5,
            file_prefix: "checkpoint".to_string(),
        }
    }
}

/// Checkpoint manager for creating and loading checkpoints
pub struct CheckpointManager {
    /// Directory for checkpoint files
    checkpoint_dir: PathBuf,
    /// Configuration
    config: CheckpointConfig,
    /// Current sequence number
    sequence: u64,
}

impl CheckpointManager {
    /// Create a new checkpoint manager
    pub fn new(checkpoint_dir: impl AsRef<Path>, config: CheckpointConfig) -> Result<Self> {
        let checkpoint_dir = checkpoint_dir.as_ref().to_path_buf();

        // Create directory if it doesn't exist
        fs::create_dir_all(&checkpoint_dir).map_err(|e| {
            io_error(format!(
                "Failed to create checkpoint directory: {}",
                e
            ))
        })?;

        // Find the highest sequence number from existing checkpoints
        let sequence = Self::find_max_sequence(&checkpoint_dir, &config.file_prefix)?;

        info!(
            "Checkpoint manager initialized at {}, sequence={}",
            checkpoint_dir.display(),
            sequence
        );

        Ok(Self {
            checkpoint_dir,
            config,
            sequence,
        })
    }

    /// Find the maximum sequence number from existing checkpoint files
    fn find_max_sequence(dir: &Path, prefix: &str) -> Result<u64> {
        let mut max_sequence = 0;

        if !dir.exists() {
            return Ok(0);
        }

        for entry in fs::read_dir(dir).map_err(|e| {
            io_error(format!("Failed to read checkpoint directory: {}", e))
        })? {
            let entry = entry.map_err(|e| {
                io_error(format!("Failed to read directory entry: {}", e))
            })?;

            let file_name = entry.file_name();
            let name = file_name.to_string_lossy();

            if let Some(seq_str) = name
                .strip_prefix(prefix)
                .and_then(|s| s.strip_prefix('-'))
                .and_then(|s| s.strip_suffix(".bin"))
            {
                if let Ok(seq) = seq_str.parse::<u64>() {
                    max_sequence = max_sequence.max(seq);
                }
            }
        }

        Ok(max_sequence)
    }

    /// Create a new checkpoint with the given sessions
    pub fn create(&mut self, sessions: Vec<SessionSnapshot>) -> Result<PathBuf> {
        self.sequence += 1;
        let checkpoint = Checkpoint {
            version: CHECKPOINT_VERSION,
            timestamp: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
            sequence: self.sequence,
            sessions,
        };

        let path = self.checkpoint_path(self.sequence);
        self.write_checkpoint(&path, &checkpoint)?;

        info!(
            "Created checkpoint {} at {}",
            self.sequence,
            path.display()
        );

        // Clean up old checkpoints
        self.cleanup_old_checkpoints()?;

        Ok(path)
    }

    /// Write a checkpoint to disk
    fn write_checkpoint(&self, path: &Path, checkpoint: &Checkpoint) -> Result<()> {
        // Write to a temporary file first, then rename for atomicity
        let temp_path = path.with_extension("tmp");

        let file = File::create(&temp_path).map_err(|e| {
            io_error(format!("Failed to create checkpoint file: {}", e))
        })?;

        let mut writer = BufWriter::new(file);

        // Write magic bytes
        writer.write_all(&CHECKPOINT_MAGIC).map_err(|e| {
            io_error(format!("Failed to write checkpoint magic: {}", e))
        })?;

        // Serialize and write checkpoint data
        let data = bincode::serialize(checkpoint).map_err(|e| {
            serialization_error(format!("Failed to serialize checkpoint: {}", e))
        })?;

        writer.write_all(&data).map_err(|e| {
            io_error(format!("Failed to write checkpoint data: {}", e))
        })?;

        writer.flush().map_err(|e| {
            io_error(format!("Failed to flush checkpoint: {}", e))
        })?;

        // Sync to disk
        writer.into_inner().map_err(|e| {
            io_error(format!("Failed to get file handle: {}", e))
        })?.sync_all().map_err(|e| {
            io_error(format!("Failed to sync checkpoint: {}", e))
        })?;

        // Atomic rename
        fs::rename(&temp_path, path).map_err(|e| {
            io_error(format!("Failed to rename checkpoint file: {}", e))
        })?;

        debug!("Wrote checkpoint to {}", path.display());

        Ok(())
    }

    /// Load the most recent valid checkpoint
    pub fn load_latest(&self) -> Result<Option<Checkpoint>> {
        let checkpoints = self.list_checkpoints()?;

        // Try checkpoints from newest to oldest
        for path in checkpoints.iter().rev() {
            match self.load_checkpoint(path) {
                Ok(checkpoint) => {
                    info!("Loaded checkpoint from {}", path.display());
                    return Ok(Some(checkpoint));
                }
                Err(e) => {
                    warn!(
                        "Failed to load checkpoint {}: {}",
                        path.display(),
                        e
                    );
                    continue;
                }
            }
        }

        Ok(None)
    }

    /// Load a specific checkpoint file
    pub fn load_checkpoint(&self, path: &Path) -> Result<Checkpoint> {
        let file = File::open(path).map_err(|e| {
            io_error(format!("Failed to open checkpoint: {}", e))
        })?;

        let mut reader = BufReader::new(file);

        // Read and verify magic bytes
        let mut magic = [0u8; 4];
        reader.read_exact(&mut magic).map_err(|e| {
            io_error(format!("Failed to read checkpoint magic: {}", e))
        })?;

        if magic != CHECKPOINT_MAGIC {
            return Err(validation_error(
                "Invalid checkpoint file: wrong magic bytes",
            ));
        }

        // Read remaining data
        let mut data = Vec::new();
        reader.read_to_end(&mut data).map_err(|e| {
            io_error(format!("Failed to read checkpoint data: {}", e))
        })?;

        // Deserialize
        let checkpoint: Checkpoint = bincode::deserialize(&data).map_err(|e| {
            serialization_error(format!("Failed to deserialize checkpoint: {}", e))
        })?;

        // Validate version
        if checkpoint.version > CHECKPOINT_VERSION {
            return Err(validation_error(format!(
                "Checkpoint version {} is newer than supported version {}",
                checkpoint.version, CHECKPOINT_VERSION
            )));
        }

        debug!(
            "Loaded checkpoint: version={}, sequence={}, sessions={}",
            checkpoint.version,
            checkpoint.sequence,
            checkpoint.sessions.len()
        );

        Ok(checkpoint)
    }

    /// List all checkpoint files sorted by sequence number
    fn list_checkpoints(&self) -> Result<Vec<PathBuf>> {
        let mut checkpoints = Vec::new();

        if !self.checkpoint_dir.exists() {
            return Ok(checkpoints);
        }

        for entry in fs::read_dir(&self.checkpoint_dir).map_err(|e| {
            io_error(format!("Failed to read checkpoint directory: {}", e))
        })? {
            let entry = entry.map_err(|e| {
                io_error(format!("Failed to read directory entry: {}", e))
            })?;

            let path = entry.path();
            if path.extension().map(|e| e == "bin").unwrap_or(false) {
                let name = path.file_name().unwrap().to_string_lossy();
                if name.starts_with(&self.config.file_prefix) {
                    checkpoints.push(path);
                }
            }
        }

        // Sort by sequence number
        checkpoints.sort_by(|a, b| {
            let seq_a = Self::extract_sequence(a, &self.config.file_prefix);
            let seq_b = Self::extract_sequence(b, &self.config.file_prefix);
            seq_a.cmp(&seq_b)
        });

        Ok(checkpoints)
    }

    /// Extract sequence number from checkpoint path
    fn extract_sequence(path: &Path, prefix: &str) -> u64 {
        path.file_name()
            .and_then(|n| n.to_str())
            .and_then(|n| n.strip_prefix(prefix))
            .and_then(|n| n.strip_prefix('-'))
            .and_then(|n| n.strip_suffix(".bin"))
            .and_then(|n| n.parse().ok())
            .unwrap_or(0)
    }

    /// Get path for a checkpoint with given sequence
    fn checkpoint_path(&self, sequence: u64) -> PathBuf {
        self.checkpoint_dir
            .join(format!("{}-{:010}.bin", self.config.file_prefix, sequence))
    }

    /// Clean up old checkpoints, keeping only max_checkpoints newest
    fn cleanup_old_checkpoints(&self) -> Result<()> {
        let checkpoints = self.list_checkpoints()?;

        if checkpoints.len() <= self.config.max_checkpoints {
            return Ok(());
        }

        let to_remove = checkpoints.len() - self.config.max_checkpoints;

        for path in checkpoints.iter().take(to_remove) {
            match fs::remove_file(path) {
                Ok(_) => {
                    debug!("Removed old checkpoint: {}", path.display());
                }
                Err(e) => {
                    warn!(
                        "Failed to remove old checkpoint {}: {}",
                        path.display(),
                        e
                    );
                }
            }
        }

        Ok(())
    }

    /// Get current sequence number
    pub fn sequence(&self) -> u64 {
        self.sequence
    }

    /// Validate a checkpoint for consistency
    pub fn validate(&self, checkpoint: &Checkpoint) -> Result<()> {
        // Check version
        if checkpoint.version > CHECKPOINT_VERSION {
            return Err(validation_error(format!(
                "Unsupported checkpoint version: {}",
                checkpoint.version
            )));
        }

        // Validate session structure
        for session in &checkpoint.sessions {
            // Check active window exists
            if let Some(active_id) = session.active_window_id {
                if !session.windows.iter().any(|w| w.id == active_id) {
                    return Err(validation_error(format!(
                        "Active window {} not found in session {}",
                        active_id, session.id
                    )));
                }
            }

            // Validate windows
            for window in &session.windows {
                // Check parent reference
                if window.session_id != session.id {
                    return Err(validation_error(format!(
                        "Window {} has wrong session ID",
                        window.id
                    )));
                }

                // Check active pane exists
                if let Some(active_id) = window.active_pane_id {
                    if !window.panes.iter().any(|p| p.id == active_id) {
                        return Err(validation_error(format!(
                            "Active pane {} not found in window {}",
                            active_id, window.id
                        )));
                    }
                }

                // Validate panes
                for pane in &window.panes {
                    if pane.window_id != window.id {
                        return Err(validation_error(format!(
                            "Pane {} has wrong window ID",
                            pane.id
                        )));
                    }
                }
            }
        }

        debug!("Checkpoint validation passed");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use ccmux_protocol::PaneState;
    use tempfile::TempDir;
    use uuid::Uuid;

    use crate::persistence::types::{PaneSnapshot, WindowSnapshot};

    fn create_test_manager() -> (TempDir, CheckpointManager) {
        let temp_dir = TempDir::new().unwrap();
        let manager = CheckpointManager::new(
            temp_dir.path().join("checkpoints"),
            CheckpointConfig::default(),
        )
        .unwrap();
        (temp_dir, manager)
    }

    fn create_test_session() -> SessionSnapshot {
        let session_id = Uuid::new_v4();
        let window_id = Uuid::new_v4();
        let pane_id = Uuid::new_v4();

        SessionSnapshot {
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
                    cwd: Some("/home/user".to_string()),
                    created_at: 12345,
                    scrollback: None,
                }],
                active_pane_id: Some(pane_id),
                created_at: 12345,
            }],
            active_window_id: Some(window_id),
            created_at: 12345,
            metadata: HashMap::new(),
        }
    }

    #[test]
    fn test_checkpoint_manager_new() {
        let (_temp_dir, manager) = create_test_manager();
        assert_eq!(manager.sequence(), 0);
    }

    #[test]
    fn test_checkpoint_create_and_load() {
        let (_temp_dir, mut manager) = create_test_manager();

        let session = create_test_session();
        let path = manager.create(vec![session.clone()]).unwrap();

        assert!(path.exists());
        assert_eq!(manager.sequence(), 1);

        let loaded = manager.load_latest().unwrap().unwrap();
        assert_eq!(loaded.version, CHECKPOINT_VERSION);
        assert_eq!(loaded.sequence, 1);
        assert_eq!(loaded.sessions.len(), 1);
        assert_eq!(loaded.sessions[0].name, "test-session");
    }

    #[test]
    fn test_checkpoint_multiple() {
        let (_temp_dir, mut manager) = create_test_manager();

        for i in 0..3 {
            let mut session = create_test_session();
            session.name = format!("session-{}", i);
            manager.create(vec![session]).unwrap();
        }

        assert_eq!(manager.sequence(), 3);

        let loaded = manager.load_latest().unwrap().unwrap();
        assert_eq!(loaded.sequence, 3);
        assert_eq!(loaded.sessions[0].name, "session-2");
    }

    #[test]
    fn test_checkpoint_cleanup() {
        let temp_dir = TempDir::new().unwrap();
        let config = CheckpointConfig {
            max_checkpoints: 2,
            ..Default::default()
        };
        let mut manager = CheckpointManager::new(
            temp_dir.path().join("checkpoints"),
            config,
        )
        .unwrap();

        // Create 5 checkpoints
        for _ in 0..5 {
            manager.create(vec![create_test_session()]).unwrap();
        }

        // Should only have 2 checkpoints
        let checkpoints = manager.list_checkpoints().unwrap();
        assert_eq!(checkpoints.len(), 2);
    }

    #[test]
    fn test_checkpoint_empty() {
        let (_temp_dir, mut manager) = create_test_manager();

        let path = manager.create(vec![]).unwrap();
        assert!(path.exists());

        let loaded = manager.load_latest().unwrap().unwrap();
        assert!(loaded.sessions.is_empty());
    }

    #[test]
    fn test_checkpoint_validation_valid() {
        let (_temp_dir, manager) = create_test_manager();

        let session = create_test_session();
        let checkpoint = Checkpoint {
            version: CHECKPOINT_VERSION,
            timestamp: 12345,
            sequence: 1,
            sessions: vec![session],
        };

        assert!(manager.validate(&checkpoint).is_ok());
    }

    #[test]
    fn test_checkpoint_validation_wrong_session_id() {
        let (_temp_dir, manager) = create_test_manager();

        let session_id = Uuid::new_v4();
        let mut session = create_test_session();
        session.id = session_id;
        // Window has different session_id
        session.windows[0].session_id = Uuid::new_v4();

        let checkpoint = Checkpoint {
            version: CHECKPOINT_VERSION,
            timestamp: 12345,
            sequence: 1,
            sessions: vec![session],
        };

        assert!(manager.validate(&checkpoint).is_err());
    }

    #[test]
    fn test_checkpoint_validation_missing_active_window() {
        let (_temp_dir, manager) = create_test_manager();

        let mut session = create_test_session();
        session.active_window_id = Some(Uuid::new_v4()); // Non-existent window

        let checkpoint = Checkpoint {
            version: CHECKPOINT_VERSION,
            timestamp: 12345,
            sequence: 1,
            sessions: vec![session],
        };

        assert!(manager.validate(&checkpoint).is_err());
    }

    #[test]
    fn test_checkpoint_config_default() {
        let config = CheckpointConfig::default();
        assert_eq!(config.max_checkpoints, 5);
        assert_eq!(config.file_prefix, "checkpoint");
    }

    #[test]
    fn test_checkpoint_no_existing() {
        let (_temp_dir, manager) = create_test_manager();

        let loaded = manager.load_latest().unwrap();
        assert!(loaded.is_none());
    }

    #[test]
    fn test_checkpoint_sequence_recovery() {
        let temp_dir = TempDir::new().unwrap();
        let checkpoint_dir = temp_dir.path().join("checkpoints");

        // Create manager and some checkpoints
        {
            let mut manager = CheckpointManager::new(
                &checkpoint_dir,
                CheckpointConfig::default(),
            )
            .unwrap();

            for _ in 0..3 {
                manager.create(vec![create_test_session()]).unwrap();
            }

            assert_eq!(manager.sequence(), 3);
        }

        // Create new manager - should recover sequence
        let manager = CheckpointManager::new(
            &checkpoint_dir,
            CheckpointConfig::default(),
        )
        .unwrap();

        assert_eq!(manager.sequence(), 3);
    }

    #[test]
    fn test_checkpoint_invalid_magic() {
        let temp_dir = TempDir::new().unwrap();
        let checkpoint_dir = temp_dir.path().join("checkpoints");
        fs::create_dir_all(&checkpoint_dir).unwrap();

        // Write invalid checkpoint
        let path = checkpoint_dir.join("checkpoint-0000000001.bin");
        fs::write(&path, b"XXXX invalid data").unwrap();

        let manager = CheckpointManager::new(
            &checkpoint_dir,
            CheckpointConfig::default(),
        )
        .unwrap();

        let result = manager.load_checkpoint(&path);
        assert!(result.is_err());
    }
}
