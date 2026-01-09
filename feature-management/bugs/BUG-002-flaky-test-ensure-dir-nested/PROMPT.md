# BUG-002: Flaky test `test_ensure_dir_nested` due to shared temp directory

**Type**: Bug
**Priority**: P2
**Status**: open
**Created**: 2026-01-09
**Component**: ccmux-utils

## Description

The test `test_ensure_dir_nested` intermittently fails when running the full test suite in parallel, but passes when run in isolation.

## Root Cause

Two tests share the same base directory path using `std::process::id()`:
- `test_ensure_dir_creates_directory` uses `ccmux_test_{pid}/`
- `test_ensure_dir_nested` uses `ccmux_test_{pid}/nested/deep`

When tests run in parallel, one test may delete the shared base directory while the other test is attempting to use it, causing a race condition.

## Reproduction Steps

1. Run `cargo test --workspace`
2. Test may fail intermittently (not always reproducible)
3. Running `cargo test -p ccmux-utils test_ensure_dir_nested` passes consistently

## Error Message

```
thread 'paths::tests::test_ensure_dir_nested' panicked at ccmux-utils/src/paths.rs:428:9:
assertion failed: result.is_ok()
```

## Fix Location

`ccmux-utils/src/paths.rs:413`

## Suggested Fix

Use `tempfile::TempDir` for test isolation, or use unique directory names for each test (e.g., include test function name in the path).

```rust
fn test_ensure_dir_nested() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let test_dir = temp_dir.path().join("nested").join("deep");
    // ... rest of test
}
```

## Notes

- P2 priority - doesn't block functionality, just causes intermittent CI failures
- May affect other tests using similar patterns
