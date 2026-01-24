//! Recovery logic for crash recovery
//!
//! This module handles detecting unclean shutdowns and recovering session state
//! from checkpoints and WAL entries.

use std::collections::HashMap;
use std::path::Path;

use tracing::{debug, info};
use uuid::Uuid;

use super::checkpoint::{CheckpointConfig, CheckpointManager};
use super::types::{PaneSnapshot, RecoveryState, SessionSnapshot, WalEntry, WindowSnapshot};
use super::wal::{Wal, WalConfig};
use fugue_utils::{CcmuxError, Result};

// Helper functions for specific error types
fn io_error(msg: impl Into<String>) -> CcmuxError {
    CcmuxError::persistence(msg)
}

fn validation_error(msg: impl Into<String>) -> CcmuxError {
    CcmuxError::persistence(msg)
}

/// Recovery manager for crash recovery
pub struct RecoveryManager {
    /// Checkpoint manager
    checkpoint_manager: CheckpointManager,
    /// WAL
    wal: Wal,
}

impl RecoveryManager {
    /// Create a new recovery manager
    pub fn new(
        state_dir: impl AsRef<Path>,
        checkpoint_config: CheckpointConfig,
        wal_config: WalConfig,
    ) -> Result<Self> {
        let state_dir = state_dir.as_ref();

        let checkpoint_manager = CheckpointManager::new(
            state_dir.join("checkpoints"),
            checkpoint_config,
        )?;

        let wal = Wal::open(state_dir.join("wal"), wal_config)?;

        Ok(Self {
            checkpoint_manager,
            wal,
        })
    }

    /// Perform recovery from checkpoint and WAL
    pub fn recover(&self) -> Result<RecoveryState> {
        info!("Starting recovery process");

        let mut state = RecoveryState::default();

        // Step 1: Load latest checkpoint
        let checkpoint = self.checkpoint_manager.load_latest()?;

        let (mut sessions, checkpoint_sequence) = match checkpoint {
            Some(cp) => {
                info!(
                    "Loaded checkpoint: sequence={}, sessions={}",
                    cp.sequence,
                    cp.sessions.len()
                );
                state.last_checkpoint_sequence = cp.sequence;
                (cp.sessions, cp.sequence)
            }
            None => {
                info!("No checkpoint found, starting fresh");
                (Vec::new(), 0)
            }
        };

        // Step 2: Replay WAL entries after checkpoint
        let wal_entries = self.wal.read_after_checkpoint(checkpoint_sequence)?;

        info!("Replaying {} WAL entries", wal_entries.len());

        // Build session map for efficient lookups
        let mut session_map: HashMap<Uuid, usize> = sessions
            .iter()
            .enumerate()
            .map(|(i, s)| (s.id, i))
            .collect();

        for entry in wal_entries {
            match self.apply_wal_entry(&mut sessions, &mut session_map, entry) {
                Ok(_) => state.wal_entries_replayed += 1,
                Err(e) => {
                    state.add_warning(format!("Failed to apply WAL entry: {}", e));
                }
            }
        }

        state.sessions = sessions;

        // Determine if this was a clean shutdown
        // (A clean shutdown would have no pending WAL entries after the last checkpoint)
        state.clean_shutdown = state.wal_entries_replayed == 0;

        info!(
            "Recovery complete: {} sessions, {} WAL entries replayed, clean_shutdown={}",
            state.session_count(),
            state.wal_entries_replayed,
            state.clean_shutdown
        );

        Ok(state)
    }

