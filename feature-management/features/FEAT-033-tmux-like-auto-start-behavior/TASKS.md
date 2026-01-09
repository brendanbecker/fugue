# Task Breakdown: FEAT-033

**Work Item**: [FEAT-033: tmux-like Auto-Start Behavior](PROMPT.md)
**Status**: Not Started
**Last Updated**: 2026-01-09

## Prerequisites

- [ ] Read and understand PROMPT.md
- [ ] Review PLAN.md and update if needed
- [ ] Verify FEAT-011 (Client Connection) is complete
- [ ] Identify current connection code location in ccmux-client

## Phase 1: Core Logic

### 1.1 Server Binary Discovery
- [ ] Add `find_server_binary() -> Result<PathBuf>` function
- [ ] Check same directory as current executable first
- [ ] Fall back to PATH search (consider adding `which` crate or implement manually)
- [ ] Return clear error message if not found

### 1.2 Daemon Spawning
- [ ] Add `start_server_daemon() -> Result<()>` function
- [ ] Use `std::process::Command` with null stdio
- [ ] Call `find_server_binary()` to get path
- [ ] Handle spawn errors with helpful messages

### 1.3 Error Classification
- [ ] Add `is_server_not_running(error: &Error) -> bool` function
- [ ] Handle `ErrorKind::ConnectionRefused`
- [ ] Handle `ErrorKind::NotFound` (socket file missing)
- [ ] Handle any platform-specific error variants

## Phase 2: Retry Logic

### 2.1 Connection Retry
- [ ] Add `connect_with_retry(socket_path, timeout) -> Result<Connection>` function
- [ ] Implement retry loop with 200ms intervals
- [ ] Add initial 100ms delay for server startup
- [ ] Respect timeout parameter (default 2s)

### 2.2 Auto-Start Wrapper
- [ ] Add `connect_with_auto_start(socket_path, auto_start) -> Result<Connection>` function
- [ ] First attempt normal connection
- [ ] On server-not-running error, spawn daemon
- [ ] Call retry function after spawn
- [ ] Return appropriate errors for other failure modes

### 2.3 Error Messages
- [ ] Clear message when server binary not found
- [ ] Clear message when server fails to start
- [ ] Suggestion to check server logs on timeout
- [ ] Distinguish between "server not running" and other errors

## Phase 3: CLI Integration

### 3.1 CLI Flag
- [ ] Add `--no-auto-start` flag to argument parser (clap)
- [ ] Pass flag value to connection function
- [ ] Add flag to help text with description

### 3.2 Main Function Updates
- [ ] Replace direct `connect()` call with `connect_with_auto_start()`
- [ ] Pass auto_start flag based on CLI argument
- [ ] Ensure proper error propagation and display

## Phase 4: Configuration (Optional)

### 4.1 Config File Options
- [ ] Add `auto_start_server: bool` option (default: true)
- [ ] Add `server_start_timeout: u64` option (default: 2000ms)
- [ ] Add `connection_retries: u32` option (optional, derive from timeout)
- [ ] Load and apply config values

### 4.2 Config Priority
- [ ] CLI flag overrides config file
- [ ] Config file overrides defaults
- [ ] Document precedence in config file comments

## Phase 5: Testing

### 5.1 Unit Tests
- [ ] Test `find_server_binary()` with binary in same directory
- [ ] Test `find_server_binary()` with binary in PATH
- [ ] Test `find_server_binary()` when binary not found
- [ ] Test `is_server_not_running()` with various error types
- [ ] Test retry timing logic

### 5.2 Integration Tests
- [ ] Test full auto-start flow (server not running -> connect succeeds)
- [ ] Test connect when server already running (no spawn)
- [ ] Test --no-auto-start flag (fail fast when server not running)
- [ ] Test timeout behavior when server won't start

### 5.3 Manual Testing
- [ ] Test fresh start scenario
- [ ] Test with server already running
- [ ] Test multiple concurrent client starts
- [ ] Test on target platform (Linux)

## Phase 6: Documentation

### 6.1 Code Documentation
- [ ] Add doc comments to new functions
- [ ] Document error conditions
- [ ] Add usage examples in comments

### 6.2 User Documentation
- [ ] Document --no-auto-start flag
- [ ] Document config file options
- [ ] Update README or man page if applicable

## Completion Checklist

- [ ] All Phase 1 tasks complete (Core Logic)
- [ ] All Phase 2 tasks complete (Retry Logic)
- [ ] All Phase 3 tasks complete (CLI Integration)
- [ ] Phase 4 complete or deferred (Configuration)
- [ ] All Phase 5 tasks complete (Testing)
- [ ] All tests passing
- [ ] PLAN.md updated with final approach
- [ ] feature_request.json status updated
- [ ] Ready for review/merge

---
*Check off tasks as you complete them. Update status field above.*
