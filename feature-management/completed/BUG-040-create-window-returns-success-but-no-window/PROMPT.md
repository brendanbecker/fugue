# BUG-040: ccmux_create_window returns success but doesn't actually create windows

**Priority**: P1
**Component**: mcp
**Severity**: high
**Status**: new

## Problem Statement

The `ccmux_create_window` MCP tool returns a successful response with `window_id` and `pane_id`, but the window is not actually created. Subsequent calls to `list_windows` or `list_sessions` show the `window_count` unchanged. This may be a regression introduced by the BUG-034 fix which modified create_window session handling.

## Evidence

### create_window Response (appears successful)
```json
{
  "window_id": "724332b6-fa99-4168-acf7-aa692674b36f",
  "pane_id": "b4ffb2d2-5151-4205-bf06-ebb161746cc4",
  "session": "session-0",
  "status": "created"
}
```

### list_sessions After create_window (unchanged)
```json
{
  "session": "session-0",
  "window_count": 1
}
```

### Related Fix
- BUG-034 fix commit `3e14861` modified `create_window` session handling
- Files changed: `ccmux-server/src/mcp/handlers.rs`, `ccmux-server/src/session/manager.rs`

## Steps to Reproduce

1. Check session-0 has 1 window: `list_sessions` shows window_count: 1
2. Call `create_window` with session="session-0"
3. Observe response shows success with new window_id
4. Check again: `list_sessions` still shows window_count: 1
5. `list_windows` for session-0 shows only the original window

## Expected Behavior

After calling `create_window`, the new window should appear in `list_windows` and `list_sessions` should show an incremented `window_count`.

## Actual Behavior

`create_window` returns success with `window_id` and `pane_id`, but the window is not persisted. `list_sessions` shows unchanged `window_count` and `list_windows` does not include the new window.

## Root Cause

Suspected regression from BUG-034 fix - windows may be created transiently but not persisted to session state.

## Implementation Tasks

### Section 1: Investigation
- [ ] Review BUG-034 fix changes in `ccmux-server/src/mcp/handlers.rs`
- [ ] Review BUG-034 fix changes in `ccmux-server/src/session/manager.rs`
- [ ] Trace the create_window code path to identify where window creation fails
- [ ] Verify window is being added to session's window list
- [ ] Check if window creation is occurring but not being persisted

### Section 2: Fix Implementation
- [ ] Identify the specific location where window persistence fails
- [ ] Implement fix to ensure created windows are added to session state
- [ ] Ensure window_count is updated correctly
- [ ] Verify list_windows returns newly created windows

### Section 3: Testing
- [ ] Add test case for create_window verifying window appears in list_windows
- [ ] Add test case verifying window_count increments after create_window
- [ ] Run existing MCP handler tests to ensure no regression
- [ ] Manual testing of create_window workflow

### Section 4: Verification
- [ ] Confirm create_window creates persistent windows
- [ ] Confirm list_sessions shows updated window_count
- [ ] Confirm list_windows includes new windows
- [ ] Verify BUG-034 fix still works (session parameter respected)

## Acceptance Criteria

- [ ] `create_window` creates a window that persists in session state
- [ ] `list_windows` returns newly created windows
- [ ] `list_sessions` shows correct `window_count` after window creation
- [ ] BUG-034 fix remains intact (session parameter is respected)
- [ ] Tests added to prevent regression
- [ ] No new bugs introduced

## Notes

This is likely a regression from the BUG-034 fix (commit 3e14861). The fix modified how `create_window` handles the session parameter. The window may be created in isolation but not properly linked to the session's window collection.

Key files to investigate:
- `ccmux-server/src/mcp/handlers.rs` - MCP handler for create_window
- `ccmux-server/src/session/manager.rs` - Session manager window operations
