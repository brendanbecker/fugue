# Task Breakdown: BUG-051

**Work Item**: [BUG-051: Split pane direction parameter has no effect](PROMPT.md)
**Status**: Not Started
**Last Updated**: 2026-01-17

## Prerequisites

- [ ] Read and understand PROMPT.md
- [ ] Review PLAN.md and update if needed
- [ ] Identify affected code paths

## Investigation Tasks

- [ ] Locate MCP handler for `split_pane` command
- [ ] Trace direction parameter from MCP handler
- [ ] Check protocol message definition for SplitPane
- [ ] Check daemon handler for split_pane requests
- [ ] Identify where direction parameter is lost/ignored
- [ ] Document findings in PLAN.md

## Implementation Tasks

- [ ] Fix the root cause of direction being ignored
- [ ] Ensure direction enum values map correctly
- [ ] Verify both directions produce different layouts
- [ ] Add any missing direction handling logic
- [ ] Self-review changes

## Testing Tasks

- [ ] Test split_pane with direction="horizontal"
- [ ] Test split_pane with direction="vertical"
- [ ] Verify the two directions produce different layouts
- [ ] Test ccmux_create_pane direction parameter (if applicable)
- [ ] Run existing test suite

## Verification Tasks

- [ ] Confirm expected behavior is restored
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
