# Task Breakdown: FEAT-008

**Work Item**: [FEAT-008: Utilities - Error Types, Logging, and Path Helpers](PROMPT.md)
**Status**: Completed
**Last Updated**: 2026-01-08

## Prerequisites

- [x] Read and understand PROMPT.md
- [x] Review PLAN.md and update if needed
- [x] Review existing ccmux crate structure

## Phase 1: Crate Setup and Error Types

### Cargo.toml Setup
- [x] Create/update ccmux-utils/Cargo.toml
- [x] Add thiserror dependency
- [x] Add tracing dependencies
- [x] Add tracing-subscriber with env-filter and json features
- [x] Add dirs-next dependency

### Error Module (error.rs)
- [x] Create ccmux-utils/src/error.rs
- [x] Define CcmuxError enum with variants:
  - [x] Io(std::io::Error)
  - [x] Config(String)
  - [x] Session(String)
  - [x] Protocol(String)
  - [x] Pty(String)
- [x] Implement From<std::io::Error> for CcmuxError
- [x] Define Result<T> type alias
- [x] Add documentation for error types

### Verify Phase 1
- [x] ccmux-utils crate compiles
- [x] Error types are usable
- [x] cargo doc generates documentation

## Phase 2: Logging Infrastructure

### LogConfig Types (logging.rs)
- [x] Create ccmux-utils/src/logging.rs
- [x] Define LogOutput enum (Stderr, File, Both)
- [x] Define LogConfig struct with filter and output fields
- [x] Implement Default for LogConfig

### Initialization Function
- [x] Implement init_logging(config: &LogConfig) -> Result<()>
- [x] Parse filter string using EnvFilter
- [x] Configure stderr output
- [x] Configure file output (when applicable)
- [x] Handle CCMUX_LOG environment variable override
- [x] Add JSON output option

### Verify Phase 2
- [x] Logging initializes without error
- [x] Filter strings are respected
- [x] CCMUX_LOG overrides config
- [x] Output goes to correct destination

## Phase 3: Path Utilities

### XDG Path Functions (paths.rs)
- [x] Create ccmux-utils/src/paths.rs
- [x] Implement config_dir() -> PathBuf
- [x] Implement state_dir() -> PathBuf
- [x] Implement runtime_dir() -> PathBuf
- [x] Implement log_dir() -> PathBuf

### Directory Management
- [x] Implement ensure_dir(path: &Path) -> Result<()>
- [x] Create directories with appropriate permissions
- [x] Handle permission errors gracefully

### Verify Phase 3
- [x] Paths respect XDG environment variables
- [x] Fallback paths work when XDG vars not set
- [x] Directories are created when needed
- [x] Permissions are correct (especially runtime_dir)

## Phase 4: Module Integration

### lib.rs Setup
- [x] Create/update ccmux-utils/src/lib.rs
- [x] Declare error, logging, paths modules
- [x] Re-export public types at crate root
- [x] Add crate-level documentation

### API Ergonomics
- [x] Verify error types are easy to use
- [x] Verify logging initialization is simple
- [x] Verify path functions are intuitive
- [x] Add prelude module if helpful

### Verify Phase 4
- [x] Crate compiles with all modules
- [x] Public API is clean and documented
- [x] Examples in doc comments work

## Phase 5: Testing

### Unit Tests
- [x] Test CcmuxError From implementations
- [x] Test Result type alias usage
- [x] Test LogConfig default values
- [x] Test path functions with mocked environment

### Integration Tests
- [x] Test logging initialization end-to-end
- [x] Test CCMUX_LOG override
- [x] Test directory creation
- [x] Test path values on current platform

### Documentation Tests
- [x] All doc examples compile and pass
- [x] README has usage examples
- [x] API documentation is complete

### Verify Phase 5
- [x] All tests pass
- [x] Coverage is adequate
- [x] No warnings in test output

## Completion Checklist

- [x] All implementation tasks complete
- [x] All tests passing
- [x] PLAN.md updated with final approach
- [x] feature_request.json status updated to "completed"
- [x] Ready for review/merge

---
*Check off tasks as you complete them. Update status field above.*
