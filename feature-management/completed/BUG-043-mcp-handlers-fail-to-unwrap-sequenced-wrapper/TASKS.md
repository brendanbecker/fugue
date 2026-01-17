# Task Breakdown: BUG-043

**Work Item**: [BUG-043: MCP tool handlers fail to unwrap Sequenced message wrapper](PROMPT.md)
**Status**: Not Started
**Last Updated**: 2026-01-16

## Prerequisites

- [ ] Read and understand PROMPT.md
- [ ] Review PLAN.md and update if needed
- [ ] Locate recv_response_from_daemon() in connection.rs
- [ ] Understand ServerMessage::Sequenced structure

## Investigation Tasks

- [ ] Verify ServerMessage::Sequenced enum variant exists
- [ ] Confirm is_broadcast_message() missing Sequenced handling
- [ ] Trace message flow: daemon -> bridge -> handler
- [ ] Identify all callers of recv_response_from_daemon()
- [ ] Review FEAT-075 for Sequenced wrapper purpose

## Implementation Tasks

- [ ] Modify recv_response_from_daemon() to unwrap Sequenced
- [ ] Handle potential nested Sequenced (if applicable)
- [ ] Add debug logging for wrapped/unwrapped tracking (optional)
- [ ] Update is_broadcast_message() if needed for Sequenced variant
- [ ] Self-review changes

## Testing Tasks

- [ ] Test kill_session - should return success/error, not Sequenced
- [ ] Test set_tags - should return TagsList
- [ ] Test get_tags - should return TagsList
- [ ] Test broadcast - should return Ok
- [ ] Test list_sessions - should return SessionList
- [ ] Test beads_assign - should work correctly
- [ ] Test beads_find_pane - should work correctly
- [ ] Test beads_pane_history - should work correctly
- [ ] Verify create_pane still works (regression check)
- [ ] Verify close_pane still works (regression check)
- [ ] Verify list_panes still works (regression check)
- [ ] Verify read_pane still works (regression check)
- [ ] Test with persistence enabled
- [ ] Test WAL replay still works

## Verification Tasks

- [ ] All affected tools return correct response types
- [ ] No regression in previously working tools
- [ ] Persistence/WAL tracking unaffected
- [ ] Update bug_report.json status
- [ ] Document resolution in comments.md (if created)

## Completion Checklist

- [ ] All implementation tasks complete
- [ ] All tests passing
- [ ] PLAN.md updated with final approach
- [ ] Ready for review/merge

---
*Check off tasks as you complete them. Update status field above.*
