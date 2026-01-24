# BUG-002: Flaky test `test_ensure_dir_nested` due to shared temp directory

**Type**: Bug
**Priority**: P2
**Status**: open
**Created**: 2026-01-09
**Component**: fugue-utils

## Problem Statement

The test `test_ensure_dir_nested` intermittently fails when running the full test suite in parallel (`cargo test --workspace`), but passes consistently when run in isolation. This causes CI failures that are not related to actual code issues, wasting developer time investigating false positives.

## Affected Tests

| Test | File | Line | Description |
|------|------|------|-------------|
| `test_ensure_dir_creates_directory` | `fugue-utils/src/paths.rs` | 393 | Creates `fugue_test_{pid}/` |
| `test_ensure_dir_nested` | `fugue-utils/src/paths.rs` | 412 | Creates `fugue_test_{pid}/nested/deep` |

Both tests use `std::process::id()` (PID) to generate their temp directory path, resulting in:
- Test 1: `/tmp/fugue_test_12345/`
- Test 2: `/tmp/fugue_test_12345/nested/deep`

## Root Cause

The two tests share the same base directory path (`fugue_test_{pid}`). When Rust's test runner executes tests in parallel (which is the default behavior):

1. **Race Condition Scenario A**: `test_ensure_dir_creates_directory` creates `fugue_test_{pid}/`, then deletes it in cleanup (line 409). Meanwhile, `test_ensure_dir_nested` is trying to use that same base directory.

2. **Race Condition Scenario B**: `test_ensure_dir_nested` creates `fugue_test_{pid}/nested/deep` and then calls `remove_dir_all` on the base (line 433), which deletes the base directory while `test_ensure_dir_creates_directory` is still using it.

The fundamental issue is that the PID is the same for all tests in a single test run, providing no isolation between tests running concurrently.

## Reproduction Steps

1. Run `cargo test --workspace` (multiple times if needed - flaky by nature)
2. Observe intermittent failure of `test_ensure_dir_nested`
3. Run `cargo test -p fugue-utils test_ensure_dir_nested` - passes consistently in isolation

To increase reproduction likelihood:
```bash
# Run tests 50 times to observe flakiness
for i in {1..50}; do cargo test --workspace 2>&1 | grep -E "(FAILED|passed)"; done
```

## Error Message

```
thread 'paths::tests::test_ensure_dir_nested' panicked at fugue-utils/src/paths.rs:428:9:
assertion failed: result.is_ok()
```

The `ensure_dir` call fails because the parent directory was deleted by the other test during a race condition.

## Fix Approach

Replace manual temp directory management with `tempfile::TempDir`, which is already a dev-dependency in `fugue-utils/Cargo.toml`.

**Current Code (lines 393-410 and 412-434):**
```rust
#[test]
fn test_ensure_dir_creates_directory() {
    let temp_dir = std::env::temp_dir();
    let test_dir = temp_dir.join(format!("fugue_test_{}", std::process::id()));
    // ... cleanup logic with remove_dir_all
}

#[test]
fn test_ensure_dir_nested() {
    let temp_dir = std::env::temp_dir();
    let test_dir = temp_dir
        .join(format!("fugue_test_{}", std::process::id()))
        .join("nested")
        .join("deep");
    // ... cleanup logic with remove_dir_all
}
```

**Fixed Code:**
```rust
#[test]
fn test_ensure_dir_creates_directory() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let test_dir = temp_dir.path().join("test_dir");

    let result = ensure_dir(&test_dir);
    assert!(result.is_ok());
    assert!(test_dir.exists());
    assert!(test_dir.is_dir());
    // TempDir auto-cleans on drop - no manual cleanup needed
}

#[test]
fn test_ensure_dir_nested() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let test_dir = temp_dir.path().join("nested").join("deep");

    let result = ensure_dir(&test_dir);
    assert!(result.is_ok());
    assert!(test_dir.exists());
    assert!(test_dir.is_dir());
    // TempDir auto-cleans on drop - no manual cleanup needed
}
```

Each test gets a unique directory (e.g., `/tmp/.tmpXXXXXX/`), eliminating any possibility of collision.

## Implementation Tasks

### Section 1: Fix the Flaky Tests
- [ ] Update `test_ensure_dir_creates_directory` to use `tempfile::TempDir`
- [ ] Update `test_ensure_dir_nested` to use `tempfile::TempDir`
- [ ] Remove manual cleanup code (no longer needed with TempDir)

### Section 2: Audit for Similar Issues
- [ ] Check `test_ensure_dir_already_exists` - uses different path suffix, but could be improved
- [ ] Scan for other tests in `fugue-utils` using `std::process::id()` pattern
- [ ] Scan for other tests in the workspace using similar patterns

### Section 3: Verification
- [ ] Run `cargo test -p fugue-utils` multiple times to verify fix
- [ ] Run `cargo test --workspace` multiple times (at least 10 runs)
- [ ] Optionally run stress test: `for i in {1..50}; do cargo test --workspace 2>&1 | grep FAILED; done`

## Acceptance Criteria

- [ ] `test_ensure_dir_nested` passes consistently in parallel test runs
- [ ] `test_ensure_dir_creates_directory` passes consistently in parallel test runs
- [ ] No tests use the `fugue_test_{pid}` pattern anymore
- [ ] All ensure_dir tests use `tempfile::TempDir` for isolation
- [ ] `cargo test --workspace` passes 50 consecutive runs without flakiness

## How to Verify the Fix

```bash
# Quick verification (should pass 100% of runs)
for i in {1..10}; do
    echo "Run $i..."
    cargo test --workspace -q 2>&1 | tail -1
done

# Stress test (for confidence)
for i in {1..50}; do
    cargo test --workspace 2>&1 | grep -E "(FAILED|passed)" | head -1
done | sort | uniq -c
# Expected output: "50 ... passed" with no FAILED lines
```

## Notes

- P2 priority - doesn't block functionality, just causes intermittent CI failures
- `tempfile` crate is already available as a dev-dependency
- The fix simplifies the test code by removing manual cleanup
- `TempDir` automatically cleans up when it goes out of scope (RAII pattern)
- Consider adding a comment explaining why TempDir is used for future maintainers
