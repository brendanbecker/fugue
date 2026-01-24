# BUG-062: fugue_close_pane times out for mirror panes

**Priority**: P2
**Component**: mcp
**Severity**: medium
**Status**: new

## Problem

Closing a mirror pane via `fugue_close_pane` times out after 24 seconds, even though the pane is successfully closed.

```
MCP error -32603: Timeout waiting for daemon response after 24s
```

## Reproduction Steps

1. Create a mirror pane: `fugue_mirror_pane(source_pane_id: "<pane-uuid>")`
2. Close the mirror: `fugue_close_pane(pane_id: "<mirror-uuid>")`
3. Observe: Timeout error after ~24s
4. List panes: Mirror is gone (close succeeded)

## Expected Behavior

`fugue_close_pane` returns success response when mirror pane is closed.

## Actual Behavior

Operation times out but the mirror pane is actually closed. The MCP bridge never receives the `PaneClosed` response.

## Root Cause Analysis

This is the same pattern as BUG-059. The handler for closing mirror panes likely uses `BroadcastToSession` (which only sends to OTHER clients) instead of `RespondWithBroadcast` (which sends to the requesting client AND broadcasts to others).

The close pane handler needs to detect when closing a mirror pane and return `RespondWithBroadcast` so the MCP bridge receives the response.

## Fix

Check the close pane handler in `fugue-server/src/handlers/pane.rs` (likely `handle_close_pane` or similar). If it's using `BroadcastToSession`, change to `RespondWithBroadcast` for mirror pane closures.

## Acceptance Criteria

- [ ] `fugue_close_pane` returns success for mirror panes
- [ ] No timeout when closing mirror panes
- [ ] Normal pane close behavior unaffected

## Related

- BUG-059: Same root cause pattern (mirror_pane creation)
- BUG-037: Previous close_pane AbortError (different issue - unbounded channel)

## Notes

- Discovered during QA of BUG-059 fix
- Mirror pane feature (FEAT-062) may have incomplete MCP response handling
