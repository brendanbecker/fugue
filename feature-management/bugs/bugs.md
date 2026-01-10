# Bug Reports

**Project**: ccmux
**Last Updated**: 2026-01-10

## Summary Statistics
- Total Bugs: 10
- New: 4
- In Progress: 0
- Resolved: 5

## Bugs by Priority

### P0 - Critical (0)

*No open P0 bugs*

### P1 - High Priority (4)

#### BUG-010: MCP pane creation broadcast not received by TUI [NEW]

**Status**: New
**Filed**: 2026-01-09
**Component**: ccmux-server / ccmux-client
**Directory**: [BUG-010-mcp-pane-broadcast-not-received](BUG-010-mcp-pane-broadcast-not-received/)

**Description**:
When panes are created via MCP tools (e.g., `ccmux_create_pane`), the TUI client does not receive the `PaneCreated` broadcast. The pane exists on the server but the TUI is unaware of it - no split is rendered, and `Ctrl+B o` cannot switch to the new pane.

**Symptoms**:
- MCP `ccmux_create_pane` returns success
- Server shows 2 panes (via `ccmux_list_panes`)
- TUI shows only 1 pane (no split)
- New pane has default 80x24 dimensions (not resized)

**Suspected Root Cause**:
FEAT-039 implemented `ResponseWithBroadcast` but the broadcast is not reaching TUI clients. Possible issues: session ID mismatch, client not registered in session_clients, or channel delivery failure.

**Impact**:
MCP-based pane splitting is broken. Claude cannot effectively split panes via MCP tools.

#### BUG-004: Client hangs when reattaching to session with dead pane [RESOLVED]

**Status**: Resolved
**Filed**: 2026-01-09
**Resolved**: 2026-01-09
**Component**: ccmux-server

**Description**:
Client becomes unresponsive when attaching to a session whose pane's shell has exited. The session/pane remained in server state as zombies after the PTY output poller exited.

**Root Cause**:
PTY output poller only broadcast `PaneClosed` to clients but did not clean up pane/session from server state, leaving zombie sessions that could be attached to but had no active output poller.

**Resolution**:
Added automatic cleanup when PTY processes exit:
- New `PaneClosedNotification` channel from output pollers
- `run_pane_cleanup_loop()` task removes dead panes, empty windows, empty sessions
- All new panes get cleanup channel via `HandlerContext`

#### BUG-005: Sideband parsing not integrated into PTY output flow [RESOLVED]

**Status**: Resolved
**Filed**: 2026-01-09
**Resolved**: 2026-01-09
**Component**: ccmux-server
**Directory**: [BUG-005-sideband-parsing-not-integrated](BUG-005-sideband-parsing-not-integrated/)

**Description**:
Sideband commands (`<ccmux:spawn>`, `<ccmux:focus>`, etc.) output by Claude are displayed as literal text instead of being parsed and executed. The sideband parsing infrastructure (FEAT-019, FEAT-030) exists but is not wired into the PTY output flow.

**Root Cause**:
- FEAT-019 implemented `SidebandParser` and command types
- FEAT-030 implemented `CommandExecutor` with spawn functionality
- The integration point - wiring parser/executor into `PtyOutputPoller::flush()` - was never completed
- `SidebandParser` and `CommandExecutor` are only instantiated in test code

**Resolution**:
Integrated `SidebandParser` into `PtyOutputPoller` to filter output and execute commands before broadcasting to clients.

#### BUG-006: Viewport not sizing to terminal dimensions [RESOLVED]

**Status**: Resolved
**Filed**: 2026-01-09
**Resolved**: 2026-01-09
**Component**: ccmux-client

**Description**:
The ccmux viewport does not size itself to match the actual terminal dimensions. When ccmux is started in a full-screen terminal, the viewport renders at approximately quarter-screen size instead of filling the available space.

**Root Cause**:
Chicken-and-egg problem: Server creates panes at 80x24 default, client used server's dimensions instead of its own terminal size when creating UI panes, and no resize was sent on attach.

**Resolution**:
Modified `ccmux-client/src/ui/app.rs` to use client's terminal size when creating UI panes on attach, and send resize messages to server for all panes immediately after attach.

#### BUG-007: Shift+Tab not passed through to PTY [RESOLVED]

**Status**: Resolved
**Filed**: 2026-01-09
**Resolved**: 2026-01-09
**Component**: ccmux-client

**Description**:
Shift+Tab keystrokes are silently dropped instead of being sent to the PTY. Programs like Claude Code that use Shift+Tab don't receive the keystroke.

**Root Cause**:
`ccmux-client/src/input/keys.rs` has no match arm for `KeyCode::BackTab`. Crossterm sends `KeyCode::BackTab` for Shift+Tab (not `KeyCode::Tab` with SHIFT modifier), so it falls through to `_ => None` and is dropped.

**Resolution**:
Added `KeyCode::BackTab => Some(b"\x1b[Z".to_vec())` to `keys.rs`.

### P2 - Medium Priority (3)

#### BUG-012: Text selection not working in TUI [NEW]

**Status**: New
**Filed**: 2026-01-10
**Component**: ccmux-client
**Directory**: [BUG-012-text-selection-not-working](BUG-012-text-selection-not-working/)

