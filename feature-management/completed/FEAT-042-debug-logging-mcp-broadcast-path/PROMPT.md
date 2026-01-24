# FEAT-042: Debug Logging for MCP Pane Broadcast Path

**Priority**: P0
**Component**: fugue-server / fugue-client
**Type**: enhancement (diagnostic)
**Estimated Effort**: medium
**Business Value**: high
**Status**: new

## Overview

Add comprehensive debug logging throughout the MCP pane creation and broadcast path to diagnose BUG-010. The issue is that the TUI never receives `PaneCreated` broadcasts when panes are created via MCP, even though unit tests pass. We need to trace the actual message flow in a live system to find where messages get lost.

## Problem Statement

BUG-010 describes a situation where:
- MCP creates panes successfully (server confirms pane exists)
- TUI never receives the `PaneCreated` broadcast
- Unit tests pass but the bug persists in practice
- We cannot determine where messages are lost without tracing

### Current Investigation Status

We've reviewed the code but cannot find the root cause:
- `handle_create_pane_with_options` returns `ResponseWithBroadcast`
- `broadcast_to_session_except` should send to all session clients
- TUI should receive via `poll_server_messages`
- All code looks correct but messages aren't reaching TUI

### Why Logging Is Needed

The message passes through many components:
1. Handler creates `ResponseWithBroadcast`
2. Main routing receives and processes it
3. Registry broadcasts to session clients
4. Client handler receives on channel
5. Client connection reads from socket
6. App processes in `poll_server_messages`

Any of these steps could be failing silently.

## Requirements

### Logging Points

All logging should use the `tracing` crate with appropriate levels (debug/info) and include relevant IDs (client_id, session_id, pane_id) for correlation.

#### 1. Server - Handler (`fugue-server/src/handlers/mcp_bridge.rs`)

```rust
// When handle_create_pane_with_options is called
tracing::debug!(
    session_filter = ?session_filter,
    window_filter = ?window_filter,
    command = ?command,
    "handle_create_pane_with_options called"
);

// When returning ResponseWithBroadcast
tracing::info!(
    session_id = %session_id,
    pane_id = %pane_id,
    broadcast_type = "PaneCreated",
    "Returning ResponseWithBroadcast for pane creation"
);
```

#### 2. Server - Main Routing (`fugue-server/src/main.rs`)

```rust
// When ResponseWithBroadcast is received from handler
tracing::debug!(
    session_id = %session_id,
    message_type = ?message_type,
    "Received ResponseWithBroadcast from handler"
);

// Before calling broadcast_to_session_except
tracing::debug!(
    session_id = %session_id,
    except_client = ?except_client,
    "About to broadcast to session"
);

// After broadcast, log the result count
tracing::info!(
    session_id = %session_id,
    clients_notified = count,
    "Broadcast complete"
);
```

#### 3. Server - Registry (`fugue-server/src/registry.rs`)

```rust
// Log all clients in session_clients[session_id]
tracing::debug!(
    session_id = %session_id,
    total_clients = clients.len(),
    client_ids = ?client_ids,
    "Clients registered for session"
);

// Log which clients are being sent to (after filtering except_client)
tracing::debug!(
    session_id = %session_id,
    target_clients = ?target_client_ids,
    excluded_client = ?except_client,
    "Sending broadcast to clients"
);

// Log success/failure of each send_to_client call
tracing::debug!(
    client_id = %client_id,
    success = result.is_ok(),
    "send_to_client result"
);

// Log channel send results
tracing::debug!(
    client_id = %client_id,
    channel_result = ?result,
    "Channel send complete"
);
```

#### 4. Server - Client Handler (`fugue-server/src/main.rs`)

```rust
// When broadcast is received on rx.recv()
tracing::debug!(
    client_id = %client_id,
    message_type = ?message_type,
    "Client handler received broadcast from channel"
);

// When writing to framed_writer
tracing::debug!(
    client_id = %client_id,
    bytes = frame.len(),
    "Writing broadcast to socket"
);
```

#### 5. Client - Connection (`fugue-client/src/connection/client.rs`)

```rust
// When message received from socket
tracing::debug!(
    bytes = frame.len(),
    "Received message from server socket"
);

// After deserializing, log message type
tracing::debug!(
    message_type = ?message_type,
    "Deserialized server message"
);
```

#### 6. Client - App (`fugue-client/src/ui/app.rs`)

```rust
// When poll_server_messages finds a message
tracing::debug!(
    message_type = ?message_type,
    "poll_server_messages received message"
);

// When handling ServerMessage::PaneCreated
tracing::info!(
    pane_id = %pane_id,
    session_id = %session_id,
    "Handling PaneCreated broadcast"
);
```

