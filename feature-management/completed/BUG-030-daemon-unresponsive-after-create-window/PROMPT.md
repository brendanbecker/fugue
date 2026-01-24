# BUG-030: Daemon Becomes Unresponsive After create_window Error

## Summary
After `fugue_create_window` returns an unexpected `SessionFocused` response (BUG-029), the daemon becomes completely unresponsive. All subsequent MCP calls fail with `AbortError: The operation was aborted`.

## Steps to Reproduce

1. Start fugue and connect Claude Code
2. Create a session and perform various operations (all succeed)
3. Call `fugue_create_window` - fails with BUG-029 error
4. Attempt any subsequent MCP call (e.g., `fugue_list_panes`, `fugue_connection_status`)
5. Observe: All calls timeout with AbortError

## Expected Behavior
- Even if create_window fails, the daemon should remain responsive
- Other MCP operations should continue to work
- Error should be isolated to the failing operation

## Actual Behavior
- Daemon becomes completely unresponsive after the create_window error
- All subsequent MCP calls timeout
- Error: `MCP error -32001: AbortError: The operation was aborted`
- Even `fugue_connection_status` fails (which should be a simple health check)

## Environment
- fugue version: current main branch
- Platform: Linux (WSL2)
- Triggered during: QA demo run
- Preceded by: BUG-029 (create_window unexpected response)

## Impact
- **Severity**: P0 - Complete daemon failure
- **Affected Component**: daemon, error handling, MCP server
- **Workaround**: Restart daemon (loses all sessions)

## Analysis
This appears to be a cascading failure:
1. BUG-029: create_window returns wrong response type
2. The MCP handler may be stuck waiting for a response that will never come
3. Or the daemon may have crashed/deadlocked during error handling
4. Result: Complete loss of daemon responsiveness

## Related Bugs
- BUG-029: fugue_create_window Returns Unexpected SessionFocused Response (root cause)
- BUG-028: Daemon Crashes on fugue_create_layout (similar daemon stability issue)

## Notes
- FEAT-060 (daemon auto-recovery) does not appear to help in this case
- The connection_status call itself fails, suggesting the issue is at the MCP layer
- Requires full daemon restart to recover
