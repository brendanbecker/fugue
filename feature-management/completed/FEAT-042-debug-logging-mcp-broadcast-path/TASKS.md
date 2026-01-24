# Task Breakdown: FEAT-042

**Work Item**: [FEAT-042: Debug Logging for MCP Pane Broadcast Path](PROMPT.md)
**Status**: Not Started
**Last Updated**: 2026-01-10

## Prerequisites

- [ ] Read and understand PROMPT.md
- [ ] Review PLAN.md and update if needed
- [ ] Verify tracing crate is already in Cargo.toml dependencies
- [ ] Understand the message flow: Handler -> Main -> Registry -> Client Handler -> Socket -> Client App

## Phase 1: Server Handler Logging (mcp_bridge.rs)

### 1.1 Locate Handler Function
- [ ] Find `handle_create_pane_with_options` in fugue-server/src/handlers/mcp_bridge.rs
- [ ] Identify entry point and return points
- [ ] Note the session_id variable used for broadcast

### 1.2 Add Entry Logging
- [ ] Add `use tracing::{debug, info};` import if not present
- [ ] Add debug log at function entry with session_filter, window_filter, command params
- [ ] Example: `debug!(?session_filter, ?window_filter, "handle_create_pane_with_options called")`

### 1.3 Add Exit Logging
- [ ] Add info log before returning ResponseWithBroadcast
- [ ] Include session_id and pane_id in log
- [ ] Example: `info!(%session_id, %pane_id, "Returning ResponseWithBroadcast for pane creation")`

## Phase 2: Server Main Routing Logging (main.rs)

### 2.1 Locate ResponseWithBroadcast Handling
- [ ] Find where ResponseWithBroadcast is matched/handled in main.rs
- [ ] Identify the broadcast_to_session_except call

### 2.2 Add Receipt Logging
- [ ] Add debug log when ResponseWithBroadcast variant is received
- [ ] Include session_id and message type
- [ ] Example: `debug!(%session_id, ?message_type, "Received ResponseWithBroadcast from handler")`

### 2.3 Add Pre-Broadcast Logging
- [ ] Add debug log before calling broadcast_to_session_except
- [ ] Include session_id and except_client
- [ ] Example: `debug!(%session_id, ?except_client, "About to broadcast to session")`

### 2.4 Add Post-Broadcast Logging
- [ ] Add info log after broadcast completes
- [ ] Include session_id and count of clients notified
- [ ] Example: `info!(%session_id, clients_notified = count, "Broadcast complete")`

## Phase 3: Server Registry Logging (registry.rs)

### 3.1 Locate broadcast_to_session_except
- [ ] Find `broadcast_to_session_except` in fugue-server/src/registry.rs
- [ ] Understand how it iterates over session_clients

### 3.2 Add Session Clients Dump
- [ ] Log total clients registered for the session
- [ ] Log list of client_ids in session_clients[session_id]
- [ ] Example: `debug!(%session_id, total_clients = clients.len(), ?client_ids, "Clients registered for session")`

### 3.3 Add Target Clients Logging
- [ ] After filtering except_client, log which clients will receive broadcast
- [ ] Example: `debug!(%session_id, ?target_client_ids, ?excluded_client, "Sending broadcast to clients")`

### 3.4 Add send_to_client Result Logging
- [ ] Log success/failure for each send_to_client call
- [ ] Include client_id and result
- [ ] Example: `debug!(%client_id, success = result.is_ok(), "send_to_client result")`

### 3.5 Add Channel Send Logging
- [ ] If send_to_client uses a channel, log the channel send result
- [ ] Example: `debug!(%client_id, ?channel_result, "Channel send complete")`

### 3.6 Add attach_to_session Logging
- [ ] Locate `attach_to_session` function
- [ ] Add info log when client attaches to session
- [ ] Log session_clients state after attach
- [ ] Example: `info!(%client_id, %session_id, "Client attached to session")`

