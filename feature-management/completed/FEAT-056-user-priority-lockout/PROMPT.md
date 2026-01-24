# FEAT-056: User Priority Lockout for MCP Focus Control

**Priority**: P2
**Component**: fugue-server, fugue-client, fugue-protocol
**Type**: new_feature
**Estimated Effort**: medium
**Business Value**: medium
**Technical Complexity**: medium
**Status**: new

## Overview

Add a user priority lockout system that prevents MCP focus operations from conflicting with user keyboard commands. When a user enters command mode (prefix key like Ctrl+B), the server should temporarily block or defer MCP focus-changing operations.

## Problem Statement

fugue has two control paths that can conflict:

1. **TUI Client**: User presses Ctrl+B (prefix key) which enters `PrefixPending` mode for 500ms, then the next key executes a command (pane navigation, window switching, etc.)

2. **MCP Server**: Claude agents can call `fugue_focus_pane`, `fugue_select_window`, `fugue_select_session` to change focus

When both paths operate simultaneously, the user's intended target can change unexpectedly. For example:
- User presses Ctrl+B intending to press `n` for next window
- During the 500ms timeout, MCP agent calls `fugue_select_pane`
- Focus changes to MCP's target
- User presses `n` which now operates on the wrong context

This creates a "fighting" experience that frustrates users.

## Architecture

```
+-------------+     UserCommandModeEntered(timeout)   +-------------+
|  TUI Client | ------------------------------------->|   Server    |
|             |                                       |             |
|  Ctrl+B     |                                       |  user_lock  |
|  pressed    |                                       |  timestamp  |
+-------------+                                       +------+------+
                                                             |
                    +------------------------------------+
                    |
                    v
              +-------------+
              | MCP Handler |
              |             |
              | check lock  |---> Focus blocked (return busy/retry)
              | before      |     OR wait for lock expiry
              | focus ops   |
              +-------------+
```

## Requirements

### Part 1: New Protocol Messages

Add messages for client to inform server of command mode state:

```rust
pub enum ClientMessage {
    // ... existing variants ...

    /// Client entered command mode (prefix key pressed)
    UserCommandModeEntered {
        /// How long the command mode window lasts (typically 500ms)
        timeout_ms: u32,
    },

    /// Client exited command mode (command completed or cancelled)
    UserCommandModeExited,
}
```

### Part 2: Server-Side User Priority Tracking

Add state tracking to the server (per-client or global):

```rust
pub struct UserPriorityState {
    /// When the lock expires (None = no lock)
    until: Option<Instant>,
    /// Which client holds the lock
    client_id: Option<ClientId>,
}

impl UserPriorityState {
    pub fn is_active(&self) -> bool {
        self.until.map(|t| Instant::now() < t).unwrap_or(false)
    }

    pub fn set_lock(&mut self, client_id: ClientId, duration: Duration) {
        self.until = Some(Instant::now() + duration);
        self.client_id = Some(client_id);
    }

    pub fn release(&mut self) {
        self.until = None;
        self.client_id = None;
    }
}
```

### Part 3: MCP Handler Updates

Modify focus-changing MCP handlers to check the lock:

```rust
// In handlers.rs for fugue_focus_pane, fugue_select_window, fugue_select_session

async fn focus_pane(&self, params: FocusPaneParams) -> Result<FocusPaneResponse, McpError> {
    // Check user priority lock
    if self.user_priority.is_active() {
        match self.config.user_priority_behavior {
            UserPriorityBehavior::Reject => {
                return Err(McpError::UserPriorityActive {
                    message: "User is entering a command. Retry in a moment.".to_string(),
                    retry_after_ms: 500,
                });
            }
            UserPriorityBehavior::Wait => {
                // Wait for lock to expire (with timeout)
                self.wait_for_user_priority_release().await?;
            }
            UserPriorityBehavior::Warn => {
                // Continue but include warning in response
            }
        }
    }

    // ... existing focus logic ...
}
```

### Part 4: Client-Side Integration

Update input handling to send lock messages:

```rust
// In fugue-client/src/input/mod.rs

fn handle_key_event(&mut self, key: KeyEvent) -> Option<Action> {
    match &self.state {
        InputState::Normal => {
            if key == self.prefix_key {
                // Enter prefix pending mode
                self.state = InputState::PrefixPending;

                // Send lock message to server
                self.send_message(ClientMessage::UserCommandModeEntered {
                    timeout_ms: self.prefix_timeout_ms,
                });

                return Some(Action::StartPrefixTimeout);
            }
        }
        InputState::PrefixPending => {
            // Command received, execute it
            let action = self.handle_prefix_command(key);

            // Release lock
            self.send_message(ClientMessage::UserCommandModeExited);

            self.state = InputState::Normal;
            return action;
        }
        // ...
    }
}

fn on_prefix_timeout(&mut self) {
    // Timeout expired without command
    self.send_message(ClientMessage::UserCommandModeExited);
    self.state = InputState::Normal;
}
```

### Part 5: Configuration

Add configuration options:

