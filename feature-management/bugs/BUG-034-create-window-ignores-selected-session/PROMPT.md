# BUG-034: ccmux_create_window Ignores Selected Session

## Summary
`ccmux_create_window` creates windows in the wrong session. After calling `ccmux_select_session` to switch to a different session, `ccmux_create_window` (without explicit session parameter) still creates the window in the original session instead of the selected one.

## Steps to Reproduce

1. Start in session-0 (original session)
2. Create a new session "dev-qa" via `ccmux_create_session`
3. Call `ccmux_select_session` with dev-qa's session_id
4. Verify selection succeeded (returns `status: "selected"`)
5. Call `ccmux_create_window` with name "logs" (no session parameter)
6. Check which session the window was created in

## Expected Behavior
- Window "logs" should be created in dev-qa (the selected session)
- Per tool docs: "Uses active session if omitted"

## Actual Behavior
- Window "logs" was created in session-0 (the original session)
- Response even shows `session: "session-0"` confirming wrong session
- `ccmux_list_windows` for dev-qa shows only 1 window (the original)
- `ccmux_list_windows` for session-0 shows 2 windows including "logs"

## Evidence from QA Run

```
# Selected dev-qa
ccmux_select_session(session_id: "58816ea5-79a5-4c6c-a611-e800cddb4773")
-> {session_id: "58816ea5-...", status: "selected"}

# Created window without session param
ccmux_create_window(name: "logs")
-> {session: "session-0", window_id: "a12c31d8-...", status: "created"}
                    ^^^^^^^^^^^ WRONG SESSION
```

## Environment
- ccmux version: current main branch (commit c8b0904)
- Platform: Linux (WSL2)
- Triggered during: QA demo run

## Impact
- **Severity**: P2 - Creates windows in wrong session
- **Affected Component**: daemon, create_window handler
- **Workaround**: Always explicitly pass `session` parameter to `ccmux_create_window`

## Side Effect
The stray "logs" window now exists in session-0 (the orchestrator's session), cluttering the workspace. This is a real-world consequence of the bug - windows end up in unexpected places.

## Root Cause Hypothesis
- `select_session` may only affect TUI display, not MCP context
- Or MCP handlers may be using wrong session lookup for "active session"
- Or there's a race condition between select and create
