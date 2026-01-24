# Implementation Plan: BUG-040

**Work Item**: [BUG-040: fugue_create_window returns success but doesn't actually create windows](PROMPT.md)
**Component**: mcp
**Priority**: P1
**Created**: 2026-01-11

## Overview

The `fugue_create_window` MCP tool returns a successful response with `window_id` and `pane_id`, but the window is not actually created. This is suspected to be a regression from the BUG-034 fix which modified how `create_window` handles the session parameter.

## Architecture Decisions

- **Approach**: Trace the create_window code path from MCP handler through session manager to identify where the window is lost
- **Key Insight**: BUG-034 fix (commit 3e14861) modified both `handlers.rs` and `manager.rs` - the bug is likely in the interaction between these components
- **Trade-offs**: Need to ensure fix doesn't break BUG-034 fix (session parameter handling)

## Affected Components

| Component | Type of Change | Risk Level |
|-----------|----------------|------------|
| fugue-server/src/mcp/handlers.rs | Primary Investigation | Medium |
| fugue-server/src/session/manager.rs | Primary Investigation | Medium |

## Dependencies

None - this is a standalone bug fix.

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Breaking BUG-034 fix while fixing this | Medium | High | Ensure session parameter tests still pass |
| Regression in other window operations | Low | Medium | Run full MCP test suite |
| Window state inconsistency | Low | High | Verify window lifecycle end-to-end |

## Investigation Strategy

1. **Trace create_window path**:
   - Start at MCP handler in `handlers.rs`
   - Follow through to session manager in `manager.rs`
   - Identify where window is created vs where it should be persisted

2. **Compare with BUG-034 changes**:
   - Review diff from commit 3e14861
   - Identify what changed in window creation logic
   - Look for missing persistence step

3. **Verify window state management**:
   - Check if window is added to session's windows collection
   - Verify window_count calculation
   - Confirm list_windows query includes new windows

## Rollback Strategy

If fix causes issues:
1. Revert commits associated with this work item
2. Consider reverting BUG-034 fix if tightly coupled
3. Re-implement both fixes together with proper persistence

## Implementation Notes

<!-- Add notes during implementation -->

---
*This plan should be updated as implementation progresses.*
