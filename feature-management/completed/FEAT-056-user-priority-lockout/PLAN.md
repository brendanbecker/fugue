# Implementation Plan: FEAT-056

**Work Item**: [FEAT-056: User Priority Lockout for MCP Focus Control](PROMPT.md)
**Component**: fugue-server, fugue-client, fugue-protocol
**Priority**: P2
**Created**: 2026-01-11

## Overview

Implement a user priority lockout system that prevents MCP focus operations from conflicting with user keyboard commands. When users enter command mode (prefix key), the server temporarily blocks MCP focus changes.

## Architecture Decisions

- **Approach**: Client-server protocol extension with server-side lock state and MCP handler checks
- **Trade-offs**:
  - Per-client locks vs global lock: Choosing per-client for multi-user scenarios
  - Reject vs wait vs warn: Implementing configurable behavior, defaulting to reject for predictability
  - Lock granularity: Focus operations only (not all MCP calls) to minimize impact on agents
  - Time-based expiry vs explicit release: Both - explicit release on command complete, timeout as fallback

## Affected Components

| Component | Type of Change | Risk Level |
|-----------|----------------|------------|
| fugue-protocol/src/lib.rs | Add 2 new message types | Low |
| fugue-server/src/session/mod.rs | Add UserPriorityState struct | Low |
| fugue-server/src/mcp/handlers.rs | Add lock checks to 3 handlers | Medium |
| fugue-server/src/handlers/client.rs | Handle new messages | Low |
| fugue-server/src/config.rs | Add configuration section | Low |
| fugue-client/src/input/mod.rs | Send lock messages | Medium |
| fugue-client/src/ui/app.rs | Wire message sending | Low |

## Implementation Details

### 1. Protocol Messages

Add to `fugue-protocol/src/lib.rs`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClientMessage {
    // ... existing variants ...

    /// Notify server that user entered command mode (prefix key pressed)
    UserCommandModeEntered {
        /// Lock duration in milliseconds
        timeout_ms: u32,
    },

    /// Notify server that user exited command mode
    UserCommandModeExited,
}
```

### 2. Server-Side State

New struct in `fugue-server/src/session/user_priority.rs`:

```rust
use std::time::{Duration, Instant};
use uuid::Uuid;

#[derive(Debug, Default)]
pub struct UserPriorityState {
    /// Active locks per client
    locks: HashMap<Uuid, Instant>,
}

impl UserPriorityState {
    pub fn is_any_active(&self) -> bool {
        let now = Instant::now();
        self.locks.values().any(|&expiry| now < expiry)
    }

    pub fn is_client_active(&self, client_id: Uuid) -> bool {
        self.locks.get(&client_id)
            .map(|&expiry| Instant::now() < expiry)
            .unwrap_or(false)
    }

    pub fn set_lock(&mut self, client_id: Uuid, duration: Duration) {
        let expiry = Instant::now() + duration;
        self.locks.insert(client_id, expiry);
    }

    pub fn release(&mut self, client_id: Uuid) {
        self.locks.remove(&client_id);
    }

    pub fn cleanup_expired(&mut self) {
        let now = Instant::now();
        self.locks.retain(|_, &mut expiry| now < expiry);
    }

    pub fn time_until_release(&self) -> Option<Duration> {
        let now = Instant::now();
        self.locks.values()
            .filter(|&&expiry| now < expiry)
            .map(|&expiry| expiry - now)
            .min()
    }
}
```

### 3. MCP Handler Lock Check

Add to each focus handler in `fugue-server/src/mcp/handlers.rs`:

```rust
async fn check_user_priority(&self) -> Result<(), McpError> {
    let state = self.user_priority.read().await;

    if !state.is_any_active() {
        return Ok(());
    }

    match self.config.user_priority.behavior {
        UserPriorityBehavior::Reject => {
            let retry_after = state.time_until_release()
                .map(|d| d.as_millis() as u32)
                .unwrap_or(500);

            Err(McpError::UserPriorityActive {
                message: "User is entering a command. Retry shortly.".into(),
                retry_after_ms: retry_after,
            })
        }
        UserPriorityBehavior::Wait => {
            drop(state);
            self.wait_for_release().await
        }
        UserPriorityBehavior::Warn => {
            // Log warning, allow operation
            tracing::warn!("MCP focus operation during user command mode");
            Ok(())
        }
    }
}

