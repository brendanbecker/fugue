# Task Breakdown: BUG-048

**Work Item**: [BUG-048: TUI flickers on every keystroke when Claude Code is detected](PROMPT.md)
**Status**: Not Started
**Last Updated**: 2026-01-16

## Prerequisites

- [ ] Read and understand PROMPT.md
- [ ] Review PLAN.md and update if needed
- [ ] Understand the agent detection flow

## Investigation Tasks

- [x] Reproduce the bug consistently
- [x] Identify root cause
- [x] Document affected code in PLAN.md
- [x] Determine fix approach

## Implementation Tasks

- [ ] Modify `analyze()` in `fugue-server/src/agents/claude/mod.rs`
- [ ] Change from ignoring `inner.analyze()` return value to using it
- [ ] Self-review changes

## Testing Tasks

- [ ] Add unit test: `analyze()` returns `None` on repeated calls without state change
- [ ] Add unit test: `analyze()` returns `Some` only on state transitions
- [ ] Run existing test suite to check for regressions
- [ ] Manual test: no flicker when typing in Claude Code pane

## Verification Tasks

- [ ] Confirm TUI no longer flickers on keystrokes
- [ ] Verify agent state still updates on real changes
- [ ] Update bug_report.json status
- [ ] Document resolution in comments.md

## Completion Checklist

- [ ] All implementation tasks complete
- [ ] All tests passing
- [ ] PLAN.md updated with final approach
- [ ] Ready for review/merge

---
*Check off tasks as you complete them. Update status field above.*
