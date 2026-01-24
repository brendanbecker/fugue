# Task Breakdown: FEAT-022

**Work Item**: [FEAT-022: Client Message Routing and Handlers](PROMPT.md)
**Status**: Not Started
**Last Updated**: 2026-01-09

## Prerequisites

- [ ] Read and understand PROMPT.md
- [ ] Review PLAN.md and update if needed
- [ ] Verify FEAT-021 (Server Socket Listen Loop) is complete or in progress
- [ ] Review `fugue-protocol/src/messages.rs` - understand all ClientMessage/ServerMessage types
- [ ] Review `fugue-server/src/session/manager.rs` - understand session operations API
- [ ] Review `fugue-server/src/pty/manager.rs` - understand PTY operations API
- [ ] Review `fugue-protocol/src/types.rs` - understand ReplyMessage, ViewportState types

## Design Tasks

- [ ] Finalize HandlerContext struct fields
- [ ] Design ClientRegistry for broadcast
- [ ] Document error code mapping from internal errors
- [ ] Plan lock ordering to prevent deadlocks

## Implementation Tasks

### Module Structure (fugue-server/src/handlers/)

- [ ] Create `fugue-server/src/handlers/mod.rs`
- [ ] Create `fugue-server/src/handlers/message_router.rs`
- [ ] Create `fugue-server/src/handlers/session_handlers.rs`
- [ ] Create `fugue-server/src/handlers/input_handlers.rs`
- [ ] Create `fugue-server/src/handlers/orchestration_handlers.rs`
- [ ] Add `pub mod handlers;` to `fugue-server/src/lib.rs`
- [ ] Define `HandlerContext` struct
- [ ] Define `HandleError` enum with From<HandleError> for ServerMessage
- [ ] Define `MessageRouter` trait or struct

### Connection Handlers (session_handlers.rs)

- [ ] Implement `handle_connect()` - validate protocol version, return Connected
- [ ] Implement `handle_ping()` - return Pong
- [ ] Implement `handle_sync()` - gather full state, return Attached-like response
- [ ] Implement `handle_detach()` - disconnect client, keep session running

### Session Handlers (session_handlers.rs)

- [ ] Implement `handle_list_sessions()` - query session_manager, return SessionList
- [ ] Implement `handle_create_session()` - create session, return SessionCreated
- [ ] Implement `handle_attach_session()` - load state, return Attached
- [ ] Implement `handle_create_window()` - create window, broadcast WindowCreated

### Pane Handlers (session_handlers.rs)

- [ ] Implement `handle_create_pane()` - create pane, spawn PTY, broadcast PaneCreated
- [ ] Implement `handle_select_pane()` - update focused pane
- [ ] Implement `handle_close_pane()` - kill PTY, update state, broadcast PaneClosed
- [ ] Implement `handle_resize()` - resize PTY and pane dimensions

### Input Handlers (input_handlers.rs)

- [ ] Implement `handle_input()` - write data to PTY
- [ ] Implement `handle_reply()` - forward reply to reply mechanism
- [ ] Implement `handle_set_viewport_offset()` - update viewport scroll position
- [ ] Implement `handle_jump_to_bottom()` - reset viewport to follow

### Orchestration Handlers (orchestration_handlers.rs)

- [ ] Implement `handle_send_orchestration()` - main entry point
- [ ] Implement routing for `OrchestrationTarget::Orchestrator`
- [ ] Implement routing for `OrchestrationTarget::Session(Uuid)`
- [ ] Implement routing for `OrchestrationTarget::Broadcast`
- [ ] Implement routing for `OrchestrationTarget::Worktree(String)`
- [ ] Return OrchestrationDelivered with delivery count

### Message Router (message_router.rs)

- [ ] Implement `route_message()` function with match on ClientMessage
- [ ] Add exhaustive handling for all variants
- [ ] Add logging for incoming messages
- [ ] Add logging for outgoing responses
- [ ] Handle unexpected/malformed messages gracefully

### Client Registry and Broadcasting

- [ ] Implement `ClientRegistry` struct
- [ ] Implement `register_client()` method
- [ ] Implement `unregister_client()` method
- [ ] Implement `attach_to_session()` method
- [ ] Implement `detach_from_session()` method
- [ ] Implement `broadcast_to_session()` method
- [ ] Implement `get_attached_clients()` method

### Server Integration

- [ ] Wire message router into server listen loop (from FEAT-021)
- [ ] Pass HandlerContext to router
- [ ] Handle router errors appropriately
- [ ] Ensure proper async integration

## Testing Tasks

### Unit Tests - Connection Handlers

- [ ] Test handle_connect with valid protocol version
- [ ] Test handle_connect with mismatched protocol version
- [ ] Test handle_ping returns Pong
- [ ] Test handle_sync returns full state

### Unit Tests - Session Handlers

- [ ] Test handle_list_sessions with no sessions
- [ ] Test handle_list_sessions with multiple sessions
- [ ] Test handle_create_session success
- [ ] Test handle_attach_session success
- [ ] Test handle_attach_session with invalid session_id
- [ ] Test handle_create_window success
- [ ] Test handle_create_window with invalid session_id

### Unit Tests - Pane Handlers

- [ ] Test handle_create_pane success
- [ ] Test handle_create_pane with invalid window_id
- [ ] Test handle_select_pane success
- [ ] Test handle_close_pane success
- [ ] Test handle_resize success

### Unit Tests - Input Handlers

- [ ] Test handle_input success
- [ ] Test handle_input with invalid pane_id
- [ ] Test handle_reply success
- [ ] Test handle_reply when pane not awaiting
- [ ] Test handle_set_viewport_offset success
- [ ] Test handle_jump_to_bottom success

### Unit Tests - Orchestration

- [ ] Test handle_send_orchestration to specific session
- [ ] Test handle_send_orchestration broadcast
- [ ] Test handle_send_orchestration with no recipients

### Integration Tests

- [ ] Test full message flow: Connect -> CreateSession -> Attach
- [ ] Test full message flow: CreateWindow -> CreatePane -> Input
- [ ] Test broadcast: CreatePane broadcasts to all attached clients
- [ ] Test error flow: Invalid session returns SessionNotFound

## Documentation Tasks

- [ ] Document handler module structure in README or module docs
- [ ] Document error code mapping
- [ ] Document broadcast behavior
- [ ] Add inline documentation for public functions

## Verification Tasks

- [ ] All acceptance criteria from PROMPT.md met
- [ ] All ClientMessage variants handled
- [ ] All tests passing
- [ ] No compiler warnings
- [ ] Update feature_request.json status
- [ ] Document completion in PLAN.md

## Completion Checklist

- [ ] All implementation tasks complete
- [ ] All tests passing
- [ ] Documentation updated
- [ ] PLAN.md reflects final implementation
- [ ] Ready for review/merge

---
*Check off tasks as you complete them. Update status field above.*
