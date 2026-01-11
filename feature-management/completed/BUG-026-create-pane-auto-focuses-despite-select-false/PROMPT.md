# BUG-026: Focus management broken - auto-focus on create, no effect on focus_pane

## Summary

Focus management is broken in three ways:

1. **Auto-focus on create**: When calling `ccmux_create_pane` without the `select` parameter (which should default to `false`), the new pane is automatically focused anyway. This steals focus from the original pane unexpectedly.

2. **focus_pane has no effect**: When calling `ccmux_focus_pane`, the tool returns `{"status": "focused"}` but focus does not actually change visually.

3. **select_window has no effect**: When calling `ccmux_select_window`, the tool returns `{"status": "selected"}` but window does not actually switch.

## Steps to Reproduce

**Symptom 1 - Auto-focus on create:**
1. Have a session with one pane focused
2. Call `ccmux_create_pane` without passing `select: true`
3. Observe which pane has focus

**Symptom 2 - focus_pane no effect:**
1. Have multiple panes in a session
2. Call `ccmux_focus_pane` with a different pane's ID
3. Tool returns success but focus doesn't visually change

**Symptom 3 - select_window no effect:**
1. Create a new window using `ccmux_create_window`
2. Call `ccmux_select_window` with the new window's ID
3. Tool returns `{"status": "selected"}` but window doesn't switch

## Expected Behavior

1. On create: Focus should remain on the original pane per MCP tool description:
   > "select": {"default": false, "description": "If true, focus the new pane after creation (default: false, keeps current focus)"}

2. On focus_pane: Focus should switch to the specified pane

3. On select_window: Active window should switch to the specified window

## Actual Behavior

1. Focus automatically switches to the newly created pane (unwanted)
2. `focus_pane` returns `{"pane_id": "...", "status": "focused"}` but focus doesn't change (wanted action doesn't happen)
3. `select_window` returns `{"status": "selected", "window_id": "..."}` but window doesn't switch (wanted action doesn't happen)

## Impact

- Disrupts user workflow when orchestrator creates background panes
- Unexpected context switch for the human viewer
- Priority P1 since it affects usability significantly

## Component

MCP handler for `ccmux_create_pane` or underlying pane creation logic

## Notes

- Discovered during QA demo run on 2026-01-11
- User noted this bug may have been observed before but not yet resolved
- Also affects `ccmux_split_pane` which has the same `select` parameter
