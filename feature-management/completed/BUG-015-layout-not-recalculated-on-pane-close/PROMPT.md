# BUG-015: Layout Doesn't Recalculate When Panes Are Closed

**Priority**: P2 (Medium)
**Component**: ccmux-client
**Status**: new
**Created**: 2026-01-10

## Summary

When multiple panes exist (e.g., quadrant layout with 4 panes) and some panes are closed, the remaining pane(s) do not expand to fill the available window space. Instead, the remaining pane stays at its previous size (e.g., top half of window) leaving empty/unused space.

## Symptoms

- Remaining pane stays at partial size after other panes are closed
- Empty/dead space visible in the window where closed panes were
- Layout tree is not recalculated or simplified when nodes are removed
- Workaround: restart ccmux to restore full-window pane

## Steps to Reproduce

1. Start ccmux with a single pane (fills full window)
2. Create a vertical split (2 side-by-side panes)
3. Create horizontal splits on each side (4 quadrant panes)
4. Close 3 of the 4 panes
5. Observe: remaining pane only occupies its quadrant area instead of expanding to fill full window

## Expected Behavior

When panes are closed, the layout should recalculate and remaining panes should expand to fill available space, similar to how tmux handles pane closure. The layout tree should be pruned/simplified when nodes are removed.

## Actual Behavior

Remaining pane stays at partial size, leaving empty/dead space in the window. The layout tree is not being recalculated when panes are removed.

## Relationship to Other Components

The TUI layout system in ccmux-client handles pane arrangement. Key areas to investigate:

1. **Layout Tree Management**: How the layout tree is structured and updated
2. **Pane Close Handler**: What happens when a pane is closed (PaneClosed message)
3. **Layout Recalculation**: Whether/when layout recalculation is triggered
4. **Tree Pruning**: Logic for simplifying the tree when nodes are removed

## Data Flow to Investigate

When a pane is closed:

```
PaneClosed Message -> Client Handler -> Remove Pane from Tree -> (Missing?) Recalculate Layout
```

The issue is likely that the layout recalculation step is missing or incomplete.

## Areas to Investigate

### 1. Client Pane Close Handling

How does the client handle `PaneClosed` messages? Does it:
- Remove the pane from the layout tree?
- Trigger a layout recalculation?
- Simplify the tree (merge parent nodes when only one child remains)?

**Files to check:**
- `ccmux-client/src/ui/app.rs` - Main application handling
- `ccmux-client/src/ui/layout.rs` - Layout tree implementation (if exists)
- `ccmux-client/src/ui/panes.rs` - Pane management (if exists)

### 2. Layout Tree Structure

How is the layout tree structured?
- Binary tree with Split nodes?
- Does removing a leaf properly update parent nodes?
- Is there logic to collapse unnecessary split nodes?

### 3. Resize Propagation

When a pane is removed:
- Are the remaining panes resized?
- Is the server notified of new pane dimensions?
- Is the terminal size updated for remaining PTYs?

### 4. Comparison with tmux

tmux handles this correctly:
- When a pane is closed, sibling panes expand to fill the space
- The layout tree is automatically simplified
- No dead space remains

## Acceptance Criteria

- [ ] Root cause identified and documented
- [ ] Remaining panes expand to fill available space when other panes are closed
- [ ] Layout tree is properly pruned when nodes are removed
- [ ] Server is notified of updated pane dimensions
- [ ] PTY receives resize signal for new dimensions
- [ ] No dead/empty space remains after closing panes
- [ ] Works correctly for any pane configuration (not just quadrants)
- [ ] Add test case to prevent regression

## Implementation Tasks

### Section 1: Investigation

- [ ] Reproduce the bug with multiple pane layouts
- [ ] Trace what happens when a pane is closed
- [ ] Identify where layout recalculation should occur
- [ ] Document root cause in PLAN.md

### Section 2: Fix Implementation

- [ ] Implement layout tree pruning when panes are removed
- [ ] Trigger layout recalculation after pane removal
- [ ] Ensure remaining panes expand to fill available space
- [ ] Send resize messages to server for affected panes
- [ ] Handle edge cases (last pane, deeply nested layouts)

### Section 3: Testing

- [ ] Add unit test for layout tree pruning
- [ ] Add test for layout recalculation on pane close
- [ ] Manual test with various pane configurations
- [ ] Verify no dead space remains after closing panes

### Section 4: Verification

- [ ] Confirm remaining panes fill available space
- [ ] Confirm server receives updated dimensions
- [ ] All acceptance criteria met
- [ ] Update bug_report.json with resolution details

## Notes

This is a P2 bug because:
- It is a functional issue affecting usability
- There is a workaround (restart ccmux)
- It does not cause crashes or data loss
- It primarily affects users who frequently open and close panes

The fix should:
1. Properly prune the layout tree when panes are removed
2. Recalculate and redistribute space among remaining panes
3. Follow tmux's behavior as the expected standard
