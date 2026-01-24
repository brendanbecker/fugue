# Implementation Plan: BUG-022

**Work Item**: [BUG-022: Viewport gets stuck above bottom after subagent finishes](PROMPT.md)
**Component**: tui
**Priority**: P2
**Created**: 2026-01-10

## Overview

When a Claude Code subagent finishes in fugue, the viewport sometimes doesn't render all the way to the bottom - it appears offset a few lines above where it should be. The issue is intermittent and doesn't happen every time.

## Architecture Decisions

- **Approach**: Fix scrollback synchronization in resize() and process_output() methods
- **Trade-offs**: Slightly more work on each resize/output (reading back scrollback value) for guaranteed state consistency

## Affected Components

| Component | Type of Change | Risk Level |
|-----------|----------------|------------|
| fugue-client/src/ui/pane.rs | Primary - resize() and process_output() | Low |

## Root Cause Analysis

The VT100 parser maintains its own scrollback state internally. When the terminal is resized, the parser may adjust scrollback positions based on the new dimensions. However, the Pane struct maintains a separate `scroll_offset` field that can become desynchronized from the parser's internal state.

**Desynchronization scenarios:**
1. Terminal resize changes parser's internal scrollback calculation
2. Pane's local `scroll_offset` doesn't reflect the parser's state
3. Next render uses the stale `scroll_offset`, causing viewport misalignment

## Fix Strategy

1. **Guard against unnecessary resizes**: Only resize when dimensions actually change
2. **Reset scrollback on resize**: When size changes, reset to bottom (scrollback=0)
3. **Sync local state**: Always read back the parser's scrollback value after modifications
4. **Apply same pattern to process_output**: Ensure consistency when new output arrives

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Breaking intentional scroll position | Low | Medium | Only reset on actual size change, not every frame |
| Performance impact from read-back | Very Low | Low | Single field read is negligible |
| Regression in existing functionality | Low | Medium | Existing scroll tests should catch issues |

## Rollback Strategy

If implementation causes issues:
1. Revert commits associated with this work item
2. Verify system returns to previous state
3. Document what went wrong in comments.md

## Implementation Notes

Fix has been implemented. Key changes:

1. `resize()` method now:
   - Checks if size actually changed before resizing
   - Resets scrollback to 0 after resize
   - Reads back parser state to sync local field

2. `process_output()` method now:
   - Reads back scrollback value after setting to 0

The fix ensures the Pane's `scroll_offset` field always reflects the parser's actual scrollback state, preventing viewport misalignment.

---
*Status: Fix implemented, awaiting user verification*
