# BUG-029: fugue_create_window Returns Unexpected SessionFocused Response

## Summary
Calling `fugue_create_window` returns an error about an unexpected `SessionFocused` response instead of creating the window.

## Steps to Reproduce

1. Start fugue and connect Claude Code
2. Create a session: `fugue_create_session` with name "dev-qa" - **succeeds**
3. Select the session: `fugue_select_session` - **succeeds**
4. Call `fugue_create_window` with session "dev-qa" and name "logs"
5. Observe: MCP returns error about unexpected response

## Expected Behavior
- A new window named "logs" should be created in the dev-qa session
- Response should include window_id and success status

## Actual Behavior
- Error returned: `MCP error -32603: Unexpected response: SessionFocused { session_id: 2eb65a90-5aee-42aa-9e47-e552cb78b6bc }`
- No window created

## Environment
- fugue version: current main branch
- Platform: Linux (WSL2)
- Triggered during: QA demo run

## Impact
- **Severity**: P1 - Window creation blocked
- **Affected Component**: daemon, create_window handler
- **Workaround**: None known - cannot create additional windows

## Notes
- The session was successfully selected immediately before this call
- The error suggests the daemon is returning a SessionFocused message when it should return a WindowCreated message
- May be a message routing/matching issue in the MCP handler

## Resolution

**Root Cause**: The `is_broadcast_message()` filter in `fugue-server/src/mcp/bridge.rs` was missing three focus-related broadcast message types added for BUG-026:
- `SessionFocused`
- `WindowFocused`
- `PaneFocused`

When `tool_select_session()` was called, it sent `SelectSession` to the daemon but returned immediately without waiting for a response. The daemon then broadcast `SessionFocused` to all clients including the MCP bridge. This broadcast message sat in the MCP bridge's receive channel.

When the next MCP tool (`create_window`) called `recv_response_from_daemon()`, it received the stale `SessionFocused` broadcast message instead of the expected `WindowCreatedWithDetails` response, causing the "Unexpected response: SessionFocused" error.

**Fix**: Added `SessionFocused`, `WindowFocused`, and `PaneFocused` to the `is_broadcast_message()` filter so they are skipped when waiting for tool responses.

**Files Changed**:
- `fugue-server/src/mcp/bridge.rs`: Added three message types to `is_broadcast_message()` filter and added regression tests

**Related**:
- BUG-027: Similar issue where broadcast messages leaked into response channel (fixed: filter broadcast messages in recv_response_from_daemon)
- BUG-026: Added the focus broadcast messages that were missing from the filter