### Additional Diagnostic Logging

#### Session Registration

```rust
// In attach_to_session
tracing::info!(
    client_id = %client_id,
    session_id = %session_id,
    "Client attached to session"
);

// Log current session_clients state after attach
tracing::debug!(
    session_id = %session_id,
    total_clients = clients.len(),
    "Session clients after attach"
);
```

#### Client Connection State

```rust
// When client connects
tracing::info!(
    client_id = %client_id,
    "New client connected"
);

// When client disconnects
tracing::info!(
    client_id = %client_id,
    "Client disconnected"
);
```

## Files Affected

| File | Changes |
|------|---------|
| `fugue-server/src/handlers/mcp_bridge.rs` | Add tracing for pane creation handler |
| `fugue-server/src/main.rs` | Add tracing for ResponseWithBroadcast routing and client handler |
| `fugue-server/src/registry.rs` | Add tracing for broadcast_to_session_except and client tracking |
| `fugue-client/src/connection/client.rs` | Add tracing for message reception |
| `fugue-client/src/ui/app.rs` | Add tracing for poll_server_messages and PaneCreated handling |

## Implementation Tasks

### Section 1: Server Handler Logging
- [ ] Add tracing to `handle_create_pane_with_options` entry point
- [ ] Log session_id being used for broadcast
- [ ] Log when returning `ResponseWithBroadcast`

### Section 2: Server Main Routing Logging
- [ ] Add tracing when receiving `ResponseWithBroadcast`
- [ ] Log before calling `broadcast_to_session_except`
- [ ] Log broadcast result count

### Section 3: Server Registry Logging
- [ ] Log clients registered for session in `broadcast_to_session_except`
- [ ] Log which clients are receiving broadcast (after filter)
- [ ] Log each `send_to_client` result
- [ ] Log session attachment in `attach_to_session`

### Section 4: Server Client Handler Logging
- [ ] Log when broadcast received on channel (rx.recv)
- [ ] Log when writing to framed_writer

### Section 5: Client Connection Logging
- [ ] Log raw message received from socket
- [ ] Log message type after deserialization

### Section 6: Client App Logging
- [ ] Log in `poll_server_messages` when message found
- [ ] Log when handling `ServerMessage::PaneCreated`

### Section 7: Verification
- [ ] All logging compiles without errors
- [ ] Logs are at appropriate levels (debug/info)
- [ ] IDs included for correlation
- [ ] Run live test with RUST_LOG=debug
- [ ] Trace message path to find where it breaks

## Acceptance Criteria

- [ ] All logging points from requirements are implemented
- [ ] Logs use `tracing` crate with appropriate levels (debug/info)
- [ ] Logs include relevant IDs (client_id, session_id, pane_id)
- [ ] Logging can be enabled via RUST_LOG=debug or RUST_LOG=fugue=debug
- [ ] Running a live test with logging enabled shows complete message path
- [ ] Or: logging reveals exactly where the message is lost

## Usage

After implementation, run with debug logging enabled:

```bash
# Full debug logging
RUST_LOG=debug fugue

# Just fugue modules
RUST_LOG=fugue_server=debug,fugue_client=debug fugue

# Focus on specific area
RUST_LOG=fugue_server::registry=debug fugue
```

Then trigger MCP pane creation and inspect logs for the message flow.

## Expected Outcome

The logging should reveal one of these scenarios:

1. **Handler returns broadcast correctly, main routing receives it**
   - Continue tracing to registry...

2. **Registry receives broadcast, finds session clients**
   - Continue tracing to send_to_client...

3. **send_to_client succeeds, client handler receives**
   - Continue tracing to socket write...

4. **Socket write succeeds, client receives**
   - Continue tracing to app...

5. **App receives message, handles PaneCreated**
   - Bug is elsewhere (e.g., rendering)

Or, the logs will show where the chain breaks:
- "Returning ResponseWithBroadcast" but no "Received ResponseWithBroadcast" = routing issue
- "Sending broadcast to clients" with 0 clients = registration issue
- "Channel send complete" with error = channel issue
- etc.

## Related Work Items

- **BUG-010**: MCP Pane Creation Broadcast Not Received by TUI - this feature exists to diagnose this bug
- **FEAT-039**: MCP Pane Creation Broadcast - the feature that implemented the broadcast
- **FEAT-040**: MCP Pane Reliability Improvements - recent changes to MCP pane handling

## Notes

- This is a diagnostic enhancement, not a fix for BUG-010
- Logging should remain in codebase (useful for future debugging)
- Use tracing spans for request-scoped correlation where appropriate
- Consider adding a unique request_id that flows through the entire path
- After BUG-010 is fixed, some logging can be downgraded to trace level
