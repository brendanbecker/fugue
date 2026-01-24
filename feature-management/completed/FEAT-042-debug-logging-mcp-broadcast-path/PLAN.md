# Implementation Plan: FEAT-042

**Work Item**: [FEAT-042: Debug Logging for MCP Pane Broadcast Path](PROMPT.md)
**Component**: fugue-server / fugue-client
**Priority**: P0
**Created**: 2026-01-10

## Overview

Add comprehensive debug logging throughout the MCP pane creation and broadcast path to diagnose BUG-010. This is a diagnostic enhancement to trace where `PaneCreated` broadcast messages get lost between server handler and TUI client.

## Architecture Decisions

### Decision 1: Logging Framework

**Choice**: Use `tracing` crate (already in use by the project).

**Rationale**:
- Already integrated into fugue codebase
- Structured logging with span support
- Configurable via RUST_LOG environment variable
- Zero-cost when disabled

**Trade-offs**:
- None - this is the standard choice for Rust async applications

### Decision 2: Log Levels

**Choice**: Use `debug` for detailed tracing, `info` for key milestones.

| Category | Level | Example |
|----------|-------|---------|
| Entry/exit points | debug | "handle_create_pane_with_options called" |
| Key milestones | info | "Returning ResponseWithBroadcast for pane creation" |
| Loop iterations | debug | "send_to_client result for client X" |
| State dumps | debug | "Clients registered for session" |

**Rationale**:
- Debug is appropriate for diagnostic tracing
- Info level for events we want to see in normal debug runs
- Avoids polluting logs when running at info level

**Trade-offs**:
- Must run with RUST_LOG=debug to see full trace
- Info level entries visible in normal operation (acceptable for diagnostics)

### Decision 3: ID Correlation

**Choice**: Include client_id, session_id, and pane_id in all relevant log entries.

**Rationale**:
- Enables filtering logs by specific request/session
- Critical for multi-client scenarios
- grep-friendly for debugging

**Implementation**:
```rust
tracing::debug!(
    client_id = %client_id,
    session_id = %session_id,
    pane_id = %pane_id,
    "Message description"
);
```

**Trade-offs**:
- Slightly more verbose logging code
- Larger log output

### Decision 4: Span Usage

**Choice**: Consider using tracing spans for the broadcast request flow.

**Optional Implementation**:
```rust
let span = tracing::debug_span!("broadcast_pane_created",
    session_id = %session_id,
    pane_id = %pane_id
);
let _guard = span.enter();
// All subsequent logs in this scope include span context
```

**Rationale**:
- Groups related logs together
- Automatic timing information
- Better correlation in async contexts

**Trade-offs**:
- More complex implementation
- May be overkill for this diagnostic task

**Decision**: Start without spans, add if needed.

## Affected Components

| Component | File | Type of Change | Risk Level |
|-----------|------|----------------|------------|
| MCP Handler | fugue-server/src/handlers/mcp_bridge.rs | Add tracing | Very Low |
| Server Main | fugue-server/src/main.rs | Add tracing | Very Low |
| Registry | fugue-server/src/registry.rs | Add tracing | Very Low |
| Client Connection | fugue-client/src/connection/client.rs | Add tracing | Very Low |
| Client App | fugue-client/src/ui/app.rs | Add tracing | Very Low |

## Implementation Order

### Phase 1: Server Handler (mcp_bridge.rs)

**Goal**: Trace entry and exit of pane creation handler.

1. Locate `handle_create_pane_with_options` function
2. Add entry logging with parameters
3. Add exit logging with session_id and pane_id

**Deliverable**: Can verify handler is called correctly.

### Phase 2: Server Main Routing (main.rs)

**Goal**: Trace ResponseWithBroadcast handling.

1. Locate where `ResponseWithBroadcast` is processed
2. Add logging when variant is matched
3. Add logging before/after `broadcast_to_session_except`

**Deliverable**: Can verify broadcast routing works.

### Phase 3: Server Registry (registry.rs)

**Goal**: Trace broadcast to session clients.

1. Locate `broadcast_to_session_except` function
2. Log session_clients state for target session
3. Log which clients are being sent to (after filtering)
4. Log each send_to_client result
5. Add logging to `attach_to_session` for client registration

**Deliverable**: Can verify client registration and broadcast delivery.

### Phase 4: Server Client Handler (main.rs)

**Goal**: Trace client-side message handling on server.

1. Locate the client connection handler task
2. Add logging when message received on channel (rx.recv)
3. Add logging when writing to framed_writer

**Deliverable**: Can verify messages are written to socket.

