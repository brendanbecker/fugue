# Persistence and Crash Recovery

> State checkpointing, write-ahead logging, and recovery strategies

## Overview

ccmux must survive terminal crashes, SSH disconnects, and system reboots. The persistence layer combines periodic checkpoints with a write-ahead log (WAL) to provide durable state with minimal data loss.

## Storage Layout

```
~/.ccmux/
├── ccmux.sock              # Unix socket (not persisted)
├── sessions/
│   ├── state.bin           # Latest checkpoint
│   ├── state.bin.tmp       # Atomic write temp file
│   ├── wal/
│   │   ├── 000001.wal      # WAL segments
│   │   ├── 000002.wal
│   │   └── ...
│   └── crash_recovery.json # Crash marker file
├── claude-configs/         # Per-pane Claude isolation
│   └── pane-<uuid>/
│       └── .claude.json
└── config/
    └── ccmux.toml          # User configuration
```

## Hybrid Persistence Strategy

### Two-Layer Approach

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           Persistence Layer                                  │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│   ┌─────────────────────────────────────────────────────────────────────┐   │
│   │                    Checkpoints (every 30-60s)                        │   │
│   │  ┌─────────────────────────────────────────────────────────────┐    │   │
│   │  │  • Full session/window/pane topology                         │    │   │
│   │  │  • Pane metadata (type, command, cwd)                       │    │   │
│   │  │  • Claude session IDs                                        │    │   │
│   │  │  • Terminal screen snapshots (optional)                      │    │   │
│   │  │  • Config version hash                                       │    │   │
│   │  └─────────────────────────────────────────────────────────────┘    │   │
│   └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│   ┌─────────────────────────────────────────────────────────────────────┐   │
│   │                    Write-Ahead Log (continuous)                      │   │
│   │  ┌─────────────────────────────────────────────────────────────┐    │   │
│   │  │  • Output chunks since last checkpoint                       │    │   │
│   │  │  • State transitions                                         │    │   │
│   │  │  • User input events                                         │    │   │
│   │  │  • Pane creation/destruction                                 │    │   │
│   │  └─────────────────────────────────────────────────────────────┘    │   │
│   │  Trimmed after each successful checkpoint                          │   │
│   └─────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Why Hybrid?

| Approach | Pros | Cons |
|----------|------|------|
| Checkpoint only | Simple, small files | Data loss between checkpoints |
| WAL only | Minimal loss | Large files, slow recovery |
| **Hybrid** | Best of both | Moderate complexity |

## Checkpoint Design

### Checkpoint Contents

```rust
#[derive(Serialize, Deserialize)]
pub struct Checkpoint {
    /// Checkpoint format version
    pub version: u32,

    /// When this checkpoint was created
    pub timestamp: SystemTime,

    /// Hash of config at checkpoint time
    pub config_hash: String,

    /// All sessions
    pub sessions: Vec<SessionSnapshot>,

    /// Global server state
    pub server_state: ServerState,
}

#[derive(Serialize, Deserialize)]
pub struct SessionSnapshot {
    pub name: String,
    pub created_at: SystemTime,
    pub windows: Vec<WindowSnapshot>,
}

#[derive(Serialize, Deserialize)]
pub struct WindowSnapshot {
    pub index: usize,
    pub name: Option<String>,
    pub layout: Layout,
    pub panes: Vec<PaneSnapshot>,
    pub active_pane: usize,
}

#[derive(Serialize, Deserialize)]
pub struct PaneSnapshot {
    pub id: Uuid,
    pub pane_type: PaneType,
    pub command: String,
    pub cwd: PathBuf,
    pub env: HashMap<String, String>,

    /// Terminal screen state (rows, cols, cursor)
    pub screen: Option<ScreenSnapshot>,

    /// Claude-specific data
    pub claude: Option<ClaudeSnapshot>,
}

#[derive(Serialize, Deserialize)]
pub struct ClaudeSnapshot {
    pub session_id: Option<String>,
    pub state: ClaudeState,
    pub config_dir: PathBuf,
    pub depth: u32,
    pub parent_pane: Option<Uuid>,
}

#[derive(Serialize, Deserialize)]
pub struct ScreenSnapshot {
    pub rows: u16,
    pub cols: u16,
    pub contents: String,  // Last N lines of terminal content
    pub cursor: (u16, u16),
}
```

### Checkpoint Schedule

```rust
const CHECKPOINT_INTERVAL: Duration = Duration::from_secs(30);
const CHECKPOINT_ON_CHANGE_DELAY: Duration = Duration::from_secs(5);

impl PersistenceManager {
    pub async fn run(&mut self) {
        let mut interval = tokio::time::interval(CHECKPOINT_INTERVAL);
        let mut pending_change = false;

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    self.checkpoint().await?;
                    pending_change = false;
                }

                event = self.events.recv() => {
                    match event {
                        // Immediate checkpoint for critical events
                        Event::SessionCreated | Event::SessionDestroyed => {
                            self.checkpoint().await?;
                        }

                        // Debounced checkpoint for frequent events
                        Event::PaneOutput | Event::StateChange => {
                            pending_change = true;
                        }
                    }
                }
            }
        }
    }
}
```

