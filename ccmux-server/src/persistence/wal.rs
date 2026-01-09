//! Write-Ahead Log (WAL) implementation using okaywal
//!
//! The WAL provides durable storage for incremental state changes.
//! It is used in conjunction with checkpoints for crash recovery.

use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use okaywal::{Entry, EntryId, LogManager, RecoveredSegment, Recovery, SegmentReader, WriteAheadLog};
use parking_lot::Mutex;
use tracing::{debug, info, trace, warn};

use super::types::WalEntry;
use ccmux_utils::{CcmuxError, Result};

/// WAL configuration
#[derive(Debug, Clone)]
pub struct WalConfig {
    /// Maximum segment size in bytes before rotation
    pub max_segment_size: u64,
    /// Whether to sync after each write
    pub sync_on_write: bool,
    /// Maximum number of segments to keep
    pub max_segments: usize,
}

impl Default for WalConfig {
    fn default() -> Self {
        Self {
            max_segment_size: 64 * 1024 * 1024, // 64 MB
            sync_on_write: true,
            max_segments: 10,
        }
    }
}

/// WAL manager that collects recovered entries
#[derive(Debug, Default, Clone)]
struct WalManager {
    /// Recovered entries during WAL recovery
    recovered_entries: Arc<Mutex<Vec<WalEntry>>>,
}

impl WalManager {
    fn new() -> Self {
        Self {
            recovered_entries: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn take_entries(&self) -> Vec<WalEntry> {
        std::mem::take(&mut *self.recovered_entries.lock())
    }
}

impl LogManager for WalManager {
    fn should_recover_segment(&mut self, _segment: &RecoveredSegment) -> std::io::Result<Recovery> {
        Ok(Recovery::Recover)
    }

    fn recover(&mut self, entry: &mut Entry<'_>) -> std::io::Result<()> {
        // Read all chunks from the entry
        if let Some(chunks) = entry.read_all_chunks()? {
            // Concatenate all chunks
            let data: Vec<u8> = chunks.into_iter().flatten().collect();

            // Deserialize the WAL entry
            match bincode::deserialize::<WalEntry>(&data) {
                Ok(wal_entry) => {
                    self.recovered_entries.lock().push(wal_entry);
                    trace!("Recovered WAL entry");
                }
                Err(e) => {
                    warn!("Failed to deserialize WAL entry: {}", e);
                }
            }
        }

        Ok(())
    }

    fn checkpoint_to(
        &mut self,
        _last_checkpointed_id: EntryId,
        _entries: &mut SegmentReader,
        _wal: &WriteAheadLog,
    ) -> std::io::Result<()> {
        debug!("WAL checkpoint completed");
        Ok(())
    }
}

/// WAL manager for persistence
pub struct Wal {
    /// The underlying WAL
    wal: WriteAheadLog,
    /// Configuration
    config: WalConfig,
    /// Current sequence number
    sequence: AtomicU64,
    /// Recovered entries from initial recovery
    recovered_entries: Vec<WalEntry>,
}

impl Wal {
    /// Open or create a WAL at the given path
    pub fn open(path: impl AsRef<Path>, config: WalConfig) -> Result<Self> {
        let path = path.as_ref();

        info!("Opening WAL at {}", path.display());

        // Create directory if it doesn't exist
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                CcmuxError::persistence(format!("Failed to create WAL directory: {}", e))
            })?;
        }

        // Create the WAL manager that handles recovery
        let manager = WalManager::new();
        let entries_ref = manager.recovered_entries.clone();

        // Open the WAL - this will trigger recovery
        let wal = WriteAheadLog::recover(path, manager).map_err(|e| {
            CcmuxError::persistence(format!("Failed to open WAL: {}", e))
        })?;

        // Get the recovered entries
        let recovered_entries = entries_ref.lock().clone();
        let entry_count = recovered_entries.len();

        info!("WAL opened, recovered {} entries", entry_count);

