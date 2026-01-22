# BUG-068: ccmux_focus_pane returns AbortError

**Priority**: P2
**Component**: mcp/pane
**Severity**: medium
**Status**: new

## Problem

The `ccmux_focus_pane` MCP tool fails with an `AbortError` when attempting to focus on a valid pane.

## Reproduction Steps

1. Have an active ccmux session with multiple panes
2. Get the UUID of a pane (e.g., via `ccmux_list_panes`)
3. Call `ccmux_focus_pane(pane_id: "<valid-pane-uuid>")`
4. Observe the error

## Expected Behavior

The specified pane should become focused (active), and the tool should return success.

## Actual Behavior

```
MCP error -32001: AbortError: The operation was aborted.
```

## Context

- The pane being focused was a mirror pane, but the pane existed and was valid
- Attempted before performing a split operation on the pane
- The pane UUID was correct and the pane was visible in `ccmux_list_panes`

## Root Cause Analysis

Possible causes:
1. **Mirror pane limitation**: Focus operations may not be implemented/supported for mirror panes
2. **Race condition**: The focus operation may be timing out or conflicting with other operations
3. **Missing handler**: The MCP handler may be incomplete or not properly wired up
4. **Session context**: The focus operation may require specific session attachment

## Relevant Code

- `ccmux-server/src/mcp/bridge/handlers.rs` - MCP handler for focus_pane
- `ccmux-server/src/handlers/pane.rs` - Pane handler implementation
- `ccmux-server/src/mcp/tools.rs` - Tool schema definition

## Acceptance Criteria

- [ ] `ccmux_focus_pane` succeeds for regular panes
- [ ] `ccmux_focus_pane` succeeds for mirror panes (or returns clear error if unsupported)
- [ ] Error messages are descriptive rather than generic "AbortError"
- [ ] Tool returns meaningful response on success

## Impact

Medium severity - prevents programmatic pane management. Orchestrators cannot focus specific panes before performing operations, affecting workflow automation.

## Workarounds

1. Use `ccmux_select_window` to switch windows instead
2. Perform operations directly on panes without explicit focus
3. Use keyboard navigation in the TUI client

## Related

- BUG-059: Mirror pane AbortError (may be related pattern)
- FEAT-062: Mirror pane implementation
