# Task Breakdown: FEAT-064

**Work Item**: [FEAT-064: Refactor MCP bridge.rs into modular components](PROMPT.md)
**Status**: Not Started
**Last Updated**: 2026-01-13

## Prerequisites

- [ ] Read and understand PROMPT.md
- [ ] Review PLAN.md and update if needed
- [ ] Audit `fugue-server/src/mcp/bridge.rs` to understand current structure
- [ ] Run existing tests to establish baseline: `cargo test -p fugue-server`

## Phase 1: Health Module Extraction

- [ ] Create `fugue-server/src/mcp/health.rs`
- [ ] Move `ConnectionState` enum to `health.rs`
- [ ] Move heartbeat-related functions to `health.rs`
- [ ] Move `health_monitor` task implementation to `health.rs`
- [ ] Update `bridge.rs` imports to use new module
- [ ] Update `mod.rs` to export `health` module
- [ ] Run tests and verify they pass

## Phase 2: Connection Module Extraction

- [ ] Create `fugue-server/src/mcp/connection.rs`
- [ ] Move `connect_to_daemon()` to `connection.rs`
- [ ] Move reconnection logic to `connection.rs`
- [ ] Move retry/backoff utilities to `connection.rs`
- [ ] Move connection state tracking to `connection.rs`
- [ ] Update `bridge.rs` imports to use new module
- [ ] Update `mod.rs` to export `connection` module
- [ ] Run tests and verify they pass

## Phase 3: Bridge Cleanup

- [ ] Remove all extracted code from `bridge.rs`
- [ ] Reorganize remaining code for clarity
- [ ] Verify `bridge.rs` is focused on orchestration only
- [ ] Verify `bridge.rs` is under target line count (<500 lines)
- [ ] Update doc comments in `bridge.rs`
- [ ] Run full test suite

## Phase 4: Handler Assessment (Conditional)

- [ ] Count lines in `handlers.rs`
- [ ] If >500 lines, plan decomposition into `tools/` directory
- [ ] Create `tools/mod.rs` if needed
- [ ] Extract session-related handlers to `tools/session.rs`
- [ ] Extract pane-related handlers to `tools/pane.rs`
- [ ] Extract other tool categories as appropriate
- [ ] Update imports and exports
- [ ] Run tests and verify they pass

## Testing Tasks

- [ ] Run `cargo test -p fugue-server`
- [ ] Run `cargo test -p fugue-server -- mcp` (MCP-specific tests)
- [ ] Test MCP tools manually via Claude Code integration
- [ ] Test reconnection by stopping/starting daemon
- [ ] Test health monitoring behavior
- [ ] Verify no warnings from `cargo clippy -p fugue-server`

## Documentation Tasks

- [ ] Add module-level doc comments to `connection.rs`
- [ ] Add module-level doc comments to `health.rs`
- [ ] Update `bridge.rs` module doc comment
- [ ] Update `mod.rs` with module overview comment
- [ ] Review and update any affected documentation

## Verification Tasks

- [ ] All tests passing
- [ ] No compilation warnings
- [ ] Each new module under 500 lines
- [ ] `bridge.rs` under 500 lines (or close)
- [ ] MCP functionality works identically to before
- [ ] Update `feature_request.json` status to completed

## Completion Checklist

- [ ] All phase tasks complete
- [ ] All tests passing
- [ ] Documentation updated
- [ ] PLAN.md updated with final implementation details
- [ ] Ready for review/merge

---
*Check off tasks as you complete them. Update status field above.*