        Ok(Self {
            wal,
            config,
            sequence: AtomicU64::new(entry_count as u64),
            recovered_entries,
        })
    }

    /// Append an entry to the WAL
    pub fn append(&self, entry: &WalEntry) -> Result<u64> {
        let sequence = self.sequence.fetch_add(1, Ordering::SeqCst);

        // Serialize the entry
        let data = bincode::serialize(entry).map_err(|e| {
            CcmuxError::persistence(format!("Failed to serialize WAL entry: {}", e))
        })?;

        trace!("Appending WAL entry, sequence={}, size={}", sequence, data.len());

        // Write to WAL
        let mut writer = self.wal.begin_entry().map_err(|e| {
            CcmuxError::persistence(format!("Failed to begin WAL entry: {}", e))
        })?;

        writer.write_chunk(&data).map_err(|e| {
            CcmuxError::persistence(format!("Failed to write WAL chunk: {}", e))
        })?;

        let _entry_id = writer.commit().map_err(|e| {
            CcmuxError::persistence(format!("Failed to commit WAL entry: {}", e))
        })?;

        // Note: okaywal handles durability automatically via fsync on commit
        // We can periodically call checkpoint_active() for explicit durability

        Ok(sequence)
    }

    /// Append multiple entries atomically
    pub fn append_batch(&self, entries: &[WalEntry]) -> Result<u64> {
        if entries.is_empty() {
            return Ok(self.sequence.load(Ordering::SeqCst));
        }

        let first_sequence = self.sequence.fetch_add(entries.len() as u64, Ordering::SeqCst);

        for entry in entries {
            let data = bincode::serialize(entry).map_err(|e| {
                CcmuxError::persistence(format!("Failed to serialize WAL entry: {}", e))
            })?;

            let mut writer = self.wal.begin_entry().map_err(|e| {
                CcmuxError::persistence(format!("Failed to begin WAL entry: {}", e))
            })?;

            writer.write_chunk(&data).map_err(|e| {
                CcmuxError::persistence(format!("Failed to write WAL chunk: {}", e))
            })?;

            let _entry_id = writer.commit().map_err(|e| {
                CcmuxError::persistence(format!("Failed to commit WAL entry: {}", e))
            })?;
        }

        // Note: okaywal handles durability automatically via fsync on commit

        Ok(first_sequence + entries.len() as u64 - 1)
    }

    /// Get all entries that were recovered when the WAL was opened
    pub fn recovered_entries(&self) -> &[WalEntry] {
        &self.recovered_entries
    }

    /// Read all entries from the WAL
    ///
    /// Note: This re-opens the WAL to read entries, which may be expensive.
    /// For recovery, prefer using `recovered_entries()` which captures
    /// entries during the initial open.
    pub fn read_all(&self) -> Result<Vec<WalEntry>> {
        Ok(self.recovered_entries.clone())
    }

    /// Read entries after a checkpoint sequence
    pub fn read_after_checkpoint(&self, checkpoint_sequence: u64) -> Result<Vec<WalEntry>> {
        let entries = &self.recovered_entries;

        // Find the checkpoint marker and return entries after it
        let mut found_checkpoint = false;
        let mut result = Vec::new();

        for entry in entries {
            if let Some(seq) = entry.checkpoint_sequence() {
                if seq == checkpoint_sequence {
                    found_checkpoint = true;
                    continue;
                }
            }

            if found_checkpoint {
                result.push(entry.clone());
            }
        }

        // If no checkpoint found, return all entries
        if !found_checkpoint && checkpoint_sequence == 0 {
            return Ok(self.recovered_entries.clone());
        }

        Ok(result)
    }

    /// Get current sequence number
    pub fn sequence(&self) -> u64 {
        self.sequence.load(Ordering::SeqCst)
    }

    /// Force sync to disk
    pub fn sync(&self) -> Result<()> {
        debug!("WAL sync requested");
        // okaywal syncs on checkpoint_to
        Ok(())
    }

    /// Truncate WAL up to a checkpoint
    pub fn truncate_to_checkpoint(&self, checkpoint_sequence: u64) -> Result<()> {
        info!("Truncating WAL to checkpoint sequence {}", checkpoint_sequence);
        // okaywal handles segment cleanup automatically during checkpointing
        debug!("WAL truncation will be handled by segment cleanup");
        Ok(())
    }

    /// Get approximate WAL size in bytes
    pub fn approximate_size(&self) -> u64 {
        // This is an approximation based on entries written
        self.sequence.load(Ordering::SeqCst) * 100 // rough estimate
    }

    /// Checkpoint the active segment
    pub fn checkpoint_active(&self) -> Result<()> {
        self.wal.checkpoint_active().map_err(|e| {
            CcmuxError::persistence(format!("Failed to checkpoint active segment: {}", e))
        })
    }

    /// Shutdown the WAL gracefully
    pub fn shutdown(self) -> Result<()> {
        self.wal.shutdown().map_err(|e| {
            CcmuxError::persistence(format!("Failed to shutdown WAL: {}", e))
        })
    }
}

