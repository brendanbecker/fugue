# BUG-009: Flaky Persistence/Recovery Tests Due to Test Isolation Issues

**Priority**: P2 (Medium)
**Component**: ccmux-server
**Status**: resolved
**Resolved**: 2026-01-10
**Created**: 2026-01-09

## Summary

The persistence/recovery tests in ccmux-server have intermittent race conditions causing non-deterministic test failures. A different test fails on each run - it's not one specific test but rather test isolation issues affecting the entire persistence test suite.

## Symptoms

- Running `cargo test --workspace` sometimes fails with 1 test failure
- The failing test is different each time:
  - `persistence::recovery::tests::test_recovery_from_wal`
  - `persistence::recovery::tests::test_recovery_active_window_pane`
  - `persistence::recovery::tests::test_recovery_pane_updates`
  - `persistence::tests::test_persistence_log_operations`
- Tests pass when run individually
- Tests pass most of the time when run in parallel, but fail approximately 30% of runs

## Location

Tests are in `ccmux-server/src/persistence/` directory:

| File | Affected Tests |
|------|----------------|
| `recovery.rs` | `test_recovery_from_wal`, `test_recovery_active_window_pane`, `test_recovery_pane_updates` |
| `mod.rs` | `test_persistence_log_operations` |

Related files that may contribute to the issue:
- `wal.rs` - Write-ahead log implementation
- `checkpoint.rs` - Checkpoint file handling
- `types.rs` - Persistence data types

## Reproduction Steps

1. Run `cargo test --workspace` multiple times (at least 5-10 times)
2. Observe that approximately 30% of runs fail with 1 test failure
3. Note which test failed
4. Run that specific test in isolation: `cargo test -p ccmux-server <test_name>`
5. The test passes when run alone
6. Run `cargo test --workspace` again
7. A different test fails this time

## Expected Behavior

All persistence tests should pass reliably 100% of the time when run in parallel as part of the full test suite.

## Actual Behavior

Tests pass individually but fail intermittently when run in parallel. The failure rate is approximately 30%, with varying tests failing each time.

## Root Cause Analysis

**Suspected causes** (requires investigation):

1. **Shared temp directories**: Tests may use the same hardcoded paths (e.g., based on `std::process::id()` like the BUG-002 pattern)
2. **File handle leaks**: File handles may not be properly closed before assertions
3. **Missing cleanup**: Test teardown may not properly clean up created files/directories
4. **Timing assumptions**: Tests may rely on specific timing that breaks under parallel execution
5. **Shared mutable state**: Global or static state may be shared between tests
6. **WAL file locking**: The write-ahead log may have concurrent access issues in tests

## Impact

- **CI/test noise**: Hard to determine if new code is actually broken or just flaky tests
- **Developer productivity**: Requires re-running tests multiple times
- **Confidence erosion**: Developers may start ignoring test failures
- Has been plaguing the project for multiple sessions

## Investigation Required

This bug requires deep investigation before implementing a fix. The fix agent should:

### 1. Launch a codebase-investigator agent to thoroughly analyze:

- All persistence tests and their setup/teardown patterns
- How temp directories are created and cleaned up
- Whether tests share any global state (files, directories, env vars)
- The WAL (write-ahead log) implementation and file locking
- Test parallelism and potential race windows

### 2. Look for patterns like:

- Tests using the same hardcoded paths
- Missing cleanup in test teardown
- File handles not being properly closed
- Tests relying on timing assumptions
- Shared mutable state between tests
- Use of `std::process::id()` for path generation (known problematic pattern from BUG-002)

### 3. Consider solutions:

- Use unique temp directories per test (e.g., `tempfile::TempDir`)
- Add proper file locking
- Use `#[serial]` attribute from `serial_test` crate for tests that can't run in parallel
- Ensure all file handles are dropped before assertions
- Add explicit cleanup in test teardown

### 4. The fix should be comprehensive:

- Don't just fix one test - fix the root cause affecting all persistence tests
- Apply the same pattern used to fix BUG-002 (`tempfile::TempDir`)
- Ensure consistency across all persistence test files

## Acceptance Criteria

- [ ] Root cause identified and documented
- [ ] All persistence tests use proper isolation (unique temp directories)
- [ ] All file handles properly closed before cleanup
- [ ] All tests pass consistently (0% failure rate over 20+ consecutive runs)
- [ ] Tests still pass when run individually
- [ ] No performance regression in test execution time
- [ ] Pattern documented for future test development

## Implementation Tasks

### Section 1: Investigation

- [ ] Read and understand all persistence tests
- [ ] Identify how temp directories are created
- [ ] Trace file handle lifecycle in tests
- [ ] Identify shared state patterns
- [ ] Document findings in PLAN.md

### Section 2: Fix Implementation

- [ ] Convert tests to use `tempfile::TempDir`
- [ ] Ensure all file handles are properly scoped
- [ ] Add explicit cleanup where needed
- [ ] Consider `#[serial]` for tests that truly can't parallelize

### Section 3: Verification

- [ ] Run `cargo test --workspace` at least 20 times consecutively
- [ ] Verify 0% failure rate
- [ ] Run tests individually to verify they still work
- [ ] Check test execution time hasn't regressed significantly

## Related Items

- **BUG-002**: Flaky test `test_ensure_dir_nested` due to shared temp directory (same root cause pattern)
- **FEAT-016**: Persistence - Checkpoint and WAL for Crash Recovery (created the persistence module)

## Notes

This is a P2 bug because it doesn't affect functionality, only test reliability. However, it significantly impacts development velocity and CI trustworthiness. The fix should follow the same pattern used in BUG-002 (using `tempfile::TempDir` for test isolation).

The key insight from BUG-002 was that using `std::process::id()` for temp directory names causes race conditions when tests run in parallel - all tests in the same process share the same PID. The persistence tests likely have a similar issue.
