# Task Breakdown: FEAT-021

**Work Item**: [FEAT-021: Server Socket Listen Loop](PROMPT.md)
**Status**: Not Started
**Last Updated**: 2026-01-09

## Prerequisites

- [ ] Read and understand PROMPT.md
- [ ] Review PLAN.md and update if needed
- [ ] Review existing main.rs structure (lines 316-326)
- [ ] Review test patterns in client.rs (lines 215-230)
- [ ] Understand fugue_utils::socket_path() behavior
- [ ] Review fugue-protocol MessageCodec

## Design Tasks

- [ ] Decide on Server struct client tracking approach
- [ ] Design shutdown signal propagation
- [ ] Plan error handling strategy for accept failures
- [ ] Design client ID generation scheme

## Implementation Tasks

### Socket Setup (fugue-server/src/main.rs)

- [ ] Add socket path validation function
- [ ] Implement stale socket detection and cleanup
- [ ] Create parent directories for socket if needed
- [ ] Implement UnixListener::bind() with error handling
- [ ] Set socket file permissions (if needed)
- [ ] Add socket cleanup on Drop/shutdown

### Accept Loop (fugue-server/src/main.rs or server.rs)

- [ ] Create async accept loop function
- [ ] Add tokio::select! for accept + shutdown
- [ ] Spawn tokio task for each accepted connection
- [ ] Add client connection logging (tracing)
- [ ] Handle accept errors gracefully (log and continue)
- [ ] Track number of active clients

### Client Handler (fugue-server/src/client.rs)

- [ ] Create ClientHandler struct
- [ ] Split stream into reader/writer
- [ ] Wrap with FramedRead/FramedWrite using MessageCodec
- [ ] Implement message receive loop
- [ ] Add stub message routing (placeholder)
- [ ] Handle stream errors and EOF
- [ ] Implement cleanup on disconnect

### Server Integration (fugue-server/src/server.rs)

- [ ] Add client registry to Server struct
- [ ] Implement register_client() method
- [ ] Implement unregister_client() method
- [ ] Add shutdown_signal() method
- [ ] Add get_client_count() method

### Shutdown Handling

- [ ] Set up tokio::signal handlers (SIGTERM, SIGINT)
- [ ] Propagate shutdown signal to accept loop
- [ ] Wait for active clients with timeout
- [ ] Remove socket file on clean shutdown
- [ ] Log shutdown progress

## Testing Tasks

### Unit Tests

- [ ] Test socket creation at valid path
- [ ] Test socket creation with missing parent dirs
- [ ] Test stale socket cleanup
- [ ] Test bind error handling (permission denied mock)
- [ ] Test client ID generation uniqueness

### Integration Tests

- [ ] Test single client connect and disconnect
- [ ] Test multiple concurrent client connections
- [ ] Test client message echo (once routing works)
- [ ] Test graceful server shutdown
- [ ] Test client disconnect during message send
- [ ] Test server restart with stale socket

### Stress Tests

- [ ] Test rapid connect/disconnect cycles
- [ ] Test many concurrent connections (100+)
- [ ] Test long-running connection stability

## Documentation Tasks

- [ ] Document socket path configuration
- [ ] Document shutdown behavior
- [ ] Add troubleshooting for common socket errors
- [ ] Update README with server startup instructions

## Verification Tasks

- [ ] All acceptance criteria from PROMPT.md met
- [ ] Tests passing
- [ ] Server starts and accepts connections
- [ ] Clean shutdown works
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
