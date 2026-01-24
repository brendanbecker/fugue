# Implementation Plan: BUG-005

**Work Item**: [BUG-005: Sideband Parsing Not Integrated into PTY Output Flow](PROMPT.md)
**Component**: fugue-server
**Priority**: P1
**Created**: 2026-01-09

## Overview

Wire the existing `SidebandParser` and `CommandExecutor` into `PtyOutputPoller` so that sideband commands embedded in PTY output are parsed, stripped from display, and executed.

## Architecture Decision: Integration Approach

### Option A: Integrate Directly into PtyOutputPoller (Chosen)

**Rationale**:
- Simplest integration path
- PtyOutputPoller already has access to most required components
- Sideband parsing is core functionality, not optional
- Parser is lightweight (no allocation on non-command data)
- Existing unit tests cover parser and executor independently

### Integration Points

```
Before:
  PTY Read -> handle_output(data) -> buffer.extend(data) -> flush() -> broadcast(data)

After:
  PTY Read -> handle_output(data) -> parse(data) -> {
      execute(commands)
      buffer.extend(display_text)
  } -> flush() -> broadcast(display_text)
```

## Component Ownership

### CommandExecutor Lifetime

```rust
// Create once at server startup
let command_executor = Arc::new(CommandExecutor::new(
    session_manager.clone(),   // Arc<RwLock<SessionManager>>
    pty_manager.clone(),       // Arc<RwLock<PtyManager>>
    registry.clone(),          // Arc<ClientRegistry>
));

// Store in SharedState
pub struct SharedState {
    // ... existing fields ...
    pub command_executor: Arc<CommandExecutor>,
}
```

### SidebandParser Lifetime

```rust
// Create one per PtyOutputPoller instance
pub struct PtyOutputPoller {
    // ... existing fields ...
    sideband_parser: SidebandParser,  // Owned, not shared
    command_executor: Arc<CommandExecutor>,
}
```

Each pane gets its own parser instance because:
- Parser has internal buffer for incomplete tags
- Buffer state is pane-specific
- No benefit to sharing (regex compilation happens once per instance anyway)

## Handling Spawn Results

When `CommandExecutor::execute_spawn_command()` succeeds, it returns a `SpawnResult`:

```rust
pub struct SpawnResult {
    pub session_id: Uuid,
    pub pane_id: Uuid,
    pub pane_info: PaneInfo,
    pub pty_reader: Arc<Mutex<Box<dyn Read + Send>>>,
}
```

The caller needs to start an output poller for the new pane. Options:

### Option A: PtyOutputPoller Starts Sibling Pollers (Chosen)

Add optional `PollerManager` reference to PtyOutputPoller. When spawn succeeds, register new poller.

```rust
pub struct PtyOutputPoller {
    // ... existing fields ...
    /// Optional manager to register new pollers (for spawn handling)
    poller_manager: Option<Arc<Mutex<PollerManager>>>,
}
```

Pro: Self-contained, no async coordination needed
Con: Couples PtyOutputPoller to PollerManager

### Option B: Queue Spawn Results for Main Loop

Send spawn results through a channel to the main server loop, which handles poller creation.

Pro: Cleaner separation
Con: More complex, async coordination

### Decision: Option A

The coupling is acceptable because:
1. Spawning panes is a core responsibility of sideband handling
2. PtyOutputPoller already handles pane lifecycle (closure notifications)
3. PollerManager is simple and has no complex state

## UTF-8 Handling Strategy

PTY output may contain:
1. Valid UTF-8 text
2. Binary data (escape sequences, images)
3. Partial UTF-8 sequences at buffer boundaries

### Strategy: Lossy Conversion + Byte Preservation

```rust
async fn handle_output(&mut self, data: &[u8]) {
    // For sideband parsing, convert to string (lossy for invalid UTF-8)
    let text = String::from_utf8_lossy(data);

    // Parse for commands
    let (display_text, commands) = self.sideband_parser.parse(&text);

    // Execute commands
    for cmd in commands {
        self.execute_command(cmd).await;
    }

    // If no sideband commands found, pass original bytes unchanged
    if commands.is_empty() && display_text == text {
        // No parsing happened, preserve exact bytes
        self.buffer.extend_from_slice(data);
    } else {
        // Commands stripped, buffer the filtered text
        self.buffer.extend_from_slice(display_text.as_bytes());
    }

    self.last_data_time = Instant::now();
    // ... flush logic
}
```

