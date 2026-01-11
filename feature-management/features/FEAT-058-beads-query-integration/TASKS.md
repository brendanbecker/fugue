# Task Breakdown: FEAT-058

**Work Item**: [FEAT-058: Beads Query Integration - TUI Visibility into Work Queue](PROMPT.md)
**Status**: Not Started
**Last Updated**: 2026-01-11

## Prerequisites

- [ ] Read and understand PROMPT.md
- [ ] Review PLAN.md and update if needed
- [ ] Understand beads daemon RPC protocol
- [ ] Review existing status bar implementation in ccmux-client
- [ ] Review input handling for adding new keybinds
- [ ] FEAT-057 (Beads Passive Awareness) completed

## Design Tasks

- [ ] Review requirements and acceptance criteria
- [ ] Document beads daemon RPC protocol format
- [ ] Design status bar layout with beads indicator
- [ ] Design beads panel UI/UX
- [ ] Decide on caching strategy for daemon queries
- [ ] Update PLAN.md with findings

## Implementation Tasks

### Beads Client Module (ccmux-server/src/beads/)

- [ ] Create `ccmux-server/src/beads/mod.rs` module structure
- [ ] Create `ccmux-server/src/beads/types.rs` with BeadsTask, BeadsStatus
- [ ] Create `ccmux-server/src/beads/discovery.rs`
- [ ] Implement `discover_beads_socket()` function
- [ ] Implement `discover_beads_root()` function
- [ ] Create `ccmux-server/src/beads/client.rs`
- [ ] Implement `BeadsClient::new()`
- [ ] Implement `BeadsClient::connect()` with async socket
- [ ] Implement `BeadsClient::query_ready()` RPC call
- [ ] Implement `BeadsClient::is_socket_available()`
- [ ] Add timeout handling to all socket operations
- [ ] Implement graceful error handling (no panics)

### Beads Manager (ccmux-server/src/beads/)

- [ ] Create `ccmux-server/src/beads/manager.rs`
- [ ] Implement client caching per repo root
- [ ] Implement status caching with TTL
- [ ] Implement `get_status()` with cache check
- [ ] Implement `get_ready_tasks()` fresh query
- [ ] Implement refresh loop task
- [ ] Integrate manager with server state

### Protocol Messages (ccmux-protocol)

- [ ] Add `BeadsStatus` struct to protocol
- [ ] Add `BeadsTask` struct to protocol
- [ ] Add `BeadsStatusUpdate` to ServerMessage enum
- [ ] Add `RequestBeadsReady` to ClientMessage enum
- [ ] Add `BeadsReadyResponse` to ServerMessage enum
- [ ] Add serde derives for all new types
- [ ] Add unit tests for serialization

### Configuration (ccmux-server/src/config.rs)

- [ ] Add `BeadsConfig` struct
- [ ] Add `BeadsQueryConfig` struct
- [ ] Add `enabled` field (default: true)
- [ ] Add `show_ready_count` field (default: true)
- [ ] Add `refresh_interval` field (default: 30)
- [ ] Add `socket_timeout` field (default: 1000)
- [ ] Add `auto_discover` field (default: true)
- [ ] Add `[beads.query]` section to config schema
- [ ] Add default implementations

### Server-Side Integration

- [ ] Initialize BeadsManager in server startup
- [ ] Handle pane focus change - update beads context
- [ ] Send BeadsStatusUpdate to clients on refresh
- [ ] Handle RequestBeadsReady from client
- [ ] Send BeadsReadyResponse with task list
- [ ] Honor enabled config flag

### Status Bar Integration (ccmux-client/src/ui/status.rs)

- [ ] Add `beads_status: Option<BeadsStatus>` to StatusBar
- [ ] Implement `set_beads_status()` method
- [ ] Implement `render_beads_indicator()` function
- [ ] Style indicator: green (0 ready), yellow (>0 ready)
- [ ] Handle daemon unavailable state
- [ ] Position indicator appropriately in status bar

### Beads Panel Component (ccmux-client/src/ui/beads_panel.rs)

