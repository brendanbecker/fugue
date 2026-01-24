# BUG-003: Session Creation Doesn't Create Default Window/Pane

**Type**: Bug
**Priority**: P0
**Status**: open
**Created**: 2026-01-09
**Component**: fugue-server
**Found During**: HA-001 manual testing

## Description

When a new session is created via `CreateSession`, the session is empty with 0 windows and 0 panes. The client attaches to an empty session and shows "No active pane" with no input handling.

This is a P0 blocker because users cannot interact with the terminal multiplexer at all after creating a session - there is no shell to type commands into.

## Reproduction Steps

1. Start server: `./target/release/fugue-server`
2. Start client: `./target/release/fugue`
3. Press 'n' to create new session
4. Observe: Client shows "No active pane", keyboard input ignored

## Server Logs

```
Client attached to session 0b53814a-c918-4b05-9a8e-c5950c5cab51 (0 windows, 0 panes)
```

## Expected Behavior

When a session is created, it should automatically contain:
- 1 default window (named "0" or based on index)
- 1 default pane with a spawned PTY (shell)

The client should attach and immediately see a working shell prompt.

## Root Cause

`handle_create_session()` in `fugue-server/src/handlers/session.rs` calls `session_manager.create_session()` which creates an empty session. No window or pane is auto-created, and critically, no PTY is spawned.

### Current Code (session.rs:28-53)

```rust
/// Handle CreateSession message - create a new session
pub async fn handle_create_session(&self, name: String) -> HandlerResult {
    info!(
        "CreateSession '{}' request from {}",
        name, self.client_id
    );

    let mut session_manager = self.session_manager.write().await;

    match session_manager.create_session(&name) {
        Ok(session) => {
            let session_info = session.to_info();
            info!("Session '{}' created with ID {}", name, session_info.id);

            HandlerResult::Response(ServerMessage::SessionCreated {
                session: session_info,
            })
        }
        Err(e) => {
            debug!("Failed to create session '{}': {}", name, e);
            HandlerContext::error(
                ErrorCode::InvalidOperation,
                format!("Failed to create session: {}", e),
            )
        }
    }
}
```

The problem: After `session_manager.create_session()` returns, no further setup happens. The session exists but has no windows, panes, or PTYs.

## Implementation Tasks

### Section 1: Analysis and Design

- [ ] **Review existing patterns**: Study `handle_create_window()` and `handle_create_pane()` handlers to understand the established patterns for creating windows, panes, and spawning PTYs
- [ ] **Identify PTY spawn pattern**: Review how `PtyManager::spawn()` is used elsewhere (e.g., in pane creation flows)
- [ ] **Design decision**: Determine if PTY should be spawned inline in handler or if a helper function should be created for reuse

### Section 2: Modify handle_create_session()

Location: `fugue-server/src/handlers/session.rs:28-53`

The fix requires modifying `handle_create_session()` to:

1. Create the session (existing)
2. Create a default window in the session
3. Create a default pane in that window
4. Initialize the pane's vt100 parser
5. Spawn a PTY for the pane
6. Return session info with the window/pane included

#### Proposed Implementation

```rust
/// Handle CreateSession message - create a new session
pub async fn handle_create_session(&self, name: String) -> HandlerResult {
    info!(
        "CreateSession '{}' request from {}",
        name, self.client_id
    );

    let mut session_manager = self.session_manager.write().await;

    match session_manager.create_session(&name) {
        Ok(session) => {
            let session_id = session.id();
            info!("Session '{}' created with ID {}", name, session_id);

            // Get mutable session reference to create window and pane
            let session = session_manager.get_session_mut(session_id).unwrap();

            // Create default window (named "0" for first window)
            let window = session.create_window(None);
            let window_id = window.id();
            info!("Default window created with ID {}", window_id);

            // Create default pane in the window
            let window = session.get_window_mut(window_id).unwrap();
            let pane = window.create_pane();
            let pane_id = pane.id();
            let (cols, rows) = pane.dimensions();
            info!("Default pane created with ID {}", pane_id);

            // Initialize the vt100 parser for terminal emulation
            let pane = window.get_pane_mut(pane_id).unwrap();
            pane.init_parser();

            // Release session manager lock before spawning PTY
            let session_info = session_manager.get_session(session_id).unwrap().to_info();
            drop(session_manager);

            // Spawn PTY for the default pane
            {
                let mut pty_manager = self.pty_manager.write().await;
                let pty_config = PtyConfig::shell().with_size(cols, rows);

                match pty_manager.spawn(pane_id, pty_config) {
                    Ok(_) => {
                        info!("PTY spawned for default pane {}", pane_id);
                    }
                    Err(e) => {
                        // Log error but don't fail session creation
                        // User can manually create a pane if PTY spawn fails
                        warn!("Failed to spawn PTY for default pane: {}", e);
                    }
                }
            }

            HandlerResult::Response(ServerMessage::SessionCreated {
                session: session_info,
            })
        }
        Err(e) => {
            debug!("Failed to create session '{}': {}", name, e);
            HandlerContext::error(
                ErrorCode::InvalidOperation,
                format!("Failed to create session: {}", e),
            )
        }
    }
}
```

