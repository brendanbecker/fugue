# Implementation Plan: FEAT-008

**Work Item**: [FEAT-008: Utilities - Error Types, Logging, and Path Helpers](PROMPT.md)
**Component**: ccmux-utils
**Priority**: P1
**Status**: completed
**Created**: 2026-01-08

## Overview

Implement foundational utilities that all ccmux crates depend on. This includes a unified error type, structured logging with tracing, and XDG-compliant path utilities.

## Architecture Decisions

### 1. Centralized Error Type

**Decision**: Define a single `CcmuxError` enum in ccmux-utils that all crates use.

**Rationale**:
- Consistent error handling across the project
- Single place to add error context and formatting
- Enables unified error reporting and logging
- Simplifies API boundaries between crates

**Trade-offs**:
- All crates depend on ccmux-utils
- Error enum may grow large over time
- Some variant-specific logic may be less ergonomic

### 2. Tracing for Logging

**Decision**: Use the `tracing` crate ecosystem instead of `log`.

**Rationale**:
- Structured logging with spans and fields
- Better async support with context propagation
- Rich ecosystem (tracing-subscriber, tracing-appender)
- Can output to multiple formats (text, JSON)

**Trade-offs**:
- Slightly more complex API than log
- Larger dependency footprint
- Learning curve for span-based tracing

### 3. XDG Compliance

**Decision**: Follow XDG Base Directory Specification on Linux, with sensible defaults on other platforms.

**Rationale**:
- Standard on Linux, respected by power users
- Keeps user home directory clean
- Separates config, state, and runtime data
- Easy to backup config vs state

**Paths**:
| Purpose | Linux | macOS | Windows |
|---------|-------|-------|---------|
| Config | `~/.config/ccmux` | `~/Library/Application Support/ccmux` | `%APPDATA%\ccmux` |
| State | `~/.local/state/ccmux` | `~/Library/Application Support/ccmux` | `%LOCALAPPDATA%\ccmux` |
| Runtime | `$XDG_RUNTIME_DIR/ccmux` | `/tmp/ccmux-$UID` | `%TEMP%\ccmux` |
| Logs | `~/.local/state/ccmux/logs` | `~/Library/Logs/ccmux` | `%LOCALAPPDATA%\ccmux\logs` |

### 4. Environment Variable Override

**Decision**: CCMUX_LOG environment variable takes precedence over all other log configuration.

**Rationale**:
- Consistent with RUST_LOG convention
- Easy debugging without modifying config files
- Works across all invocation methods

## Affected Components

| Component | Type of Change | Risk Level |
|-----------|----------------|------------|
| `ccmux-utils/src/error.rs` | New file | Low |
| `ccmux-utils/src/logging.rs` | New file | Low |
| `ccmux-utils/src/paths.rs` | New file | Low |
| `ccmux-utils/src/lib.rs` | Module setup | Low |
| `ccmux-utils/Cargo.toml` | Dependencies | Low |

## Dependencies

This feature has no dependencies on other features. Other features depend on this:
- All crates use CcmuxError for error handling
- All crates use logging infrastructure
- Server and client use path utilities

## Implementation Phases

### Phase 1: Crate Setup and Error Types

1. Create/update `ccmux-utils/Cargo.toml`:
   ```toml
   [dependencies]
   thiserror = "1.0"
   tracing = "0.1"
   tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }
   dirs-next = "2.0"
   ```

2. Implement `error.rs`:
   - Define CcmuxError enum
   - Implement From traits
   - Define Result type alias

### Phase 2: Logging Infrastructure

1. Implement `logging.rs`:
   - Define LogConfig and LogOutput
   - Implement init_logging()
   - Support CCMUX_LOG override
   - Optional JSON formatting

2. Test logging:
   - Verify filter parsing
   - Verify output destinations
   - Verify environment override

### Phase 3: Path Utilities

1. Implement `paths.rs`:
   - config_dir()
   - state_dir()
   - runtime_dir()
   - log_dir()
   - ensure_dir() helper

2. Test paths:
   - Verify XDG compliance on Linux
   - Verify fallbacks work
   - Verify directory creation

### Phase 4: Integration and Documentation

1. Export all public API from lib.rs
2. Add documentation for all public items
3. Add usage examples in doc comments
4. Integration tests for common workflows

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Path incompatibility on Windows | Low | Medium | Use dirs-next, test on Windows |
| Logging initialization failures | Low | High | Graceful fallback to stderr |
| Error enum growing unwieldy | Medium | Low | Use variants with String payloads |

## Rollback Strategy

If implementation causes issues:

1. Revert commits associated with FEAT-008
2. Other crates can temporarily define their own error types
3. Document what went wrong in comments.md

## Testing Strategy

### Unit Tests
- Error conversion tests (From impls)
- Path function return values
- LogConfig parsing

### Integration Tests
- Logging initialization with various configs
- CCMUX_LOG override behavior
- Directory creation on first access

### Manual Testing
- Verify paths on Linux and macOS
- Verify logging output format
- Verify error messages are clear

## Implementation Notes

Implementation completed. All modules implemented with:
- CcmuxError enum with thiserror for io, config, session, protocol, and pty errors
- Logging infrastructure using tracing with CCMUX_LOG environment override
- XDG-compliant path utilities for config, state, runtime, and log directories

---
*This plan should be updated as implementation progresses.*
