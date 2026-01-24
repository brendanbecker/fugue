# Task Breakdown: FEAT-039

**Work Item**: [FEAT-039: MCP Pane Creation Broadcast - Sync TUI Clients on MCP Splits](PROMPT.md)
**Status**: Not Started
**Last Updated**: 2026-01-09

## Prerequisites

- [ ] Read and understand PROMPT.md
- [ ] Review PLAN.md and update if needed
- [ ] Review `fugue-server/src/handlers/mcp_bridge.rs` - `handle_create_pane_with_options`
- [ ] Review `fugue-server/src/handlers/pane.rs` - `handle_create_pane` (reference implementation)
- [ ] Understand `HandlerResult::ResponseWithBroadcast` structure
- [ ] Understand `PaneInfo` struct fields

## Design Tasks

- [ ] Confirm pane index is available after pane creation
- [ ] Confirm `PaneInfo` import exists or needs to be added
- [ ] Document any edge cases (e.g., pane creation failure)

## Implementation Tasks

### Phase 1: Capture Pane Data

- [ ] In `handle_create_pane_with_options`, locate where pane is created
- [ ] Capture pane index from the created pane object
- [ ] Ensure `session_id` is available for broadcast

### Phase 2: Update Return Type

- [ ] Import `PaneInfo` if not already in scope
- [ ] Construct `PaneInfo` struct:
  ```rust
  let pane_info = PaneInfo {
      id: pane_id,
      window_id,
      session_id,
      index: pane_index,
      title: None,
      cwd: None,
  };
  ```
- [ ] Change return statement from:
  ```rust
  HandlerResult::Response(ServerMessage::PaneCreatedWithDetails { ... })
  ```
  to:
  ```rust
  HandlerResult::ResponseWithBroadcast {
      response: ServerMessage::PaneCreatedWithDetails { ... },
      session_id,
      broadcast: ServerMessage::PaneCreated { pane: pane_info },
  }
  ```

### Phase 3: Code Review

- [ ] Verify all fields are populated correctly
- [ ] Ensure no unused variable warnings
- [ ] Check for consistent error handling

## Testing Tasks

### Unit Tests

- [ ] Add test: `test_create_pane_with_options_returns_broadcast`
  - Call `handle_create_pane_with_options`
  - Assert result is `HandlerResult::ResponseWithBroadcast`
  - Assert response is `PaneCreatedWithDetails`
  - Assert broadcast is `PaneCreated`
  - Assert broadcast pane_id matches response pane_id

### Integration Tests

- [ ] Test MCP pane creation still works for MCP client
- [ ] Test TUI client receives broadcast after MCP pane creation

### Manual Testing

- [ ] Start fugue server: `cargo run --bin fugue-server`
- [ ] Attach TUI client: `cargo run --bin fugue-client attach`
- [ ] Use MCP tool to create pane (via Claude or direct MCP call)
- [ ] Verify TUI client shows the new pane split immediately
- [ ] Verify pane switching (Ctrl+B o) includes the new pane
- [ ] Test with multiple TUI clients attached to same session

## Documentation Tasks

- [ ] Update PLAN.md with implementation notes
- [ ] Add code comments explaining broadcast purpose
- [ ] Document any discovered edge cases

## Verification Tasks

- [ ] All acceptance criteria from PROMPT.md met:
  - [ ] MCP `fugue_create_pane` returns `PaneCreatedWithDetails` to MCP client
  - [ ] TUI clients receive `PaneCreated` broadcast
  - [ ] TUI split pane rendering updates automatically
  - [ ] No changes required in client code
  - [ ] All existing tests pass
  - [ ] New test covers MCP-to-TUI broadcast scenario
- [ ] `cargo test` passes in fugue-server
- [ ] `cargo clippy` has no new warnings
- [ ] Update feature_request.json status

## Completion Checklist

- [ ] All implementation tasks complete
- [ ] All tests passing
- [ ] Code reviewed (self-review)
- [ ] PLAN.md updated with implementation notes
- [ ] Manual testing completed
- [ ] Ready for merge

---
*Check off tasks as you complete them. Update status field above.*
