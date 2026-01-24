# Implementation Plan: FEAT-022

**Work Item**: [FEAT-022: Client Message Routing and Handlers](PROMPT.md)
**Component**: fugue-server
**Priority**: P0
**Created**: 2026-01-09

## Overview

Route incoming ClientMessage types to appropriate handlers and respond with ServerMessages. This is a critical path feature that enables all client-server communication beyond the initial socket connection.

## Architecture Decisions

### Handler Module Structure

```
fugue-server/src/
  handlers/
    mod.rs                    # Module exports, MessageRouter trait
    message_router.rs         # Main routing logic, dispatch table
    session_handlers.rs       # Session/window/pane CRUD operations
    input_handlers.rs         # Input, resize, viewport operations
    orchestration_handlers.rs # Inter-session messaging
```

### Handler Context

Each handler receives a context struct containing:

```rust
pub struct HandlerContext {
    pub session_manager: Arc<SessionManager>,
    pub pty_manager: Arc<PtyManager>,
    pub client_registry: Arc<ClientRegistry>,  // For broadcasting
    pub client_id: Uuid,                       // Current client
}
```

### Message Routing Pattern

Use a match expression for routing (simple, fast, exhaustive):

```rust
pub async fn route_message(
    ctx: &HandlerContext,
    message: ClientMessage,
) -> Result<ServerMessage, HandleError> {
    match message {
        ClientMessage::Connect { client_id, protocol_version } => {
            handle_connect(ctx, client_id, protocol_version).await
        }
        ClientMessage::ListSessions => handle_list_sessions(ctx).await,
        ClientMessage::CreateSession { name } => {
            handle_create_session(ctx, name).await
        }
        // ... remaining handlers
    }
}
```

### Broadcasting Strategy

Use a channel-based approach to avoid lock contention:

```rust
pub struct ClientRegistry {
    clients: RwLock<HashMap<Uuid, ClientHandle>>,
}

pub struct ClientHandle {
    tx: mpsc::Sender<ServerMessage>,
    attached_session: Option<Uuid>,
}
```

When state changes occur:
1. Handler completes the operation
2. Handler calls `broadcast_to_session(session_id, message)`
3. ClientRegistry iterates attached clients and sends via channels
4. No locks held during actual network send

### Error Handling

Map internal errors to protocol ErrorCodes:

```rust
pub enum HandleError {
    SessionNotFound(Uuid),
    WindowNotFound(Uuid),
    PaneNotFound(Uuid),
    InvalidOperation(String),
    ProtocolMismatch { expected: u32, actual: u32 },
    Internal(anyhow::Error),
}

impl From<HandleError> for ServerMessage {
    fn from(err: HandleError) -> Self {
        match err {
            HandleError::SessionNotFound(_) => ServerMessage::Error {
                code: ErrorCode::SessionNotFound,
                message: err.to_string(),
            },
            // ...
        }
    }
}
```

## Affected Components

| Component | Type of Change | Risk Level |
|-----------|----------------|------------|
| fugue-server/src/handlers/ | New module | Low |
| fugue-server/src/lib.rs | Add handlers module | Low |
| fugue-server/src/server.rs | Wire in message router | Medium |
| fugue-server/src/session/manager.rs | Use existing API | Low |
| fugue-server/src/pty/manager.rs | Use existing API | Low |

## Dependencies

- **FEAT-021**: Server Socket Listen Loop - Must be complete to wire in handlers
- Uses: `fugue-protocol::messages::{ClientMessage, ServerMessage, ErrorCode}`
- Uses: `fugue-server::session::SessionManager`
- Uses: `fugue-server::pty::PtyManager`

## Handler Implementation Details

### Connect Handler
```rust
async fn handle_connect(
    ctx: &HandlerContext,
    client_id: Uuid,
    protocol_version: u32,
) -> Result<ServerMessage, HandleError> {
    const CURRENT_PROTOCOL_VERSION: u32 = 1;

    if protocol_version != CURRENT_PROTOCOL_VERSION {
        return Err(HandleError::ProtocolMismatch {
            expected: CURRENT_PROTOCOL_VERSION,
            actual: protocol_version,
        });
    }

    ctx.client_registry.register(client_id)?;

    Ok(ServerMessage::Connected {
        server_version: env!("CARGO_PKG_VERSION").to_string(),
        protocol_version: CURRENT_PROTOCOL_VERSION,
    })
}
```

### CreatePane Handler (Complex Example)
```rust
async fn handle_create_pane(
    ctx: &HandlerContext,
    window_id: Uuid,
    direction: SplitDirection,
) -> Result<ServerMessage, HandleError> {
    // 1. Validate window exists
    let window = ctx.session_manager
        .get_window(window_id)
        .ok_or(HandleError::WindowNotFound(window_id))?;

    // 2. Create pane in session manager
    let pane = ctx.session_manager.create_pane(window_id, direction)?;

    // 3. Spawn PTY for the pane
    let pty_handle = ctx.pty_manager.spawn_pty(
        pane.id,
        pane.cols,
        pane.rows,
    ).await?;

    // 4. Build response
    let pane_info = PaneInfo {
        id: pane.id,
        window_id,
        rows: pane.rows,
        cols: pane.cols,
        // ...
    };

    // 5. Broadcast to attached clients
    let session_id = window.session_id;
    ctx.client_registry.broadcast_to_session(
        session_id,
        ServerMessage::PaneCreated { pane: pane_info.clone() },
    ).await;

    Ok(ServerMessage::PaneCreated { pane: pane_info })
}
```

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Deadlock from nested locks | Medium | High | Use channels for broadcast, audit lock order |
| Handler panics | Low | Medium | Wrap handlers in catch_unwind |
| Slow handler blocks others | Medium | Medium | Use tokio::spawn for long operations |
| State inconsistency | Low | High | Transaction-like patterns, rollback on error |

## Implementation Phases

### Phase 1: Module Structure (30 min)
- Create handlers module with proper exports
- Define HandlerContext and HandleError
- Set up basic routing skeleton

### Phase 2: Simple Handlers (45 min)
- Connect, Ping/Pong, ListSessions, Sync
- These have minimal state interaction

### Phase 3: Session/Window Handlers (45 min)
- CreateSession, AttachSession, CreateWindow
- Focus on correct session_manager integration

### Phase 4: Pane and Input Handlers (60 min)
- CreatePane, ClosePane, SelectPane, Resize
- Input, Reply, SetViewportOffset, JumpToBottom
- PTY integration is the complex part

### Phase 5: Orchestration and Broadcast (45 min)
- SendOrchestration with target routing
- ClientRegistry broadcast implementation

### Phase 6: Testing (60 min)
- Unit tests per handler
- Integration tests for full message flow
- Error case coverage

## Rollback Strategy

If implementation causes issues:
1. Revert commits associated with this work item
2. Keep handlers module but stub out implementations
3. Return `Error { code: InternalError, message: "Not implemented" }` for all handlers
4. Verify server still accepts connections

## Implementation Notes

<!-- Add notes during implementation -->

---
*This plan should be updated as implementation progresses.*
