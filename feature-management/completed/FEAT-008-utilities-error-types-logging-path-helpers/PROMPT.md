# FEAT-008: Utilities - Error Types, Logging, and Path Helpers

**Priority**: P1
**Component**: fugue-utils
**Type**: new_feature
**Estimated Effort**: small
**Business Value**: high
**Status**: completed

## Overview

Common utilities including CcmuxError enum, logging infrastructure with tracing, and XDG-compliant path utilities for config/state/runtime directories.

This feature provides foundational infrastructure that all other fugue crates depend on for consistent error handling, structured logging, and platform-appropriate file path management.

## Technical Design

### CcmuxError Enum

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CcmuxError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Session error: {0}")]
    Session(String),

    #[error("Protocol error: {0}")]
    Protocol(String),

    #[error("PTY error: {0}")]
    Pty(String),
}

pub type Result<T> = std::result::Result<T, CcmuxError>;
```

### Logging Infrastructure

```rust
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

pub struct LogConfig {
    /// Filter string (e.g., "fugue=debug,warn")
    pub filter: String,
    /// Output mode: stderr, file, or both
    pub output: LogOutput,
}

pub enum LogOutput {
    Stderr,
    File(PathBuf),
    Both(PathBuf),
}

pub fn init_logging(config: &LogConfig) -> Result<()>;
```

### XDG Path Utilities

```rust
/// Get config directory: $XDG_CONFIG_HOME/fugue or ~/.config/fugue
pub fn config_dir() -> PathBuf;

/// Get state directory: $XDG_STATE_HOME/fugue or ~/.local/state/fugue
pub fn state_dir() -> PathBuf;

/// Get runtime directory: $XDG_RUNTIME_DIR/fugue or /tmp/fugue-$UID
pub fn runtime_dir() -> PathBuf;

/// Get log directory: state_dir()/logs
pub fn log_dir() -> PathBuf;
```

## Requirements

1. **CcmuxError enum with thiserror derive**
   - Cover all major error categories: IO, Config, Session, Protocol, PTY
   - Implement From traits for common error types
   - Provide Result type alias for convenience

2. **Logging infrastructure using tracing crate**
   - Support configurable log filters
   - Support multiple output destinations (stderr, file)
   - JSON structured logging option for machine parsing

3. **LogConfig with filter strings and output modes**
   - Parse RUST_LOG-style filter strings
   - Support file rotation (optional, future)
   - Configure via config file or environment

4. **XDG-compliant path utilities**
   - Respect XDG_CONFIG_HOME, XDG_STATE_HOME, XDG_RUNTIME_DIR
   - Provide sensible defaults for each platform
   - Auto-create directories on first access

5. **FUGUE_LOG environment variable override**
   - Override config file log settings via environment
   - Consistent with RUST_LOG format
   - Takes precedence over all other log configuration

## Affected Files

| File | Type of Change |
|------|----------------|
| `fugue-utils/src/lib.rs` | Module exports and re-exports |
| `fugue-utils/src/error.rs` | CcmuxError enum definition |
| `fugue-utils/src/logging.rs` | Logging infrastructure |
| `fugue-utils/src/paths.rs` | XDG path utilities |
| `fugue-utils/Cargo.toml` | Dependencies (thiserror, tracing, dirs) |

## Implementation Tasks

### Section 1: Error Types
- [x] Create fugue-utils crate with Cargo.toml
- [x] Define CcmuxError enum with thiserror
- [x] Implement From traits for std::io::Error
- [x] Define Result<T> type alias
- [x] Add error documentation

### Section 2: Logging Infrastructure
- [x] Add tracing and tracing-subscriber dependencies
- [x] Define LogConfig struct
- [x] Define LogOutput enum
- [x] Implement init_logging function
- [x] Add FUGUE_LOG environment variable support
- [x] Add optional JSON formatting

### Section 3: Path Utilities
- [x] Add dirs-next dependency for XDG paths
- [x] Implement config_dir() function
- [x] Implement state_dir() function
- [x] Implement runtime_dir() function
- [x] Implement log_dir() function
- [x] Add auto-create option for directories

### Section 4: Testing
- [x] Unit tests for error type conversions
- [x] Unit tests for path functions
- [x] Integration tests for logging initialization
- [x] Test FUGUE_LOG override behavior

### Section 5: Verification
- [x] All unit tests passing
- [x] Documentation complete
- [x] API is ergonomic and consistent

## Acceptance Criteria

- [x] CcmuxError enum covers all error categories
- [x] Error messages are clear and actionable
- [x] Logging can be configured via LogConfig
- [x] FUGUE_LOG environment variable overrides config
- [x] Path utilities return correct XDG paths
- [x] Path utilities respect environment variables
- [x] All tests passing
- [x] Documentation updated

## Notes

### Design Considerations

1. **Error Granularity**: Start with broad categories (Io, Config, Session, Protocol, Pty) and add specific variants as needed. Too many variants early can complicate API.

2. **Logging Performance**: Use tracing's lazy evaluation to avoid formatting overhead when logs are filtered out.

3. **Platform Compatibility**: Path utilities must work on Linux, macOS, and Windows (WSL). Use dirs-next for cross-platform XDG support.

4. **Runtime Directory**: On systems without XDG_RUNTIME_DIR (e.g., macOS), fall back to /tmp/fugue-$UID with appropriate permissions.

### Dependencies

- `thiserror` - Error derive macros
- `tracing` - Structured logging facade
- `tracing-subscriber` - Logging backend
- `dirs-next` - XDG directory lookup
