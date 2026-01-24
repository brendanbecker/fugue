# Implementation Plan: FEAT-031

**Work Item**: [FEAT-031: Session Delete/Kill Keybind in Session Select UI](PROMPT.md)
**Component**: fugue-client
**Priority**: P2
**Created**: 2026-01-09

## Overview

Add a keybind (Ctrl+D) to the session selection UI that allows users to delete/kill sessions directly from the UI. This requires changes to the client, protocol, and server.

## Architecture Decisions

### Keybind Choice

**Decision**: Use `Ctrl+D` as the delete keybind

Rationale:
- `Ctrl+D` is a common "delete" or "close" action in many applications
- The modifier key prevents accidental deletion
- `Ctrl+X` could work but is often associated with "cut" operations
- Single keypress with modifier is better UX than a confirmation dialog for power users

### Confirmation Approach

**Decision**: No explicit confirmation dialog; modifier key provides sufficient protection

Rationale:
- Power users prefer speed over confirmation dialogs
- Modifier key (Ctrl) already requires deliberate action
- Sessions can be recreated; this isn't a destructive operation on user data
- Consistent with tmux behavior (`tmux kill-session`)

### Protocol Extension

**Decision**: Add `DestroySession` message to `ClientMessage` enum

The message should include only the session_id. Server responds by broadcasting an updated session list (reusing existing `SessionList` message).

## Affected Components

| Component | Type of Change | Risk Level |
|-----------|----------------|------------|
| fugue-protocol/src/lib.rs | Add new message variant | Low |
| fugue-client/src/ui/app.rs | Add keybind handler | Low |
| fugue-server/src/server.rs | Add message handler | Medium |
| fugue-server/src/session.rs | Session removal logic | Medium |

## Dependencies

- **FEAT-024**: Session Selection UI - base functionality
- **FEAT-012**: Session Management - SessionManager operations

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| PTY process leak | Medium | Medium | Ensure all PTYs killed on session destroy |
| Race condition (concurrent delete) | Low | Low | Handle "session not found" gracefully |
| Accidental deletion | Low | Low | Ctrl modifier prevents accidental trigger |
| Client desync after delete | Low | Medium | Broadcast session list after deletion |

## Implementation Approach

### Phase 1: Protocol Extension
1. Add `DestroySession { session_id: String }` to `ClientMessage` enum
2. No new response type needed; reuse `SessionList` broadcast

### Phase 2: Server Handler
1. Add match arm in message handler for `DestroySession`
2. Remove session from SessionManager
3. Kill all PTY processes in session
4. Broadcast updated session list to all clients

### Phase 3: Client Handler
1. Add Ctrl+D keybind in `handle_session_select_input()`
2. Guard against no selection or empty list
3. Send `DestroySession` message to server
4. Refresh happens automatically via `SessionList` broadcast

### Phase 4: UI Polish
1. Update help text to show Ctrl+D keybind
2. Handle selection index after deletion (move to previous or stay)
3. Handle empty state after last session deleted

## Implementation Details

### Protocol Change

```rust
// fugue-protocol/src/lib.rs
pub enum ClientMessage {
    // ... existing variants ...
    DestroySession { session_id: String },
}
```

### Client Keybind

```rust
// fugue-client/src/ui/app.rs in handle_session_select_input()
KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
    if let Some(idx) = self.session_select_index {
        if let Some(session) = self.sessions.get(idx) {
            let session_id = session.id.clone();
            self.send_message(ClientMessage::DestroySession { session_id }).await?;
            // Selection adjustment handled when SessionList is received
        }
    }
}
```

### Server Handler

```rust
// fugue-server/src/server.rs in handle_client_message()
ClientMessage::DestroySession { session_id } => {
    info!("Destroying session: {}", session_id);
    if let Err(e) = self.session_manager.destroy_session(&session_id).await {
        warn!("Failed to destroy session {}: {}", session_id, e);
    }
    // Broadcast updated session list to all clients
    self.broadcast_session_list().await;
}
```

### SessionManager Method

```rust
// fugue-server/src/session.rs
impl SessionManager {
    pub async fn destroy_session(&mut self, session_id: &str) -> Result<()> {
        if let Some(session) = self.sessions.remove(session_id) {
            // Kill all PTY processes
            for window in session.windows() {
                for pane in window.panes() {
                    if let Err(e) = pane.kill() {
                        warn!("Failed to kill pane {}: {}", pane.id(), e);
                    }
                }
            }
            info!("Destroyed session: {}", session_id);
            Ok(())
        } else {
            Err(anyhow!("Session not found: {}", session_id))
        }
    }
}
```

## Rollback Strategy

If implementation causes issues:
1. Remove Ctrl+D keybind from client
2. Remove `DestroySession` variant from protocol
3. Remove server handler
4. All changes are additive; no risk to existing functionality

## Testing Strategy

1. **Unit Tests**: Test `destroy_session()` method on SessionManager
2. **Integration Tests**: Test client-server message flow
3. **Manual Tests**:
   - Delete session with one pane
   - Delete session with multiple windows/panes
   - Delete last session (empty state)
   - Delete while other clients are connected to same session

## Implementation Notes

- Ensure PTY cleanup includes SIGKILL if SIGTERM doesn't work
- Consider adding a brief delay or visual feedback after deletion
- Watch for memory leaks in session/window/pane structures

---
*This plan should be updated as implementation progresses.*
