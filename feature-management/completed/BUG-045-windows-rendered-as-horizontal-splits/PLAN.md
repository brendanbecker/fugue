# Implementation Plan: BUG-045

**Work Item**: [BUG-045: Windows rendered as horizontal splits instead of separate tabs/screens](PROMPT.md)
**Component**: tui, layout-engine
**Priority**: P2
**Created**: 2026-01-16

## Overview

The TUI layout engine is rendering all windows in a session simultaneously instead of only the active window. This causes new windows to appear as horizontal splits rather than separate "tabs" that the user switches between.

## Architecture Decisions

### Key Design Choice: Window Visibility Model

**Decision**: Only the active window's panes should be rendered at any given time.

**Rationale**:
- Matches tmux mental model (windows = tabs, panes = splits)
- Maximizes screen real estate per window
- Enables clean window isolation for agent workflows
- Consistent user expectations from terminal multiplexer experience

**Trade-offs**:
- Cannot see multiple windows simultaneously (by design - use panes for that)
- Requires explicit window switching via `select_window`

### Implementation Approach

Two potential approaches:

1. **Filter at render time**: In the TUI render loop, filter panes to only include those belonging to the active window
2. **Filter at layout calculation time**: When calculating layouts, only consider panes from the active window

**Recommended**: Option 2 - Filter at layout calculation time. This ensures:
- Layout calculations are based on correct pane set
- Cleaner separation of concerns
- Pane dimensions accurately reflect available space

## Affected Components

| Component | Type of Change | Risk Level |
|-----------|----------------|------------|
| `tui/` | Layout calculation filtering | Medium |
| `session/` or `state/` | Active window tracking | Low |
| `layout-engine` | Pane collection/filtering | Medium |
| `mcp/` handlers | Potential window switch on create | Low |

## Investigation Areas

Key code areas to examine:

1. **Pane collection for layout**: Where are panes gathered for layout calculation?
2. **Active window state**: How is the "active window" tracked per session?
3. **Layout algorithm**: Does the layout algorithm assume all panes should be rendered?
4. **Window creation**: Does `create_window` properly set up window state?

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Regression in pane rendering | Medium | High | Comprehensive testing of pane splits |
| Active window state inconsistency | Low | Medium | Clear state management pattern |
| Performance impact from filtering | Low | Low | Filtering is O(n) for small n |
| Break existing layouts | Medium | High | Test existing layout scenarios |

## Rollback Strategy

If implementation causes issues:
1. Revert commits associated with this work item
2. Verify system returns to previous state (all windows visible)
3. Document what went wrong in comments.md

## Implementation Notes

<!-- Add notes during implementation -->

### Suspected Code Locations

- Layout calculation: likely in `tui/` or a dedicated layout module
- Active window tracking: session state management
- Pane iteration: wherever `panes` are collected for rendering

### Questions to Answer

1. Is there already an "active window" concept in the codebase?
2. How does `select_window` currently work, and what does it update?
3. Are there any existing tests for multi-window scenarios?

---
*This plan should be updated as implementation progresses.*
