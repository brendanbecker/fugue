# BUG-067: Mirror pane splits in wrong direction

**Priority**: P3
**Component**: mcp/mirror
**Severity**: low
**Status**: new

## Problem

`fugue_mirror_pane` with `direction: "vertical"` creates a horizontal split (top/bottom stacked panes) instead of a vertical split (side-by-side panes).

## Reproduction Steps

1. Have an active fugue session with a pane
2. Call `fugue_mirror_pane(source_pane_id: "<pane-id>", direction: "vertical")`
3. Observe the resulting layout

## Expected Behavior

`direction: "vertical"` should create side-by-side panes (left/right arrangement), consistent with the documentation and other fugue tools like `fugue_create_pane` and `fugue_split_pane`.

The MCP tool schema for `fugue_create_pane` states:
> "Split direction: 'vertical' creates side-by-side panes, 'horizontal' creates stacked panes"

## Actual Behavior

`direction: "vertical"` creates stacked panes (top/bottom arrangement), which is the opposite of the documented behavior.

## Root Cause Analysis

Likely one of two issues:
1. **Inverted logic**: The `handle_create_mirror` function may have the direction logic inverted compared to other split operations
2. **Inconsistent naming convention**: "vertical split" could be interpreted as either "split along vertical axis" (creating left/right) or "create vertical arrangement" (top/bottom)

## Relevant Code

- `fugue-server/src/mcp/bridge/handlers.rs` - MCP handler for mirror_pane
- `fugue-server/src/handlers/pane.rs` - `handle_create_mirror` implementation
- Compare with `handle_split_pane` for consistency

## Acceptance Criteria

- [ ] `direction: "vertical"` creates side-by-side (left/right) panes
- [ ] `direction: "horizontal"` creates stacked (top/bottom) panes
- [ ] Behavior matches `fugue_create_pane` and `fugue_split_pane` semantics
- [ ] Documentation accurately describes the behavior

## Impact

Minor usability issue. Users may get unexpected layouts when using mirror panes, but can work around it by specifying the opposite direction.

## Workarounds

Use the opposite direction value from what you intend:
- Want side-by-side? Use `direction: "horizontal"`
- Want stacked? Use `direction: "vertical"`

## Related

- FEAT-062: Original mirror pane implementation
- BUG-066: Mirror pane cross-session output (fixed)