    /// Apply a WAL entry to the session state
    fn apply_wal_entry(
        &self,
        sessions: &mut Vec<SessionSnapshot>,
        session_map: &mut HashMap<Uuid, usize>,
        entry: WalEntry,
    ) -> Result<()> {
        match entry {
            WalEntry::SessionCreated {
                id,
                name,
                created_at,
            } => {
                if session_map.contains_key(&id) {
                    return Err(validation_error(format!(
                        "Session {} already exists",
                        id
                    )));
                }

                let session = SessionSnapshot {
                    id,
                    name,
                    windows: Vec::new(),
                    active_window_id: None,
                    created_at,
                    metadata: HashMap::new(),
                    environment: HashMap::new(),
                };

                session_map.insert(id, sessions.len());
                sessions.push(session);

                debug!("Applied: SessionCreated {}", id);
            }

            WalEntry::SessionDestroyed { id } => {
                if let Some(idx) = session_map.remove(&id) {
                    sessions.remove(idx);

                    // Update indices in map
                    for (_, v) in session_map.iter_mut() {
                        if *v > idx {
                            *v -= 1;
                        }
                    }

                    debug!("Applied: SessionDestroyed {}", id);
                }
            }

            WalEntry::SessionRenamed { id, new_name } => {
                if let Some(&idx) = session_map.get(&id) {
                    sessions[idx].name = new_name;
                    debug!("Applied: SessionRenamed {}", id);
                }
            }

            WalEntry::WindowCreated {
                id,
                session_id,
                name,
                index,
                created_at,
            } => {
                if let Some(&idx) = session_map.get(&session_id) {
                    let window = WindowSnapshot {
                        id,
                        session_id,
                        name,
                        index,
                        panes: Vec::new(),
                        active_pane_id: None,
                        created_at,
                    };

                    sessions[idx].windows.push(window);
                    debug!("Applied: WindowCreated {}", id);
                }
            }

            WalEntry::WindowDestroyed { id, session_id } => {
                if let Some(&idx) = session_map.get(&session_id) {
                    sessions[idx].windows.retain(|w| w.id != id);
                    debug!("Applied: WindowDestroyed {}", id);
                }
            }

            WalEntry::WindowRenamed { id, new_name } => {
                for session in sessions.iter_mut() {
                    if let Some(window) = session.windows.iter_mut().find(|w| w.id == id) {
                        window.name = new_name;
                        debug!("Applied: WindowRenamed {}", id);
                        break;
                    }
                }
            }

            WalEntry::ActiveWindowChanged {
                session_id,
                window_id,
            } => {
                if let Some(&idx) = session_map.get(&session_id) {
                    sessions[idx].active_window_id = window_id;
                    debug!("Applied: ActiveWindowChanged {}", session_id);
                }
            }

            WalEntry::PaneCreated {
                id,
                window_id,
                index,
                cols,
                rows,
                created_at,
            } => {
                for session in sessions.iter_mut() {
                    if let Some(window) = session.windows.iter_mut().find(|w| w.id == window_id) {
                        let pane = PaneSnapshot {
                            id,
                            window_id,
                            index,
                            cols,
                            rows,
                            state: fugue_protocol::PaneState::Normal,
                            name: None,
                            title: None,
                            cwd: None,
                            created_at,
                            scrollback: None,
                        };

                        window.panes.push(pane);
                        debug!("Applied: PaneCreated {}", id);
                        break;
                    }
                }
            }

            WalEntry::PaneDestroyed { id, window_id } => {
                for session in sessions.iter_mut() {
                    if let Some(window) = session.windows.iter_mut().find(|w| w.id == window_id) {
                        window.panes.retain(|p| p.id != id);
                        debug!("Applied: PaneDestroyed {}", id);
                        break;
                    }
                }
            }

            WalEntry::PaneResized { id, cols, rows } => {
                for session in sessions.iter_mut() {
                    for window in session.windows.iter_mut() {
                        if let Some(pane) = window.panes.iter_mut().find(|p| p.id == id) {
                            pane.cols = cols;
                            pane.rows = rows;
                            debug!("Applied: PaneResized {}", id);
                            return Ok(());
                        }
                    }
                }
            }

            WalEntry::PaneStateChanged { id, state } => {
                for session in sessions.iter_mut() {
                    for window in session.windows.iter_mut() {
                        if let Some(pane) = window.panes.iter_mut().find(|p| p.id == id) {
                            pane.state = state;
                            debug!("Applied: PaneStateChanged {}", id);
                            return Ok(());
                        }
                    }
                }
            }

            WalEntry::PaneTitleChanged { id, title } => {
                for session in sessions.iter_mut() {
                    for window in session.windows.iter_mut() {
                        if let Some(pane) = window.panes.iter_mut().find(|p| p.id == id) {
                            pane.title = title;
                            debug!("Applied: PaneTitleChanged {}", id);
                            return Ok(());
                        }
                    }
                }
            }

            WalEntry::PaneCwdChanged { id, cwd } => {
                for session in sessions.iter_mut() {
                    for window in session.windows.iter_mut() {
                        if let Some(pane) = window.panes.iter_mut().find(|p| p.id == id) {
                            pane.cwd = cwd;
                            debug!("Applied: PaneCwdChanged {}", id);
                            return Ok(());
                        }
                    }
                }
            }

            WalEntry::ActivePaneChanged { window_id, pane_id } => {
                for session in sessions.iter_mut() {
                    if let Some(window) = session.windows.iter_mut().find(|w| w.id == window_id) {
                        window.active_pane_id = pane_id;
                        debug!("Applied: ActivePaneChanged {}", window_id);
                        break;
                    }
                }
            }

            WalEntry::PaneOutput { pane_id, data } => {
                // Scrollback data - would be applied to pane's scrollback buffer
                // For now, we skip this during recovery as scrollback is in the checkpoint
                debug!("Skipped: PaneOutput for {} ({} bytes)", pane_id, data.len());
            }

            WalEntry::CheckpointMarker { sequence, .. } => {
                // Checkpoint markers are used for WAL replay positioning
                debug!("Encountered: CheckpointMarker sequence={}", sequence);
            }

            WalEntry::SessionMetadataSet { session_id, key, value } => {
                if let Some(&idx) = session_map.get(&session_id) {
                    sessions[idx].metadata.insert(key.clone(), value.clone());
                    debug!("Applied: SessionMetadataSet {} key={}", session_id, key);
                }
            }

            WalEntry::SessionEnvironmentSet { session_id, key, value } => {
                if let Some(&idx) = session_map.get(&session_id) {
                    sessions[idx].environment.insert(key.clone(), value.clone());
                    debug!("Applied: SessionEnvironmentSet {} key={}", session_id, key);
                }
            }
        }

        Ok(())
    }