## Phase 4: Server Client Handler Logging (main.rs)

### 4.1 Locate Client Handler Task
- [ ] Find the per-client connection handler in main.rs
- [ ] Identify where it receives messages from channel (rx.recv())

### 4.2 Add Channel Receive Logging
- [ ] Log when a message is received on the channel
- [ ] Include client_id and message type
- [ ] Example: `debug!(%client_id, ?message_type, "Client handler received broadcast from channel")`

### 4.3 Add Socket Write Logging
- [ ] Log when writing message to framed_writer
- [ ] Include client_id and frame size
- [ ] Example: `debug!(%client_id, bytes = frame.len(), "Writing broadcast to socket")`

## Phase 5: Client Connection Logging (client.rs)

### 5.1 Locate Message Reception
- [ ] Find message reading code in fugue-client/src/connection/client.rs
- [ ] Identify where frames are read from socket

### 5.2 Add Raw Frame Logging
- [ ] Log when frame received from socket
- [ ] Include frame size
- [ ] Example: `debug!(bytes = frame.len(), "Received message from server socket")`

### 5.3 Add Deserialization Logging
- [ ] Log after message is deserialized
- [ ] Include message type
- [ ] Example: `debug!(?message_type, "Deserialized server message")`

## Phase 6: Client App Logging (app.rs)

### 6.1 Locate poll_server_messages
- [ ] Find `poll_server_messages` in fugue-client/src/ui/app.rs
- [ ] Understand how it processes messages

### 6.2 Add Message Found Logging
- [ ] Log when poll_server_messages finds a message
- [ ] Include message type
- [ ] Example: `debug!(?message_type, "poll_server_messages received message")`

### 6.3 Locate PaneCreated Handler
- [ ] Find where ServerMessage::PaneCreated is handled
- [ ] Typically in a match statement

### 6.4 Add PaneCreated Handling Logging
- [ ] Log when PaneCreated message is being handled
- [ ] Include pane_id and session_id
- [ ] Example: `info!(%pane_id, %session_id, "Handling PaneCreated broadcast")`

## Phase 7: Verification

### 7.1 Build Verification
- [ ] Run `cargo build --workspace` - must compile without errors
- [ ] Fix any import issues
- [ ] Fix any unused variable warnings

### 7.2 Log Level Verification
- [ ] Verify debug logs use `debug!()` macro
- [ ] Verify milestone logs use `info!()` macro
- [ ] No error/warn levels unless appropriate

### 7.3 ID Inclusion Verification
- [ ] Review all logs include relevant IDs
- [ ] session_id where applicable
- [ ] client_id where applicable
- [ ] pane_id where applicable

### 7.4 Live Test
- [ ] Start fugue with `RUST_LOG=debug fugue`
- [ ] Create/attach to a session
- [ ] Trigger MCP pane creation
- [ ] Capture log output

### 7.5 Log Analysis
- [ ] Verify handler entry/exit logs appear
- [ ] Verify broadcast routing logs appear
- [ ] Verify registry logs appear
- [ ] Verify client handler logs appear
- [ ] Verify client connection logs appear
- [ ] Verify client app logs appear
- [ ] Identify where message chain breaks (if applicable)

## Completion Checklist

- [ ] Phase 1 complete (Server Handler)
- [ ] Phase 2 complete (Server Main Routing)
- [ ] Phase 3 complete (Server Registry)
- [ ] Phase 4 complete (Server Client Handler)
- [ ] Phase 5 complete (Client Connection)
- [ ] Phase 6 complete (Client App)
- [ ] Phase 7 complete (Verification)
- [ ] All code compiles without warnings
- [ ] Live test shows complete message trace
- [ ] PLAN.md updated with any discoveries
- [ ] feature_request.json status updated to "completed"
- [ ] Ready for BUG-010 diagnosis

---
*Check off tasks as you complete them. Update status field above.*
