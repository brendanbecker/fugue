# Task Breakdown: FEAT-023

**Work Item**: [FEAT-023: PTY Output Polling and Broadcasting](PROMPT.md)
**Status**: Not Started
**Last Updated**: 2026-01-09

## Prerequisites

- [ ] Read and understand PROMPT.md
- [ ] Review PLAN.md and update if needed
- [ ] Verify FEAT-021 (Server Socket) is complete
- [ ] Verify FEAT-022 (Message Routing) is complete
- [ ] Review existing PtyHandle implementation
- [ ] Review ScrollbackBuffer API

## Design Tasks

- [ ] Finalize output buffer flush strategy
- [ ] Document broadcast channel design
- [ ] Plan task cancellation coordination
- [ ] Define message format for Output and PtyEof

## Implementation Tasks

### Output Polling Module (fugue-server/src/pty/output.rs)

- [ ] Create output.rs module file
- [ ] Implement `OutputBuffer` struct
  - [ ] `push()` method with newline detection
  - [ ] `should_timeout_flush()` method
  - [ ] `flush()` method
  - [ ] Configurable max_size and flush_timeout
- [ ] Implement `PtyOutputPoller` struct
  - [ ] Fields: pane_id, reader, broadcast_tx, cancel_token, buffer
  - [ ] `spawn()` constructor that returns JoinHandle and CancellationToken
  - [ ] `run()` async method with select! loop
  - [ ] `read_with_timeout()` method
  - [ ] `handle_output()` method
  - [ ] `send_output()` method
  - [ ] `send_eof()` method
  - [ ] `flush_final()` for cleanup
- [ ] Add module to fugue-server/src/pty/mod.rs exports

### Protocol Messages (fugue-protocol/src/messages.rs)

- [ ] Add `ServerMessage::Output { pane_id, data }` variant
- [ ] Add `ServerMessage::PtyEof { pane_id }` variant
- [ ] Update serde derives if needed
- [ ] Update message codec if needed

### Server Integration (fugue-server/src/server/)

- [ ] Import output polling module
- [ ] Create broadcast channel for server messages
- [ ] Spawn PtyOutputPoller when PTY is created
- [ ] Store cancellation tokens in pane/session state
- [ ] Cancel poller tasks on pane close
- [ ] Handle broadcast channel in client connection handler

### Lifecycle Management

- [ ] Implement proper task cancellation on pane close
- [ ] Handle PTY EOF (task self-terminates)
- [ ] Clean up broadcast channel resources
- [ ] Handle server shutdown (cancel all poller tasks)
- [ ] Log task start/stop for debugging

## Testing Tasks

### Unit Tests

- [ ] Test OutputBuffer newline flush
- [ ] Test OutputBuffer timeout flush
- [ ] Test OutputBuffer max size limit
- [ ] Test OutputBuffer partial line handling
- [ ] Test PtyOutputPoller with mock reader
- [ ] Test cancellation token stops task

### Integration Tests

- [ ] Test single pane output to client
- [ ] Test multi-pane output routing
- [ ] Test PTY EOF message delivery
- [ ] Test client disconnection handling
- [ ] Test pane close during output
- [ ] Test server shutdown during output

### Performance Tests

- [ ] Test high-throughput output (10MB/s)
- [ ] Test many concurrent panes (50+)
- [ ] Measure latency from PTY read to client delivery
- [ ] Test memory usage under sustained output

## Documentation Tasks

- [ ] Document PtyOutputPoller API
- [ ] Document buffer configuration options
- [ ] Add architecture diagram to PLAN.md
- [ ] Document troubleshooting for output issues

## Verification Tasks

- [ ] All acceptance criteria from PROMPT.md met
- [ ] Tests passing
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