async fn wait_for_release(&self) -> Result<(), McpError> {
    let max_wait = Duration::from_millis(self.config.user_priority.max_wait_ms as u64);
    let deadline = Instant::now() + max_wait;

    loop {
        let state = self.user_priority.read().await;
        if !state.is_any_active() {
            return Ok(());
        }

        let remaining = state.time_until_release().unwrap_or(Duration::ZERO);
        drop(state);

        if Instant::now() >= deadline {
            return Err(McpError::UserPriorityTimeout);
        }

        tokio::time::sleep(remaining.min(Duration::from_millis(50))).await;
    }
}
```

### 4. Client Input Integration

Modify `fugue-client/src/input/mod.rs`:

```rust
// When prefix key detected (entering PrefixPending state)
fn enter_prefix_mode(&mut self) {
    self.input_state = InputState::PrefixPending;
    self.prefix_start = Some(Instant::now());

    // Notify server of command mode
    if let Some(ref mut conn) = self.connection {
        let _ = conn.send(ClientMessage::UserCommandModeEntered {
            timeout_ms: self.config.prefix_timeout_ms,
        });
    }
}

// When command completes or times out
fn exit_prefix_mode(&mut self) {
    self.input_state = InputState::Normal;
    self.prefix_start = None;

    // Release lock
    if let Some(ref mut conn) = self.connection {
        let _ = conn.send(ClientMessage::UserCommandModeExited);
    }
}
```

### 5. Configuration

Add to `fugue-server/src/config.rs`:

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct UserPriorityConfig {
    #[serde(default = "default_enabled")]
    pub enabled: bool,

    #[serde(default = "default_behavior")]
    pub behavior: UserPriorityBehavior,

    #[serde(default = "default_max_wait")]
    pub max_wait_ms: u32,

    #[serde(default = "default_timeout")]
    pub default_timeout_ms: u32,
}

#[derive(Debug, Clone, Copy, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum UserPriorityBehavior {
    #[default]
    Reject,
    Wait,
    Warn,
}

fn default_enabled() -> bool { true }
fn default_behavior() -> UserPriorityBehavior { UserPriorityBehavior::Reject }
fn default_max_wait() -> u32 { 1000 }
fn default_timeout() -> u32 { 500 }
```

## Dependencies

- FEAT-046: MCP Focus/Select Control (provides fugue_focus_pane, fugue_select_window, fugue_select_session)

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Lock never releases (bug) | Low | High | Time-based expiry as fallback, cleanup on client disconnect |
| Agents confused by new error | Medium | Low | Clear error message with retry_after hint |
| Race condition in lock check | Low | Medium | Use RwLock for atomic read-check-modify |
| Performance impact from lock checks | Low | Low | Lock check is O(1), minimal overhead |
| Config not reloaded on change | Low | Low | Document restart requirement for now |

## Rollback Strategy

If implementation causes issues:
1. Revert commits associated with this work item
2. MCP focus operations return to immediate execution
3. User experience returns to "fighting" behavior (degraded but functional)
4. Document issues in comments.md

## Testing Strategy

1. **Unit tests**: UserPriorityState expiry logic, lock/release behavior
2. **Protocol tests**: New message serialization/deserialization
3. **Integration tests**: Full flow from client prefix to MCP rejection
4. **Manual testing**:
   - Press Ctrl+B, trigger MCP focus, verify rejection
   - Verify lock auto-expires
   - Verify lock releases on command complete
5. **Edge cases**: Multiple clients, rapid lock/unlock, client disconnect with active lock

## Implementation Notes

<!-- Add notes during implementation -->

---
*This plan should be updated as implementation progresses.*
