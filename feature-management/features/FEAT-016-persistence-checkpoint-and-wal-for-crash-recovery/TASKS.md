# Task Breakdown: FEAT-016

**Work Item**: [FEAT-016: Persistence - Checkpoint and WAL for Crash Recovery](PROMPT.md)
**Status**: Not Started
**Last Updated**: 2026-01-08

## Prerequisites

- [ ] Read and understand PROMPT.md
- [ ] Review PLAN.md and update if needed
- [ ] Verify FEAT-012 (Session Management) is complete
- [ ] Review okaywal crate documentation
- [ ] Review bincode serialization patterns
- [ ] Understand existing SessionManager API

## Design Tasks

- [ ] Finalize WAL entry type definitions
- [ ] Finalize checkpoint format specification
- [ ] Design persistence directory structure
- [ ] Design recovery state machine
- [ ] Plan scrollback compression strategy
- [ ] Design configuration schema for persistence
- [ ] Document error handling strategy

## Implementation Tasks

### Persistence Module Setup
- [ ] Create `ccmux-server/src/persistence/mod.rs`
- [ ] Create `ccmux-server/src/persistence/wal.rs`
- [ ] Create `ccmux-server/src/persistence/checkpoint.rs`
- [ ] Create `ccmux-server/src/persistence/recovery.rs`
- [ ] Create `ccmux-server/src/persistence/serialization.rs`
- [ ] Add okaywal dependency to Cargo.toml
- [ ] Add bincode dependency to Cargo.toml
- [ ] Define PersistenceError enum
- [ ] Implement persistence directory creation/validation

### WAL Implementation
- [ ] Define WalEntry enum with all entry types
- [ ] Implement Wal struct wrapping okaywal
- [ ] Implement Wal::new() with directory setup
- [ ] Implement Wal::append() for writing entries
- [ ] Implement Wal::read_from() for replay
- [ ] Implement Wal::truncate_before() for cleanup
- [ ] Implement entry serialization with bincode
- [ ] Add checksums for entry validation
- [ ] Handle WAL file rotation
- [ ] Implement WAL compaction strategy

### Checkpoint Implementation
- [ ] Define Checkpoint struct
- [ ] Define SessionSnapshot, WindowSnapshot, PaneSnapshot structs
- [ ] Implement Checkpoint::create() from SessionManager
- [ ] Implement Checkpoint::save() to disk
- [ ] Implement Checkpoint::load() from disk
- [ ] Implement Checkpoint::validate() for consistency
- [ ] Add atomic write with temporary file and rename
- [ ] Implement checkpoint file naming with sequence numbers
- [ ] Implement old checkpoint cleanup
- [ ] Add checkpoint compression (optional)

### Serialization Helpers
- [ ] Implement Serialize/Deserialize for Session (if not already)
- [ ] Implement Serialize/Deserialize for Window (if not already)
- [ ] Implement Serialize/Deserialize for Pane (if not already)
- [ ] Implement scrollback buffer serialization
- [ ] Add version field handling for compatibility
- [ ] Implement scrollback compression with zstd/lz4
- [ ] Handle serialization errors gracefully

### Recovery Logic
- [ ] Implement RecoveryManager struct
- [ ] Implement unclean shutdown detection
- [ ] Implement find_latest_checkpoint()
- [ ] Implement load_and_validate_checkpoint()
- [ ] Implement replay_wal_entries()
- [ ] Implement rebuild_session_manager()
- [ ] Implement respawn_ptys()
- [ ] Implement restore_scrollback_buffers()
- [ ] Handle partial recovery scenarios
- [ ] Implement recovery status reporting

### SessionManager Integration
- [ ] Add WAL writer to SessionManager
- [ ] Emit WAL entries on session create/delete
- [ ] Emit WAL entries on window create/delete
- [ ] Emit WAL entries on pane create/delete
- [ ] Emit WAL entries on active selection change
- [ ] Emit WAL entries on PTY output (batched)
- [ ] Add checkpoint trigger method
- [ ] Implement graceful shutdown with final checkpoint

### Server Integration
- [ ] Add persistence configuration to server config
- [ ] Initialize persistence on server startup
- [ ] Run recovery if persistence data exists
- [ ] Schedule periodic checkpoint creation
- [ ] Handle checkpoint in server shutdown
- [ ] Add persistence status to server health

### Configuration
- [ ] Add PersistenceConfig struct
- [ ] Add persistence section to config schema
- [ ] Implement config validation for persistence settings
- [ ] Add runtime config reload for persistence
- [ ] Document configuration options

## Testing Tasks

### Unit Tests
- [ ] Test WAL entry serialization/deserialization
- [ ] Test WAL append and read operations
- [ ] Test checkpoint creation
- [ ] Test checkpoint save/load roundtrip
- [ ] Test checkpoint validation
- [ ] Test scrollback serialization
- [ ] Test recovery state machine

### Integration Tests
- [ ] Test full checkpoint/restore cycle
- [ ] Test WAL replay after checkpoint
- [ ] Test recovery with missing checkpoint
- [ ] Test recovery with corrupted WAL
- [ ] Test recovery with corrupted checkpoint
- [ ] Test PTY respawn after recovery
- [ ] Test scrollback restore

### Stress Tests
- [ ] Test WAL performance under high write load
- [ ] Test checkpoint creation with many sessions
- [ ] Test recovery time with large state
- [ ] Test disk space handling

## Documentation Tasks

- [ ] Document persistence module API
- [ ] Document WAL entry format
- [ ] Document checkpoint file format
- [ ] Document recovery process
- [ ] Document configuration options
- [ ] Add troubleshooting guide for persistence issues
- [ ] Document backup/restore procedures

## Verification Tasks

- [ ] All acceptance criteria from PROMPT.md met
- [ ] Tests passing
- [ ] Update feature_request.json status
- [ ] Performance benchmarks acceptable
- [ ] Disk space usage reasonable

## Completion Checklist

- [ ] All implementation tasks complete
- [ ] All tests passing
- [ ] Documentation updated
- [ ] PLAN.md reflects final implementation
- [ ] Configuration documented
- [ ] Ready for review/merge

---
*Check off tasks as you complete them. Update status field above.*
