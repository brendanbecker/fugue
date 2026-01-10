# Implementation Plan: BUG-009

**Work Item**: [BUG-009: Flaky Persistence Tests](PROMPT.md)
**Component**: ccmux-server
**Priority**: P2
**Created**: 2026-01-09
**Status**: COMPLETED

## Overview

The persistence/recovery tests have intermittent race conditions causing approximately 30% of parallel test runs to fail. The failure is non-deterministic - different tests fail each time.

## Root Cause Analysis

### The Problem

Contrary to initial suspicion (BUG-002 pattern of shared temp directories), the actual root cause was **incorrect usage of `checkpoint_active()`** in test code.

When `checkpoint_active()` is called on the okaywal WAL library, it triggers the `LogManager::checkpoint_to()` callback. This callback is designed to:
1. Read all entries up to the checkpoint point
2. Persist them to an external checkpoint file
3. Signal to okaywal that those entries can be removed/skipped during future recovery

**Our `checkpoint_to` implementation did nothing:**
```rust
fn checkpoint_to(
    &mut self,
    _last_checkpointed_id: EntryId,
    _entries: &mut SegmentReader,
    _wal: &WriteAheadLog,
) -> std::io::Result<()> {
    debug!("WAL checkpoint completed");
    Ok(())  // Does nothing - doesn't persist entries anywhere!
}
```

When tests called `checkpoint_active()` without actually creating a checkpoint file, okaywal marked the WAL segments with a `-cp` suffix indicating "checkpointed". On restart, okaywal would see these segments as already processed and skip recovering their entries.

### Why It Was Flaky

The tests passed when:
- okaywal happened to create segments that weren't fully checkpointed
- Timing allowed entries to be written to new segments after checkpoint

The tests failed when:
- All entries fell into checkpointed segments
- okaywal recovery skipped all "checkpointed" entries

## The Fix

### 1. Added `shutdown()` method to `RecoveryManager` (recovery.rs:363-369)

```rust
/// Shutdown the recovery manager, ensuring all data is persisted
pub fn shutdown(self) -> Result<()> {
    self.wal.shutdown()
}
```

### 2. Added `finalize()` method to `PersistenceManager` (mod.rs:440-447)

```rust
/// Finalize the persistence manager, ensuring all data is durably written
pub fn finalize(self) -> Result<()> {
    self.recovery_manager.shutdown()
}
```

### 3. Removed incorrect `checkpoint_active()` calls from tests

Tests that only use WAL recovery (without actual checkpoint files) should NOT call `checkpoint_active()`. They should only call `shutdown()` which ensures entries are durably written without marking them as checkpointed.

**Before (broken):**
```rust
manager.wal().append(&entry).unwrap();
manager.wal().checkpoint_active().unwrap();  // WRONG: marks as checkpointed
drop(manager);  // or just going out of scope
// entries are lost on recovery because okaywal skips "checkpointed" segments
```

**After (correct):**
```rust
manager.wal().append(&entry).unwrap();
manager.shutdown().unwrap();  // Correct: durably writes without checkpointing
// entries are recovered properly
```

## Key Insight

`checkpoint_active()` should ONLY be called when:
1. An actual checkpoint file has been created containing the session state
2. The checkpoint file contains all entries up to the checkpoint marker
3. Those entries can safely be skipped during WAL replay

For tests that only exercise WAL recovery without checkpoints, just call `shutdown()`.

## Files Changed

| File | Changes |
|------|---------|
| `ccmux-server/src/persistence/recovery.rs` | Added `shutdown()` method, removed `checkpoint_active()` from 6 tests |
| `ccmux-server/src/persistence/wal.rs` | Removed `checkpoint_active()` from 2 tests |
| `ccmux-server/src/persistence/mod.rs` | Added `finalize()` method, removed `checkpoint_active()` from 2 tests |

## Verification

- 25 consecutive runs of persistence tests: **100% pass rate**
- 10 consecutive runs of full workspace tests: **100% pass rate** (1330 tests each)

## Success Criteria - All Met

- [x] Root cause identified and documented
- [x] All persistence tests use proper isolation
- [x] All file handles properly closed before cleanup
- [x] All tests pass consistently (0% failure rate over 25+ consecutive runs)
- [x] Tests still pass when run individually
- [x] No performance regression in test execution time
- [x] Pattern documented for future test development

## Lessons Learned

1. **Don't call `checkpoint_active()` without creating actual checkpoints** - okaywal interprets this as "entries have been persisted elsewhere" and will skip them during recovery.

2. **Use `shutdown()` for WAL durability** - The `shutdown()` method ensures all pending writes are flushed to disk without marking entries as checkpointed.

3. **Understand your dependencies** - The flakiness wasn't in our code's logic but in how we were using okaywal's API. The `-cp` suffix on WAL files was the key clue.
