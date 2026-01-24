# BUG-051: Split pane direction parameter has no effect - always creates horizontal panes

**Priority**: P1
**Component**: mcp-handlers
**Severity**: medium
**Status**: new

## Problem Statement

When using `fugue_split_pane` with `direction: "vertical"` or `direction: "horizontal"`, both produce the same result - horizontally stacked panes (one above the other). The direction parameter appears to have no effect on the actual layout.

## Evidence

| Direction Parameter | Expected Layout | Actual Layout |
|---------------------|-----------------|---------------|
| `horizontal` | Side-by-side (left \| right) OR top/bottom | top/bottom |
| `vertical` | Top/bottom OR side-by-side (left \| right) | top/bottom |

Both direction values produce identical results, indicating the parameter is being ignored.

## Steps to Reproduce

1. Create a new session
2. Split the pane with `direction: "horizontal"`
3. Observe panes are stacked horizontally (top/bottom)
4. Kill session and recreate
5. Split the pane with `direction: "vertical"`
6. Observe panes are STILL stacked horizontally (top/bottom)

## Expected Behavior

The direction parameter should control the split orientation:
- One direction should create side-by-side panes (left | right) - the split line is vertical
- The other direction should create stacked panes (top / bottom) - the split line is horizontal

The two options should produce visually different layouts.

## Actual Behavior

Both direction values (horizontal and vertical) produce horizontally stacked panes (top/bottom layout). The direction parameter is ignored.

## Root Cause

To be determined. Possible causes:
- MCP handler ignores the direction parameter
- Direction parameter not passed through to PTY/layout system
- Hardcoded default split direction in layout engine
- Parameter mapping issue between MCP API and underlying split logic
- Direction enum values swapped or not implemented

## Implementation Tasks

### Section 1: Investigation
- [ ] Trace direction parameter flow from MCP handler through to layout system
- [ ] Check MCP handler implementation for `split_pane` command
- [ ] Check PTY/layout system for how split direction is applied
- [ ] Identify where the direction parameter is being lost or ignored
- [ ] Document the expected vs actual code path

### Section 2: Fix Implementation
- [ ] Implement fix for root cause
- [ ] Ensure both direction values produce different layouts
- [ ] Add validation for direction parameter values
- [ ] Update any related documentation if naming was confusing

### Section 3: Testing
- [ ] Manual test: split with horizontal direction
- [ ] Manual test: split with vertical direction
- [ ] Verify visually distinct layouts for each direction
- [ ] Test direction parameter with fugue_create_pane if it also has this parameter

### Section 4: Verification
- [ ] Confirm both directions work correctly
- [ ] Verify no regression in existing split functionality
- [ ] Update bug report with resolution details

## Acceptance Criteria

- [ ] `direction: "horizontal"` produces one layout (e.g., side-by-side)
- [ ] `direction: "vertical"` produces a different layout (e.g., top/bottom)
- [ ] The two layouts are visually distinct and match documented behavior
- [ ] No regression in pane creation functionality

## Notes

This bug may be related to BUG-045 (windows rendered as horizontal splits) which was about window rendering, but this is specifically about the split_pane direction parameter being ignored. The fix for BUG-045 addressed window rendering, not the direction parameter for splits.

The naming convention for horizontal vs vertical splits can be confusing:
- "Horizontal split" often means the split LINE is horizontal (creating top/bottom panes)
- "Vertical split" often means the split LINE is vertical (creating left/right panes)

The fix should ensure the implementation matches the documented/intended semantics.