#### Required Imports

Add at the top of `session.rs`:
```rust
use crate::pty::PtyConfig;
use tracing::warn;
```

### Section 3: Update Tests

- [ ] **Update existing test**: Modify `test_handle_create_session_success` to verify window/pane creation:

```rust
#[tokio::test]
async fn test_handle_create_session_success() {
    let ctx = create_test_context();
    let result = ctx.handle_create_session("new-session".to_string()).await;

    match result {
        HandlerResult::Response(ServerMessage::SessionCreated { session }) => {
            assert_eq!(session.name, "new-session");
            // Updated: Session should now have 1 window
            assert_eq!(session.window_count, 1);
        }
        _ => panic!("Expected SessionCreated response"),
    }

    // Verify window and pane were created
    let session_manager = ctx.session_manager.read().await;
    let session = session_manager.get_session_by_name("new-session").unwrap();
    assert_eq!(session.window_count(), 1);

    let window = session.windows().next().unwrap();
    assert_eq!(window.pane_count(), 1);

    let pane = window.panes().next().unwrap();
    assert!(pane.has_parser()); // Parser should be initialized
}
```

- [ ] **Add PTY spawn test**: Verify PTY is spawned for the default pane:

```rust
#[tokio::test]
async fn test_handle_create_session_spawns_pty() {
    let ctx = create_test_context();
    ctx.handle_create_session("test-session".to_string()).await;

    // Find the pane ID
    let pane_id = {
        let session_manager = ctx.session_manager.read().await;
        let session = session_manager.get_session_by_name("test-session").unwrap();
        let window = session.windows().next().unwrap();
        window.panes().next().unwrap().id()
    };

    // Verify PTY was spawned
    let pty_manager = ctx.pty_manager.read().await;
    assert!(pty_manager.contains(pane_id), "PTY should be spawned for default pane");
}
```

- [ ] **Add error handling test**: Verify session creation succeeds even if PTY spawn fails (edge case)

### Section 4: Integration Testing

- [ ] **Manual test flow**:
  1. Start server: `cargo run --release -p fugue-server`
  2. Start client: `cargo run --release -p fugue`
  3. Create new session (press 'n')
  4. Verify: Should immediately see shell prompt
  5. Type commands and verify they work
  6. Verify server logs show: `Client attached to session ... (1 windows, 1 panes)`

- [ ] **Test session list**: After creating session, verify `SessionInfo::window_count` is 1 (not 0)

## Acceptance Criteria

- [ ] New sessions are created with exactly 1 window and 1 pane
- [ ] The pane has an initialized vt100 parser (`has_parser()` returns true)
- [ ] A PTY is spawned for the pane (running user's default shell)
- [ ] Client can immediately type commands after session creation
- [ ] Server logs show "(1 windows, 1 panes)" when client attaches
- [ ] Existing tests pass (update `test_handle_create_session_success` expected values)
- [ ] New tests added for window/pane/PTY verification
- [ ] PTY spawn failure does not fail session creation (graceful degradation)

## Testing Approach

### Unit Tests

1. **Handler-level tests** in `fugue-server/src/handlers/session.rs`:
   - Verify `SessionCreated` response includes `window_count: 1`
   - Verify session in `SessionManager` has 1 window with 1 pane
   - Verify pane has initialized parser
   - Verify PTY manager contains entry for the pane

2. **Integration test** for full flow:
   - Create session via handler
   - Attach to session
   - Verify `Attached` response includes window and pane info

### Manual Tests

1. **Happy path**: Create session, verify shell appears
2. **Multiple sessions**: Create multiple sessions, verify each has its own window/pane/PTY
3. **Resize**: Create session, resize terminal, verify PTY receives resize

## Files to Modify

| File | Change |
|------|--------|
| `fugue-server/src/handlers/session.rs` | Modify `handle_create_session()` to create window, pane, and spawn PTY |
| `fugue-server/src/handlers/session.rs` | Add `use crate::pty::PtyConfig;` import |
| `fugue-server/src/handlers/session.rs` | Add `use tracing::warn;` import |
| `fugue-server/src/handlers/session.rs` | Update tests for new expected behavior |

## Alternative Considered

**Client-side fix**: Have client send `CreateWindow` + `CreatePane` after receiving `Attached` response with empty panes.

**Rejected because**:
- Requires client logic to detect empty session and fix it
- Creates race condition if multiple clients attach simultaneously
- Violates principle of least surprise - users expect usable sessions
- Server-side fix ensures sessions are never empty by construction

## Related

- **HA-001**: Manual testing action that discovered this bug
- **FEAT-001**: MVP feature that depends on this bug being fixed

## Notes

- P0 priority - this blocks basic usability of the terminal multiplexer
- The session manager, window, pane, and PTY spawning code all exist and work correctly individually
- This is purely an integration issue in the handler - the pieces just need to be connected
- The fix is straightforward but touches multiple subsystems (session, window, pane, PTY)
