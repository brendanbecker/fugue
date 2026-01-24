# Task Breakdown: BUG-020

**Work Item**: [BUG-020: Session Reattach from Session Manager Creates Client Without PTY](PROMPT.md)
**Status**: Resolved
**Last Updated**: 2026-01-10
**Resolution**: Fixed in commit 8f53895 - Server now sends scrollback content on session reattach

## Prerequisites

- [ ] Read and understand PROMPT.md
- [ ] Review PLAN.md and update if needed
- [ ] Understand fugue client attachment flow
- [ ] Understand PTY output poller architecture

## Investigation Tasks

### Reproduce the Bug

- [ ] Start fugue and create a session with a running process
- [ ] Detach from the session
- [ ] Open session manager UI
- [ ] Select the existing session to reattach
- [ ] Confirm client connects but PTY is not visible
- [ ] Confirm input doesn't reach the pane
- [ ] Compare behavior with direct attach (fugue attach -s SESSION)

### Code Path Analysis

- [ ] Trace session manager attach code path in client
- [ ] Trace session manager attach code path in server
- [ ] Compare with direct attach code path
- [ ] Identify where the paths diverge
- [ ] Document differences in PLAN.md

### Output Poller Investigation

- [ ] Check how output poller tracks connected clients
- [ ] Verify client is added to output poller on session manager attach
- [ ] Check if output poller is started for existing sessions
- [ ] Add debug logging to output poller client registration

### Client Registration Investigation

- [ ] Check session_clients after session manager attach
- [ ] Verify client ID is correctly registered
- [ ] Check if session ID matches between client and server
- [ ] Verify broadcast targets include the new client

### Attach Handler Investigation

- [ ] Read attach handler code
- [ ] Check if session manager uses same attach handler
- [ ] Identify any missing steps in session manager path
- [ ] Check for async/race conditions in registration

### Root Cause Determination

- [ ] Confirm root cause from investigation
- [ ] Document root cause in PLAN.md
- [ ] Choose fix approach
- [ ] Update PLAN.md with chosen solution

## Implementation Tasks

### If Root Cause is Missing Output Subscription

- [ ] Add client to output poller broadcast list on session manager attach
- [ ] Ensure subscription happens after client registration
- [ ] Handle case where output poller already running
- [ ] Test with multiple concurrent attachments

### If Root Cause is Separate Attach Paths

- [ ] Identify common attach logic
- [ ] Extract shared functionality
- [ ] Call shared logic from both paths
- [ ] Remove duplicate code
- [ ] Test both paths

### If Root Cause is Race Condition

- [ ] Add synchronization for client registration
- [ ] Ensure registration completes before output starts
- [ ] Add proper ordering guarantees
- [ ] Test concurrent attach/detach scenarios

### If Root Cause is Session vs Pane Mismatch

- [ ] Fix session manager to attach to correct pane
- [ ] Ensure active pane is correctly identified
- [ ] Send pane focus along with attach
- [ ] Test with multi-pane sessions

### General Implementation

- [ ] Implement chosen fix
- [ ] Add error handling for edge cases
- [ ] Add logging for debugging
- [ ] Self-review changes

## Testing Tasks

### Unit Tests

- [ ] Add test for session manager attach flow
- [ ] Add test for output subscription on reattach
- [ ] Add test for multiple client attachment
- [ ] Add test for attach after detach cycle

### Integration Tests

- [ ] Test session manager attach receives PTY output
- [ ] Test input routing after session manager attach
- [ ] Test multiple clients on same session
- [ ] Test rapid attach/detach cycles
- [ ] Test attach to session with active output

### Manual Testing

- [ ] Verify session manager attach works
- [ ] Verify direct attach still works
- [ ] Verify output is visible after reattach
- [ ] Verify input reaches PTY after reattach
- [ ] Test with Claude session
- [ ] Test with shell session
- [ ] Test with multiple panes

### Regression Testing

- [ ] Run full test suite
- [ ] Verify no existing tests broken
- [ ] Verify BUG-004 fix not regressed
- [ ] Verify normal attach still works

## Verification Tasks

- [ ] Confirm session manager attach connects to PTY
- [ ] Confirm output visible after reattach
- [ ] Confirm input works after reattach
- [ ] Confirm multiple clients see output
- [ ] All acceptance criteria from PROMPT.md met
- [ ] Update bug_report.json status
- [ ] Document resolution in PLAN.md

## Completion Checklist

- [ ] All investigation tasks complete
- [ ] Root cause identified and documented
- [ ] All implementation tasks complete
- [ ] All tests passing
- [ ] PLAN.md updated with final approach
- [ ] Ready for review/merge

---
*Check off tasks as you complete them. Update status field above.*
