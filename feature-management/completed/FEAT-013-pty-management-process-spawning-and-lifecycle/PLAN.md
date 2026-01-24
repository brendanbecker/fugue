# Implementation Plan: FEAT-013

**Work Item**: [FEAT-013: PTY Management - Process Spawning and Lifecycle](PROMPT.md)
**Component**: fugue-server
**Priority**: P1
**Created**: 2026-01-08
**Status**: Completed

## Overview

PTY spawning via portable-pty, process lifecycle management (spawn/kill/wait), resize support, and async I/O.

## Architecture Decisions

### PTY Handle Abstraction

The `PtyHandle` struct wraps portable-pty's `PtyPair` and `Child`:

```rust
pub struct PtyHandle {
    master: Box<dyn MasterPty + Send>,
    child: Box<dyn Child + Send + Sync>,
    config: PtyConfig,
    state: PtyState,
}
```

### Process Lifecycle States

```
Spawning -> Running -> Terminating -> Exited
                  \-> Failed
```

### Shell Detection Order

1. Explicit config value
2. SHELL environment variable
3. /etc/passwd lookup (Unix)
4. Fallback to /bin/sh (Unix) or cmd.exe (Windows)

### Environment Variables

Standard variables set for spawned PTY:
- `TERM=xterm-256color`
- `FUGUE_PANE_ID={pane_id}`
- `FUGUE_SESSION_ID={session_id}`
- `FUGUE_VERSION={version}`

## Affected Components

| Component | Type of Change | Risk Level |
|-----------|----------------|------------|
| fugue-server/src/pty/handle.rs | New - PTY handle implementation | Medium |
| fugue-server/src/pty/config.rs | New - PTY configuration | Low |
| fugue-server/src/pty/mod.rs | New - Module exports | Low |

## Dependencies

- `portable-pty` crate for cross-platform PTY support
- `tokio` for async runtime integration

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Platform differences | Medium | Medium | Use portable-pty abstractions, test on all platforms |
| Zombie processes | Low | High | Proper wait() handling, signal management |
| I/O blocking | Medium | Medium | Async I/O with proper timeout handling |
| Resource leaks | Low | Medium | RAII patterns, Drop implementations |

## Implementation Phases

### Phase 1: Core PTY Infrastructure (Completed)
- PtyConfig struct with shell, env, cwd settings
- PtyHandle struct with spawn implementation
- Basic lifecycle management

### Phase 2: Process Lifecycle (Completed)
- Spawn with proper error handling
- Kill with graceful shutdown (SIGTERM -> SIGKILL)
- Wait with exit status capture

### Phase 3: Resize Support (Completed)
- Resize API on PtyHandle
- SIGWINCH propagation
- Dimension validation

### Phase 4: Async I/O Integration (Completed)
- Async read/write methods
- EOF and error handling
- Tokio integration

## Rollback Strategy

If implementation causes issues:
1. Revert commits associated with this work item
2. Remove pty module from fugue-server
3. Document what went wrong in comments.md

## Testing Strategy

1. **Unit Tests**: PtyConfig validation, state transitions
2. **Integration Tests**: Full spawn/interact/kill cycle
3. **Platform Tests**: Verify behavior on Linux/macOS
4. **Stress Tests**: Multiple concurrent PTYs

## Implementation Notes

Implementation completed. Key decisions made:
- Used portable-pty for cross-platform compatibility
- Implemented async I/O via tokio::io traits
- Added comprehensive error handling for spawn failures
- Resize validates dimensions before applying

---
*Implementation completed 2026-01-08.*
