# BUG-028: Daemon Crashes on fugue_create_layout

## Summary
Calling `fugue_create_layout` with a nested layout specification causes the daemon to crash, resulting in "Daemon connection lost" errors for all subsequent MCP calls.

## Steps to Reproduce

1. Start fugue and connect Claude Code
2. Create a new session: `fugue_create_session` with name "dev-qa" - **succeeds**
3. Call `fugue_create_layout` with the following layout:
```json
{
  "direction": "horizontal",
  "splits": [
    {
      "ratio": 0.7,
      "layout": {
        "direction": "vertical",
        "splits": [
          {"ratio": 0.6, "layout": {"pane": {"name": "editor"}}},
          {"ratio": 0.4, "layout": {"pane": {"name": "sidebar"}}}
        ]
      }
    },
    {
      "ratio": 0.3,
      "layout": {"pane": {"name": "terminal"}}
    }
  ]
}
```
4. Observe: MCP returns error "Daemon connection lost"
5. All subsequent MCP calls fail with same error

## Expected Behavior
- Layout should be created with 3 panes in the IDE-style configuration
- Daemon should remain stable

## Actual Behavior
- Daemon crashes/disconnects
- All MCP tools become unusable
- Error: `MCP error -32603: Daemon connection lost`

## Environment
- fugue version: current main branch
- Platform: Linux (WSL2)
- Triggered during: QA demo run

## Impact
- **Severity**: P0 - Blocks all MCP functionality after trigger
- **Affected Component**: daemon, create_layout handler
- **Workaround**: Restart fugue daemon (loses all sessions)

## Notes
- The `fugue_create_session` call succeeded immediately before this
- The session "dev-qa" was created with session_id `878c67c0-3198-4a50-8590-95a06863f107`
- The layout spec appears valid per the API documentation
