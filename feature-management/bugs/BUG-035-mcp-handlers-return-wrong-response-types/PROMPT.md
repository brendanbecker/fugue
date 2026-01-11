# BUG-035: MCP Handlers Return Wrong Response Types

## Summary
MCP list handlers are returning the wrong response enum variants, causing deserialization failures. The responses contain valid data but wrapped in the wrong type.

## Steps to Reproduce

1. Run several MCP operations (create sessions, windows, panes)
2. Call `ccmux_list_windows` with a session parameter
3. Observe: Returns `SessionList` variant instead of `WindowList`
4. Call `ccmux_list_panes` with a session parameter
5. Observe: Returns `WindowList` variant instead of `PaneList`

## Expected Behavior
- `list_windows` should return `WindowList { windows: [...] }`
- `list_panes` should return `PaneList { panes: [...] }`

## Actual Behavior
- `list_windows` returns `SessionList { sessions: [...] }`
- `list_panes` returns `WindowList { windows: [...] }`

The data inside is correct, but the wrapper type is wrong, causing:
```
MCP error -32603: Unexpected response: SessionList { sessions: [...] }
MCP error -32603: Unexpected response: WindowList { windows: [...] }
```

## Evidence from QA Run

```
# list_windows for session-0
ccmux_list_windows(session: "session-0")
-> Error: Unexpected response: SessionList { sessions: [SessionInfo {...}] }

# list_panes for session-0
ccmux_list_panes(session: "session-0")
-> Error: Unexpected response: WindowList { windows: [WindowInfo {...}] }
```

## Environment
- ccmux version: current main branch (commit c8b0904)
- Platform: Linux (WSL2)
- Triggered during: QA demo run
- Note: These same calls worked earlier in the session

## Impact
- **Severity**: P1 - List operations intermittently fail
- **Affected Component**: daemon, MCP response serialization
- **Workaround**: Retry the call (may work on subsequent attempts)

## Root Cause Hypothesis
- Response enum variants may be getting mixed up in the handler dispatch
- Could be a race condition in response routing
- Or the response type selection logic has an off-by-one error
- Possibly related to response caching or reuse

## Notes
- The data is correct, just wrapped in wrong type
- Suggests the handler logic works, but response type selection is broken
- This appeared later in the QA run after many operations
