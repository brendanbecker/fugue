# Task Breakdown: BUG-045

**Work Item**: [BUG-045: Windows rendered as horizontal splits instead of separate tabs/screens](PROMPT.md)
**Status**: Not Started
**Last Updated**: 2026-01-16

## Prerequisites

- [ ] Read and understand PROMPT.md
- [ ] Review PLAN.md and update if needed
- [ ] Identify affected code paths in layout/rendering

## Investigation Tasks

- [ ] Locate the layout calculation code in the TUI module
- [ ] Identify where panes are collected for rendering
- [ ] Find the "active window" state tracking (if it exists)
- [ ] Trace what `select_window` currently does
- [ ] Document how pane dimensions are calculated (is window considered?)
- [ ] Check if FEAT-082 adaptive layout has relevant code

## Implementation Tasks

- [ ] Add/verify active window tracking per session
- [ ] Filter panes to active window in layout calculation
- [ ] Update layout dimensions to use full screen for single window
- [ ] Ensure `create_window` sets proper window state
- [ ] Verify `select_window` triggers re-render with new window's panes
- [ ] Self-review changes

## Testing Tasks

- [ ] Test: create_window does not split view
- [ ] Test: only active window panes are visible
- [ ] Test: select_window switches rendered content
- [ ] Test: pane splits within a window still work
- [ ] Test: multiple windows in multiple sessions work correctly
- [ ] Test: window switching preserves pane layouts
- [ ] Run full test suite

## Verification Tasks

- [ ] Confirm expected behavior matches tmux window model
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
