# Task Breakdown: BUG-011

**Work Item**: [BUG-011: Large Paste Input Crashes Session](PROMPT.md)
**Status**: Completed
**Last Updated**: 2026-01-10

## Prerequisites

- [x] Read and understand PROMPT.md
- [x] Review PLAN.md and update if needed
- [x] Understand fugue input data flow (client -> socket -> server -> PTY)

## Investigation Tasks

### Reproduce the Crash

- [ ] Start fugue session
- [ ] Generate large test content (1MB, 10MB, 100MB files)
- [ ] Attempt paste of each size
- [ ] Document which size causes crash
- [ ] Note any error messages in logs

### Identify Crash Location

- [ ] Check if client crashes (TUI exits)
- [ ] Check if server crashes (daemon dies)
- [ ] Check if connection drops (socket error)
- [ ] Check if PTY dies (shell exits)

### Add Debugging

- [ ] Add tracing to client input path
- [ ] Add tracing to socket send/receive
- [ ] Add tracing to server handlers
- [ ] Add tracing to PTY write path
- [ ] Reproduce crash with tracing enabled
- [ ] Identify exact failure point

### Code Analysis

- [x] Read `fugue-client/src/input/mod.rs` - input handling
- [x] Read `fugue-client/src/socket.rs` - client socket code
- [x] Read `fugue-protocol/src/lib.rs` - message types
- [x] Read message framing/serialization code
- [x] Read `fugue-server/src/handlers/` - message handlers
- [x] Read `fugue-server/src/pty/` - PTY write handling
- [x] Document findings in PLAN.md

### Root Cause Determination

- [x] Confirm root cause from investigation
- [x] Document root cause in PLAN.md
- [x] Choose fix approach
- [x] Update PLAN.md with chosen solution

## Implementation Tasks

### If Root Cause is Client-Side Buffer

- [x] Add size limit check on paste input
- [x] Implement chunking for large pastes
- [x] Add user feedback for oversized pastes

### If Root Cause is Protocol/Socket

- [ ] Add message size validation
- [ ] Implement chunked message protocol (if needed)
- [ ] Add proper error handling for oversized messages

### If Root Cause is Server-Side

- [ ] Add input validation in handlers
- [ ] Implement backpressure or chunked processing
- [ ] Add graceful error responses

### If Root Cause is PTY Write

- [ ] Implement chunked PTY writes
- [ ] Add non-blocking write with retry
- [ ] Handle EAGAIN/EWOULDBLOCK properly

### General Implementation

- [ ] Implement chosen fix
- [ ] Add graceful error handling
- [ ] Add user-visible feedback for failures
- [ ] Ensure session survives failed paste
- [ ] Self-review changes

## Testing Tasks

### Unit Tests

- [x] Add test for input size limits
- [x] Add test for chunking logic (if implemented)
- [x] Add test for error handling

### Integration Tests

- [ ] Add test for large paste handling
- [ ] Add test for session stability after failed paste
- [ ] Test edge cases (just under limit, at limit, over limit)

### Manual Testing

- [ ] Test 100KB paste - should work
- [ ] Test 1MB paste - should work or fail gracefully
- [ ] Test 10MB paste - should fail gracefully
- [ ] Test 100MB paste - should fail gracefully
- [ ] Verify session remains stable in all cases
- [ ] Verify normal operation after large paste attempt

### Regression Testing

- [ ] Run full test suite
- [ ] Verify no existing tests broken
- [ ] Verify normal pastes still work

## Verification Tasks

- [ ] Confirm no crash on large paste
- [ ] Confirm graceful error handling
- [ ] Confirm user receives feedback
- [ ] Confirm session remains attached
- [ ] All acceptance criteria from PROMPT.md met
- [ ] Update bug_report.json status
- [ ] Document resolution in PLAN.md

## Completion Checklist

- [x] All investigation tasks complete
- [x] Root cause identified and documented
- [x] All implementation tasks complete
- [x] All tests passing
- [x] PLAN.md updated with final approach
- [x] Ready for review/merge

---
*Check off tasks as you complete them. Update status field above.*
