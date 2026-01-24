# BUG-027: MCP response types are swapped between handlers

## Summary

MCP response routing is broken - response types are being returned to the wrong handlers:
- `fugue_read_pane` returns `SessionList` (should return `PaneContent`)
- `fugue_list_panes` returns `PaneContent` (should return pane list)

The responses are swapped/crossed.

## Steps to Reproduce

1. Create a pane using `fugue_create_pane`
2. Send input to the pane using `fugue_send_input`
3. Call `fugue_read_pane` - get SessionList error
4. Call `fugue_list_panes` - get PaneContent error (contains the data read_pane should have returned!)

## Expected Behavior

Each MCP tool should return its correct response type.

## Actual Behavior

`fugue_read_pane` error:
```
MCP error -32603: Unexpected response: SessionList { sessions: [...] }
```

`fugue_list_panes` error:
```
MCP error -32603: Unexpected response: PaneContent { pane_id: 12236b1e-..., content: "test result: ok. 135 passed..." }
```

`fugue_get_status` error:
```
MCP error -32603: Unexpected response: AllPanesList { panes: [...] }
```

`fugue_create_pane` (second call) error:
```
MCP error -32603: Unexpected response: PaneStatus { pane_id: 12236b1e-..., state: Normal, ... }
```

Note: Each handler receives a valid response - just the wrong type! The data is correct but misrouted. Responses appear to be queued and delivered to the next request regardless of type.

## Impact

- **Critical**: Multiple MCP tools broken due to response routing
- Blocks monitoring of background processes
- Priority P0 - breaks fundamental MCP capabilities

## Component

MCP server response dispatch - responses being sent to wrong request handlers

## Root Cause Hypothesis

Likely an async race condition or channel mixup where responses are being delivered to the wrong pending request.

**Additional observation**: Simple response tools work correctly:
- `fugue_send_input` → `{"status": "sent"}` ✅
- `fugue_focus_pane` → `{"pane_id": "...", "status": "focused"}` ✅

Complex response tools fail:
- `fugue_read_pane` → wrong type ❌
- `fugue_list_panes` → wrong type ❌
- `fugue_get_status` → wrong type ❌
- `fugue_create_pane` → wrong type ❌
- `fugue_list_sessions` → wrong type ❌

This suggests the bug may be in how complex/structured responses are dispatched, not simple status responses.

**Critical insight**: Actions execute correctly despite wrong responses!
- `fugue_close_pane` - panes actually close, response is misrouted
- `fugue_create_pane` - panes actually create, response is misrouted
- `fugue_read_pane` - data exists (seen via wrong handler)

The MCP server is processing requests correctly - only the response dispatch is broken. This means the bug is isolated to response routing, not request handling.

## Notes

- Discovered during QA demo run on 2026-01-11
- Pane ID used: `12236b1e-34e5-4e5d-863e-319a52aca3a4`
- The actual data exists and is correct - just routed to wrong handlers
