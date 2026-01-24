# Task Breakdown: BUG-014

**Work Item**: [BUG-014: Large Output Buffer Overflow](PROMPT.md)
**Status**: Not Started
**Last Updated**: 2026-01-10

## Prerequisites

- [ ] Read and understand PROMPT.md
- [ ] Review PLAN.md and update if needed
- [ ] Understand fugue output data flow (PTY -> poller -> broadcast -> client -> screen)

## Investigation Tasks

### Reproduce the Bug

- [ ] Start fugue session with Claude
- [ ] Generate large output (large diff, verbose tool output, etc.)
- [ ] Confirm input becomes unresponsive
- [ ] Confirm detach (Ctrl+B d) works
- [ ] Confirm reattach shows same unresponsive state
- [ ] Note approximate output size that triggers the issue

### Measure Resource Usage

- [ ] Monitor memory usage during large output
- [ ] Monitor CPU usage during large output
- [ ] Check if server or client memory is growing
- [ ] Check if there's a specific threshold

### Identify Unresponsive Location

- [ ] Determine if client event loop is blocked
- [ ] Determine if server is still processing
- [ ] Check if input events reach the server
- [ ] Check if PTY receives input during freeze

### Add Debugging

- [ ] Add tracing to PTY output poller
- [ ] Add tracing to broadcast/message sending
- [ ] Add tracing to client message receive
- [ ] Add tracing to client render loop
- [ ] Add tracing to input event processing
- [ ] Add memory/buffer size logging
- [ ] Reproduce with tracing and identify bottleneck

### Code Analysis

- [ ] Read PTY output polling code
- [ ] Read broadcast/message queue implementation
- [ ] Read client main event loop
- [ ] Read screen buffer / scrollback implementation
- [ ] Read attach/state sync handler
- [ ] Check for any existing buffer limits
- [ ] Document findings in PLAN.md

### Root Cause Determination

- [ ] Confirm root cause from investigation
- [ ] Document root cause in PLAN.md
- [ ] Choose fix approach
- [ ] Update PLAN.md with chosen solution

## Implementation Tasks

### If Root Cause is Unbounded Scrollback

- [ ] Determine appropriate default scrollback limit
- [ ] Implement configurable scrollback limit
- [ ] Implement ring buffer or LRU eviction
- [ ] Apply limit to both client and server state
- [ ] Add warning when buffer is at capacity

### If Root Cause is Output Flooding

- [ ] Implement backpressure mechanism
- [ ] Add output batching or rate limiting
- [ ] Ensure input events interleaved with output processing
- [ ] Add client readiness signal before sending more output

### If Root Cause is Event Loop Starvation

- [ ] Add yield points in output processing
- [ ] Implement fair scheduling for input vs output
- [ ] Process input events between output batches
- [ ] Add priority queue for input events

### If Root Cause is Server-Side State Bloat

- [ ] Limit server-side terminal state retention
- [ ] Implement incremental state updates
- [ ] Add state compression for attach sync
- [ ] Reduce initial state size on reattach

### If Root Cause is Message Queue Overflow

- [ ] Add bounded queue for output messages
- [ ] Implement drop policy for old messages
- [ ] Add queue depth monitoring
- [ ] Add warning when queue is full

### General Implementation

- [ ] Implement chosen fix
- [ ] Add configuration options for limits
- [ ] Add graceful degradation behavior
- [ ] Ensure input always remains responsive
- [ ] Self-review changes

## Testing Tasks

### Unit Tests

- [ ] Add test for scrollback limit enforcement
- [ ] Add test for buffer eviction policy
- [ ] Add test for output rate limiting (if implemented)
- [ ] Add test for backpressure signaling (if implemented)

### Integration Tests

- [ ] Add test for large output handling
- [ ] Add test for input responsiveness during output flood
- [ ] Add test for detach/reattach after large output
- [ ] Add test for memory usage bounds

### Manual Testing

- [ ] Test with Claude generating large diff
- [ ] Test with `cat` of large file
- [ ] Test with `find /` or similar verbose command
- [ ] Verify input responsive during all scenarios
- [ ] Verify scrollback works after large output
- [ ] Verify memory usage stays bounded
- [ ] Test detach/reattach restores usable session

### Regression Testing

- [ ] Run full test suite
- [ ] Verify no existing tests broken
- [ ] Verify normal output still works
- [ ] Verify scrollback navigation works

## Verification Tasks

- [ ] Confirm input responsive during large output
- [ ] Confirm scrollback limit works
- [ ] Confirm detach/reattach works after large output
- [ ] Confirm memory stays bounded
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
