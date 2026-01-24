# Task Breakdown: BUG-005

**Work Item**: [BUG-005: Sideband Parsing Not Integrated into PTY Output Flow](PROMPT.md)
**Status**: Not Started
**Last Updated**: 2026-01-09

## Prerequisites

- [ ] Read and understand PROMPT.md
- [ ] Review PLAN.md and update if needed
- [ ] Review `fugue-server/src/pty/output.rs` (PtyOutputPoller)
- [ ] Review `fugue-server/src/sideband/mod.rs` (SidebandParser, CommandExecutor)
- [ ] Review `fugue-server/src/main.rs` (SharedState, server startup)
- [ ] Understand existing output poller flow (spawn, handle_output, flush)

## Phase 1: Add CommandExecutor to SharedState

### Create Executor at Startup

- [ ] In `fugue-server/src/main.rs`, add import: `use sideband::CommandExecutor;`
- [ ] Create `CommandExecutor` in `run_daemon()` after managers are created
- [ ] Need to convert managers to `Arc<Mutex<_>>` for CommandExecutor (it uses parking_lot::Mutex)
  - Note: Server uses `Arc<RwLock<_>>` but CommandExecutor uses `Arc<parking_lot::Mutex<_>>`
  - May need to create wrapper or modify CommandExecutor to use RwLock
- [ ] Add `command_executor: Arc<CommandExecutor>` field to `SharedState`
- [ ] Initialize in `SharedState` construction

### Verify Compilation

- [ ] Run `cargo check -p fugue-server`
- [ ] Fix any type mismatches between RwLock and parking_lot::Mutex

## Phase 2: Extend PtyOutputPoller Struct

### Add New Fields

- [ ] In `fugue-server/src/pty/output.rs`, add import for sideband types
- [ ] Add field: `sideband_parser: SidebandParser`
- [ ] Add field: `command_executor: Arc<CommandExecutor>`
- [ ] Add field: `poller_manager: Option<Arc<parking_lot::Mutex<PollerManager>>>`

### Create New Constructor

- [ ] Create `spawn_with_sideband()` method with full parameter set
- [ ] Initialize `SidebandParser::new()` in constructor
- [ ] Store executor reference
- [ ] Store optional poller_manager reference

### Update Existing Constructors

- [ ] Update `spawn()` to call `spawn_with_sideband()` with None for optional params
- [ ] Update `spawn_with_cleanup()` to call `spawn_with_sideband()`
- [ ] Update `spawn_with_config()` to call `spawn_with_sideband()`
- [ ] Ensure backward compatibility (existing callers unchanged)

### Verify Compilation

- [ ] Run `cargo check -p fugue-server`
- [ ] All existing tests should still compile

## Phase 3: Implement Sideband Parsing

### Modify handle_output()

- [ ] Convert incoming `data: &[u8]` to string using `String::from_utf8_lossy()`
- [ ] Call `self.sideband_parser.parse(&text)` to get `(display_text, commands)`
- [ ] If commands is empty and display_text equals input, use original bytes
- [ ] Otherwise, use `display_text.as_bytes()` for buffer
- [ ] Iterate through commands and call executor

### Execute Commands

- [ ] Create helper method `execute_command(&mut self, cmd: SidebandCommand)`
- [ ] Match on command type and call appropriate executor method
- [ ] Handle `SidebandCommand::Spawn` specially (needs poller registration)
- [ ] For non-spawn commands, call `self.command_executor.execute(cmd, self.pane_id)`
- [ ] Log warnings on execution failures

### Handle Spawn Commands

- [ ] When spawn command is detected, call `command_executor.execute_spawn_command()`
- [ ] On success, extract `SpawnResult`
- [ ] If `poller_manager` is Some, register new poller for spawned pane
- [ ] Use `poller_manager.lock().start_with_sideband()` (need to implement this)
- [ ] Handle case where poller_manager is None (log warning)

### Verify Compilation

- [ ] Run `cargo check -p fugue-server`
- [ ] Run `cargo test -p fugue-server` (may fail, tests need updating)

## Phase 4: Update PollerManager

### Add Sideband-Aware Start Method