    /// Get the WAL reference for writing
    pub fn wal(&self) -> &Wal {
        &self.wal
    }

    /// Get the checkpoint manager reference
    pub fn checkpoint_manager(&self) -> &CheckpointManager {
        &self.checkpoint_manager
    }

    /// Get mutable checkpoint manager for creating checkpoints
    pub fn checkpoint_manager_mut(&mut self) -> &mut CheckpointManager {
        &mut self.checkpoint_manager
    }

    /// Check if there's state to recover
    pub fn has_state_to_recover(&self) -> Result<bool> {
        // Check if there's a checkpoint or WAL entries
        let has_checkpoint = self.checkpoint_manager.load_latest()?.is_some();
        let wal_entries = self.wal.read_all()?;
        let has_wal = !wal_entries.is_empty();

        Ok(has_checkpoint || has_wal)
    }

    /// Shutdown the recovery manager, ensuring all data is persisted
    ///
    /// This must be called before dropping the manager if data durability
    /// is required (e.g., in tests that reopen the WAL).
    pub fn shutdown(self) -> Result<()> {
        self.wal.shutdown()
    }
}

/// Detect if the previous shutdown was clean
pub fn detect_unclean_shutdown(state_dir: impl AsRef<Path>) -> Result<bool> {
    let state_dir = state_dir.as_ref();
    let lock_file = state_dir.join(".lock");

    // If lock file exists, previous shutdown was unclean
    if lock_file.exists() {
        info!("Detected unclean shutdown (lock file exists)");
        return Ok(true);
    }

    Ok(false)
}

/// Create shutdown marker to indicate clean shutdown
pub fn mark_clean_shutdown(state_dir: impl AsRef<Path>) -> Result<()> {
    let state_dir = state_dir.as_ref();
    let lock_file = state_dir.join(".lock");

    // Remove lock file
    if lock_file.exists() {
        std::fs::remove_file(&lock_file).map_err(|e| {
            io_error(format!("Failed to remove lock file: {}", e))
        })?;
    }

    info!("Marked clean shutdown");
    Ok(())
}

