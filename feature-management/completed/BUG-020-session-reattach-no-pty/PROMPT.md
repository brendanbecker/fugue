# BUG-020: Session Reattach from Session Manager Creates Client Without PTY

## Priority: P1
## Status: New
## Created: 2026-01-10

## Problem Summary

When a user selects an existing session from the session manager/session selection UI, it creates an additional client connection to the session but the client doesn't get a PTY. The user cannot interact with the terminal - they see the pane but can't type or see output.

## Symptoms Observed

1. **Client connects**: Additional client is registered with the server
2. **Visual display**: Pane is visible in the TUI
3. **No PTY**: PTY is not assigned/visible to the new client
4. **No interaction**: Cannot type or see output in the terminal pane

## Steps to Reproduce

1. Start fugue and create a session
2. Detach from the session (Ctrl+B d) or open session manager
3. From the session manager UI, select the existing session to reattach
4. Observe that the client appears to connect
5. Attempt to interact with the terminal pane
6. Observe that input is not accepted and output is not visible

## Expected Behavior

- Selecting a session should attach the client to the existing PTY
- User should be able to interact with the terminal immediately
- Output from the PTY should be visible
- Input should be routed to the PTY

## Actual Behavior

- Client connects and shows the pane visually
- No PTY is assigned/visible to the new client
- Cannot interact with the terminal pane
- No input accepted, no output visible

## Suspected Root Cause

May be related to how session attachment handles:
1. PTY reader cloning
2. Output poller registration
3. Client subscription to PTY output stream
4. Session vs pane attachment semantics

The attach handler may not be properly connecting the new client to the existing PTY output stream when reattaching through the session manager.

## Context

- Discovered during testing of BUG-019 fix (UTF-8 panic hang)
- This is separate from BUG-019
- Session manager path may differ from direct attach path

## Files to Investigate

- `fugue-server/src/handlers/attach.rs` - Attach handler logic
- `fugue-server/src/handlers/session.rs` - Session management
- `fugue-server/src/pty/output.rs` - PTY output poller and client registration
- `fugue-server/src/registry/` - Client and session registry
- `fugue-client/src/ui/session_manager.rs` - Session manager UI (if exists)
- `fugue-client/src/ui/app.rs` - Client app attach flow

## Related Issues

- **BUG-019**: UTF-8 panic hang (separate issue, discovered during same testing)
- **BUG-004**: Client hangs when reattaching to session with dead pane (related pattern)
- **BUG-018**: TUI pane interaction failure (may share symptoms)

## Acceptance Criteria

- [ ] Reattaching to session via session manager connects client to PTY
- [ ] User can see output from the PTY after reattach
- [ ] User can send input to the PTY after reattach
- [ ] Multiple clients can attach to same session and all see output
- [ ] No regression in direct attach functionality

## Resolution

_To be determined after investigation_
