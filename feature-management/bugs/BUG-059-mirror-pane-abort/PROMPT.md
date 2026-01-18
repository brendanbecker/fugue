# BUG-059: ccmux_mirror_pane tool aborts with error

**Priority**: P3
**Component**: mcp
**Severity**: medium
**Status**: new

## Problem

Calling `ccmux_mirror_pane` via MCP returns an abort error:

```
MCP error -32001: AbortError: The operation was aborted.
```

## Reproduction Steps

1. Have a pane running (e.g., Claude Code)
2. Call `ccmux_mirror_pane(source_pane_id: "<pane-uuid>")`
3. Observe: AbortError returned

## Expected Behavior

A read-only mirror pane is created that displays the source pane's output in real-time.

## Actual Behavior

Operation aborts with error code -32001.

## Analysis

Looking at BUG-047, `mirror_pane` method in `mcp/handlers.rs` is listed as **dead code**. This suggests the feature may not be fully implemented - the MCP tool schema exists but the handler may be incomplete or not wired up correctly.

### Possible Causes

1. **Incomplete Implementation**: The mirror_pane handler exists but doesn't actually create the mirror
2. **Missing Backend Support**: The session/pane manager may not support mirror panes yet
3. **Timeout/Async Issue**: The operation starts but times out waiting for completion

## Investigation Steps

### Section 1: Check Implementation Status
- [ ] Review `ccmux-server/src/mcp/bridge/handlers.rs` for mirror_pane handler
- [ ] Check if it's actually wired up to the MCP router
- [ ] Verify if backend support exists in session manager

### Section 2: Trace the Abort
- [ ] Add logging to mirror_pane handler
- [ ] Check what triggers the AbortError
- [ ] Determine if it's a timeout or explicit abort

### Section 3: Implement if Missing
- [ ] If not implemented, create proper implementation
- [ ] Mirror pane should forward output from source without accepting input
- [ ] Update tests

## Acceptance Criteria

- [ ] `ccmux_mirror_pane` creates a working mirror pane
- [ ] Mirror displays source pane output in real-time
- [ ] Mirror is read-only (input disabled)
- [ ] No AbortError on valid source_pane_id

## Related Files

- `ccmux-server/src/mcp/bridge/handlers.rs` - mirror_pane handler (dead code per BUG-047)
- `ccmux-server/src/mcp/tools.rs` - MCP tool schema
- `ccmux-server/src/session/pane.rs` - Pane management

## Notes

- BUG-047 lists `mirror_pane` as dead code, confirming incomplete implementation
- Feature is described in DEMO-MULTI-AGENT.md as "plate spinning" visibility
- Discovered during multi-agent orchestration demo on 2026-01-18
