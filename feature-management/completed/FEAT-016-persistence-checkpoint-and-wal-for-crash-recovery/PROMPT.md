# FEAT-016: Persistence - Checkpoint and WAL for Crash Recovery

**Priority**: P2
**Component**: fugue-server
**Type**: new_feature
**Estimated Effort**: large
**Business Value**: medium
**Status**: new

## Overview

Checkpoint + WAL persistence using okaywal and bincode for crash recovery, session state serialization. This feature enables reliable recovery of terminal sessions after server crashes or unexpected restarts, preserving session hierarchy, PTY state, and scrollback buffers.

## Requirements

### Write-Ahead Log (WAL) using okaywal crate
- Implement WAL for recording state changes before they are applied
- Use okaywal crate for reliable write-ahead logging
- Log session, window, and pane state changes
- Log PTY output for scrollback recovery
- Support log rotation and compaction

### Periodic Checkpointing of Session State
- Snapshot entire session hierarchy at configurable intervals
- Checkpoint includes all sessions, windows, and panes
- Store checkpoint with timestamp and sequence number
- Support incremental checkpoints for performance
- Clean up old checkpoints after successful new checkpoint

### Bincode Serialization for State Snapshots
- Use bincode for efficient binary serialization
- Serialize Session, Window, and Pane structs
- Serialize PTY configuration and environment
- Serialize scrollback buffer contents
- Handle versioning for format compatibility

### Crash Recovery on Server Restart
- Detect incomplete shutdown on startup
- Load most recent valid checkpoint
- Replay WAL entries since checkpoint
- Validate recovered state consistency
- Report recovery status to user

### Session Restoration with PTY Reconnection
- Restore session hierarchy from persisted state
- Spawn new PTYs for recovered panes
- Restore working directories and environment
- Handle cases where original process cannot be restored
- Notify user of restoration status per pane

### Scrollback Buffer Persistence
- Persist scrollback buffer contents to disk
- Compress scrollback data for storage efficiency
- Lazy-load scrollback on demand
- Configurable scrollback persistence limits
- Handle large scrollback buffers gracefully

### Configurable Checkpoint Interval
- Configuration option for checkpoint interval (seconds)
- Configuration option for WAL flush frequency
- Configuration option for maximum WAL size before checkpoint
- Configuration option for scrollback persistence depth
- Runtime configuration updates via hot-reload

## Affected Files

- `fugue-server/src/persistence/checkpoint.rs` - Checkpoint creation and loading
- `fugue-server/src/persistence/wal.rs` - Write-ahead log implementation
- `fugue-server/src/persistence/mod.rs` - Persistence module exports
- `fugue-server/src/persistence/recovery.rs` - Crash recovery logic
- `fugue-server/src/persistence/serialization.rs` - Bincode serialization helpers
- `fugue-server/src/config/schema.rs` - Persistence configuration schema

## Implementation Tasks

### Section 1: Persistence Module Setup
- [ ] Create persistence module structure
- [ ] Add okaywal and bincode dependencies
- [ ] Define persistence configuration schema
- [ ] Implement persistence directory management
- [ ] Add persistence error types

### Section 2: Write-Ahead Log
- [ ] Implement WAL wrapper around okaywal
- [ ] Define WAL entry types (session, window, pane, output)
- [ ] Implement WAL write operations
- [ ] Implement WAL read/replay operations
- [ ] Add WAL rotation and compaction
- [ ] Handle WAL corruption detection

### Section 3: Checkpointing
- [ ] Define checkpoint file format
- [ ] Implement checkpoint creation
- [ ] Implement checkpoint loading
- [ ] Add checkpoint validation
- [ ] Implement checkpoint cleanup
- [ ] Add incremental checkpoint support

### Section 4: Serialization
- [ ] Implement Serialize/Deserialize for Session
- [ ] Implement Serialize/Deserialize for Window
- [ ] Implement Serialize/Deserialize for Pane
- [ ] Add version field for format compatibility
- [ ] Implement scrollback buffer serialization
- [ ] Add compression for large data

### Section 5: Recovery Logic
- [ ] Detect unclean shutdown
- [ ] Load and validate checkpoint
- [ ] Replay WAL entries
- [ ] Restore session hierarchy
- [ ] Spawn replacement PTYs
- [ ] Handle recovery failures gracefully

### Section 6: Integration
- [ ] Integrate WAL with SessionManager
- [ ] Add checkpoint trigger to server main loop
- [ ] Wire persistence configuration
- [ ] Add recovery to server startup
- [ ] Implement graceful shutdown with final checkpoint

### Section 7: Testing
- [ ] Unit tests for WAL operations
- [ ] Unit tests for checkpoint creation/loading
- [ ] Unit tests for serialization
- [ ] Integration tests for crash recovery
- [ ] Stress tests for WAL performance

## Acceptance Criteria

- [ ] WAL records all state changes reliably
- [ ] Checkpoints are created at configured intervals
- [ ] Server recovers session state after crash
- [ ] PTYs are respawned with correct configuration
- [ ] Scrollback buffers are restored
- [ ] Configuration options control persistence behavior
- [ ] Recovery handles corruption gracefully
- [ ] All unit and integration tests pass

## Dependencies

- FEAT-012: Session Management - Session/Window/Pane Hierarchy (prerequisite)

## Notes

- okaywal provides ACID guarantees for the write-ahead log
- bincode is chosen for its speed and compact binary format
- Consider using memory-mapped files for large scrollback buffers
- PTY processes cannot be truly reconnected; new processes must be spawned
- Recovery should be fast to minimize server restart time
- WAL should be fsync'd to ensure durability guarantees
