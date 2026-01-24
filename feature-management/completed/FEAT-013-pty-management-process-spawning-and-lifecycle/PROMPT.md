# FEAT-013: PTY Management - Process Spawning and Lifecycle

**Priority**: P1
**Component**: fugue-server
**Type**: new_feature
**Estimated Effort**: medium
**Business Value**: high
**Status**: completed

## Overview

PTY spawning via portable-pty, process lifecycle management (spawn/kill/wait), resize support, and async I/O.

## Requirements

- PTY spawning using portable-pty crate
- Process lifecycle management (spawn, kill, wait for exit)
- PTY resize support (SIGWINCH)
- Async I/O for PTY read/write
- Environment variable setup (TERM, FUGUE_* vars)
- Working directory configuration
- Shell detection and configuration

## Affected Files

- `fugue-server/src/pty/handle.rs`
- `fugue-server/src/pty/config.rs`
- `fugue-server/src/pty/mod.rs`

## Implementation Tasks

### Section 1: Design
- [x] Review portable-pty crate API
- [x] Design PTY handle abstraction
- [x] Design process lifecycle state machine
- [x] Plan async I/O integration

### Section 2: PTY Spawning
- [x] Implement PTY spawning with portable-pty
- [x] Configure shell detection (SHELL env, /etc/passwd, fallback)
- [x] Set up environment variables (TERM, FUGUE_PANE_ID, etc.)
- [x] Configure working directory
- [x] Handle spawn errors gracefully

### Section 3: Process Lifecycle
- [x] Implement process spawn
- [x] Implement process kill (graceful SIGTERM, then SIGKILL)
- [x] Implement wait for exit with exit status capture
- [x] Handle zombie process cleanup
- [x] Implement lifecycle state tracking

### Section 4: PTY Resize
- [x] Implement resize support (SIGWINCH propagation)
- [x] Handle resize during active I/O
- [x] Validate resize dimensions

### Section 5: Async I/O
- [x] Implement async PTY read
- [x] Implement async PTY write
- [x] Handle I/O errors and EOF
- [x] Integrate with tokio runtime

### Section 6: Testing
- [x] Unit tests for PTY config
- [x] Integration tests for spawn/kill/wait
- [x] Test resize functionality
- [x] Test async I/O under load

## Acceptance Criteria

- [x] PTY can be spawned with configurable shell
- [x] Process lifecycle is properly managed
- [x] PTY resize works correctly
- [x] Async I/O is non-blocking
- [x] Environment variables are properly set
- [x] Working directory is configurable
- [x] All tests passing

## Dependencies

None - this is a foundational PTY management feature.

## Notes

- portable-pty provides cross-platform PTY support
- Consider platform-specific behavior differences (Linux vs macOS)
- PTY I/O should integrate with the session event loop