### Phase 5: Client Connection (client.rs)

**Goal**: Trace message reception from socket.

1. Locate message reading code
2. Add logging when frame received
3. Add logging with message type after deserialization

**Deliverable**: Can verify messages arrive at client.

### Phase 6: Client App (app.rs)

**Goal**: Trace message processing in TUI app.

1. Locate `poll_server_messages` function
2. Add logging when message found
3. Add logging in `ServerMessage::PaneCreated` handler

**Deliverable**: Can verify app receives and processes message.

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Logging affects performance | Very Low | Very Low | Only active at debug level |
| Logging changes behavior | Very Low | Low | Logging is read-only |
| Missing important log point | Medium | Low | Iterate and add more if needed |
| Too much log output | Medium | Low | Use debug level, filter by module |

## Testing Strategy

### Verification Steps

1. **Build successfully**
   ```bash
   cargo build --workspace
   ```

2. **Run with debug logging**
   ```bash
   RUST_LOG=fugue_server=debug,fugue_client=debug cargo run
   ```

3. **Create pane via MCP**
   - Use Claude Code or MCP client to call `fugue_create_pane`

4. **Inspect logs**
   - Verify complete message chain is logged
   - Or identify where chain breaks

### Expected Log Output (Success Case)

```
[DEBUG fugue_server::handlers::mcp_bridge] handle_create_pane_with_options called session_filter=None window_filter=None
[INFO  fugue_server::handlers::mcp_bridge] Returning ResponseWithBroadcast for pane creation session_id=abc-123 pane_id=def-456
[DEBUG fugue_server] Received ResponseWithBroadcast from handler session_id=abc-123
[DEBUG fugue_server] About to broadcast to session session_id=abc-123
[DEBUG fugue_server::registry] Clients registered for session session_id=abc-123 total_clients=1 client_ids=["client-789"]
[DEBUG fugue_server::registry] Sending broadcast to clients target_clients=["client-789"]
[DEBUG fugue_server::registry] send_to_client result client_id=client-789 success=true
[INFO  fugue_server] Broadcast complete session_id=abc-123 clients_notified=1
[DEBUG fugue_server] Client handler received broadcast from channel client_id=client-789
[DEBUG fugue_server] Writing broadcast to socket client_id=client-789 bytes=256
[DEBUG fugue_client::connection::client] Received message from server socket bytes=256
[DEBUG fugue_client::connection::client] Deserialized server message message_type=PaneCreated
[DEBUG fugue_client::ui::app] poll_server_messages received message message_type=PaneCreated
[INFO  fugue_client::ui::app] Handling PaneCreated broadcast pane_id=def-456 session_id=abc-123
```

### Expected Log Output (Failure Cases)

**Case: No clients registered for session**
```
[DEBUG fugue_server::registry] Clients registered for session session_id=abc-123 total_clients=0 client_ids=[]
[DEBUG fugue_server::registry] Sending broadcast to clients target_clients=[]
[INFO  fugue_server] Broadcast complete session_id=abc-123 clients_notified=0
```
Diagnosis: Client not attached to correct session.

**Case: Channel send fails**
```
[DEBUG fugue_server::registry] send_to_client result client_id=client-789 success=false channel_error="channel closed"
```
Diagnosis: Client connection dropped or channel issue.

**Case: Client doesn't process message**
```
[DEBUG fugue_client::connection::client] Received message from server socket bytes=256
[DEBUG fugue_client::connection::client] Deserialized server message message_type=PaneCreated
# No further logs from app
```
Diagnosis: poll_server_messages not being called or not processing message.

## Rollback Strategy

If logging causes issues:
1. Comment out individual log statements
2. Change log levels to trace (effectively disabled)
3. No functional changes to revert

## Implementation Notes

### Typical Tracing Pattern

```rust
use tracing::{debug, info};

pub async fn handle_create_pane_with_options(
    session_filter: Option<SessionFilter>,
    window_filter: Option<WindowFilter>,
    // ...
) -> Result<ResponseWithBroadcast, Error> {
    debug!(
        ?session_filter,
        ?window_filter,
        "handle_create_pane_with_options called"
    );

    // ... implementation ...

    info!(
        %session_id,
        %pane_id,
        "Returning ResponseWithBroadcast for pane creation"
    );

    Ok(ResponseWithBroadcast { ... })
}
```

### Import Requirements

Ensure these are imported in each file:
```rust
use tracing::{debug, info};
// Or for specific needs:
use tracing::{debug, info, warn, error};
```

---
*This plan should be updated as implementation progresses.*
