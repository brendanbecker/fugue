# FEAT-031: Session Delete/Kill Keybind in Session Select UI

**Priority**: P2
**Component**: ccmux-client
**Type**: enhancement
**Estimated Effort**: small (1-2 hours)
**Business Value**: medium

## Overview

Add the ability to delete/kill sessions from the session selection UI screen. Users should be able to clean up zombie sessions or sessions they no longer need without having to use external tools.

## Requirements

### 1. Keybind for Session Deletion
- Use a modifier key combination to prevent accidental deletion
- Recommended: `Ctrl+D` (delete) or `Ctrl+X` (kill)
- Only active in session select state, not in attached mode
- Should be clearly documented in help text

### 2. Deliberate Action Pattern
- Option A: Use modifier key (Ctrl+D/X) - single deliberate action
- Option B: Require confirmation prompt before deletion
- Recommendation: Start with Option A (modifier key) for simplicity

### 3. Client-Server Communication
- Client sends `DestroySession { session_id }` message to server
- Server destroys the session and all its windows/panes
- Server sends updated session list to client
- Client refreshes the session select UI

### 4. UI Feedback
- Show keybind in help text (bottom of session select screen)
- Optionally flash/highlight when session is deleted
- Handle edge case: deleting last session shows empty state

### 5. Error Handling
- Handle case where session no longer exists (race condition)
- Handle server communication failure gracefully
- Log deletion events for debugging

## Current State

The session selection UI exists in `ccmux-client/src/ui/app.rs`:
- `handle_session_select_input()` handles keyboard input for session select
- Current keybinds: Up/Down/j/k (navigate), Enter (attach), n (new), r (refresh), q (quit)
- No delete/kill functionality exists yet

The protocol layer may need a new `DestroySession` message type.

## Location

- **Primary file**: `ccmux-client/src/ui/app.rs`
- **Handler**: `handle_session_select_input()`
- **Protocol**: `ccmux-protocol/src/lib.rs` (for new message type)
- **Server handler**: `ccmux-server/src/server.rs` (to handle destroy request)

## Affected Files

- `ccmux-client/src/ui/app.rs` - Add Ctrl+D/X handler in session select
- `ccmux-protocol/src/lib.rs` - Add `DestroySession` message type (if not exists)
- `ccmux-server/src/server.rs` - Handle destroy session request
- `ccmux-server/src/session.rs` - Session destruction logic

## Implementation Tasks

### Section 1: Protocol Extension
- [ ] Check if `DestroySession` message exists in protocol
- [ ] If not, add `DestroySession { session_id: String }` to ClientMessage
- [ ] Add corresponding response type if needed

### Section 2: Server Handler
- [ ] Add handler for `DestroySession` message
- [ ] Implement session destruction (remove from SessionManager)
- [ ] Clean up associated windows and panes
- [ ] Kill associated PTY processes
- [ ] Broadcast updated session list to all connected clients

### Section 3: Client Handler
- [ ] Add Ctrl+D (or Ctrl+X) keybind in `handle_session_select_input()`
- [ ] Send `DestroySession` message to server when triggered
- [ ] Handle case where no session is selected
- [ ] Request session list refresh after deletion

### Section 4: UI Updates
- [ ] Add keybind to help text in session select screen
- [ ] Handle empty state after deleting last session
- [ ] Update selection index if deleted session was selected

### Section 5: Testing
- [ ] Manual test: Delete a session with Ctrl+D
- [ ] Manual test: Delete last session (empty state)
- [ ] Manual test: Delete non-selected session
- [ ] Manual test: Rapid deletion of multiple sessions
- [ ] Verify PTY processes are cleaned up

## Acceptance Criteria

- [ ] Ctrl+D (or Ctrl+X) deletes the currently selected session
- [ ] Session and all its panes/windows are destroyed on server
- [ ] Session list refreshes after deletion
- [ ] Keybind is documented in help text
- [ ] No accidental deletion with regular keys
- [ ] Clean error handling for edge cases

## Dependencies

- **FEAT-024**: Session Selection UI - provides the base UI this enhances
- **FEAT-012**: Session Management - provides SessionManager for destruction

## Technical Notes

### Protocol Message (Example)

```rust
// In ClientMessage enum
DestroySession { session_id: String }

// In ServerMessage enum (optional response)
SessionDestroyed { session_id: String, success: bool }
```

### Keybind Detection

```rust
// In handle_session_select_input()
KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
    if let Some(idx) = self.session_select_index {
        if let Some(session) = self.sessions.get(idx) {
            self.send_message(ClientMessage::DestroySession {
                session_id: session.id.clone(),
            }).await?;
        }
    }
}
```

### Server Handler (Example)

```rust
// In handle_client_message()
ClientMessage::DestroySession { session_id } => {
    if let Some(session) = self.session_manager.remove_session(&session_id) {
        // Kill all PTY processes in session
        for window in session.windows() {
            for pane in window.panes() {
                pane.kill_pty();
            }
        }
        // Broadcast updated session list
        self.broadcast_session_list().await;
    }
}
```
