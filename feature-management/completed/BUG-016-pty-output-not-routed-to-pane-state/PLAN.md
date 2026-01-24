# Implementation Plan: BUG-016

**Work Item**: [BUG-016: PTY output not routed to pane state](PROMPT.md)
**Component**: fugue-server
**Priority**: P1
**Created**: 2026-01-10

## Overview

The PtyOutputPoller broadcasts PTY output to connected TUI clients but never routes the output back to the pane's scrollback buffer or through pane.process() for Claude detection. This breaks MCP read_pane (always returns empty) and Claude detection (always false).

## Architecture Decisions

### Approach: Pass SessionManager reference to PtyOutputPoller

The recommended approach is to provide the `PtyOutputPoller` with access to the `SessionManager` (wrapped in `Arc<RwLock<...>>`), allowing it to look up the pane by ID and call `pane.process()` on each output chunk.

**Rationale**:
- `PtyOutputPoller` already has the `pane_id` and `session_id`
- `SessionManager` provides safe access to pane state
- Consistent with existing patterns in MCP handlers
- Allows for future enhancements (Claude state change broadcasts)

**Trade-offs**:
- Adds a dependency on SessionManager from the PTY layer
- Requires careful locking to avoid deadlocks
- Slight performance overhead from lookup + lock per flush

### Alternative Considered: Channel-based decoupling

Send output to a separate task that handles pane state updates.

**Why not chosen**:
- More complex architecture
- Additional buffering and latency
- Harder to guarantee ordering with broadcasts

### Alternative Considered: Arc<Mutex<Pane>> passed directly

Pass a reference to the specific pane at spawn time.

**Why not chosen**:
- Requires holding pane lock across async boundaries
- Makes pane relocation/replacement complex
- Less flexible than SessionManager lookup

## Affected Components

| Component | Type of Change | Risk Level |
|-----------|----------------|------------|
| fugue-server/src/pty/output.rs | Primary - add SessionManager access, call pane.process() | Medium |
| fugue-server/src/pty/mod.rs | Add SessionManager to spawn functions | Low |
| fugue-server/src/session/manager.rs | May need additional lookup methods | Low |
| fugue-server/src/server/main.rs | Pass SessionManager to output pollers | Low |

## Implementation Details

### Step 1: Add SessionManager to PtyOutputPoller

```rust
pub struct PtyOutputPoller {
    // ... existing fields ...
    /// Session manager for pane state access
    session_manager: Option<Arc<RwLock<SessionManager>>>,
}
```

### Step 2: Modify spawn functions

Add `session_manager` parameter to:
- `PtyOutputPoller::spawn()`
- `PtyOutputPoller::spawn_with_cleanup()`
- `PtyOutputPoller::spawn_with_config()`
- `PtyOutputPoller::spawn_with_sideband()`

### Step 3: Route output to pane in handle_output()

```rust
async fn handle_output(&mut self, data: &[u8]) {
    self.last_data_time = Instant::now();

    // Route to pane state for scrollback + Claude detection
    if let Some(session_manager) = &self.session_manager {
        let manager = session_manager.read().await;
        if let Some(session) = manager.get_session(self.session_id) {
            if let Some(pane) = session.get_pane_mut(self.pane_id) {
                if let Some(claude_state) = pane.process(data) {
                    // Claude state changed - could broadcast here
                    debug!(pane_id = %self.pane_id, "Claude state detected");
                }
            }
        }
    }

    // ... existing sideband parsing and buffering ...
}
```

### Step 4: Update callers

All places that spawn output pollers need to pass the SessionManager:
- Server listen loop (new session/pane creation)
- MCP create_pane handler
- Sideband spawn command execution

## Dependencies

None - this is a bug fix for existing functionality.

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Deadlock from nested locks | Medium | High | Use RwLock, acquire read lock briefly, release before broadcast |
| Performance regression | Low | Medium | Profile if needed; pane.process() is lightweight |
| Regression in TUI display | Low | High | Keep broadcast path unchanged; add tests |
| Thread-safety issues | Medium | High | Use proper async-aware locks (tokio RwLock) |

## Rollback Strategy

If implementation causes issues:
1. Revert commits associated with this work item
2. Verify system returns to previous state (broken but stable)
3. Document what went wrong in comments.md

## Testing Strategy

1. **Unit test**: Mock SessionManager, verify pane.process() is called
2. **Integration test**: Create pane, send output, verify scrollback populated
3. **Integration test**: Run Claude-like output, verify is_claude becomes true
4. **Manual test**: MCP read_pane returns actual content

## Implementation Notes

<!-- Add notes during implementation -->

---
*This plan should be updated as implementation progresses.*
