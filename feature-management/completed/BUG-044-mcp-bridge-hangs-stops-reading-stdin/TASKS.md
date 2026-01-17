# Task Breakdown: BUG-044

**Work Item**: [BUG-044: MCP bridge process hangs indefinitely, stops reading stdin](PROMPT.md)
**Status**: Not Started
**Last Updated**: 2026-01-16

## Prerequisites

- [ ] Read and understand PROMPT.md
- [ ] Review PLAN.md and update if needed
- [ ] Identify affected code paths in mcp-bridge

## Investigation Tasks

- [ ] Locate main stdin reading loop in mcp-bridge
- [ ] Review async/sync boundaries in the code
- [ ] Add diagnostic logging:
  - [ ] Before/after stdin line reads
  - [ ] Before/after handle_request().await calls
  - [ ] In ConnectionManager lock acquisitions
  - [ ] In recv_filtered timeout handling
- [ ] Reproduce the bug (or set up monitoring)
- [ ] Analyze logs/thread state when hung
- [ ] Confirm root cause hypothesis

## Implementation Tasks

### If Async/Sync Issue (Primary Hypothesis)
- [ ] Convert stdin reading to async:
  - [ ] Replace `std::io::stdin().lock().lines()` with tokio async stdin
  - [ ] Use `tokio::io::BufReader` with async `lines()`
  - [ ] Ensure proper error handling for async reads
- [ ] Test that stdin reading works correctly after conversion
- [ ] Verify request processing still functions

### If Deadlock in ConnectionManager
- [ ] Identify lock contention points
- [ ] Refactor to reduce lock scope
- [ ] Consider using `tokio::sync::RwLock` if not already
- [ ] Add lock acquisition timeouts where appropriate

### If Timeout Issue
- [ ] Review recv_filtered timeout logic
- [ ] Ensure tokio::time::timeout wraps the correct scope
- [ ] Verify timeout propagates correctly to caller
- [ ] Add tests for timeout scenarios

### Watchdog Implementation
- [ ] Design watchdog task:
  - [ ] Track last successful stdin read timestamp
  - [ ] Track last successful request completion
  - [ ] Log warning after 15 seconds of inactivity
  - [ ] Log error after 25 seconds
- [ ] Implement watchdog background task
- [ ] Add activity tracking to main loop
- [ ] Test watchdog detection

## Testing Tasks

- [ ] Add unit test for async stdin reading
- [ ] Add integration test for rapid MCP tool calls
- [ ] Add test for simulated slow daemon response
- [ ] Verify 25-second timeout triggers correctly
- [ ] Run stress test with 100+ rapid MCP calls
- [ ] Perform soak test (30+ minutes of periodic usage)
- [ ] Run full test suite

## Verification Tasks

- [ ] Confirm hang no longer occurs under normal usage
- [ ] Verify timeout triggers within expected window
- [ ] Check no performance regression
- [ ] Verify all acceptance criteria from PROMPT.md:
  - [ ] Exact blocking point identified
  - [ ] Diagnostic logging added
  - [ ] 25-second timeout works
  - [ ] No MCP call hangs > 30 seconds
- [ ] Update bug_report.json status
- [ ] Document resolution in comments.md

## Completion Checklist

- [ ] Root cause identified and documented
- [ ] Fix implemented and tested
- [ ] All tests passing
- [ ] PLAN.md updated with final approach
- [ ] Stress test passed
- [ ] Ready for review/merge

---
*Check off tasks as you complete them. Update status field above.*
