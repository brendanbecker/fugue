# Task Breakdown: BUG-022

**Work Item**: [BUG-022: Viewport gets stuck above bottom after subagent finishes](PROMPT.md)
**Status**: In Progress
**Last Updated**: 2026-01-10

## Prerequisites

- [x] Read and understand PROMPT.md
- [x] Review PLAN.md and update if needed
- [x] Identify affected code paths

## Investigation Tasks

- [x] Reproduce the bug consistently
- [x] Identify root cause (scrollback sync issue on resize)
- [x] Document affected code in PLAN.md
- [x] Determine fix approach (guard + reset + sync)

## Implementation Tasks

- [x] Implement fix for root cause in resize()
- [x] Implement fix for root cause in process_output()
- [x] Add comments explaining the fix
- [x] Self-review changes

## Testing Tasks

- [ ] Add unit tests to prevent regression
- [ ] Test fix in affected scenarios (subagent completion)
- [ ] Test fix with split pane operations
- [ ] Test fix with window resize operations
- [ ] Verify no side effects in scroll functionality
- [ ] Run full test suite

## Verification Tasks

- [ ] Confirm expected behavior is restored (user testing)
- [ ] Verify all acceptance criteria from PROMPT.md
- [ ] Update bug_report.json status when verified
- [ ] Document resolution in comments.md

## Completion Checklist

- [x] All implementation tasks complete
- [ ] All tests passing
- [x] PLAN.md updated with final approach
- [ ] Ready for review/merge (awaiting user verification)

---
*Check off tasks as you complete them. Update status field above.*