**Key insight**: If no `<fugue:` prefix is found, the parser returns input unchanged. We can detect this and pass original bytes through, preserving binary data.

### Edge Case: `<fugue:` Split Across Reads

The parser already handles this via internal buffering. When it sees `<fugue:` but no closing `>` or `/>`, it buffers until more data arrives.

## Modified Signatures

### PtyOutputPoller::spawn_with_cleanup (existing, needs update)

```rust
// Before
pub fn spawn_with_cleanup(
    pane_id: Uuid,
    session_id: Uuid,
    pty_reader: Arc<Mutex<Box<dyn Read + Send>>>,
    registry: Arc<ClientRegistry>,
    pane_closed_tx: Option<mpsc::Sender<PaneClosedNotification>>,
) -> PollerHandle

// After
pub fn spawn_with_sideband(
    pane_id: Uuid,
    session_id: Uuid,
    pty_reader: Arc<Mutex<Box<dyn Read + Send>>>,
    registry: Arc<ClientRegistry>,
    pane_closed_tx: Option<mpsc::Sender<PaneClosedNotification>>,
    command_executor: Arc<CommandExecutor>,
    poller_manager: Option<Arc<Mutex<PollerManager>>>,
) -> PollerHandle
```

### PollerManager::start (existing, needs update)

```rust
// Before
pub fn start(
    &mut self,
    pane_id: Uuid,
    session_id: Uuid,
    pty_reader: Arc<Mutex<Box<dyn Read + Send>>>,
    registry: Arc<ClientRegistry>,
)

// After
pub fn start_with_sideband(
    &mut self,
    pane_id: Uuid,
    session_id: Uuid,
    pty_reader: Arc<Mutex<Box<dyn Read + Send>>>,
    registry: Arc<ClientRegistry>,
    command_executor: Arc<CommandExecutor>,
)
```

## Implementation Phases

### Phase 1: Add CommandExecutor to SharedState

1. Create `CommandExecutor` in `run_daemon()` before shared state
2. Add to `SharedState` struct
3. Verify compilation

### Phase 2: Extend PtyOutputPoller

1. Add `sideband_parser` and `command_executor` fields
2. Create new constructor that accepts these
3. Keep existing constructors for backward compatibility (wrap new one)
4. Verify compilation, existing tests pass

### Phase 3: Implement Parsing in handle_output

1. Add parsing logic to `handle_output()`
2. Execute commands synchronously (most are fast)
3. Handle spawn specially (needs poller registration)
4. Add logging for parsed commands

### Phase 4: Handle Spawn Results

1. Add `poller_manager` field to `PtyOutputPoller`
2. When spawn succeeds, register new poller via manager
3. Handle errors gracefully (log, continue)

### Phase 5: Wire Up in Session Handler

1. Update `handle_create_session` to use new poller constructors
2. Update any other pane creation paths
3. Ensure restored sessions also get sideband handling

### Phase 6: Integration Testing

1. Manual test: echo spawn command, verify pane creation
2. Manual test: verify command stripped from display
3. Add automated integration tests

## Open Questions (Resolved)

1. **Should spawn be async?**
   - No. `execute_spawn_command` is synchronous (locks managers).
   - Poller registration is also synchronous.
   - Keep simple; async adds complexity without benefit.

2. **What about command execution failures?**
   - Log warning and continue.
   - Don't let command failures break output flow.
   - Consider future: emit error notification to client.

3. **Should we rate-limit commands?**
   - Not initially. Parser handles rapid commands.
   - Monitor for abuse patterns later.
   - Could add rate limiting in executor if needed.

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Regression in output display | Low | High | Existing tests + byte preservation |
| Performance degradation | Low | Medium | Parser is O(n) with early exit for non-command |
| Spawn fails silently | Medium | Medium | Logging + error notifications |
| Deadlock on manager locks | Low | High | Careful lock ordering, already established patterns |

## Rollback Strategy

If integration causes issues:
1. Revert `handle_output()` changes in `output.rs`
2. Remove executor from `SharedState`
3. Commands will be displayed but not executed (pre-bug behavior)
4. All other functionality unaffected

## Success Criteria

- [ ] `echo '<fugue:spawn direction="v" />'` creates new pane
- [ ] XML tag not visible in terminal
- [ ] `<fugue:notify>` appears in server logs
- [ ] All 135+ existing tests pass
- [ ] New integration tests pass
- [ ] No measurable performance impact on normal output

---
*This plan should be updated as implementation progresses.*
