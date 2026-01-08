# Implementation Plan: FEAT-016

**Work Item**: [FEAT-016: Persistence - Checkpoint and WAL for Crash Recovery](PROMPT.md)
**Component**: ccmux-server
**Priority**: P2
**Created**: 2026-01-08
**Status**: Not Started

## Overview

Checkpoint + WAL persistence using okaywal and bincode for crash recovery, session state serialization. This enables the terminal multiplexer to survive server crashes and restarts while preserving user sessions.

## Architecture Decisions

### Persistence Strategy

The persistence layer uses a two-tier approach:

1. **Write-Ahead Log (WAL)**: Records every state change as it happens
   - Provides durability for recent changes
   - Enables point-in-time recovery
   - Uses okaywal for ACID guarantees

2. **Checkpoints**: Periodic full state snapshots
   - Reduces WAL replay time on recovery
   - Enables WAL truncation
   - Uses bincode for efficient serialization

```
                    +------------------+
                    |  SessionManager  |
                    +--------+---------+
                             |
              +--------------+--------------+
              |                             |
              v                             v
    +---------+----------+        +---------+----------+
    |  WAL (okaywal)     |        |  Checkpoint Files  |
    |  - Entry log       |        |  - Full snapshots  |
    |  - Sequential I/O  |        |  - Bincode format  |
    +--------------------+        +--------------------+
              |                             |
              v                             v
    +---------+----------------------------+----------+
    |              Persistence Directory              |
    |  /run/ccmux/{session-id}/                       |
    |    - wal/                                       |
    |    - checkpoints/                               |
    |    - scrollback/                                |
    +-------------------------------------------------+
```

### WAL Entry Types

```rust
enum WalEntry {
    // Session operations
    SessionCreated { id: SessionId, name: String, created_at: Timestamp },
    SessionDeleted { id: SessionId },
    SessionRenamed { id: SessionId, new_name: String },

    // Window operations
    WindowCreated { session_id: SessionId, window_id: WindowId, name: String },
    WindowDeleted { session_id: SessionId, window_id: WindowId },

    // Pane operations
    PaneCreated { session_id: SessionId, window_id: WindowId, pane_id: PaneId, config: PaneConfig },
    PaneDeleted { session_id: SessionId, window_id: WindowId, pane_id: PaneId },
    PaneOutput { pane_id: PaneId, data: Vec<u8> },
    PaneResized { pane_id: PaneId, rows: u16, cols: u16 },

    // Active selection
    ActiveChanged { session_id: Option<SessionId>, window_id: Option<WindowId>, pane_id: Option<PaneId> },

    // Checkpoint marker
    CheckpointCreated { checkpoint_id: u64, timestamp: Timestamp },
}
```

### Checkpoint Format

Checkpoints are stored as bincode-serialized state:

```rust
struct Checkpoint {
    version: u32,
    checkpoint_id: u64,
    timestamp: Timestamp,
    wal_position: u64,
    sessions: Vec<SessionSnapshot>,
}

struct SessionSnapshot {
    id: SessionId,
    name: String,
    created_at: Timestamp,
    windows: Vec<WindowSnapshot>,
    active_window: Option<WindowId>,
}

struct WindowSnapshot {
    id: WindowId,
    name: String,
    panes: Vec<PaneSnapshot>,
    active_pane: Option<PaneId>,
}

struct PaneSnapshot {
    id: PaneId,
    cwd: PathBuf,
    env: HashMap<String, String>,
    shell: String,
    scrollback_file: Option<PathBuf>,
    rows: u16,
    cols: u16,
}
```

### Recovery Process

1. **Startup Detection**: Check for existence of persistence directory
2. **Checkpoint Loading**: Find and load most recent valid checkpoint
3. **WAL Replay**: Replay WAL entries from checkpoint position forward
4. **State Validation**: Verify recovered state is consistent
5. **PTY Respawn**: Create new PTY processes for each recovered pane
6. **Scrollback Restore**: Load scrollback buffers from disk

### Thread Safety

- WAL writes are serialized through a single writer channel
- Checkpoint creation acquires read lock on SessionManager
- Recovery happens before server accepts connections (single-threaded)

### Configuration Options

```toml
[persistence]
enabled = true
directory = "/run/ccmux"
checkpoint_interval_secs = 300  # 5 minutes
wal_flush_interval_ms = 100
max_wal_size_mb = 100
scrollback_persistence = true
scrollback_max_lines = 10000
```

## Affected Components

| Component | Type of Change | Risk Level |
|-----------|----------------|------------|
| ccmux-server/src/persistence/mod.rs | New module | Low |
| ccmux-server/src/persistence/wal.rs | New - WAL implementation | High |
| ccmux-server/src/persistence/checkpoint.rs | New - Checkpoint logic | High |
| ccmux-server/src/persistence/recovery.rs | New - Recovery logic | High |
| ccmux-server/src/persistence/serialization.rs | New - Serde helpers | Medium |
| ccmux-server/src/session/manager.rs | Modification - WAL integration | Medium |
| ccmux-server/src/config/schema.rs | Modification - Persistence config | Low |

## Dependencies

- FEAT-012: Session Management - Session/Window/Pane Hierarchy (provides SessionManager)

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| WAL corruption | Low | High | Checksums, validation on read |
| Checkpoint write failure | Low | Medium | Atomic rename, keep old checkpoint |
| Recovery data loss | Medium | High | Multiple checkpoint retention |
| Performance impact from WAL | Medium | Medium | Async writes, batching |
| Disk space exhaustion | Low | Medium | WAL rotation, checkpoint cleanup |
| Version incompatibility | Medium | High | Version field, migration support |

## Implementation Phases

### Phase 1: Foundation
- Create persistence module structure
- Add dependencies (okaywal, bincode)
- Implement persistence configuration
- Set up directory management

### Phase 2: WAL Implementation
- Implement WAL wrapper
- Define entry types
- Implement write path
- Implement read/replay path

### Phase 3: Checkpointing
- Define checkpoint format
- Implement checkpoint creation
- Implement checkpoint loading
- Add validation and cleanup

### Phase 4: Recovery
- Implement recovery detection
- Implement state restoration
- Implement PTY respawning
- Handle edge cases

### Phase 5: Integration
- Wire WAL into SessionManager
- Add checkpoint scheduling
- Integrate with server lifecycle
- Add configuration support

### Phase 6: Testing and Hardening
- Comprehensive test suite
- Crash simulation testing
- Performance optimization
- Error handling improvements

## Rollback Strategy

If implementation causes issues:
1. Disable persistence via configuration (`enabled = false`)
2. Revert commits associated with this work item
3. Remove persistence directory to clean up
4. Document what went wrong in comments.md

## Implementation Notes

<!-- Add notes during implementation -->

---
*This plan should be updated as implementation progresses.*
