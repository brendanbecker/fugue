# Task Breakdown: FEAT-053

**Work Item**: [FEAT-053: Auto-inject CCMUX context environment variables on pane spawn](PROMPT.md)
**Status**: Not Started
**Last Updated**: 2026-01-11

## Prerequisites

- [ ] Read and understand PROMPT.md
- [ ] Review PLAN.md and update if needed
- [ ] Audit all PtyConfig creation sites in the codebase

## Design Tasks

- [ ] Review requirements and acceptance criteria
- [ ] Identify all locations where PtyConfig is created
- [ ] Verify context (session_id, window_id, pane_id, session_name) is available at each site
- [ ] Decide on helper method location (PtyConfig impl vs free function)
- [ ] Update PLAN.md with findings

## Implementation Tasks

- [ ] Create helper method for ccmux context environment injection
- [ ] Modify `ccmux-server/src/handlers/mcp_bridge.rs`:
  - [ ] Audit and count all PtyConfig creation sites
  - [ ] Add context injection to each site
- [ ] Modify `ccmux-server/src/mcp/handlers.rs` if PTY spawning present
- [ ] Modify `ccmux-server/src/session.rs` if PTY spawning present
- [ ] Modify `ccmux-server/src/sideband/async_executor.rs` for sideband spawns
- [ ] Verify all spawn paths use the context injection
- [ ] Self-review changes

## Testing Tasks

- [ ] Add unit tests for the helper method
- [ ] Add integration test: spawn pane, verify CCMUX_PANE_ID is set
- [ ] Add integration test: spawn pane, verify CCMUX_SESSION_ID is set
- [ ] Add integration test: spawn pane, verify CCMUX_WINDOW_ID is set
- [ ] Add integration test: spawn pane, verify CCMUX_SESSION_NAME is set
- [ ] Manual test: `env | grep CCMUX` in spawned pane
- [ ] Manual test: verify values are correct UUIDs/names
- [ ] Test sideband-spawned panes have context
- [ ] Test MCP-spawned panes have context
- [ ] Run full test suite

## Documentation Tasks

- [ ] Document environment variables in README or user docs
- [ ] Add code comments to helper method
- [ ] Update CHANGELOG with new feature

## Verification Tasks

- [ ] All acceptance criteria from PROMPT.md met
- [ ] Tests passing
- [ ] Update feature_request.json status
- [ ] Document completion in comments.md

## Completion Checklist

- [ ] All implementation tasks complete
- [ ] All tests passing
- [ ] Documentation updated
- [ ] PLAN.md reflects final implementation
- [ ] Ready for review/merge

---
*Check off tasks as you complete them. Update status field above.*
