# BUG-018: TUI Pane Interaction Failure

## Priority: P1
## Status: Needs Investigation
## Created: 2026-01-10

## Problem Summary

In the fugue TUI, the user cannot see the text input bar and cannot interact with a pane that visually shows Claude Code output.

## Symptoms Observed

1. **Visual**: Pane shows Claude conversation with purple/magenta theme
2. **Missing input**: The Claude Code input prompt ("> ") is not visible at the bottom
3. **No interaction**: Keyboard input doesn't seem to reach the pane
4. **Screenshot**: `/mnt/c/Users/Brend/Pictures/Screenshots/Screenshot 2026-01-10 153538.png`

## MCP Diagnostic Data

When querying panes via MCP:
```json
{
  "pane_id": "8b2613f7-a605-4c93-a9ef-0191b6fedd23",
  "cols": 92,
  "rows": 46,
  "has_pty": true,
  "state": "normal",
  "is_claude": false,
  "is_awaiting_input": false
}
```

- `is_claude: false` - Claude detection not working (expected if server wasn't rebuilt with BUG-016 fix)
- `has_pty: true` - PTY connection exists
- `read_pane` returns empty - scrollback not populated (BUG-016 symptom)

## Possible Causes

1. **Focus issue** - Pane may not be focused, keyboard input going elsewhere
2. **Scroll position** - View scrolled up, input bar below visible area
3. **Claude process state** - Claude might have exited or be in a hung state
4. **Layout/resize bug** - Pane dimensions may be wrong, clipping the input area
5. **Related to BUG-015** - Layout not recalculated properly

## Investigation Steps

1. [ ] Rebuild server with BUG-016 fix and test if issue persists
2. [ ] Check if `send_input` via MCP reaches the pane
3. [ ] Verify Claude process is actually running (`ps aux | grep claude`)
4. [ ] Check terminal dimensions match pane dimensions
5. [ ] Try resizing terminal window to force layout recalc
6. [ ] Check if clicking on pane changes focus

## Context

- This occurred while testing BUG-016 fix (PTY output routing)
- Server was running OLD code (before BUG-016 fix was compiled)
- May be resolved after server rebuild - needs verification

## Related Issues

- **BUG-015**: Layout not recalculated on pane close
- **BUG-016**: PTY output not routed to pane state (now fixed)

## Files to Investigate

- `fugue-client/src/ui/app.rs` - Focus handling, layout
- `fugue-client/src/ui/pane_view.rs` - Pane rendering
- `fugue-server/src/handlers/` - Input routing

## Resolution

_To be determined after investigation_
