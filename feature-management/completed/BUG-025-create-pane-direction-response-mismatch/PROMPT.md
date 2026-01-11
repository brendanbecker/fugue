# BUG-025: ccmux_create_pane response direction doesn't match request

## Summary

When calling `ccmux_create_pane` with `direction: "vertical"`, the pane is created correctly (side-by-side split), but the JSON response incorrectly reports `direction: "horizontal"`.

## Steps to Reproduce

1. Call `ccmux_create_pane` with parameter `direction: "vertical"`
2. Observe the response JSON

## Expected Behavior

Response should contain `"direction": "vertical"` (or match whatever was requested/used).

## Actual Behavior

Response contains `"direction": "horizontal"` despite the pane being created with vertical (side-by-side) layout.

Request:
```json
{
  "direction": "vertical",
  "cwd": "/home/becker/projects/tools/ccmux"
}
```

Response:
```json
{
  "direction": "horizontal",
  "pane_id": "12236b1e-34e5-4e5d-863e-319a52aca3a4",
  "session": "session-b156ab03f9d44c64af415e237634f061",
  "session_id": "4c12cdc1-a27c-4873-ab74-6e55c84e3ac3",
  "status": "created",
  "window_id": "3ab57367-0fad-4a58-a046-ff3b7252bb00"
}
```

## Impact

- Misleading response could confuse orchestrators relying on response data
- Low severity since the actual operation works correctly

## Component

MCP handler for `ccmux_create_pane`

## Notes

Discovered during QA demo run on 2026-01-11.
