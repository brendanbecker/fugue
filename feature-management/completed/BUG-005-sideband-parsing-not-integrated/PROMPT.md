# BUG-005: Sideband Parsing Not Integrated into PTY Output Flow

**Priority**: P1 (High)
**Component**: fugue-server
**Status**: implemented
**Created**: 2026-01-09
**Implemented**: 2026-01-09
**Discovered During**: Code Review / Feature Completion Verification

## Summary

Sideband commands (`<fugue:spawn>`, `<fugue:focus>`, `<fugue:input>`, etc.) output by Claude are displayed as literal XML text in the terminal instead of being parsed and executed. The sideband parsing infrastructure from FEAT-019 and FEAT-030 exists but was never wired into the PTY output flow.

## Reproduction Steps

1. Start fugue server: `./target/release/fugue-server`
2. Start fugue client: `./target/release/fugue`
3. Create and attach to a session
4. In the pane's shell, manually test with: `echo '<fugue:spawn direction="vertical" />'`
5. Observe the XML tag displayed as literal text in the terminal
6. No new pane is created

**Expected**: The `<fugue:spawn ...>` tag should be parsed, stripped from display, and a new pane created.
**Actual**: The XML tag is displayed verbatim and no pane is created.

## Root Cause Analysis

### The Missing Integration Point

The sideband system has three components:
1. **SidebandParser** (`fugue-server/src/sideband/parser.rs`) - Parses XML commands from text
2. **CommandExecutor** (`fugue-server/src/sideband/executor.rs`) - Executes parsed commands
3. **PtyOutputPoller** (`fugue-server/src/pty/output.rs`) - Reads PTY output and broadcasts to clients

The intended flow (documented in `sideband/mod.rs:36-42`):
```
PTY Output -> SidebandParser -> (display_text, commands)
                   |
                   +-> Commands -> CommandExecutor -> SessionManager
                   |
                   +-> Display Text -> vt100 Parser -> Client
```

**The actual flow**:
```
PTY Output -> PtyOutputPoller::flush() -> Broadcast raw bytes to clients
```

The parser and executor exist but are **never instantiated** in the server runtime:

### Evidence 1: PtyOutputPoller bypasses parsing

In `fugue-server/src/pty/output.rs`, the `flush()` method broadcasts raw data:

```rust
// Lines 374-403
async fn flush(&mut self) {
    if self.buffer.is_empty() {
        return;
    }

    let data = std::mem::take(&mut self.buffer);
    self.buffer = Vec::with_capacity(self.config.max_buffer_size);

    // ... logging ...

    let msg = ServerMessage::Output {
        pane_id: self.pane_id,
        data,  // <-- Raw bytes, no parsing!
    };

    let delivered = self.registry.broadcast_to_session(self.session_id, msg).await;
    // ...
}
```

### Evidence 2: SidebandParser only used in tests

Search for `SidebandParser::new()` - only appears in test code:

```
fugue-server/src/sideband/mod.rs:80      let mut parser = SidebandParser::new();  // In test fn
fugue-server/src/sideband/mod.rs:119     let mut parser = SidebandParser::new();  // In test fn
fugue-server/src/sideband/mod.rs:155     let mut parser = SidebandParser::new();  // In test fn
fugue-server/src/sideband/parser.rs:297  let mut parser = SidebandParser::new();  // In test fn
... (all test code)
```

### Evidence 3: CommandExecutor only used in tests

Search for `CommandExecutor::new()` - only appears in test code:

```
fugue-server/src/sideband/mod.rs:67      let executor = CommandExecutor::new(...);  // In test fn
fugue-server/src/sideband/executor.rs:500 (executor, manager)  // In test fn
```

### Evidence 4: No sideband integration in main.rs

`fugue-server/src/main.rs` declares `pub mod sideband;` but:
- No `use sideband::*;` imports for runtime use
- No instantiation of `SidebandParser` or `CommandExecutor`
- No integration with `PtyOutputPoller`

## Why This Happened

FEAT-019 (Sideband Protocol - XML Command Parsing) implemented the parser and command types.
FEAT-030 (Sideband Pane Splitting) implemented the executor with spawn functionality.

Both features were developed in isolation with excellent unit tests, but the final integration step - wiring them into `PtyOutputPoller` - was never completed. The features were marked as "merged" based on passing tests without verifying end-to-end functionality.

## Impact

- **Claude cannot control fugue**: The core value proposition of Claude-fugue integration is non-functional
- **No autonomous pane spawning**: `<fugue:spawn>` commands are ignored
- **No input routing**: `<fugue:input>` commands are ignored
- **No notifications**: `<fugue:notify>` commands are ignored
- **All sideband commands display as garbage**: Users see raw XML tags in terminal output

## Implementation Plan

### Option A: Integrate into PtyOutputPoller (Recommended)

Modify `PtyOutputPoller` to use `SidebandParser` and `CommandExecutor`:

1. **Add parser and executor to PtyOutputPoller struct**:
   ```rust
   pub struct PtyOutputPoller {
       // ... existing fields ...
       /// Sideband parser for this pane
       sideband_parser: SidebandParser,
       /// Command executor reference
       command_executor: Arc<CommandExecutor>,
   }
   ```

2. **Modify handle_output() to parse**:
   ```rust
   async fn handle_output(&mut self, data: &[u8]) {
       // Convert bytes to string (handle partial UTF-8)
       let text = String::from_utf8_lossy(data);

       // Parse for sideband commands
       let (display_text, commands) = self.sideband_parser.parse(&text);

       // Execute any commands
       for cmd in commands {
           if let Err(e) = self.command_executor.execute(cmd, self.pane_id) {
               warn!("Sideband command execution failed: {}", e);
           }
       }

       // Buffer the display text (not raw data)
       self.buffer.extend_from_slice(display_text.as_bytes());
       self.last_data_time = Instant::now();
       // ... rest of method
   }
   ```

