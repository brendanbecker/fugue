# Task Breakdown: BUG-013

**Work Item**: [BUG-013: Mouse Scroll Wheel Not Working for Scrollback](PROMPT.md)
**Status**: Not Started
**Last Updated**: 2026-01-10

## Prerequisites

- [ ] Read and understand PROMPT.md
- [ ] Review PLAN.md and update if needed
- [ ] Review FEAT-034 implementation in completed/

## Investigation Tasks

### Understand FEAT-034 Implementation

- [ ] Locate FEAT-034 files in `completed/features/FEAT-034-mouse-scroll-support/`
- [ ] Read FEAT-034 PROMPT.md to understand intended behavior
- [ ] Read FEAT-034 implementation code
- [ ] Identify the key code changes from FEAT-034
- [ ] Trace the expected event flow from scroll to viewport update

### Verify Mouse Capture is Enabled

- [ ] Find `EnableMouseCapture` in `ccmux-client/src/main.rs`
- [ ] Verify it includes scroll events
- [ ] Check if it's ever disabled during operation
- [ ] Verify crossterm version supports scroll events

### Debug Mouse Event Reception

- [ ] Add debug logging to event loop (temporary)
- [ ] Log all mouse events received
- [ ] Specifically log `MouseEvent::ScrollUp` and `MouseEvent::ScrollDown`
- [ ] Run ccmux and attempt to scroll
- [ ] Check logs to see if events are received

### Trace Event Handling Path

- [ ] Find where mouse events are dispatched in `input/mod.rs`
- [ ] Find the scroll event handler
- [ ] Check if scroll handler is registered/called
- [ ] Verify scroll handler updates viewport state
- [ ] Check if viewport state triggers re-render

### Check for Regressions

- [ ] Review git log for changes to input handling since FEAT-034
- [ ] Check if FEAT-035 modified any mouse-related code
- [ ] Look for any changes that might affect event routing
- [ ] Compare current code to FEAT-034 expected state

### Document Findings

- [ ] Identify exact failure point in event flow
- [ ] Document root cause in PLAN.md
- [ ] Determine fix approach

## Implementation Tasks

### Fix Mouse Event Capture (if needed)

- [ ] Ensure `EnableMouseCapture` is called correctly
- [ ] Verify mouse capture mode includes scroll events
- [ ] Fix any configuration issues

### Fix Event Routing (if needed)

- [ ] Ensure scroll events are matched in event handler
- [ ] Fix any missing match arms for scroll events
- [ ] Ensure scroll handler is called

### Fix Scroll Logic (if needed)

- [ ] Verify scroll direction calculation is correct
- [ ] Ensure viewport offset is updated
- [ ] Ensure scroll bounds are respected (can't scroll past end)
- [ ] Fix any state management issues

### Fix Viewport Rendering (if needed)

- [ ] Ensure viewport re-renders after scroll
- [ ] Verify tui-term widget receives updated offset
- [ ] Fix any rendering pipeline issues

### General Implementation

- [ ] Implement identified fix
- [ ] Remove debug logging (if added)
- [ ] Self-review changes

## Testing Tasks

### Manual Testing

- [ ] Test scroll wheel scrolls up through history
- [ ] Test scroll wheel scrolls down through history
- [ ] Test scroll at top boundary (can't scroll past start)
- [ ] Test scroll at bottom boundary (scrolls to end)
- [ ] Test scroll in pane with minimal output (edge case)
- [ ] Test scroll in pane with large scrollback buffer

### Multi-Pane Testing

- [ ] Test scroll in single pane layout
- [ ] Test scroll in vertical split (scroll active pane)
- [ ] Test scroll in horizontal split (scroll active pane)
- [ ] Test scroll targets correct pane when multiple exist

### Terminal Emulator Testing

- [ ] Test in WSL2 terminal
- [ ] Test in native Linux terminal (if available)
- [ ] Test in different terminal emulators (if available)

### Regression Testing

- [ ] Verify keyboard scroll (Ctrl+PageUp/Down) still works
- [ ] Verify other mouse functionality (if any) still works
- [ ] Verify pane switching still works
- [ ] Run full test suite

## Verification Tasks

- [ ] Confirm mouse scroll works as expected
- [ ] Confirm scroll direction is correct (wheel up = scroll up in history)
- [ ] Confirm scroll speed is reasonable
- [ ] All acceptance criteria from PROMPT.md met
- [ ] Update bug_report.json status
- [ ] Document resolution in PLAN.md
- [ ] Check if BUG-012 shares root cause

## Completion Checklist

- [ ] All investigation tasks complete
- [ ] Root cause identified and documented
- [ ] All implementation tasks complete
- [ ] All tests passing
- [ ] Debug logging removed
- [ ] PLAN.md updated with final approach
- [ ] Ready for review/merge

---
*Check off tasks as you complete them. Update status field above.*