- [ ] Add `start_with_sideband()` method to `PollerManager`
- [ ] Accepts additional `command_executor: Arc<CommandExecutor>` parameter
- [ ] Calls `PtyOutputPoller::spawn_with_sideband()` internally
- [ ] Store self-reference for poller_manager (for spawn cascading)

### Self-Reference Pattern

- [ ] PollerManager needs to pass `Arc<Mutex<Self>>` to new pollers
- [ ] Consider: make PollerManager hold its own Arc reference?
- [ ] Alternative: pass poller_manager as parameter to start method

### Verify Compilation

- [ ] Run `cargo check -p fugue-server`

## Phase 5: Wire Up in Server

### Update HandlerContext

- [ ] Add `command_executor: Arc<CommandExecutor>` to `HandlerContext`
- [ ] Pass from SharedState when creating HandlerContext in handle_client()
- [ ] Update `HandlerContext::new()` signature

### Update Session Handler

- [ ] In `handle_create_session()`, use new poller constructor
- [ ] Pass command_executor to `PtyOutputPoller::spawn_with_cleanup()` (or new variant)
- [ ] Ensure pane_closed_tx is also passed

### Update Pane Handler

- [ ] Check if `handle_split_pane()` exists or similar
- [ ] Any pane creation path needs executor reference

### Update Startup Poller Creation

- [ ] In `run_daemon()`, where restored panes get pollers, use sideband-aware constructor
- [ ] Pass command_executor from SharedState

### Create PollerManager with Self-Reference

- [ ] Determine where PollerManager is created/stored
- [ ] If needed, create as `Arc<Mutex<PollerManager>>`
- [ ] Pass reference to new pollers for spawn cascading

### Verify Compilation

- [ ] Run `cargo check -p fugue-server`

## Phase 6: Testing

### Unit Tests (output.rs)

- [ ] Update existing tests if constructor signatures changed
- [ ] Add test: `test_sideband_command_parsed`
- [ ] Add test: `test_sideband_command_stripped_from_output`
- [ ] Add test: `test_non_command_data_unchanged`
- [ ] Add test: `test_partial_command_buffered`

### Integration Tests

- [ ] Create `tests/sideband_integration.rs` or add to existing
- [ ] Test: spawn command creates pane in session
- [ ] Test: notify command logs message
- [ ] Test: input command routes to target pane
- [ ] Test: multiple commands in single output chunk
- [ ] Test: command split across PTY reads

### Manual Testing

- [ ] Start server, create session
- [ ] Run: `echo '<fugue:spawn direction="vertical" />'`
- [ ] Verify: new pane appears
- [ ] Verify: XML tag NOT in terminal display
- [ ] Run: `echo '<fugue:notify title="Test">Hello</fugue:notify>'`
- [ ] Verify: server logs show notification

### Regression Testing

- [ ] Run full test suite: `cargo test --workspace`
- [ ] Verify all 135+ tests pass
- [ ] Check for any performance issues (should be negligible)

## Phase 7: Cleanup and Documentation

### Update Module Documentation

- [ ] Update `fugue-server/src/sideband/mod.rs` docs to reflect integration
- [ ] Add note about how parser/executor are wired in

### Remove TODO Comments

- [ ] Search for any TODO comments about sideband integration
- [ ] Remove or update as appropriate

### Add Logging

- [ ] Info log when sideband command parsed
- [ ] Debug log with command details
- [ ] Warn log on execution failure with error details

### Update Feature Management

- [ ] Update FEAT-019 status if needed
- [ ] Update FEAT-030 status if needed
- [ ] Mark BUG-005 as resolved when complete

## Verification Tasks

- [ ] All acceptance criteria from PROMPT.md met
- [ ] `<fugue:spawn>` creates pane
- [ ] `<fugue:notify>` logs message
- [ ] `<fugue:input>` routes to pane
- [ ] Commands stripped from display
- [ ] Non-command output unchanged
- [ ] All existing tests pass
- [ ] New integration tests pass
- [ ] Manual testing successful

## Completion Checklist

- [ ] All implementation tasks complete
- [ ] All tests passing
- [ ] Code self-reviewed
- [ ] PLAN.md updated with implementation notes
- [ ] bug_report.json status updated to "resolved"
- [ ] bugs.md updated with resolution

---
*Check off tasks as you complete them. Update status field above.*
