# BUG-050: New pane/session/window doesn't inherit expected working directory

**Priority**: P2
**Component**: mcp, session
**Severity**: medium
**Status**: new

## Problem Statement

When creating a new pane, session, or window via MCP tools (`ccmux_create_pane`, `ccmux_create_session`, `ccmux_create_window`), the new shell does not always launch in the expected working directory.

## Expected Behavior

- New panes should inherit the working directory of the parent pane (or use explicit `cwd` if provided)
- New sessions should use the caller's working directory or explicit `cwd`
- New windows should inherit from the session or use explicit `cwd`

## Actual Behavior

New panes/sessions/windows sometimes launch in:
- The user's home directory
- The daemon's working directory
- An unexpected location

## Steps to Reproduce

1. Start ccmux in `/home/user/projects/myproject`
2. Call `ccmux_create_pane` without explicit `cwd`
3. Observe the new pane's working directory
4. Compare to expected directory

## Investigation Areas

1. **MCP tool parameters** - Is `cwd` being passed correctly?
2. **PTY spawn** - What directory is used when spawning the PTY process?
3. **Session/pane hierarchy** - Is cwd tracked and inherited correctly?
4. **Environment variables** - Is `PWD` being set/inherited?

## Key Files

- `ccmux-server/src/mcp_handlers.rs` - MCP handlers for create_*
- `ccmux-session/src/pty.rs` - PTY spawning
- `ccmux-session/src/pane.rs` - Pane creation
- `ccmux-session/src/session.rs` - Session creation

## Implementation Tasks

### Section 1: Investigation
- [ ] Trace cwd handling in create_pane MCP handler
- [ ] Trace cwd handling in create_session MCP handler
- [ ] Trace cwd handling in create_window MCP handler
- [ ] Identify where cwd is lost or defaulted incorrectly

### Section 2: Fix Implementation
- [ ] Ensure cwd is passed through MCP → daemon → PTY spawn
- [ ] Implement inheritance from parent pane if cwd not specified
- [ ] Default to caller's cwd if no parent and no explicit cwd

### Section 3: Testing
- [ ] Test create_pane with explicit cwd
- [ ] Test create_pane without cwd (should inherit)
- [ ] Test create_session with cwd
- [ ] Test create_window with cwd

## Acceptance Criteria

- [ ] `create_pane` with `cwd` launches in specified directory
- [ ] `create_pane` without `cwd` inherits from parent pane
- [ ] `create_session` with `cwd` launches in specified directory
- [ ] `create_window` with `cwd` launches in specified directory
- [ ] Default behavior is sensible (not random/unexpected)

## Notes

This bug affects multi-agent workflows where agents expect to work in specific project directories.
