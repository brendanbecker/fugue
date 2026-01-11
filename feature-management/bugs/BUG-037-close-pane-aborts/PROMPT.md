# BUG-037: close_pane Returns AbortError

**Priority**: P2
**Component**: ccmux-server
**Severity**: medium
**Status**: new

## Problem Statement

`ccmux_close_pane` fails with "AbortError: The operation was aborted" instead of closing the pane.

## Steps to Reproduce

1. Have a pane open (e.g., the stray "logs" window pane from BUG-034)
2. Call `ccmux_close_pane(pane_id: "062d7f57-87d4-40c6-9238-01df075c3cee")`
3. **Observe**: Returns error instead of closing

## Expected Behavior
- Pane should be closed
- Returns success confirmation

## Actual Behavior
```
MCP error -32001: AbortError: The operation was aborted.
```

Pane remains open. User had to close it manually.

## Environment
- ccmux version: current main branch
- Platform: Linux (WSL2)
- Triggered during: QA demo cleanup

## Impact
- **Severity**: P2 - Cannot programmatically close panes
- **Workaround**: User manually closes pane via keyboard
