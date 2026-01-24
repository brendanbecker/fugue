# FEAT-047: Add fugue_set_environment MCP tool

**Priority**: P1
**Component**: fugue-server (MCP)
**Type**: new_feature
**Estimated Effort**: medium
**Business Value**: high

## Overview

Allow setting environment variables on a session that will be inherited by panes/processes. Gas Town sets environment variables on tmux sessions using `tmux set-environment -t <session> KEY VALUE`. Critical variables include GT_RIG, GT_POLECAT, BEADS_DIR, BEADS_NO_DAEMON, and BEADS_AGENT_NAME. fugue currently has no mechanism to store or propagate session-level environment variables.

## Benefits

- Enables Gas Town integration with fugue
- Allows session-level environment configuration without modifying shell profiles
- Provides parity with tmux's `set-environment` functionality
- Supports multi-session workflows with different environment contexts

## Implementation Tasks

### Section 1: Design
- [ ] Review requirements and acceptance criteria
- [ ] Design solution architecture
- [ ] Identify affected components
- [ ] Document implementation approach

### Section 2: Implementation
- [ ] Add `environment: HashMap<String, String>` field to Session struct in `fugue-server/src/session/session.rs`
- [ ] Add `SetEnvironment { session_id, key, value }` variant to ClientMessage in `fugue-protocol/src/messages.rs`
- [ ] Add MCP tool definition to tools.rs with schema:
  - `session`: string (required) - Session UUID or name
  - `key`: string (required) - Environment variable name
  - `value`: string (required) - Environment variable value
- [ ] Implement handler for SetEnvironment message in server
- [ ] When creating new panes, pass session environment to PTY spawn
- [ ] Add error handling for invalid session references

### Section 3: Testing
- [ ] Add unit tests for Session environment storage
- [ ] Add unit tests for SetEnvironment message handling
- [ ] Add integration tests for MCP tool invocation
- [ ] Test environment propagation to spawned panes
- [ ] Manual testing of key scenarios

### Section 4: Documentation
- [ ] Update MCP tool documentation
- [ ] Add usage examples for Gas Town integration
- [ ] Add code comments
- [ ] Update CHANGELOG

### Section 5: Verification
- [ ] All acceptance criteria met
- [ ] Tests passing
- [ ] Code review completed
- [ ] Ready for deployment

## Acceptance Criteria

- [ ] `fugue_set_environment` MCP tool is available and documented
- [ ] Environment variables can be set on sessions by UUID or name
- [ ] Environment variables are inherited by newly created panes in the session
- [ ] Multiple environment variables can be set on a single session
- [ ] All tests passing
- [ ] Documentation updated
- [ ] No regressions in existing functionality

## Dependencies

None

## Notes

- This is blocking Gas Town integration
- Consider also adding `fugue_get_environment` and `fugue_unset_environment` tools for completeness
- Environment variables should persist across session reconnects (may require persistence layer update)
- Consider whether environment should be included in session serialization for crash recovery
