# Task Breakdown: BUG-016

**Work Item**: [BUG-016: PTY output not routed to pane state](PROMPT.md)
**Status**: Not Started
**Last Updated**: 2026-01-10

## Prerequisites

- [ ] Read and understand PROMPT.md
- [ ] Review PLAN.md and update if needed
- [ ] Trace current PTY output flow in code
- [ ] Understand SessionManager locking patterns

## Investigation Tasks

- [ ] Confirm PtyOutputPoller has no pane state access
- [ ] Identify all places PtyOutputPoller is spawned
- [ ] Check SessionManager thread-safety (RwLock vs Mutex)
- [ ] Document current data flow in PLAN.md

## Implementation Tasks

- [ ] Add `session_manager: Option<Arc<RwLock<SessionManager>>>` to PtyOutputPoller
- [ ] Update PtyOutputPoller::new() to accept SessionManager
- [ ] Update spawn() function signature
- [ ] Update spawn_with_cleanup() function signature
- [ ] Update spawn_with_config() function signature
- [ ] Update spawn_with_sideband() function signature
- [ ] Implement pane.process() call in handle_output()
- [ ] Handle read lock acquisition safely (avoid deadlocks)
- [ ] Update PollerManager if needed
- [ ] Update server listen loop to pass SessionManager
- [ ] Update MCP create_pane handler to pass SessionManager
- [ ] Update sideband spawn execution to pass SessionManager

## Testing Tasks

- [ ] Add unit test: verify pane.process() is called on output
- [ ] Add integration test: scrollback populated from PTY output
- [ ] Add integration test: Claude detection from PTY output
- [ ] Add integration test: MCP read_pane returns content
- [ ] Run existing test suite - verify no regressions
- [ ] Manual test: TUI still receives output correctly

## Verification Tasks

- [ ] Manual test: start Claude in pane, verify is_claude becomes true
- [ ] Manual test: run commands, verify read_pane returns output
- [ ] Verify sideband commands still work
- [ ] Update bug_report.json status to resolved
- [ ] Document resolution in comments.md

## Completion Checklist

- [ ] All implementation tasks complete
- [ ] All tests passing
- [ ] PLAN.md updated with final approach
- [ ] No regressions in TUI functionality
- [ ] MCP tools work correctly
- [ ] Ready for review/merge

---
*Check off tasks as you complete them. Update status field above.*