/// Create lock file to indicate server is running
pub fn mark_server_running(state_dir: impl AsRef<Path>) -> Result<()> {
    let state_dir = state_dir.as_ref();

    std::fs::create_dir_all(state_dir).map_err(|e| {
        io_error(format!("Failed to create state directory: {}", e))
    })?;

    let lock_file = state_dir.join(".lock");

    std::fs::write(&lock_file, format!("{}", std::process::id())).map_err(|e| {
        io_error(format!("Failed to create lock file: {}", e))
    })?;

    info!("Created server lock file");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn create_test_dir() -> TempDir {
        TempDir::new().unwrap()
    }

    fn create_manager_at(state_dir: &PathBuf) -> RecoveryManager {
        RecoveryManager::new(
            state_dir,
            CheckpointConfig::default(),
            WalConfig::default(),
        )
        .unwrap()
    }

    #[test]
    fn test_recovery_empty() {
        let temp_dir = create_test_dir();
        let state_dir = temp_dir.path().join("state");
        let manager = create_manager_at(&state_dir);

        let state = manager.recover().unwrap();

        assert!(!state.has_sessions());
        assert_eq!(state.wal_entries_replayed, 0);
        assert!(state.clean_shutdown);
    }

    #[test]
    fn test_recovery_from_wal() {
        let temp_dir = create_test_dir();
        let state_dir = temp_dir.path().join("state");

        let session_id = Uuid::new_v4();
        let window_id = Uuid::new_v4();
        let pane_id = Uuid::new_v4();

        // Write WAL entries
        {
            let manager = create_manager_at(&state_dir);

            manager.wal().append(&WalEntry::SessionCreated {
                id: session_id,
                name: "test-session".to_string(),
                created_at: 12345,
            }).unwrap();

            manager.wal().append(&WalEntry::WindowCreated {
                id: window_id,
                session_id,
                name: "main".to_string(),
                index: 0,
                created_at: 12345,
            }).unwrap();

            manager.wal().append(&WalEntry::PaneCreated {
                id: pane_id,
                window_id,
                index: 0,
                cols: 80,
                rows: 24,
                created_at: 12345,
            }).unwrap();

            // Shutdown ensures entries are durably written without marking as checkpointed
            // (checkpoint_active() would mark entries as processed by an external checkpoint,
            // which we don't have, causing them to be skipped during recovery)
            manager.shutdown().unwrap();
        }

        // Re-open and recover
        let manager = create_manager_at(&state_dir);
        let state = manager.recover().unwrap();

        assert!(state.has_sessions());
        assert_eq!(state.session_count(), 1);
        assert_eq!(state.wal_entries_replayed, 3);
        assert!(!state.clean_shutdown);

        let session = &state.sessions[0];
        assert_eq!(session.name, "test-session");
        assert_eq!(session.windows.len(), 1);
        assert_eq!(session.windows[0].panes.len(), 1);
    }

    #[test]
    fn test_recovery_session_lifecycle() {
        let temp_dir = create_test_dir();
        let state_dir = temp_dir.path().join("state");

        let session_id = Uuid::new_v4();

        // Create and destroy session
        {
            let manager = create_manager_at(&state_dir);

            manager.wal().append(&WalEntry::SessionCreated {
                id: session_id,
                name: "temp".to_string(),
                created_at: 0,
            }).unwrap();

            manager.wal().append(&WalEntry::SessionDestroyed {
                id: session_id,
            }).unwrap();

            // Just shutdown - no checkpoint_active() since we're only using WAL recovery
            manager.shutdown().unwrap();
        }

        // Re-open and recover
        let manager = create_manager_at(&state_dir);
        let state = manager.recover().unwrap();
        assert!(!state.has_sessions());
    }

    #[test]
    fn test_recovery_pane_updates() {
        let temp_dir = create_test_dir();
        let state_dir = temp_dir.path().join("state");

        let session_id = Uuid::new_v4();
        let window_id = Uuid::new_v4();
        let pane_id = Uuid::new_v4();

        // Create hierarchy and update pane
        {
            let manager = create_manager_at(&state_dir);

            manager.wal().append(&WalEntry::SessionCreated {
                id: session_id,
                name: "test".to_string(),
                created_at: 0,
            }).unwrap();

            manager.wal().append(&WalEntry::WindowCreated {
                id: window_id,
                session_id,
                name: "main".to_string(),
                index: 0,
                created_at: 0,
            }).unwrap();

            manager.wal().append(&WalEntry::PaneCreated {
                id: pane_id,
                window_id,
                index: 0,
                cols: 80,
                rows: 24,
                created_at: 0,
            }).unwrap();

            // Update pane
            manager.wal().append(&WalEntry::PaneResized {
                id: pane_id,
                cols: 120,
                rows: 40,
            }).unwrap();

            manager.wal().append(&WalEntry::PaneTitleChanged {
                id: pane_id,
                title: Some("vim".to_string()),
            }).unwrap();

            manager.wal().append(&WalEntry::PaneCwdChanged {
                id: pane_id,
                cwd: Some("/home/user".to_string()),
            }).unwrap();

            // Just shutdown - no checkpoint_active() since we're only using WAL recovery
            manager.shutdown().unwrap();
        }

        // Re-open and recover
        let manager = create_manager_at(&state_dir);
        let state = manager.recover().unwrap();
        let pane = &state.sessions[0].windows[0].panes[0];

        assert_eq!(pane.cols, 120);
        assert_eq!(pane.rows, 40);
        assert_eq!(pane.title, Some("vim".to_string()));
        assert_eq!(pane.cwd, Some("/home/user".to_string()));
    }

    #[test]
    fn test_has_state_to_recover() {
        let temp_dir = create_test_dir();
        let state_dir = temp_dir.path().join("state");

        // Initially no state
        {
            let manager = create_manager_at(&state_dir);
            assert!(!manager.has_state_to_recover().unwrap());
            manager.shutdown().unwrap();
        }

        // Write and checkpoint
        {
            let manager = create_manager_at(&state_dir);
            manager.wal().append(&WalEntry::SessionCreated {
                id: Uuid::new_v4(),
                name: "test".to_string(),
                created_at: 0,
            }).unwrap();
            // Just shutdown - no checkpoint_active() since we're only using WAL recovery
            manager.shutdown().unwrap();
        }

        // Re-open - now should have state
        let manager = create_manager_at(&state_dir);
        assert!(manager.has_state_to_recover().unwrap());
    }

    #[test]
    fn test_unclean_shutdown_detection() {
        let temp_dir = TempDir::new().unwrap();
        let state_dir = temp_dir.path().join("state");

        // No lock file - clean
        assert!(!detect_unclean_shutdown(&state_dir).unwrap());

        // Create lock file
        mark_server_running(&state_dir).unwrap();
        assert!(detect_unclean_shutdown(&state_dir).unwrap());

        // Mark clean shutdown
        mark_clean_shutdown(&state_dir).unwrap();
        assert!(!detect_unclean_shutdown(&state_dir).unwrap());
    }

    #[test]
    fn test_recovery_session_rename() {
        let temp_dir = create_test_dir();
        let state_dir = temp_dir.path().join("state");

        let session_id = Uuid::new_v4();

        {
            let manager = create_manager_at(&state_dir);

            manager.wal().append(&WalEntry::SessionCreated {
                id: session_id,
                name: "old-name".to_string(),
                created_at: 0,
            }).unwrap();

            manager.wal().append(&WalEntry::SessionRenamed {
                id: session_id,
                new_name: "new-name".to_string(),
            }).unwrap();

            // Just shutdown - no checkpoint_active() since we're only using WAL recovery
            manager.shutdown().unwrap();
        }

        let manager = create_manager_at(&state_dir);
        let state = manager.recover().unwrap();
        assert_eq!(state.sessions[0].name, "new-name");
    }

    #[test]
    fn test_recovery_active_window_pane() {
        let temp_dir = create_test_dir();
        let state_dir = temp_dir.path().join("state");

        let session_id = Uuid::new_v4();
        let window_id = Uuid::new_v4();
        let pane_id = Uuid::new_v4();

        {
            let manager = create_manager_at(&state_dir);

            manager.wal().append(&WalEntry::SessionCreated {
                id: session_id,
                name: "test".to_string(),
                created_at: 0,
            }).unwrap();

            manager.wal().append(&WalEntry::WindowCreated {
                id: window_id,
                session_id,
                name: "main".to_string(),
                index: 0,
                created_at: 0,
            }).unwrap();

            manager.wal().append(&WalEntry::PaneCreated {
                id: pane_id,
                window_id,
                index: 0,
                cols: 80,
                rows: 24,
                created_at: 0,
            }).unwrap();

            manager.wal().append(&WalEntry::ActiveWindowChanged {
                session_id,
                window_id: Some(window_id),
            }).unwrap();

            manager.wal().append(&WalEntry::ActivePaneChanged {
                window_id,
                pane_id: Some(pane_id),
            }).unwrap();

            // Just shutdown - no checkpoint_active() since we're only using WAL recovery
            manager.shutdown().unwrap();
        }

        let manager = create_manager_at(&state_dir);
        let state = manager.recover().unwrap();
        assert_eq!(state.sessions[0].active_window_id, Some(window_id));
        assert_eq!(state.sessions[0].windows[0].active_pane_id, Some(pane_id));
    }

    #[test]
    fn test_recovery_session_metadata() {
        let temp_dir = create_test_dir();
        let state_dir = temp_dir.path().join("state");

        let session_id = Uuid::new_v4();

        {
            let manager = create_manager_at(&state_dir);

            manager.wal().append(&WalEntry::SessionCreated {
                id: session_id,
                name: "test-session".to_string(),
                created_at: 12345,
            }).unwrap();

            // Set metadata via WAL entries
            manager.wal().append(&WalEntry::SessionMetadataSet {
                session_id,
                key: "qa.tester".to_string(),
                value: "claude".to_string(),
            }).unwrap();

            manager.wal().append(&WalEntry::SessionMetadataSet {
                session_id,
                key: "beads.root".to_string(),
                value: "/path/to/beads".to_string(),
            }).unwrap();

            manager.shutdown().unwrap();
        }

        // Re-open and recover
        let manager = create_manager_at(&state_dir);
        let state = manager.recover().unwrap();

        assert!(state.has_sessions());
        assert_eq!(state.session_count(), 1);

        let session = &state.sessions[0];
        assert_eq!(session.metadata.get("qa.tester"), Some(&"claude".to_string()));
        assert_eq!(session.metadata.get("beads.root"), Some(&"/path/to/beads".to_string()));
    }
}