## Write-Ahead Log Design

### WAL Entry Types

```rust
#[derive(Serialize, Deserialize)]
pub enum WalEntry {
    /// Pane output chunk
    Output {
        pane_id: Uuid,
        data: Vec<u8>,
        timestamp: SystemTime,
    },

    /// User input sent to pane
    Input {
        pane_id: Uuid,
        data: Vec<u8>,
        timestamp: SystemTime,
    },

    /// Pane state change
    StateChange {
        pane_id: Uuid,
        old_state: PaneState,
        new_state: PaneState,
        timestamp: SystemTime,
    },

    /// Pane created
    PaneCreated {
        pane_id: Uuid,
        parent_pane: Option<Uuid>,
        pane_type: PaneType,
        command: String,
        timestamp: SystemTime,
    },

    /// Pane destroyed
    PaneDestroyed {
        pane_id: Uuid,
        exit_code: Option<i32>,
        timestamp: SystemTime,
    },

    /// Checkpoint marker (references checkpoint file)
    CheckpointMarker {
        checkpoint_id: u64,
        timestamp: SystemTime,
    },
}
```

### WAL Management

```rust
impl WalManager {
    /// Append entry to current WAL segment
    pub async fn append(&mut self, entry: WalEntry) -> Result<()> {
        let bytes = bincode::serialize(&entry)?;

        // Write length-prefixed entry
        self.current_segment.write_u32_le(bytes.len() as u32).await?;
        self.current_segment.write_all(&bytes).await?;
        self.current_segment.flush().await?;

        self.current_size += 4 + bytes.len();

        // Rotate if segment too large
        if self.current_size > MAX_SEGMENT_SIZE {
            self.rotate_segment().await?;
        }

        Ok(())
    }

    /// Trim WAL entries before checkpoint
    pub async fn trim(&mut self, checkpoint_id: u64) -> Result<()> {
        // Delete segments that are fully before checkpoint
        for segment in &self.segments {
            if segment.max_checkpoint_id < checkpoint_id {
                std::fs::remove_file(&segment.path)?;
            }
        }

        self.segments.retain(|s| s.max_checkpoint_id >= checkpoint_id);
        Ok(())
    }
}
```

### WAL Configuration

| Setting | Default | Description |
|---------|---------|-------------|
| `max_segment_size` | 16 MB | Rotate after this size |
| `max_segments` | 8 | Delete oldest when exceeded |
| `sync_mode` | `fsync` | Durability guarantee |
| `compression` | `none` | Trade space for speed |

## Atomic Write Pattern

All state files use the atomic write pattern:

```rust
pub fn atomic_write(path: &Path, data: &[u8]) -> Result<()> {
    let temp_path = path.with_extension("tmp");

    // Write to temp file
    let mut file = std::fs::File::create(&temp_path)?;
    file.write_all(data)?;
    file.sync_all()?;  // Ensure data on disk

    // Atomic rename
    std::fs::rename(&temp_path, path)?;

    // Sync parent directory (Linux)
    #[cfg(target_os = "linux")]
    {
        let parent = path.parent().unwrap();
        let dir = std::fs::File::open(parent)?;
        dir.sync_all()?;
    }

    Ok(())
}
```

## Recovery Flow

### Startup Recovery

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           Recovery Flow                                      │
└─────────────────────────────────────────────────────────────────────────────┘

                              Server Startup
                                    │
                                    ▼
                    ┌───────────────────────────────┐
                    │  Check crash_recovery.json    │
                    └───────────────┬───────────────┘
                                    │
                    ┌───────────────┴───────────────┐
                    │                               │
                    ▼                               ▼
            File Exists                      No File
            (Crashed)                    (Clean shutdown)
                    │                               │
                    ▼                               ▼
        ┌───────────────────┐           ┌───────────────────┐
        │ Load checkpoint   │           │ Start fresh       │
        │ Replay WAL        │           │ server            │
        └─────────┬─────────┘           └───────────────────┘
                  │
                  ▼
        ┌───────────────────┐
        │ Restore sessions  │
        │ Respawn panes     │
        └─────────┬─────────┘
                  │
          ┌───────┴───────┐
          │               │
          ▼               ▼
    Shell Panes      Claude Panes
          │               │
          ▼               ▼
    Start fresh      claude --resume
    (ghost image)    <session_id>
```

### Implementation

```rust
impl Server {
    pub async fn start_with_recovery() -> Result<Self> {
        let crash_marker = state_dir().join("crash_recovery.json");

        if crash_marker.exists() {
            log::info!("Crash detected, starting recovery");

            // Load last checkpoint
            let checkpoint = Self::load_checkpoint()?;

            // Replay WAL from checkpoint
            let wal_entries = Self::replay_wal(checkpoint.timestamp)?;

            // Reconstruct state
            let mut server = Self::from_checkpoint(checkpoint);
            for entry in wal_entries {
                server.apply_wal_entry(entry)?;
            }

            // Respawn processes
            server.respawn_all_panes().await?;

            // Remove crash marker
            std::fs::remove_file(&crash_marker)?;

            Ok(server)
        } else {
            // Clean start
            Self::write_crash_marker()?;
            Ok(Self::new())
        }
    }

