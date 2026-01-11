# BUG-036: Selection Tools Don't Switch TUI View

**Priority**: P0
**Component**: ccmux-server (handlers/mcp_bridge.rs), ccmux-client (TUI)
**Severity**: critical
**Status**: new

## Problem Statement

The MCP selection/navigation tools (`select_session`, `select_window`, `focus_pane`) return success but the TUI never visually switches. The user remains viewing the same session/window/pane throughout.

This was observed during a full QA demo run where an orchestrator Claude attempted to:
1. Switch between sessions (session-0 → dev-qa → dashboards → back)
2. Switch between windows within a session
3. Focus different panes

All calls returned `{"status": "selected"}` or `{"status": "focused"}` but the TUI never changed view.

## Steps to Reproduce

1. Start ccmux with a session (session-0)
2. From Claude, create a new session: `ccmux_create_session(name: "test-session")`
3. Call `ccmux_select_session(session_id: "<new-session-id>")`
4. **Observe**: Tool returns `{status: "selected", session_id: "..."}`
5. **Observe**: TUI still shows session-0, not test-session

Same behavior with:
- `ccmux_select_window` - returns success, TUI stays on current window
- `ccmux_focus_pane` - returns success, focus doesn't visibly change

## Expected Behavior

When `select_session` succeeds, the TUI should:
- Switch to display the selected session
- Show that session's windows and panes
- User should see the visual change immediately

Same for `select_window` and `focus_pane`.

## Actual Behavior

- Tools return success
- TUI remains unchanged
- Only automatic focus (on pane creation) works
- User must manually switch sessions/windows via keyboard

## Evidence from QA Run

```
# These all returned success but TUI never switched:
ccmux_select_session(session_id: "58816ea5-...") -> {status: "selected"}
ccmux_select_session(session_id: "b7fc4a03-...") -> {status: "selected"}
ccmux_select_session(session_id: "aa869bd1-...") -> {status: "selected"}
ccmux_select_window(window_id: "e9da9327-...") -> {status: "selected"}
ccmux_focus_pane(pane_id: "3821fc76-...") -> {status: "focused"}
ccmux_focus_pane(pane_id: "e0c6c115-...") -> {status: "focused"}

# User watching TUI: "I watched this session the whole time. Focus auto
# swapped to the new pane when created. Never else."
```

## Relationship to Other Bugs

- **BUG-026**: Claimed to fix select_pane/select_window broadcasts - may not be complete
- **BUG-032**: Covers create/split not broadcasting - different issue
- This bug is specifically about selection/navigation not affecting TUI view

## Root Cause (CONFIRMED)

**Location**: `ccmux-client/src/ui/app.rs` lines 1521-1548

The TUI handlers for `SessionFocused`, `WindowFocused`, `PaneFocused` only update local state for items already in the currently attached session. They do NOT switch sessions when a different session is focused.

**SessionFocused handler (lines 1541-1548)**:
```rust
ServerMessage::SessionFocused { session_id } => {
    // Only logs if the focused session matches current - DOES NOT SWITCH
    if let Some(ref session) = self.session {
        if session.id == session_id {
            tracing::debug!("Our session {} is now the active session (via MCP)", session_id);
        }
    }
}
```

**WindowFocused/PaneFocused** have the same issue - they only update state for known items in the current session.

## Fix Required

When receiving these messages, check if `session_id` differs from the currently attached session. If so, send `ClientMessage::AttachSession { session_id }` to switch.

```rust
ServerMessage::SessionFocused { session_id } => {
    let should_switch = match &self.session {
        Some(current) => current.id != session_id,
        None => true,
    };

    if should_switch {
        tracing::debug!("Switching to focused session {} (via MCP)", session_id);
        self.connection
            .send(ClientMessage::AttachSession { session_id })
            .await?;
    }
}
```

## Impact

- **Severity**: P0 - Navigation via MCP is completely broken
- **User Impact**: Orchestrators cannot visually demonstrate multi-session workflows
- **Workaround**: None - user must manually switch via keyboard

## Acceptance Criteria

- [ ] `ccmux_select_session` causes TUI to switch to that session
- [ ] `ccmux_select_window` causes TUI to switch to that window
- [ ] `ccmux_focus_pane` causes TUI to visually focus that pane
- [ ] Changes happen immediately (not requiring user interaction)
