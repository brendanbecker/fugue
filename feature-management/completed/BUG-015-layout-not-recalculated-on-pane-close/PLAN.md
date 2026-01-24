# Implementation Plan: BUG-015

**Work Item**: [BUG-015: Layout Doesn't Recalculate When Panes Are Closed](PROMPT.md)
**Component**: fugue-client
**Priority**: P2
**Created**: 2026-01-10

## Overview

When panes are closed in fugue, the remaining panes do not expand to fill the available space. The layout tree is not recalculated or simplified when nodes are removed, leaving dead space in the window.

## Architecture Decisions

### Approach: To Be Determined

After investigation, choose from:

1. **Layout Tree Pruning**: When a pane is removed, prune unnecessary parent nodes and redistribute space
2. **Full Layout Recalculation**: Rebuild the entire layout from remaining panes
3. **Sibling Expansion**: Directly expand sibling pane(s) to fill the vacated space
4. **Server-Side Layout Management**: Move layout calculations to server, broadcast new dimensions

### Trade-offs

| Option | Pros | Cons |
|--------|------|------|
| Tree Pruning | Minimal changes, efficient | Requires careful tree manipulation |
| Full Rebuild | Simple, guaranteed correct | May be inefficient, loses position info |
| Sibling Expansion | Direct fix for symptom | May not handle all edge cases |
| Server-Side | Single source of truth | Requires protocol changes |

**Decision**: TBD after investigation identifies current architecture.

## Layout Tree Analysis

### Expected Structure

A typical quadrant layout would look like:

```
Root (Vertical Split)
├── Left (Horizontal Split)
│   ├── Top-Left Pane
│   └── Bottom-Left Pane
└── Right (Horizontal Split)
    ├── Top-Right Pane
    └── Bottom-Right Pane
```

### After Closing 3 Panes

If only Top-Left remains, the tree should simplify to:

```
Root (Single Pane)
└── Top-Left Pane (now fills entire window)
```

But currently it may remain as:

```
Root (Vertical Split) - 50%/50%
├── Left (Horizontal Split) - 50%/50%
│   ├── Top-Left Pane (only remaining)
│   └── (empty - was Bottom-Left)
└── (empty - was Right)
```

## Files to Investigate

| File | Purpose | Risk Level |
|------|---------|------------|
| `fugue-client/src/ui/app.rs` | Main TUI application, message handling | High |
| `fugue-client/src/ui/layout.rs` | Layout tree (if exists) | Critical |
| `fugue-client/src/ui/panes.rs` | Pane management (if exists) | High |
| `fugue-client/src/ui/mod.rs` | UI module organization | Medium |
| Protocol messages for PaneClosed | How closure is communicated | Medium |

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Incorrect tree manipulation | Medium | High | Thorough testing with various layouts |
| Resize not propagated to server | Medium | Medium | Ensure resize messages sent |
| Edge case with last pane | Low | High | Special case handling |
| Regression in split behavior | Low | Medium | Test split/close cycles |

## Rollback Strategy

If implementation causes issues:
1. Revert commits associated with this work item
2. The partial-size pane behavior was the baseline - reverting returns to that
3. Consider alternative approach based on lessons learned

## Implementation Notes

<!-- Add notes during implementation -->

### Investigation Findings

*To be filled during investigation*

### Root Cause

*To be identified*

### Chosen Solution

*To be determined after investigation*

---
*This plan should be updated as implementation progresses.*
