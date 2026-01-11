# FEAT-051: Add ccmux_get_environment MCP tool

**Priority**: P2
**Component**: ccmux-server (MCP)
**Type**: new_feature
**Estimated Effort**: small
**Business Value**: medium

## Overview

Allow reading environment variables from a session via MCP. Gas Town reads environment variables using `tmux show-environment -t <session> KEY`. This tool provides parity with ccmux_set_environment and is useful for debugging and session introspection.

## Context

- Gas Town orchestration system reads environment variables using `tmux show-environment -t <session> KEY`
- ccmux already has `ccmux_set_environment` for setting environment variables
- This feature provides the read-side parity for full environment variable management

## Benefits

- Enables MCP clients to inspect session environment variables
- Useful for debugging session configuration issues
- Allows verification that environment variables were set correctly
- Supports workflow automation that needs to read session state

## Implementation Tasks

### Section 1: Protocol Changes
- [ ] Add `GetEnvironment { session_id, key: Option<String> }` variant to `ClientMessage` in ccmux-protocol
- [ ] Add `Environment { session_id, vars: HashMap<String, String> }` variant to `ServerMessage` in ccmux-protocol
- [ ] Update protocol documentation

### Section 2: Server Handler
- [ ] Implement handler for `GetEnvironment` message in ccmux-server
- [ ] If key is provided, return single key-value pair
- [ ] If key is None, return full environment map for session
- [ ] Handle session not found error appropriately

### Section 3: MCP Tool Definition
- [ ] Add `ccmux_get_environment` tool to MCP tool list
- [ ] Define schema:
  - `session`: string (required) - Session UUID or name
  - `key`: string (optional) - Specific key to get, or omit for all
- [ ] Implement tool handler that sends GetEnvironment message

### Section 4: Testing
- [ ] Add unit tests for GetEnvironment message handling
- [ ] Add integration test for MCP tool
- [ ] Test with specific key parameter
- [ ] Test with no key (return all)
- [ ] Test with invalid session ID

### Section 5: Verification
- [ ] Verify tool appears in MCP tool list
- [ ] Test end-to-end with ccmux_set_environment followed by ccmux_get_environment
- [ ] Update documentation if needed

## Acceptance Criteria

- [ ] `ccmux_get_environment` MCP tool is available
- [ ] Can retrieve a single environment variable by key
- [ ] Can retrieve all environment variables when key is omitted
- [ ] Returns appropriate error for non-existent session
- [ ] Returns empty result for non-existent key (not an error)
- [ ] All tests passing
- [ ] No regressions in existing MCP functionality

## Dependencies

None - ccmux_set_environment already exists.

## Notes

- This is a read-only operation, low risk
- Should follow the same session resolution pattern as other MCP tools (UUID or name)
- Consider whether to return inherited environment or only explicitly set variables
