# FEAT-023: PTY Output Polling and Broadcasting

**Priority**: P0 (Critical)
**Component**: fugue-server
**Type**: new_feature
**Estimated Effort**: medium (2-3 hours)
**Business Value**: high
**Status**: new

## Overview

Poll PTY output in background tasks and broadcast to connected clients. This feature enables real-time shell output visibility by spawning background tokio tasks that read from PTY handles and route output to the appropriate connected clients.

## Requirements

1. For each PTY handle, spawn a background tokio task
2. Read from `PtyHandle.reader()` (non-blocking)
3. Collect bytes into buffers
4. On newline/timeout, send `ServerMessage::Output { pane_id, data }`
5. Route output to all clients attached to that pane's session
6. Handle PTY EOF/closure gracefully

## Technical Context

**Existing Infrastructure:**
- `PtyHandle::reader()` method exists for reading PTY output
- `PtyHandle::read()` method available for direct reads
- `ScrollbackBuffer` available for output buffering
- Need to integrate with client tracking (which clients are attached to which sessions)

**Location:** New polling module or extend `fugue-server/src/pty/`

## Affected Files

- `fugue-server/src/pty/mod.rs` - Module exports
- `fugue-server/src/pty/output.rs` - New polling/broadcasting module
- `fugue-server/src/server/mod.rs` - Integration with server event loop
- `fugue-protocol/src/messages.rs` - ServerMessage::Output type

## Implementation Tasks

### Section 1: Design
- [ ] Review existing PtyHandle reader API
- [ ] Design output polling task structure
- [ ] Plan buffer management strategy (newline vs timeout flush)
- [ ] Design client routing mechanism
- [ ] Document task cancellation approach

### Section 2: Output Polling Task
- [ ] Create `PtyOutputPoller` struct
- [ ] Implement background task spawning per PTY
- [ ] Implement non-blocking read from PtyHandle::reader()
- [ ] Add configurable read buffer size
- [ ] Implement poll interval/timeout logic

### Section 3: Buffer Management
- [ ] Integrate with ScrollbackBuffer or create output buffer
- [ ] Implement newline-based flush trigger
- [ ] Implement timeout-based flush trigger
- [ ] Handle partial line buffering
- [ ] Prevent buffer overflow (max buffer size)

### Section 4: Message Broadcasting
- [ ] Create `ServerMessage::Output { pane_id, data }` message type
- [ ] Implement client session lookup
- [ ] Broadcast output to all clients attached to session
- [ ] Handle disconnected clients gracefully
- [ ] Add message serialization for IPC

### Section 5: Lifecycle Management
- [ ] Implement task cancellation on PTY close
- [ ] Handle PTY EOF gracefully
- [ ] Clean up resources on task termination
- [ ] Handle server shutdown coordination
- [ ] Implement task restart on failure (optional)

### Section 6: Testing
- [ ] Unit tests for PtyOutputPoller
- [ ] Unit tests for buffer management
- [ ] Integration test for single pane output
- [ ] Integration test for multi-pane output routing
- [ ] Test PTY EOF handling
- [ ] Test client disconnection during output
- [ ] Performance test for high-throughput output

## Acceptance Criteria

- [ ] Shell output visible to connected clients in real-time
- [ ] Multi-pane output works correctly (each pane has own poll task)
- [ ] No output loss or corruption
- [ ] Graceful handling of PTY termination
- [ ] Output routed only to clients attached to the pane's session
- [ ] Task properly cleans up on pane/session close
- [ ] All tests passing

## Dependencies

- **FEAT-021**: Server Socket - Provides server socket infrastructure for client connections
- **FEAT-022**: Message Routing - Provides client tracking (which clients attached to which sessions)

## Notes

- Consider using `tokio::select!` for combining read timeouts with cancellation
- Buffer flush timeout should be configurable (default ~50ms for responsive feel)
- Large output bursts should be handled efficiently (batch multiple reads)
- Consider backpressure if client cannot keep up with output rate
- PTY EOF should trigger task termination and notify connected clients
