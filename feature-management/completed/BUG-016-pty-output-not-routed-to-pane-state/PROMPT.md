# BUG-016: PTY output not routed to pane state - breaks Claude detection and MCP read_pane

**Priority**: P1
**Component**: fugue-server
**Severity**: high
**Status**: new

## Problem Statement

The PtyOutputPoller broadcasts PTY output to connected TUI clients via ServerMessage::Output, but never routes the output back to the pane's scrollback buffer or through pane.process() for Claude detection. This causes two critical failures:

1. **MCP read_pane returns empty** - The scrollback buffer is never populated, so fugue_read_pane always returns empty strings
2. **Claude detection never triggers** - Since pane.process() is never called with PTY output, the ClaudeDetector never analyzes any data, so is_claude is always false

## Evidence

### PtyOutputPoller::flush() (fugue-server/src/pty/output.rs, lines 535-564)

```rust
async fn flush(&mut self) {
    if self.buffer.is_empty() {
        return;
    }

    let data = std::mem::take(&mut self.buffer);
    self.buffer = Vec::with_capacity(self.config.max_buffer_size);

    trace!(
        pane_id = %self.pane_id,
        session_id = %self.session_id,
        bytes = data.len(),
        "Flushing output to session"
    );

    let msg = ServerMessage::Output {
        pane_id: self.pane_id,
        data,
    };

    let delivered = self.registry.broadcast_to_session(self.session_id, msg).await;
    // ... logging ...
}
```

**Problem**: This only broadcasts to clients. There is no call to route data back to the pane state.

### Pane::process() (fugue-server/src/session/pane.rs, lines 356-374)

```rust
pub fn process(&mut self, data: &[u8]) -> Option<ClaudeState> {
    if let Some(parser) = &mut self.parser {
        parser.process(data);
    }
    // Also push to scrollback
    self.scrollback.push_bytes(data);

    // Analyze output for Claude state changes
    let text = String::from_utf8_lossy(data);
    if let Some(_activity) = self.claude_detector.analyze(&text) {
        // State changed - update pane state and return new state
        if let Some(claude_state) = self.claude_detector.state() {
            self.state = PaneState::Claude(claude_state.clone());
            self.state_changed_at = SystemTime::now();
            return Some(claude_state);
        }
    }
    None
}
```

**Note**: This method exists and correctly handles scrollback + Claude detection, but is never called from PtyOutputPoller.

### MCP read_pane handler (fugue-server/src/mcp/handlers.rs, lines 79-91)

```rust
pub fn read_pane(&self, pane_id: Uuid, lines: usize) -> Result<String, McpError> {
    // ... pane lookup ...

    // Get lines from scrollback
    let scrollback = pane.scrollback();
    let all_lines: Vec<&str> = scrollback.get_lines().collect();
    // ...
}
```

**Problem**: This reads from scrollback which is never populated.

## Steps to Reproduce

1. Start fugue server and attach a TUI client
2. Run Claude Code in a pane (e.g., `claude` command)
3. Use MCP tool `fugue_list_panes` to check pane state
4. Observe: `is_claude` is false for all panes despite Claude running
5. Use MCP tool `fugue_read_pane` to read pane output
6. Observe: returns empty string despite visible output in TUI

## Expected Behavior

- PTY output should be routed to pane.process() to populate scrollback and trigger Claude detection
- MCP read_pane should return actual terminal output
- Claude instances should be detected (is_claude: true) when Claude Code patterns are found

## Actual Behavior

- All panes show is_claude: false
- All panes show empty scrollback (read_pane returns nothing)
- Claude detection is completely non-functional

## Root Cause

The `PtyOutputPoller` lacks access to the pane state and only has access to the `ClientRegistry` for broadcasting. The integration point that routes PTY output to pane.process() was never implemented.

## Implementation Tasks

### Section 1: Investigation
- [ ] Trace the data flow from PTY read to client broadcast
- [ ] Identify where pane state can be accessed from PtyOutputPoller
- [ ] Determine threading/async considerations (pane access from async context)
- [ ] Consider whether SessionManager access is needed

### Section 2: Design Decision
- [ ] Option A: Pass pane reference to PtyOutputPoller (requires Arc<Mutex<Pane>>)
- [ ] Option B: Pass SessionManager reference to PtyOutputPoller
- [ ] Option C: Add a channel for pane state updates (decoupled approach)
- [ ] Document chosen approach in PLAN.md

### Section 3: Implementation
- [ ] Modify PtyOutputPoller to have access to pane state
- [ ] Call pane.process() in handle_output() or flush()
- [ ] Handle Claude state changes (broadcast PaneStateChanged if needed)
- [ ] Ensure thread-safety with proper locking

### Section 4: Testing
- [ ] Add test that verifies scrollback is populated from PTY output
- [ ] Add test that verifies Claude detection triggers from PTY output
- [ ] Add integration test for MCP read_pane returning actual content
- [ ] Verify existing tests still pass

### Section 5: Verification
- [ ] Manual test: start Claude, verify is_claude becomes true
- [ ] Manual test: run commands, verify read_pane returns output
- [ ] Verify TUI still receives output (no regression)
- [ ] Update bug_report.json status to resolved

## Acceptance Criteria

- [ ] PTY output populates pane scrollback buffer
- [ ] pane.process() is called for all PTY output
- [ ] Claude detection triggers when Claude Code patterns appear
- [ ] MCP read_pane returns actual terminal output
- [ ] TUI clients still receive output via broadcast (no regression)
- [ ] Tests added to prevent regression

## Notes

This is a critical architectural gap. The pane state management (scrollback, Claude detection) exists but was never wired into the PTY output flow. The fix requires giving PtyOutputPoller access to the pane state, which may require architectural consideration around ownership and thread-safety.

The most likely approach is to pass an `Arc<RwLock<SessionManager>>` or similar to the PtyOutputPoller, allowing it to look up the pane and call process(). Alternative approaches could use channels to decouple the output routing from the broadcast path.