- [ ] Create new file `beads_panel.rs`
- [ ] Implement `BeadsPanel` struct
- [ ] Implement `show()` and `hide()` methods
- [ ] Implement `set_tasks()` method
- [ ] Implement `set_error()` method
- [ ] Implement panel rendering with border
- [ ] Implement loading state rendering
- [ ] Implement error state rendering
- [ ] Implement empty state rendering
- [ ] Implement task table rendering
- [ ] Implement row selection highlighting
- [ ] Implement footer with keybind hints
- [ ] Implement `handle_key()` for navigation
- [ ] Implement j/k and arrow key navigation
- [ ] Implement Enter for claim action
- [ ] Implement v for view details action
- [ ] Implement Esc/q for close
- [ ] Add `BeadsPanelAction` enum

### Input Handling (ccmux-client/src/input/mod.rs)

- [ ] Add `KeyCode::Char('b')` handler in prefix mode
- [ ] Map to `Action::OpenBeadsPanel`
- [ ] Add `Action::OpenBeadsPanel` variant
- [ ] Add `Action::CloseBeadsPanel` variant
- [ ] Add `Action::BeadsClaim(String)` variant
- [ ] Add `Action::BeadsViewDetails(String)` variant

### Client App Integration (ccmux-client/src/ui/app.rs)

- [ ] Add BeadsPanel to App state
- [ ] Handle BeadsStatusUpdate server message
- [ ] Handle BeadsReadyResponse server message
- [ ] Wire up OpenBeadsPanel action
- [ ] Send RequestBeadsReady when panel opens
- [ ] Handle BeadsPanelAction results
- [ ] Render BeadsPanel overlay when visible

### MCP Tools (Optional - Lower Priority)

- [ ] Add `ccmux_beads_ready` tool schema
- [ ] Implement `ccmux_beads_ready` handler
- [ ] Add `ccmux_beads_status` tool schema
- [ ] Implement `ccmux_beads_status` handler
- [ ] Add tool documentation

## Testing Tasks

### Unit Tests

- [ ] Test discover_beads_socket finds socket in current dir
- [ ] Test discover_beads_socket finds socket in parent dir
- [ ] Test discover_beads_socket returns None when not found
- [ ] Test BeadsClient timeout handling
- [ ] Test BeadsClient socket unavailable handling
- [ ] Test BeadsManager cache hit
- [ ] Test BeadsManager cache expiry
- [ ] Test BeadsStatus serialization
- [ ] Test BeadsTask serialization
- [ ] Test BeadsReadyResponse serialization

### Integration Tests

- [ ] Test full flow: server discovers socket, queries daemon
- [ ] Test status update sent to client
- [ ] Test ready list request/response cycle
- [ ] Test daemon unavailable fallback
- [ ] Test daemon restart recovery
- [ ] Test multiple repos with different sockets

### Panel Tests

- [ ] Test BeadsPanel::show() sets visible
- [ ] Test BeadsPanel::hide() clears visible
- [ ] Test BeadsPanel navigation wraps correctly
- [ ] Test BeadsPanel selection highlighting
- [ ] Test BeadsPanel keybind handling

### Manual Testing

- [ ] Test in beads repo with running daemon
- [ ] Verify status bar shows "bd: X ready"
- [ ] Verify Ctrl+B b opens panel
- [ ] Verify panel shows correct tasks
- [ ] Verify j/k navigation works
- [ ] Verify Esc closes panel
- [ ] Test in non-beads repo (no indicator)
- [ ] Test with daemon stopped (graceful fallback)
- [ ] Test daemon restart while ccmux running
- [ ] Test config changes (refresh interval)

## Documentation Tasks

- [ ] Document beads integration in ccmux docs
- [ ] Document configuration options
- [ ] Document Ctrl+B b keybind
- [ ] Add code comments to BeadsClient
- [ ] Add code comments to BeadsManager
- [ ] Add code comments to BeadsPanel
- [ ] Update CHANGELOG

## Verification Tasks

- [ ] All acceptance criteria from PROMPT.md met
- [ ] Tests passing
- [ ] No blocking on daemon communication
- [ ] Status bar display is correct
- [ ] Panel navigation is smooth
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