    async fn respawn_all_panes(&mut self) -> Result<()> {
        for session in &mut self.sessions {
            for window in &mut session.windows {
                for pane in &mut window.panes {
                    match pane.pane_type {
                        PaneType::Shell => {
                            // Start fresh shell, show "ghost image" of previous content
                            pane.spawn_shell()?;
                            if let Some(screen) = &pane.screen_snapshot {
                                pane.display_ghost_image(screen);
                            }
                        }
                        PaneType::Claude => {
                            // Resume Claude session
                            if let Some(ref metadata) = pane.claude_metadata {
                                if let Some(ref session_id) = metadata.session_id {
                                    pane.spawn_claude(Some(session_id))?;
                                } else {
                                    // No session ID, start fresh
                                    pane.spawn_claude(None)?;
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }
}
```

### Ghost Images

For shell panes that can't be resumed, display the last known screen:

```rust
impl Pane {
    pub fn display_ghost_image(&mut self, screen: &ScreenSnapshot) {
        // Render dimmed previous content
        let ghost_content = format!(
            "\x1b[2m{}\x1b[0m\n\
             \x1b[33m[Session recovered - previous output shown above]\x1b[0m\n",
            screen.contents
        );

        self.parser.process(ghost_content.as_bytes());
    }
}
```

## Crash Marker

Presence indicates unclean shutdown:

```rust
pub fn write_crash_marker() -> Result<()> {
    let marker = CrashMarker {
        pid: std::process::id(),
        started_at: SystemTime::now(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    };

    let path = state_dir().join("crash_recovery.json");
    let json = serde_json::to_string_pretty(&marker)?;
    std::fs::write(&path, json)?;

    Ok(())
}

pub fn remove_crash_marker() -> Result<()> {
    let path = state_dir().join("crash_recovery.json");
    if path.exists() {
        std::fs::remove_file(&path)?;
    }
    Ok(())
}
```

### Graceful Shutdown

```rust
impl Server {
    pub async fn shutdown(&mut self) -> Result<()> {
        log::info!("Shutting down gracefully");

        // Final checkpoint
        self.checkpoint().await?;

        // Terminate all panes
        for session in &mut self.sessions {
            for window in &mut session.windows {
                for pane in &mut window.panes {
                    pane.terminate().await?;
                }
            }
        }

        // Remove crash marker (clean shutdown)
        remove_crash_marker()?;

        Ok(())
    }
}
```

## Data Retention

### Configurable Limits

```toml
# ~/.ccmux/config/ccmux.toml

[persistence]
checkpoint_interval_secs = 30
max_wal_size_mb = 128
max_scrollback_lines = 10000
screen_snapshot_lines = 500
```

### Cleanup

```rust
impl PersistenceManager {
    pub async fn cleanup(&mut self) -> Result<()> {
        // Trim old WAL segments
        self.wal.trim(self.last_checkpoint_id).await?;

        // Cleanup orphaned Claude config dirs
        self.cleanup_claude_configs().await?;

        // Remove old backup checkpoints
        self.cleanup_old_checkpoints().await?;

        Ok(())
    }
}
```

## Error Handling

### Checkpoint Failures

```rust
impl PersistenceManager {
    async fn checkpoint(&mut self) -> Result<()> {
        match self.write_checkpoint().await {
            Ok(_) => {
                self.last_checkpoint_id += 1;
                self.wal.trim(self.last_checkpoint_id).await?;
                Ok(())
            }
            Err(e) => {
                log::error!("Checkpoint failed: {}", e);
                // Don't trim WAL on failure
                // Try again on next interval
                Err(e)
            }
        }
    }
}
```

### Corrupted State Recovery

```rust
impl Server {
    fn load_checkpoint() -> Result<Checkpoint> {
        let path = state_dir().join("sessions/state.bin");

        match std::fs::read(&path) {
            Ok(data) => {
                match bincode::deserialize(&data) {
                    Ok(checkpoint) => Ok(checkpoint),
                    Err(e) => {
                        log::warn!("Checkpoint corrupted: {}", e);
                        // Try backup
                        Self::load_backup_checkpoint()
                    }
                }
            }
            Err(e) => {
                log::warn!("Checkpoint missing: {}", e);
                Err(CcmuxError::NoCheckpoint)
            }
        }
    }
}
```

## Related Documents

- [ARCHITECTURE.md](./ARCHITECTURE.md) - System overview
- [CLAUDE_INTEGRATION.md](./CLAUDE_INTEGRATION.md) - Claude session recovery
- [CONFIGURATION.md](./CONFIGURATION.md) - Persistence settings
