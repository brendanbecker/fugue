# BUG-045: Windows rendered as horizontal splits instead of separate tabs/screens

**Priority**: P2
**Component**: tui, layout-engine
**Severity**: medium
**Status**: new

## Problem Statement

When creating a new window with `create_window`, it appears as a horizontal split in the current view rather than as a separate tab/screen. In tmux, windows are like tabs - only one window is visible at a time per session, and you switch between them. In ccmux, creating a new window causes both the original window and the new window to be displayed simultaneously, splitting the screen.

This violates the expected tmux-like window/pane mental model where:
- **Windows** are independent screens (like browser tabs) - only one visible at a time
- **Panes** are splits within a window - multiple visible simultaneously

## Evidence

```json
// list_panes shows both panes with rows=22 (half of 46 total)
{
  "id": "b2f13e35...",
  "window": 0,
  "window_name": "0",
  "rows": 22  // Half height - should be full height if only window visible
},
{
  "id": "2d95cd9c...",
  "window": 1,
  "window_name": "qa-test",
  "rows": 22  // Half height - should not be visible at all
}
```

## Steps to Reproduce

1. Start ccmux with a single session and window
2. Call `ccmux_create_window` with name "qa-test"
3. Observe that the new window's pane appears as a split in the current view
4. Both windows are now visible simultaneously, each taking half the screen height

## Expected Behavior

- Creating a new window should create a separate screen/tab
- Only one window should be visible at a time
- User should switch between windows using `select_window`
- Windows should be like tmux windows (tabs), not splits
- Each window should have access to the full screen height

## Actual Behavior

- New window appears as a horizontal split
- Both windows are visible simultaneously
- Screen is divided between all windows in the session
- Behaves like `create_pane` with direction=horizontal

## Root Cause Analysis

The layout engine appears to be rendering all windows in a session simultaneously rather than only the active window. Investigation should focus on:

1. **Layout calculation**: How does the layout engine determine which panes to render?
2. **Window filtering**: Is there logic to filter panes to only the active window?
3. **Active window tracking**: Is the "active window" concept properly tracked and used?

## Implementation Tasks

### Section 1: Investigation
- [ ] Trace the layout calculation in the TUI rendering code
- [ ] Identify where panes are collected for rendering
- [ ] Find where (or if) active window filtering should occur
- [ ] Review how `select_window` is supposed to change what's rendered

### Section 2: Fix Implementation
- [ ] Implement filtering to only render panes from the active window
- [ ] Ensure layout calculations use only active window's panes
- [ ] Update `create_window` to set new window as active (or not, based on design)
- [ ] Verify `select_window` properly changes the rendered window

### Section 3: Testing
- [ ] Test creating multiple windows - only active should be visible
- [ ] Test `select_window` switches rendered content
- [ ] Test pane creation within a window still splits correctly
- [ ] Verify layout recalculation on window switch

### Section 4: Verification
- [ ] Confirm tmux-like behavior (windows as tabs)
- [ ] Verify all acceptance criteria met
- [ ] Update bug report with resolution details

## Acceptance Criteria

- [ ] Creating a new window does not split the current view
- [ ] Only panes from the active window are rendered
- [ ] `select_window` changes which window's panes are displayed
- [ ] Each window can use the full screen space
- [ ] Pane splits within a window work correctly
- [ ] No regression in existing pane/layout functionality

## Related Items

- **FEAT-082** (adaptive layout engine) - may need to consider window-awareness in layout calculations

## Notes

This bug represents a fundamental difference in the mental model for windows. The current behavior essentially treats windows as automatic horizontal splits, which:
1. Defeats the purpose of having windows vs panes
2. Reduces usable space for each window
3. Complicates agent orchestration that expects to work in isolated windows
