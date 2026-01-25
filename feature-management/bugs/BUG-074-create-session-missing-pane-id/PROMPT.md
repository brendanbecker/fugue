# BUG-074: fugue_create_session Should Reliably Return Pane ID

**Priority**: P1
**Component**: mcp/session
**Severity**: medium
**Status**: new

## Problem

When creating a new session via `fugue_create_session`, the response should always include the pane ID of the default pane so that callers can immediately send input without needing an additional `fugue_list_panes` call.

Currently, orchestrators must:
1. Call `fugue_create_session` to create the session
2. Call `fugue_list_panes` to discover the pane ID
3. Call `fugue_send_input` with the pane ID

This should be:
1. Call `fugue_create_session` - get pane_id in response
2. Call `fugue_send_input` with the pane ID

## Current Behavior

The `fugue_create_session` response includes `pane_id`, but there may be cases where:
- The pane_id is not reliably returned
- The pane_id is returned but the pane isn't ready for input yet
- Race conditions where the pane doesn't exist when trying to send input immediately

## Expected Behavior

`fugue_create_session` should:
1. **Always** return the default pane's ID in the response
2. Ensure the pane is fully initialized and ready for input before returning
3. Document the pane_id field in the tool schema

## Investigation Steps

### Section 1: Verify Current Implementation
- [ ] Check `fugue_create_session` handler in `fugue-server/src/mcp/bridge/handlers.rs`
- [ ] Verify the response always includes `pane_id`
- [ ] Check if there's a race between session creation and pane availability
- [ ] Review the tool schema in `fugue-server/src/mcp/tools.rs`

### Section 2: Identify Failure Cases
- [ ] Test rapid session creation followed by immediate send_input
- [ ] Test with different session configurations (with/without command)
- [ ] Check if `pane_id` is ever None or missing in response

### Section 3: Implement Fix
- [ ] Ensure pane is fully ready before returning from create_session
- [ ] Add explicit `pane_id` field to tool response schema
- [ ] Add test for immediate send_input after create_session

## Acceptance Criteria

- [ ] `fugue_create_session` always returns `pane_id` in response
- [ ] Pane is ready for input immediately after create_session returns
- [ ] No need to call `fugue_list_panes` before `fugue_send_input`
- [ ] Tool schema documents the `pane_id` response field
- [ ] Integration test verifies create → send_input works without list_panes

## Key Files

- `fugue-server/src/mcp/bridge/handlers.rs` - tool_create_session handler
- `fugue-server/src/mcp/tools.rs` - tool schema definitions
- `fugue-server/src/session/mod.rs` - session/pane creation logic

## Impact

This is a high-friction issue for orchestration workflows. Every session spawn requires an extra MCP call, adding latency and complexity to multi-agent coordination patterns.

## Related

- FEAT-104: Watchdog Orchestration Skill (affected by this issue)
- FEAT-110: Watchdog Monitor Agent (affected by this issue)
