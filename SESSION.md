# Session: Stream A - Reattach Bug Fix

## Work Items
- **BUG-018**: TUI pane interaction failure (can't see input bar)
- **BUG-020**: Session reattach from session manager creates client without PTY

## Priority: P1 (Critical)

## Problem Summary

When reattaching to a session via the session manager UI, the client connects but has no PTY access:
- Pane is visible but user cannot see output or send input
- `read_pane` via MCP returns empty
- Keyboard input doesn't reach the pane

BUG-018 and BUG-020 are likely the **same root cause** - both describe inability to interact with panes after session reattachment.

## Steps to Reproduce

1. Start ccmux and create a session
2. Detach from the session (Ctrl+B d) or open session manager
3. From the session manager UI, select the existing session to reattach
4. Observe that the client appears to connect
5. Attempt to interact with the terminal pane
6. Observe that input is not accepted and output is not visible

## Suspected Root Cause

The attach handler may not be properly connecting the new client to the existing PTY output stream when reattaching through the session manager. Investigate:

1. PTY reader cloning
2. Output poller registration
3. Client subscription to PTY output stream
4. Session vs pane attachment semantics

## Files to Investigate

- `ccmux-server/src/handlers/attach.rs` - Attach handler logic
- `ccmux-server/src/handlers/session.rs` - Session management
- `ccmux-server/src/pty/output.rs` - PTY output poller and client registration
- `ccmux-server/src/registry/` - Client and session registry
- `ccmux-client/src/ui/app.rs` - Client app attach flow

## Acceptance Criteria

- [ ] Reattaching to session via session manager connects client to PTY
- [ ] User can see output from the PTY after reattach
- [ ] User can send input to the PTY after reattach
- [ ] Multiple clients can attach to same session and all see output
- [ ] No regression in direct attach functionality

## Related Work Items

- See `feature-management/bugs/BUG-018-tui-pane-interaction-failure/PROMPT.md`
- See `feature-management/bugs/BUG-020-session-reattach-no-pty/PROMPT.md`

## Commands

```bash
# Build
cargo build --release

# Run tests
cargo test --workspace

# Run ccmux for testing
./target/release/ccmux
```