```toml
[server.user_priority]
# Enable user priority lockout (default: true)
enabled = true

# Behavior when MCP tries to change focus during lock
# Options: "reject" | "wait" | "warn"
behavior = "reject"

# Maximum wait time when behavior = "wait" (ms)
max_wait_ms = 1000

# Default lock duration if client doesn't specify (ms)
default_timeout_ms = 500
```

### Part 6: MCP Response Updates

When lock is active, MCP responses should indicate this:

```json
{
    "error": {
        "type": "user_priority_active",
        "message": "User is entering a command. Retry in a moment.",
        "retry_after_ms": 500
    }
}
```

Or with warn behavior:

```json
{
    "status": "focused",
    "pane_id": "abc-123",
    "warnings": [
        "User may be entering a command - focus change may conflict"
    ]
}
```

## Files Affected

| File | Changes |
|------|---------|
| `fugue-protocol/src/lib.rs` | Add `UserCommandModeEntered`, `UserCommandModeExited` messages |
| `fugue-server/src/session/mod.rs` | Add `UserPriorityState` tracking |
| `fugue-server/src/mcp/handlers.rs` | Add lock checking to focus operations (lines 610-690) |
| `fugue-server/src/handlers/client.rs` | Handle new client messages |
| `fugue-server/src/config.rs` | Add user_priority configuration |
| `fugue-client/src/input/mod.rs` | Send lock messages on prefix key (around line 278) |
| `fugue-client/src/ui/app.rs` | Wire up sending the new messages |

## Use Cases

### 1. User Navigation Without Interference

```
User: Presses Ctrl+B (enters command mode)
Server: Receives UserCommandModeEntered, sets lock for 500ms
MCP: Calls fugue_focus_pane
Server: Rejects with user_priority_active error
User: Presses 'n' (next window)
Server: Receives UserCommandModeExited, releases lock
MCP: Retries fugue_focus_pane
Server: Success (lock released)
```

### 2. Lock Auto-Expires

```
User: Presses Ctrl+B (enters command mode)
Server: Sets lock for 500ms
User: Gets distracted, doesn't press anything
Server: Lock expires after 500ms
MCP: Calls fugue_focus_pane
Server: Success (lock expired)
```

### 3. Multiple TUI Clients

```
Client A: Presses Ctrl+B
Server: Sets lock for Client A
Client B: Using normally
MCP: Calls fugue_focus_pane
Server: Rejects (Client A has lock)
```

## Implementation Tasks

### Section 1: Protocol Changes
- [ ] Add `UserCommandModeEntered { timeout_ms: u32 }` to ClientMessage
- [ ] Add `UserCommandModeExited` to ClientMessage
- [ ] Update serialization/deserialization
- [ ] Add tests for new message types

### Section 2: Server-Side State
- [ ] Create `UserPriorityState` struct
- [ ] Add `is_active()` method with time check
- [ ] Add `set_lock()` and `release()` methods
- [ ] Integrate with session or server state
- [ ] Add per-client tracking if needed

### Section 3: Client Message Handling
- [ ] Handle `UserCommandModeEntered` in client handler
- [ ] Handle `UserCommandModeExited` in client handler
- [ ] Log lock state changes for debugging

### Section 4: MCP Handler Updates
- [ ] Add lock check to `fugue_focus_pane`
- [ ] Add lock check to `fugue_select_window`
- [ ] Add lock check to `fugue_select_session`
- [ ] Implement reject behavior
- [ ] Implement wait behavior (optional)
- [ ] Implement warn behavior (optional)
- [ ] Return appropriate error response

### Section 5: Client-Side Integration
- [ ] Send `UserCommandModeEntered` when entering prefix pending
- [ ] Send `UserCommandModeExited` when command completes
- [ ] Send `UserCommandModeExited` on prefix timeout
- [ ] Send `UserCommandModeExited` on Escape/cancel

### Section 6: Configuration
- [ ] Add `user_priority` config section
- [ ] Add `enabled` toggle
- [ ] Add `behavior` option (reject/wait/warn)
- [ ] Add `max_wait_ms` for wait behavior
- [ ] Add `default_timeout_ms` fallback

### Section 7: Testing
- [ ] Unit test UserPriorityState logic
- [ ] Test lock expiration timing
- [ ] Test MCP rejection when lock active
- [ ] Test lock release on command complete
- [ ] Test lock release on timeout
- [ ] Test multiple clients scenario
- [ ] Integration test full flow

## Acceptance Criteria

- [ ] User can use prefix commands (Ctrl+B + key) without MCP interference
- [ ] MCP receives clear feedback when user priority is active
- [ ] Lock automatically expires (no permanent blocking)
- [ ] Non-focus MCP operations are unaffected (read_pane, list_panes, etc.)
- [ ] Feature is configurable (enable/disable, timeout duration, behavior)
- [ ] Multiple TUI clients can each set their own lock
- [ ] All existing tests pass
- [ ] New feature has test coverage

## Dependencies

- FEAT-046: MCP Focus/Select Control (provides the focus tools to lock)

## Notes

- The 500ms timeout matches the default prefix timeout in the TUI
- Consider whether to extend this to other "blocking" user actions (e.g., copy mode)
- The `wait` behavior should have a maximum wait time to prevent indefinite blocking
- Logging should indicate when locks are set/released for debugging
- Future enhancement: UI indicator showing when user priority lock is active
