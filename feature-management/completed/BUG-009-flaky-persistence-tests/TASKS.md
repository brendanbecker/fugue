# Task Breakdown: BUG-009

**Work Item**: [BUG-009: Flaky Persistence Tests](PROMPT.md)
**Status**: Not Started
**Last Updated**: 2026-01-09

## Prerequisites

- [ ] Read and understand PROMPT.md
- [ ] Review PLAN.md and update if needed
- [ ] Read BUG-002 resolution for pattern reference

## Investigation Tasks

### Codebase Analysis (Launch codebase-investigator agent)

- [ ] Read all test functions in `ccmux-server/src/persistence/mod.rs`
- [ ] Read all test functions in `ccmux-server/src/persistence/recovery.rs`
- [ ] Read all test functions in `ccmux-server/src/persistence/wal.rs`
- [ ] Read all test functions in `ccmux-server/src/persistence/checkpoint.rs`
- [ ] Read all test functions in `ccmux-server/src/persistence/scrollback.rs`
- [ ] Read all test functions in `ccmux-server/src/persistence/restoration.rs`

### Pattern Identification

- [ ] Document how temp directories are currently created
- [ ] Identify any use of `std::process::id()` for path generation
- [ ] Identify hardcoded temp paths
- [ ] Check for shared static/global state
- [ ] Trace file handle lifecycle in each test
- [ ] Document cleanup patterns (or lack thereof)
- [ ] Update PLAN.md with findings

### Root Cause Confirmation

- [ ] Reproduce the flaky failure at least once
- [ ] Confirm failure is related to parallel execution
- [ ] Identify which tests conflict with each other

## Implementation Tasks

### Fix Temp Directory Isolation

- [ ] Add `tempfile` to dev-dependencies if not present
- [ ] Convert `mod.rs` tests to use `tempfile::TempDir`
- [ ] Convert `recovery.rs` tests to use `tempfile::TempDir`
- [ ] Convert `wal.rs` tests to use `tempfile::TempDir` (if applicable)
- [ ] Convert `checkpoint.rs` tests to use `tempfile::TempDir` (if applicable)
- [ ] Convert `scrollback.rs` tests to use `tempfile::TempDir` (if applicable)
- [ ] Convert `restoration.rs` tests to use `tempfile::TempDir` (if applicable)

### Fix File Handle Issues

- [ ] Ensure file handles are scoped correctly
- [ ] Add explicit drops where needed
- [ ] Verify cleanup order (handles before TempDir)

### Apply Serial Execution (If Needed)

- [ ] Add `serial_test` to dev-dependencies if needed
- [ ] Apply `#[serial]` to tests that fundamentally can't parallelize
- [ ] Document why each `#[serial]` is necessary

## Testing Tasks

- [ ] Run `cargo test --workspace` 5 times - document results
- [ ] Run `cargo test --workspace` 10 more times - document results
- [ ] Run `cargo test --workspace` 5 more times (20 total) - all should pass
- [ ] Run individual tests to verify they still work:
  - [ ] `cargo test -p ccmux-server test_recovery_from_wal`
  - [ ] `cargo test -p ccmux-server test_recovery_active_window_pane`
  - [ ] `cargo test -p ccmux-server test_recovery_pane_updates`
  - [ ] `cargo test -p ccmux-server test_persistence_log_operations`

## Verification Tasks

- [ ] Confirm 0% failure rate over 20+ runs
- [ ] Verify no performance regression
- [ ] Update bug_report.json status to "resolved"
- [ ] Document resolution in PLAN.md

## Documentation Tasks

- [ ] Document the pattern for future test development
- [ ] Add comments to tests explaining isolation approach
- [ ] Update PROMPT.md with resolution details

## Completion Checklist

- [ ] All investigation tasks complete
- [ ] All implementation tasks complete
- [ ] All tests passing consistently (0% failure rate)
- [ ] PLAN.md updated with final approach and findings
- [ ] Ready for review/merge

---
*Check off tasks as you complete them. Update status field above.*
