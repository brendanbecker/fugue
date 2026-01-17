# Task Breakdown: BUG-049

**Work Item**: [BUG-049: send_input with submit: true doesn't reliably submit input](PROMPT.md)
**Status**: Not Started
**Last Updated**: 2026-01-16

## Prerequisites

- [ ] Read and understand PROMPT.md
- [ ] Review PLAN.md and update if needed
- [ ] Identify affected code paths

## Investigation Tasks

- [ ] Trace send_input flow from MCP handler to PTY write
- [ ] Identify where submit: true triggers Enter key
- [ ] Check if text write and Enter are sent atomically or sequentially
- [ ] Add debug logging to observe timing of write operations
- [ ] Document root cause in PLAN.md

## Implementation Tasks

- [ ] Implement fix for root cause
- [ ] Ensure text write completes before Enter key is sent
- [ ] Consider combining text + newline into single PTY write
- [ ] Add flush/sync if needed
- [ ] Self-review changes

## Testing Tasks

- [ ] Add unit test for send_input with submit: true
- [ ] Test with various input lengths
- [ ] Test rapid successive calls
- [ ] Test with different target applications (shells, CLIs)
- [ ] Run full test suite

## Verification Tasks

- [ ] Confirm submit: true reliably submits input
- [ ] Verify no regression in send_input without submit
- [ ] Verify all acceptance criteria from PROMPT.md
- [ ] Update bug_report.json status
- [ ] Document resolution in comments.md

## Completion Checklist

- [ ] All implementation tasks complete
- [ ] All tests passing
- [ ] PLAN.md updated with final approach
- [ ] Ready for review/merge

---
*Check off tasks as you complete them. Update status field above.*
