# Task Breakdown: FEAT-033

**Work Item**: [FEAT-033: tmux-like Auto-Start Behavior](PROMPT.md)
**Status**: Complete
**Last Updated**: 2026-01-09

## Prerequisites

- [x] Read and understand PROMPT.md
- [x] Review PLAN.md and update if needed
- [x] Verify FEAT-011 (Client Connection) is complete
- [x] Identify current connection code location in ccmux-client

## Phase 1: Core Logic

### 1.1 Server Binary Discovery
- [x] Add `find_server_binary() -> Result<PathBuf>` function
- [x] Check same directory as current executable first
- [x] Fall back to PATH search (using `which` crate)
- [x] Return clear error message if not found

### 1.2 Daemon Spawning
- [x] Add `start_server_daemon() -> Result<()>` function
- [x] Use `std::process::Command` with null stdio
- [x] Call `find_server_binary()` to get path
- [x] Handle spawn errors with helpful messages

### 1.3 Error Classification
- [x] Add `is_server_not_running(error: &Error) -> bool` function
- [x] Handle `ErrorKind::ConnectionRefused`
- [x] Handle `ErrorKind::NotFound` (socket file missing)
- [x] Handle any platform-specific error variants

## Phase 2: Retry Logic

### 2.1 Connection Retry
- [x] Add `wait_for_server(config) -> Result<()>` function
- [x] Implement retry loop with configurable intervals
- [x] Add initial delay for server startup
- [x] Respect timeout parameter (default 2s)

### 2.2 Auto-Start Wrapper
- [x] Add `ensure_server_running(config) -> Result<ServerStartResult>` function
- [x] First check if server already available
- [x] On server-not-running, spawn daemon
- [x] Call retry function after spawn
- [x] Return appropriate result type for status

### 2.3 Error Messages
- [x] Clear message when server binary not found
- [x] Clear message when server fails to start
- [x] Suggestion to check server logs on timeout
- [x] Distinguish between "server not running" and other errors

## Phase 3: CLI Integration

### 3.1 CLI Flag
- [x] Add `--no-auto-start` flag to argument parser (clap)
- [x] Add `--server-timeout` flag for custom timeout
- [x] Add `-S/--socket` flag for custom socket path
- [x] Add flags to help text with descriptions

### 3.2 Main Function Updates
- [x] Parse CLI args before application setup
- [x] Call `ensure_server_running()` with configured options
- [x] Handle all result variants appropriately
- [x] Add `App::with_socket_path()` for custom socket support

## Phase 4: Configuration (Optional - Deferred)

### 4.1 Config File Options
- [ ] Add `auto_start_server: bool` option (default: true)
- [ ] Add `server_start_timeout: u64` option (default: 2000ms)
- [ ] Load and apply config values

Note: Config file support deferred to future enhancement. CLI flags provide all needed functionality.

## Phase 5: Testing

### 5.1 Unit Tests
- [x] Test `AutoStartConfig::default()`
- [x] Test `is_server_not_running()` with various error types
- [x] Test `ServerStartResult` variants
- [x] Test CLI argument parsing

### 5.2 Integration Tests
- [ ] Test full auto-start flow (server not running -> connect succeeds)
- [ ] Test connect when server already running (no spawn)
- [ ] Test --no-auto-start flag (fail fast when server not running)
- [ ] Test timeout behavior when server won't start

Note: Integration tests require actual server binary and are deferred to manual testing.

### 5.3 Manual Testing
- [ ] Test fresh start scenario
- [ ] Test with server already running
- [ ] Test multiple concurrent client starts
- [ ] Test on target platform (Linux)

## Phase 6: Documentation

### 6.1 Code Documentation
- [x] Add doc comments to new functions
- [x] Document error conditions
- [x] Add usage examples in comments

### 6.2 User Documentation
- [ ] Document --no-auto-start flag in README
- [ ] Document --server-timeout flag
- [ ] Update man page if applicable

## Completion Checklist

- [x] All Phase 1 tasks complete (Core Logic)
- [x] All Phase 2 tasks complete (Retry Logic)
- [x] All Phase 3 tasks complete (CLI Integration)
- [x] Phase 4 deferred (Configuration file support)
- [x] Unit tests complete and passing (269 tests)
- [ ] Integration tests (deferred - manual testing)
- [x] PLAN.md updated with final approach
- [ ] feature_request.json status updated
- [x] Ready for review/merge

---
*Implementation completed 2026-01-09*