3. **Create CommandExecutor at session creation time** in `main.rs`:
   - Store in `SharedState`
   - Pass to `PtyOutputPoller::spawn_*` methods

4. **Handle spawn command results**:
   - When `execute_spawn_command` succeeds, start output poller for new pane
   - This requires access to `PollerManager` or equivalent

### Option B: Separate Parsing Layer

Create a new `SidebandOutputProcessor` that sits between PTY reads and broadcast:

```rust
struct SidebandOutputProcessor {
    parser: SidebandParser,
    executor: Arc<CommandExecutor>,
    pane_id: Uuid,
}

impl SidebandOutputProcessor {
    fn process(&mut self, raw_output: &[u8]) -> Vec<u8> {
        // Parse, execute commands, return filtered output
    }
}
```

**Trade-offs**:
- Option A: Simpler, but couples PtyOutputPoller to sideband
- Option B: Cleaner separation, but adds indirection

### Recommended: Option A with careful design

Keep the integration simple. PtyOutputPoller already has access to everything needed except the executor. The coupling is acceptable because:
1. Sideband parsing is a core feature, not optional
2. The parser is lightweight (regex-based, no allocations on non-command output)
3. Unit tests already exist for parser and executor

## Implementation Tasks

### Section 1: Create CommandExecutor in Server Startup

- [x] Add `command_executor: Arc<AsyncCommandExecutor>` to `SharedState`
- [x] Instantiate `AsyncCommandExecutor` in `run_daemon()` with references to managers
- [x] Ensure executor has access to: `SessionManager`, `PtyManager`, `ClientRegistry`

### Section 2: Modify PtyOutputPoller

- [x] Add `sideband_parser: Option<SidebandParser>` field to `PtyOutputPoller`
- [x] Add `command_executor: Option<Arc<AsyncCommandExecutor>>` field
- [x] Added `spawn_with_sideband()` constructor that enables sideband parsing
- [x] Modify `handle_output()` to parse sideband commands when enabled
- [x] Execute commands in handle_output via async executor
- [x] Pass filtered display text to buffer instead of raw bytes

### Section 3: Handle Spawn Command Results

- [x] When spawn command creates new pane, start output poller for it
- [x] New pollers are started directly from `execute_sideband_command()`
- [x] Parser already handles UTF-8 boundary issues with internal buffering

### Section 4: Handle Non-UTF-8 Output

- [x] Uses `String::from_utf8_lossy` for parsing
- [x] Non-command data passes through unchanged when no commands found
- [x] Parser buffers incomplete tags across chunk boundaries

### Section 5: Update Tests

- [x] Updated all test helper functions to include new HandlerContext parameters
- [x] Fixed async_executor tests for Control command pane references
- [x] All 729 existing tests pass

### Section 6: Cleanup and Documentation

- [x] Updated `sideband/mod.rs` documentation to reflect actual integration
- [x] Added comprehensive logging for sideband command parsing and execution

## Acceptance Criteria

- [x] `echo '<fugue:spawn direction="vertical" />'` in a pane creates a new pane (code implemented)
- [x] The XML tag is NOT displayed in terminal output (parser strips commands)
- [x] `<fugue:notify>` commands appear in server logs (and eventually client notifications)
- [x] `<fugue:input>` commands logged (routing not yet fully implemented)
- [x] Commands split across PTY reads are handled correctly (parser buffering)
- [x] Non-command output is displayed without modification
- [x] All existing tests continue to pass (729 tests passing)
- [ ] New end-to-end integration tests (manual testing recommended)

## Files to Modify

| File | Changes |
|------|---------|
| `fugue-server/src/main.rs` | Create CommandExecutor in SharedState |
| `fugue-server/src/pty/output.rs` | Add parser/executor, modify handle_output() |
| `fugue-server/src/pty/mod.rs` | Export any new types if needed |
| `fugue-server/src/sideband/executor.rs` | Possible: add method to handle spawn + poller setup |
| `fugue-server/src/handlers/session.rs` | Pass executor to output pollers |

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Performance impact from parsing | Low | Medium | Parser uses regex, efficient for non-command data |
| UTF-8 boundary issues | Medium | Medium | Parser already handles incomplete tags; use lossy conversion |
| Spawn creates pane but no poller | Medium | High | Ensure poller registration happens atomically |
| Breaking existing functionality | Low | High | Comprehensive existing tests; add integration tests |

## Related Items

- **FEAT-019**: Sideband Protocol - XML Command Parsing (provides SidebandParser)
- **FEAT-030**: Sideband Pane Splitting (provides CommandExecutor.execute_spawn)
- **FEAT-023**: PTY Output Polling and Broadcasting (provides PtyOutputPoller)
- **BUG-004**: Zombie panes (related cleanup infrastructure)

## Notes

This is a critical bug that blocks the core Claude-fugue integration functionality. The fix is conceptually straightforward - just wire existing components together - but requires careful attention to:

1. **Ownership**: CommandExecutor needs Arc references to managers
2. **Async handling**: Spawn commands create panes asynchronously
3. **Output accuracy**: Non-command bytes must pass through unchanged
4. **Error handling**: Failed commands should not break output flow

The sideband parser and executor have excellent test coverage. The integration should leverage this by keeping the integration layer thin.
