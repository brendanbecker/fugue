# Session: Stream B - Layout Recalculation

## Work Item
- **BUG-015**: Layout doesn't recalculate when panes are closed

## Priority: P2 (Medium)

## Problem Summary

When multiple panes exist (e.g., quadrant layout with 4 panes) and some panes are closed, the remaining pane(s) do not expand to fill the available window space. Instead, the remaining pane stays at its previous size (e.g., top half of window) leaving empty/unused space.

## Steps to Reproduce

1. Start ccmux with a single pane (fills full window)
2. Create a vertical split (2 side-by-side panes)
3. Create horizontal splits on each side (4 quadrant panes)
4. Close 3 of the 4 panes
5. Observe: remaining pane only occupies its quadrant area instead of expanding

## Expected Behavior

When panes are closed, the layout should recalculate and remaining panes should expand to fill available space, similar to tmux.

## Files to Investigate

- `ccmux-client/src/ui/app.rs` - Main application handling, PaneClosed handler
- `ccmux-client/src/ui/layout.rs` - Layout tree implementation
- `ccmux-client/src/ui/panes.rs` - Pane management

## Implementation Tasks

1. **Investigation**: Trace what happens when a pane is closed
2. **Fix**: Implement layout tree pruning when panes are removed
3. **Fix**: Trigger layout recalculation after pane removal
4. **Fix**: Send resize messages to server for affected panes
5. **Test**: Add unit tests for layout tree pruning

## Acceptance Criteria

- [ ] Remaining panes expand to fill available space when other panes are closed
- [ ] Layout tree is properly pruned when nodes are removed
- [ ] Server is notified of updated pane dimensions
- [ ] PTY receives resize signal for new dimensions
- [ ] No dead/empty space remains after closing panes

## Related Work Items

- See `feature-management/bugs/BUG-015-layout-not-recalculated-on-pane-close/PROMPT.md`

## Commands

```bash
# Build
cargo build --release

# Run tests
cargo test --workspace

# Run ccmux for testing
./target/release/ccmux
```
