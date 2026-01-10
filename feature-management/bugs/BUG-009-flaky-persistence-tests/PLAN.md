# Implementation Plan: BUG-009

**Work Item**: [BUG-009: Flaky Persistence Tests](PROMPT.md)
**Component**: ccmux-server
**Priority**: P2
**Created**: 2026-01-09

## Overview

The persistence/recovery tests have intermittent race conditions causing approximately 30% of parallel test runs to fail. The failure is non-deterministic - different tests fail each time. This is the same class of issue as BUG-002 (shared temp directories), applied to the persistence module.

## Architecture Decisions

### Approach: Test Isolation via `tempfile::TempDir`

Following the proven pattern from BUG-002, each test should:

1. Create its own unique `TempDir` that is automatically cleaned up
2. Pass the temp directory path to all persistence operations
3. Ensure file handles are dropped before `TempDir` cleanup (Rust's drop order)

### Trade-offs

| Option | Pros | Cons |
|--------|------|------|
| `tempfile::TempDir` | Automatic cleanup, unique per test, proven pattern | Slight overhead |
| `#[serial]` attribute | Simple to apply | Slows test suite, doesn't fix root cause |
| Manual unique paths | No new dependencies | Error-prone, manual cleanup required |

**Decision**: Use `tempfile::TempDir` as primary fix. Only use `#[serial]` if tests fundamentally require sequential execution.

## Files to Investigate

| File | Purpose | Risk Level |
|------|---------|------------|
| `ccmux-server/src/persistence/mod.rs` | Module root, contains `test_persistence_log_operations` | High |
| `ccmux-server/src/persistence/recovery.rs` | Recovery tests (3 failing tests) | High |
| `ccmux-server/src/persistence/wal.rs` | Write-ahead log, likely creates files | Medium |
| `ccmux-server/src/persistence/checkpoint.rs` | Checkpoint files | Medium |
| `ccmux-server/src/persistence/types.rs` | Data types | Low |
| `ccmux-server/src/persistence/scrollback.rs` | Scrollback persistence | Medium |
| `ccmux-server/src/persistence/restoration.rs` | State restoration | Medium |

## Investigation Steps

### Phase 1: Identify Test Patterns

1. **Find all `#[test]` functions** in persistence module
2. **Trace temp directory usage**: Look for:
   - `std::env::temp_dir()`
   - `std::process::id()`
   - Hardcoded paths like `/tmp/ccmux_test`
   - Any path construction in tests
3. **Trace file operations**: Look for:
   - `File::create()`, `File::open()`
   - `std::fs::*` operations
   - File handles stored in structs
4. **Check for cleanup**: Look for:
   - `std::fs::remove_dir_all()`
   - `Drop` implementations
   - Explicit cleanup code

### Phase 2: Identify Shared State

1. **Global/static variables**: Search for `static`, `lazy_static`, `once_cell`
2. **Environment variables**: Tests modifying `std::env`
3. **File locks**: `flock`, `lockf`, or Rust file locking

### Phase 3: Apply Fixes

For each test found:

1. Replace path construction with `tempfile::TempDir::new()`
2. Ensure `TempDir` outlives all file handles
3. Verify cleanup happens automatically

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Some tests still flaky after fix | Low | Medium | Run 50+ iterations to verify |
| Fix breaks tests that pass | Low | High | Run each test individually after fix |
| Performance regression | Low | Low | Monitor test execution time |
| Missed tests | Medium | Medium | Systematic search for all test functions |

## Rollback Strategy

If the fix causes issues:
1. Revert commits associated with this work item
2. All changes are test-only, no production code affected
3. Consider `#[serial]` as fallback

## Success Criteria

- 0% failure rate over 20+ consecutive `cargo test --workspace` runs
- All tests still pass when run individually
- No significant increase in test execution time

## Implementation Notes

<!-- Add notes during implementation -->

### Findings

*To be filled during investigation*

### Code Patterns Identified

*To be filled during investigation*

---
*This plan should be updated as implementation progresses.*
