# Task Breakdown: FEAT-056

**Work Item**: [FEAT-056: User Priority Lockout for MCP Focus Control](PROMPT.md)
**Status**: Not Started
**Last Updated**: 2026-01-11

## Prerequisites

- [ ] Read and understand PROMPT.md
- [ ] Review PLAN.md and update if needed
- [ ] Review existing input handling in fugue-client/src/input/mod.rs
- [ ] Review MCP handlers in fugue-server/src/mcp/handlers.rs
- [ ] Understand PrefixPending state machine in client

## Design Tasks

- [ ] Review requirements and acceptance criteria
- [ ] Confirm protocol message format
- [ ] Decide on lock state storage location (session vs server level)
- [ ] Design error response format for MCP
- [ ] Update PLAN.md with findings

## Implementation Tasks

### Protocol Changes (fugue-protocol)
- [ ] Add `UserCommandModeEntered { timeout_ms: u32 }` to ClientMessage enum
- [ ] Add `UserCommandModeExited` to ClientMessage enum
- [ ] Update serde derives for new variants
- [ ] Add unit tests for message serialization
- [ ] Add unit tests for message deserialization

### Server State (fugue-server/src/session/)
- [ ] Create `user_priority.rs` module
- [ ] Implement `UserPriorityState` struct
- [ ] Implement `is_any_active()` method
- [ ] Implement `is_client_active()` method
- [ ] Implement `set_lock()` method
- [ ] Implement `release()` method
- [ ] Implement `cleanup_expired()` method
- [ ] Implement `time_until_release()` method
- [ ] Add RwLock wrapper for thread-safe access
- [ ] Add state to server or session manager

### Client Message Handler (fugue-server)
- [ ] Handle `UserCommandModeEntered` message
- [ ] Validate timeout_ms is reasonable (e.g., 100-5000)
- [ ] Call `set_lock()` with client ID and duration
- [ ] Handle `UserCommandModeExited` message
- [ ] Call `release()` with client ID
- [ ] Add logging for lock state changes
- [ ] Handle client disconnect (release any held lock)

### MCP Handler Updates (fugue-server/src/mcp/handlers.rs)
- [ ] Add `check_user_priority()` helper method
- [ ] Implement reject behavior
- [ ] Implement wait behavior with timeout
- [ ] Implement warn behavior (log + continue)
- [ ] Add lock check to `fugue_focus_pane` handler
- [ ] Add lock check to `fugue_select_window` handler
- [ ] Add lock check to `fugue_select_session` handler
- [ ] Create `McpError::UserPriorityActive` variant
- [ ] Create `McpError::UserPriorityTimeout` variant
- [ ] Format error response with retry_after_ms

### Configuration (fugue-server/src/config.rs)
- [ ] Add `UserPriorityConfig` struct
- [ ] Add `enabled: bool` field (default: true)
- [ ] Add `behavior: UserPriorityBehavior` enum field
- [ ] Add `max_wait_ms: u32` field (default: 1000)
- [ ] Add `default_timeout_ms: u32` field (default: 500)
- [ ] Add `[server.user_priority]` section to config
- [ ] Add serde deserialization
- [ ] Add default values

### Client Input Integration (fugue-client/src/input/mod.rs)
- [ ] Locate prefix key handling code
- [ ] Add message sending capability to input handler
- [ ] Send `UserCommandModeEntered` when entering PrefixPending
- [ ] Include configured timeout_ms in message
- [ ] Send `UserCommandModeExited` when command executes
- [ ] Send `UserCommandModeExited` when prefix times out
- [ ] Send `UserCommandModeExited` on Escape/cancel in prefix mode
- [ ] Handle send errors gracefully (log, don't crash)

### Client App Integration (fugue-client/src/ui/app.rs)
- [ ] Ensure input handler has access to connection/channel
- [ ] Wire up message sending path
- [ ] Test message flow from input to server

## Testing Tasks

### Unit Tests
- [ ] Test UserPriorityState::is_any_active() with no locks
- [ ] Test UserPriorityState::is_any_active() with active lock
- [ ] Test UserPriorityState::is_any_active() with expired lock
- [ ] Test UserPriorityState::set_lock() creates lock
- [ ] Test UserPriorityState::release() removes lock
- [ ] Test UserPriorityState::cleanup_expired() removes old locks
- [ ] Test UserPriorityState::time_until_release() calculation
- [ ] Test multiple clients with independent locks

### Protocol Tests
- [ ] Test UserCommandModeEntered serialization
- [ ] Test UserCommandModeEntered deserialization
- [ ] Test UserCommandModeExited serialization
- [ ] Test UserCommandModeExited deserialization

### Integration Tests
- [ ] Test full flow: prefix -> lock -> MCP reject -> release
- [ ] Test lock expires after timeout
- [ ] Test early release on command complete
- [ ] Test reject behavior error response format
- [ ] Test wait behavior waits correctly
- [ ] Test wait behavior times out after max_wait_ms
- [ ] Test warn behavior logs but continues

### Edge Case Tests
- [ ] Test client disconnect releases lock
- [ ] Test rapid lock/unlock cycles
- [ ] Test concurrent MCP requests during lock
- [ ] Test config disabled (no lock checks)
- [ ] Test invalid timeout_ms (too large)

### Manual Testing
- [ ] Press Ctrl+B, immediately call MCP focus via claude
- [ ] Verify error message is clear
- [ ] Press Ctrl+B, wait for timeout, call MCP focus
- [ ] Verify success after timeout
- [ ] Press Ctrl+B+n, call MCP focus during
- [ ] Verify lock releases after command

## Documentation Tasks

- [ ] Document new protocol messages
- [ ] Document configuration options
- [ ] Document MCP error response format
- [ ] Add code comments to UserPriorityState
- [ ] Add code comments to MCP lock check
- [ ] Update CHANGELOG

## Verification Tasks

- [ ] All acceptance criteria from PROMPT.md met
- [ ] Tests passing
- [ ] Update feature_request.json status
- [ ] Document completion in comments.md (if any issues)

## Completion Checklist

- [ ] All implementation tasks complete
- [ ] All tests passing
- [ ] Documentation updated
- [ ] PLAN.md reflects final implementation
- [ ] No regressions in existing functionality
- [ ] Ready for review/merge

---
*Check off tasks as you complete them. Update status field above.*
