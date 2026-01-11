# Task Breakdown: BUG-015

**Work Item**: [BUG-015: Layout Doesn't Recalculate When Panes Are Closed](PROMPT.md)
**Status**: Not Started
**Last Updated**: 2026-01-10

## Prerequisites

- [ ] Read and understand PROMPT.md
- [ ] Review PLAN.md and update if needed
- [ ] Understand ccmux layout tree structure

## Investigation Tasks

### Reproduce the Bug

- [ ] Start ccmux with a single pane
- [ ] Create vertical split (2 panes)
- [ ] Create horizontal splits on each side (4 quadrants)
- [ ] Close 3 panes one by one
- [ ] Confirm remaining pane stays at partial size
- [ ] Note exact behavior (which quadrant stays, size)

### Trace Pane Close Handling

- [ ] Find where `PaneClosed` message is handled in client
- [ ] Trace what happens to the layout tree when pane is removed
- [ ] Identify if layout recalculation is triggered
- [ ] Identify if tree pruning occurs
- [ ] Check if resize messages are sent to server

### Understand Layout Architecture

- [ ] Read layout tree implementation
- [ ] Understand how splits are represented
- [ ] Understand how pane dimensions are calculated
- [ ] Identify where layout recalculation code exists (or should exist)

### Code Analysis

- [ ] Review `ccmux-client/src/ui/app.rs` for pane close handling
- [ ] Review layout module for tree management
- [ ] Check for existing tree pruning logic
- [ ] Check for existing recalculation logic
- [ ] Document findings in PLAN.md

### Root Cause Determination

- [ ] Confirm root cause from investigation
- [ ] Document root cause in PLAN.md
- [ ] Choose fix approach
- [ ] Update PLAN.md with chosen solution

## Implementation Tasks

### Layout Tree Pruning

- [ ] Implement logic to detect when a split node has only one child
- [ ] Replace split node with its single remaining child
- [ ] Handle recursive pruning (multiple levels)
- [ ] Preserve pane positions relative to remaining panes

### Layout Recalculation

- [ ] Trigger layout recalculation after pane removal
- [ ] Ensure remaining panes expand to fill available space
- [ ] Calculate new dimensions for all affected panes
- [ ] Handle edge cases (deeply nested, uneven splits)

### Server Communication

- [ ] Send resize messages to server for affected panes
- [ ] Ensure PTY receives new dimensions
- [ ] Handle multiple panes being resized simultaneously

### Edge Cases

- [ ] Handle closing the last pane (session should end?)
- [ ] Handle closing all panes on one side of a split
- [ ] Handle deeply nested layouts
- [ ] Handle rapid consecutive pane closures

### General Implementation

- [ ] Implement chosen fix
- [ ] Self-review changes
- [ ] Ensure no regressions in existing functionality

## Testing Tasks

### Unit Tests

- [ ] Add test for layout tree pruning single child
- [ ] Add test for layout recalculation after removal
- [ ] Add test for dimension calculation after removal
- [ ] Add test for nested tree simplification

### Integration Tests

- [ ] Add test for pane close triggering layout update
- [ ] Add test for resize message sent after close
- [ ] Add test for multiple pane close sequence

### Manual Testing

- [ ] Test quadrant layout -> close 3 panes
- [ ] Test vertical split -> close one pane
- [ ] Test horizontal split -> close one pane
- [ ] Test complex nested layout -> close various panes
- [ ] Verify no dead space in any scenario
- [ ] Verify remaining panes fill window correctly
- [ ] Test with different terminal sizes

### Regression Testing

- [ ] Run full test suite
- [ ] Verify no existing tests broken
- [ ] Verify pane splitting still works correctly
- [ ] Verify pane resizing still works correctly

## Verification Tasks

- [ ] Confirm remaining panes expand on close
- [ ] Confirm no dead space in window
- [ ] Confirm server receives updated dimensions
- [ ] Confirm PTY gets resize signal
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
