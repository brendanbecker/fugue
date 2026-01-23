# BUG-072: ccmux_kill_session hang regression

**Priority**: P1
**Component**: mcp, client, server
**Severity**: high
**Status**: completed

## Problem

`ccmux_kill_session` is causing client hangs again, despite the fix in BUG-058 (commit `9fd2481`). This is a regression of the original issue.

## Background

BUG-058 was fixed on 2026-01-18 by broadcasting `SessionEnded` to attached clients before detaching them in `handle_destroy_session`. The fix is confirmed to be in the currently running binary (built 2026-01-20 00:33).

Original fix commit: `9fd2481bae9e07ab0d2d366088a285b8936ef960`

## Reproduction Steps

1. Have multiple sessions running
2. Call `ccmux_kill_session` via MCP to kill a session
3. Observe: Client hangs

## Expected Behavior

Session is killed and client continues operating normally.

## Actual Behavior

Client hangs after session is killed, similar to original BUG-058 symptoms.

## Investigation Steps

### Section 1: Verify Original Fix Still Present
- [ ] Confirm `SessionEnded` broadcast is still in `handle_destroy_session`
- [ ] Check if any recent commits modified session destruction logic
- [ ] Review commits since BUG-058 fix that touch session management

### Section 2: Identify Regression Cause
- [ ] Run with RUST_LOG=debug and capture logs during hang
- [ ] Compare current behavior with BUG-058 fix expectations
- [ ] Check if there are new code paths that bypass the fix
- [ ] Look for race conditions in session cleanup

### Section 3: Test Scenarios
- [ ] Test killing current session vs non-current session
- [ ] Test killing session with attached clients vs detached
- [ ] Test killing session via MCP vs via TUI (Ctrl+D)
- [ ] Test with single client vs multiple clients

## Acceptance Criteria

- [ ] `ccmux_kill_session` completes without hanging the client
- [ ] Root cause of regression identified and documented
- [ ] Fix addresses regression without breaking original fix
- [ ] Add regression test if possible

## Related Files

- `ccmux-server/src/session/manager.rs` - Session destruction logic
- `ccmux-server/src/mcp/bridge/handlers.rs` - kill_session MCP handler
- `ccmux-client/src/ui/app.rs` - Client state management
- `ccmux-protocol/src/messages.rs` - SessionEnded message

## Related Issues

- BUG-058: Original fix for kill_session client hang