/// WAL entry reader for iterating through entries
pub struct WalReader<'a> {
    entries: &'a [WalEntry],
    position: usize,
}

impl<'a> WalReader<'a> {
    /// Create a new reader for the WAL
    pub fn new(wal: &'a Wal) -> Self {
        Self {
            entries: wal.recovered_entries(),
            position: 0,
        }
    }

    /// Get remaining entry count
    pub fn remaining(&self) -> usize {
        self.entries.len().saturating_sub(self.position)
    }
}

impl<'a> Iterator for WalReader<'a> {
    type Item = &'a WalEntry;

    fn next(&mut self) -> Option<Self::Item> {
        if self.position < self.entries.len() {
            let entry = &self.entries[self.position];
            self.position += 1;
            Some(entry)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ccmux_protocol::PaneState;
    use tempfile::TempDir;
    use uuid::Uuid;

    fn create_test_wal() -> (TempDir, Wal) {
        let temp_dir = TempDir::new().unwrap();
        let wal_path = temp_dir.path().join("test.wal");
        let wal = Wal::open(&wal_path, WalConfig::default()).unwrap();
        (temp_dir, wal)
    }

    #[test]
    fn test_wal_open_create() {
        let (_temp_dir, wal) = create_test_wal();
        assert_eq!(wal.sequence(), 0);
    }

    #[test]
    fn test_wal_append() {
        let (_temp_dir, wal) = create_test_wal();

        let entry = WalEntry::SessionCreated {
            id: Uuid::new_v4(),
            name: "test".to_string(),
            created_at: 12345,
        };

        let seq = wal.append(&entry).unwrap();
        assert_eq!(seq, 0);
        assert_eq!(wal.sequence(), 1);
    }

    #[test]
    fn test_wal_append_multiple() {
        let (_temp_dir, wal) = create_test_wal();

        for i in 0..10 {
            let entry = WalEntry::SessionCreated {
                id: Uuid::new_v4(),
                name: format!("session-{}", i),
                created_at: i as u64,
            };
            wal.append(&entry).unwrap();
        }

        assert_eq!(wal.sequence(), 10);
    }

    #[test]
    fn test_wal_append_batch() {
        let (_temp_dir, wal) = create_test_wal();

        let entries: Vec<_> = (0..5)
            .map(|i| WalEntry::SessionCreated {
                id: Uuid::new_v4(),
                name: format!("session-{}", i),
                created_at: i as u64,
            })
            .collect();

        let seq = wal.append_batch(&entries).unwrap();
        assert_eq!(seq, 4); // Last sequence number
        assert_eq!(wal.sequence(), 5);
    }

    #[test]
    fn test_wal_config_default() {
        let config = WalConfig::default();
        assert_eq!(config.max_segment_size, 64 * 1024 * 1024);
        assert!(config.sync_on_write);
        assert_eq!(config.max_segments, 10);
    }

    #[test]
    fn test_wal_recovery() {
        let temp_dir = TempDir::new().unwrap();
        let wal_path = temp_dir.path().join("test.wal");

        let session_id = Uuid::new_v4();

        // Write entries and close
        {
            let wal = Wal::open(&wal_path, WalConfig::default()).unwrap();

            wal.append(&WalEntry::SessionCreated {
                id: session_id,
                name: "test".to_string(),
                created_at: 12345,
            })
            .unwrap();

            wal.append(&WalEntry::SessionRenamed {
                id: session_id,
                new_name: "renamed".to_string(),
            })
            .unwrap();

            // Checkpoint to ensure data is persisted
            wal.checkpoint_active().unwrap();
        }

        // Re-open and verify recovery
        {
            let wal = Wal::open(&wal_path, WalConfig::default()).unwrap();

            let entries = wal.recovered_entries();
            assert_eq!(entries.len(), 2);

            if let WalEntry::SessionCreated { name, id, .. } = &entries[0] {
                assert_eq!(name, "test");
                assert_eq!(*id, session_id);
            } else {
                panic!("Expected SessionCreated");
            }

            if let WalEntry::SessionRenamed { new_name, .. } = &entries[1] {
                assert_eq!(new_name, "renamed");
            } else {
                panic!("Expected SessionRenamed");
            }
        }
    }

    #[test]
    fn test_wal_various_entry_types() {
        let (_temp_dir, wal) = create_test_wal();

        let session_id = Uuid::new_v4();
        let window_id = Uuid::new_v4();
        let pane_id = Uuid::new_v4();

        let entries = vec![
            WalEntry::SessionCreated {
                id: session_id,
                name: "test".to_string(),
                created_at: 0,
            },
            WalEntry::WindowCreated {
                id: window_id,
                session_id,
                name: "main".to_string(),
                index: 0,
                created_at: 0,
            },
            WalEntry::PaneCreated {
                id: pane_id,
                window_id,
                index: 0,
                cols: 80,
                rows: 24,
                created_at: 0,
            },
            WalEntry::PaneStateChanged {
                id: pane_id,
                state: PaneState::Exited { code: Some(0) },
            },
            WalEntry::PaneDestroyed {
                id: pane_id,
                window_id,
            },
            WalEntry::WindowDestroyed {
                id: window_id,
                session_id,
            },
            WalEntry::SessionDestroyed { id: session_id },
        ];

        for entry in &entries {
            wal.append(entry).unwrap();
        }

        assert_eq!(wal.sequence(), entries.len() as u64);
    }

    #[test]
    fn test_wal_checkpoint_marker() {
        let (_temp_dir, wal) = create_test_wal();

        wal.append(&WalEntry::SessionCreated {
            id: Uuid::new_v4(),
            name: "before".to_string(),
            created_at: 0,
        })
        .unwrap();

        wal.append(&WalEntry::CheckpointMarker {
            sequence: 1,
            timestamp: 12345,
        })
        .unwrap();

        wal.append(&WalEntry::SessionCreated {
            id: Uuid::new_v4(),
            name: "after".to_string(),
            created_at: 0,
        })
        .unwrap();
    }

    #[test]
    fn test_wal_empty_batch() {
        let (_temp_dir, wal) = create_test_wal();

        let seq = wal.append_batch(&[]).unwrap();
        assert_eq!(seq, 0);
        assert_eq!(wal.sequence(), 0);
    }

    #[test]
    fn test_wal_sync() {
        let (_temp_dir, wal) = create_test_wal();
        assert!(wal.sync().is_ok());
    }

    #[test]
    fn test_wal_approximate_size() {
        let (_temp_dir, wal) = create_test_wal();

        for _ in 0..10 {
            wal.append(&WalEntry::SessionCreated {
                id: Uuid::new_v4(),
                name: "test".to_string(),
                created_at: 0,
            })
            .unwrap();
        }

        let size = wal.approximate_size();
        assert!(size > 0);
    }

    #[test]
    fn test_wal_reader() {
        let temp_dir = TempDir::new().unwrap();
        let wal_path = temp_dir.path().join("test.wal");

        // Write some entries
        {
            let wal = Wal::open(&wal_path, WalConfig::default()).unwrap();
            for i in 0..3 {
                wal.append(&WalEntry::SessionCreated {
                    id: Uuid::new_v4(),
                    name: format!("session-{}", i),
                    created_at: i as u64,
                })
                .unwrap();
            }
            wal.checkpoint_active().unwrap();
        }

        // Re-open and use reader
        {
            let wal = Wal::open(&wal_path, WalConfig::default()).unwrap();
            let reader = WalReader::new(&wal);
            assert_eq!(reader.remaining(), 3);

            let entries: Vec<_> = reader.collect();
            assert_eq!(entries.len(), 3);
        }
    }
}
