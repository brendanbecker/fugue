# FEAT-022: Client Message Routing and Handlers

**Priority**: P0 (Critical)
**Component**: fugue-server
**Type**: new_feature
**Estimated Effort**: Medium (3-4 hours)
**Business Value**: high
**Status**: new

## Overview

Route incoming ClientMessage types to appropriate handlers and respond with ServerMessages. This feature implements the complete message handling layer that connects client requests to the underlying session manager and PTY operations.

## Requirements

Handle all ClientMessage variants defined in `fugue-protocol/src/messages.rs` (lines 69-138):

1. **Connect** - Send `ServerMessage::Connected` with server version and protocol info
2. **ListSessions** - Return `ServerMessage::SessionList` with available sessions
3. **CreateSession** - Call `session_manager.create_session()`, return `SessionCreated`
4. **AttachSession** - Load session state, return `Attached` with windows/panes
5. **CreateWindow** - Call `session_manager.create_window()`, broadcast `WindowCreated`
6. **CreatePane** - Call `session_manager.create_pane()`, spawn PTY, broadcast `PaneCreated`
7. **Input** - Write to PTY, handle reply mechanism
8. **Resize** - Resize PTY and pane
9. **ClosePane** - Kill PTY, update state
10. **SelectPane** - Update focused pane
11. **Detach** - Disconnect client from session (keep session running)
12. **Sync** - Send full state dump to client
13. **Ping** - Respond with `Pong`
14. **SetViewportOffset** - Update pane viewport scroll position
15. **JumpToBottom** - Reset viewport to follow output
16. **Reply** - Forward reply to pane awaiting input
17. **SendOrchestration** - Route orchestration message to target sessions

## Location

New handler module in `fugue-server/src/`:
- `fugue-server/src/handlers/mod.rs` - Handler module
- `fugue-server/src/handlers/message_router.rs` - Main message routing logic
- `fugue-server/src/handlers/session_handlers.rs` - Session/window/pane operations
- `fugue-server/src/handlers/input_handlers.rs` - Input and PTY operations
- `fugue-server/src/handlers/orchestration_handlers.rs` - Orchestration message routing

## Technical Notes

- Session operations exist in `session/manager.rs`
- PTY operations exist in `pty/manager.rs`
- Message types defined in `fugue-protocol/src/messages.rs` (lines 69-138)
- Reply handling framework exists in `reply.rs`
- Use async/await for all operations
- Errors should return `ServerMessage::Error` with appropriate `ErrorCode`

## Implementation Tasks

### Section 1: Handler Module Structure
- [ ] Create `fugue-server/src/handlers/mod.rs`
- [ ] Create message router trait/struct
- [ ] Define handler context (session_manager, pty_manager, client connections)
- [ ] Set up error handling patterns

### Section 2: Connection Handlers
- [ ] Implement `Connect` handler - validate protocol version, return `Connected`
- [ ] Implement `Ping` handler - return `Pong`
- [ ] Implement `Sync` handler - gather and send full state
- [ ] Implement `Detach` handler - disconnect client, keep session

### Section 3: Session Handlers
- [ ] Implement `ListSessions` handler - query session manager, return list
- [ ] Implement `CreateSession` handler - create session, return `SessionCreated`
- [ ] Implement `AttachSession` handler - load state, return `Attached`
- [ ] Implement `CreateWindow` handler - create window, broadcast `WindowCreated`

### Section 4: Pane Handlers
- [ ] Implement `CreatePane` handler - create pane, spawn PTY, broadcast `PaneCreated`
- [ ] Implement `SelectPane` handler - update focused pane
- [ ] Implement `ClosePane` handler - kill PTY, update state, broadcast `PaneClosed`
- [ ] Implement `Resize` handler - resize PTY and pane

### Section 5: Input Handlers
- [ ] Implement `Input` handler - write data to PTY
- [ ] Implement `Reply` handler - forward to reply mechanism
- [ ] Implement `SetViewportOffset` handler - update viewport state
- [ ] Implement `JumpToBottom` handler - reset viewport

### Section 6: Orchestration Handlers
- [ ] Implement `SendOrchestration` handler - route to target sessions
- [ ] Handle `OrchestrationTarget` variants (Orchestrator, Session, Broadcast, Worktree)
- [ ] Return `OrchestrationDelivered` with delivery count

### Section 7: Message Router Integration
- [ ] Implement main `route_message()` function
- [ ] Wire router into server listen loop (FEAT-021)
- [ ] Add logging for message handling
- [ ] Handle unknown/malformed messages gracefully

### Section 8: Broadcasting
- [ ] Implement broadcast mechanism for state changes
- [ ] Broadcast `WindowCreated` to attached clients
- [ ] Broadcast `PaneCreated` to attached clients
- [ ] Broadcast `PaneClosed` to attached clients
- [ ] Broadcast `PaneStateChanged` as needed

### Section 9: Testing
- [ ] Unit tests for each message handler
- [ ] Unit tests for error cases (session not found, pane not found, etc.)
- [ ] Integration tests for message routing
- [ ] Integration tests for broadcast behavior

## Acceptance Criteria

- [ ] All ClientMessage types handled with appropriate responses
- [ ] State changes broadcast to all attached clients
- [ ] Error responses returned for invalid operations (SessionNotFound, PaneNotFound, etc.)
- [ ] Handler properly wired into server listen loop
- [ ] Integration tests for each message type
- [ ] All tests passing

## Dependencies

- **FEAT-021**: Server Socket Listen Loop - Provides the connection handling infrastructure

## Notes

- Handler should be stateless - all state lives in session_manager and pty_manager
- Use `Arc<RwLock<>>` or similar for shared state access
- Consider using channels for broadcasting to avoid holding locks during send
- Error codes are defined in `fugue-protocol/src/messages.rs` (ErrorCode enum)
- Reply mechanism uses types from `fugue-protocol/src/types.rs`
