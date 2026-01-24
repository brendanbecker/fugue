# FEAT-053: Auto-inject CCMUX context environment variables on pane spawn

**Priority**: P1
**Component**: fugue-server (PTY spawning)
**Type**: new_feature
**Estimated Effort**: medium
**Business Value**: high

## Overview

When spawning any pane, automatically inject environment variables that identify the fugue context:
- `FUGUE_PANE_ID` - UUID of the pane
- `FUGUE_SESSION_ID` - UUID of the session
- `FUGUE_WINDOW_ID` - UUID of the window
- `FUGUE_SESSION_NAME` - Human-readable session name

This enables Claude Code instances (and any other processes) to be self-aware of their fugue context without requiring explicit configuration.

## Use Cases

1. **Claude Code self-identification**: Claude Code can identify which pane it's running in and use MCP tools on itself
2. **Script detection**: Scripts can detect if they're running in fugue and which session
3. **Cross-pane coordination**: Enables coordination between Claude instances (e.g., "I'm in pane X, let me check pane Y")
4. **Debugging and logging**: Application logs can include fugue context for easier troubleshooting

## Benefits

- Zero-configuration context awareness for all spawned processes
- Enables powerful automation and coordination scenarios
- Consistent with how tmux exposes TMUX and TMUX_PANE environment variables
- Foundation for more sophisticated multi-agent workflows

## Implementation Tasks

### Section 1: Design
- [ ] Review requirements and acceptance criteria
- [ ] Audit all locations where `PtyConfig` is created for pane spawning
- [ ] Design helper method for centralized environment injection
- [ ] Document implementation approach in PLAN.md

### Section 2: Implementation
- [ ] Create helper method `PtyConfig::with_fugue_context(session_id, session_name, window_id, pane_id)` or similar
- [ ] Modify `fugue-server/src/handlers/mcp_bridge.rs` - approximately 10 locations create PtyConfig
- [ ] Modify `fugue-server/src/mcp/handlers.rs` if PTY spawning occurs there
- [ ] Modify `fugue-server/src/session.rs` if PTY spawning occurs there
- [ ] Modify `fugue-server/src/sideband/async_executor.rs` for sideband pane spawning
- [ ] Use existing `PtyConfig::with_env()` method to add the environment variables
- [ ] Ensure all spawn paths include the context variables

### Section 3: Testing
- [ ] Add unit tests for the helper method
- [ ] Add integration tests verifying environment variables are present in spawned panes
- [ ] Test with `env | grep CCMUX` in spawned panes
- [ ] Verify variables are correct across multiple sessions/windows/panes
- [ ] Run full test suite

### Section 4: Documentation
- [ ] Document the environment variables in README or user docs
- [ ] Add code comments explaining the purpose
- [ ] Update CHANGELOG

### Section 5: Verification
- [ ] All acceptance criteria met
- [ ] Tests passing
- [ ] Code review completed
- [ ] Ready for deployment

## Acceptance Criteria

- [ ] All panes spawned by fugue have `FUGUE_PANE_ID` set to the pane's UUID
- [ ] All panes spawned by fugue have `FUGUE_SESSION_ID` set to the session's UUID
- [ ] All panes spawned by fugue have `FUGUE_WINDOW_ID` set to the window's UUID
- [ ] All panes spawned by fugue have `FUGUE_SESSION_NAME` set to the session's name
- [ ] Environment variables are present regardless of spawn method (MCP, sideband, direct)
- [ ] Running `echo $FUGUE_PANE_ID` in any pane shows the correct UUID
- [ ] All tests passing
- [ ] Documentation updated
- [ ] No regressions in existing functionality

## Dependencies

None

## Related Work

- **FEAT-047** (fugue_set_environment MCP tool) - Complementary feature for manual environment variable setting
- This feature is about automatic injection at spawn time
- FEAT-047 is about manual setting of session-level environment variables

## Notes

- Context (session_id, window_id, pane_id, session_name) is already available at spawn time in all locations
- The existing `PtyConfig::with_env()` method provides the infrastructure needed
- Consider creating a centralized helper to avoid code duplication across ~10 spawn sites
- This is a foundation feature that enables more sophisticated multi-agent coordination scenarios