**Description**:
Text selection does not work in the ccmux TUI client. When attempting to click and drag to select text, nothing happens - there is no selection, no highlight, and no ability to copy text. This is not just a visual issue; selection is completely non-functional.

**Symptoms**:
- Click and drag does not select text
- No text can be copied from the terminal output
- Selection is completely non-functional, not just invisible

**Suspected Root Cause**:
Multiple potential causes to investigate:
1. Mouse selection events not being captured or handled
2. No selection state management implemented
3. Mouse events being consumed for other purposes (scroll) but not selection
4. Copy mode (Prefix+[) not implemented or not working
5. crossterm mouse capture may be intercepting selection before terminal can handle it

**Impact**:
Poor user experience - users cannot copy text from ccmux terminal output. This breaks a fundamental terminal workflow.

#### BUG-011: Large paste input crashes ccmux session [NEW]

**Status**: New
**Filed**: 2026-01-10
**Component**: ccmux-client / ccmux-server
**Directory**: [BUG-011-large-paste-crashes-session](BUG-011-large-paste-crashes-session/)

**Description**:
Pasting an extremely large amount of text into a ccmux terminal session causes the session to crash. There is no graceful handling or error message - the session simply dies.

**Symptoms**:
- Session crash on large paste
- No graceful handling or error message
- Requires session reattachment after crash

**Suspected Root Cause**:
Multiple potential causes to investigate:
1. Buffer overflow in input handling path
2. Message size limit exceeded on Unix socket protocol
3. PTY write buffer overwhelmed (no chunking)
4. Bincode serialization failing on huge payloads
5. Memory exhaustion from allocating large input buffer

**Impact**:
Bad user experience when accidentally pasting large content. Session loss requires reattachment and may lose unsaved work.

#### BUG-009: Flaky persistence/recovery tests due to test isolation issues [NEW]

**Status**: New
**Filed**: 2026-01-09
**Component**: ccmux-server
**Directory**: [BUG-009-flaky-persistence-tests](BUG-009-flaky-persistence-tests/)

**Description**:
The persistence/recovery tests have intermittent race conditions. A different test fails on each run - it's not one specific test but rather test isolation issues affecting the entire persistence test suite. Tests pass when run individually but fail ~30% of parallel runs.

**Affected Tests**:
- `persistence::recovery::tests::test_recovery_from_wal`
- `persistence::recovery::tests::test_recovery_active_window_pane`
- `persistence::recovery::tests::test_recovery_pane_updates`
- `persistence::tests::test_persistence_log_operations`

**Suspected Root Cause**:
Same pattern as BUG-002 - tests likely share temp directories or file handles. Requires deep investigation of test isolation patterns.

**Impact**:
CI/test noise makes it difficult to verify if new features are working correctly. Has been plaguing the project for multiple sessions.

#### BUG-002: Flaky test `test_ensure_dir_nested` due to shared temp directory [RESOLVED]

**Status**: Resolved
**Filed**: 2026-01-09
**Resolved**: 2026-01-09
**Component**: ccmux-utils
**File**: `ccmux-utils/src/paths.rs:413`

**Description**:
The test `test_ensure_dir_nested` intermittently fails when running the full test suite in parallel, but passes when run in isolation.

**Root Cause**:
Two tests share the same base directory path using `std::process::id()`:
- `test_ensure_dir_creates_directory` uses `ccmux_test_{pid}/`
- `test_ensure_dir_nested` uses `ccmux_test_{pid}/nested/deep`

When tests run in parallel, one test may delete the shared base directory while the other test is attempting to use it, causing a race condition.

**Error Message**:
```
thread 'paths::tests::test_ensure_dir_nested' panicked at ccmux-utils/src/paths.rs:428:9:
assertion failed: result.is_ok()
```

**Steps to Reproduce**:
1. Run `cargo test --workspace`
2. Test may fail intermittently (not always reproducible)
3. Running `cargo test -p ccmux-utils test_ensure_dir_nested` passes consistently

**Resolution**:
Used `tempfile::TempDir` for test isolation in ensure_dir tests.

### P3 - Low Priority (0)

*No P3 bugs*

## Recent Activity

| Date | Bug ID | Action | Description |
|------|--------|--------|-------------|
| 2026-01-10 | BUG-012 | Filed | Text selection not working in TUI |
| 2026-01-10 | BUG-011 | Filed | Large paste input crashes ccmux session |
| 2026-01-09 | BUG-010 | Filed | MCP pane broadcast not received by TUI |
| 2026-01-09 | BUG-009 | Filed | Flaky persistence tests due to test isolation issues |
| 2026-01-09 | BUG-005 | Resolved | Integrated sideband parsing into PTY output flow |
| 2026-01-09 | BUG-007 | Resolved | Added KeyCode::BackTab handler |
| 2026-01-09 | BUG-007 | Filed | Shift+Tab not passed through (missing BackTab case) |
| 2026-01-09 | BUG-006 | Resolved | Client now uses terminal size on attach |
| 2026-01-09 | BUG-006 | Filed | Viewport not sizing to terminal dimensions |
| 2026-01-09 | BUG-005 | Filed | Sideband parsing not integrated into output flow |
| 2026-01-09 | BUG-004 | Filed & Resolved | Zombie panes causing client hang |
| 2026-01-09 | BUG-002 | Filed | Flaky test due to shared temp directory |
