# BUG-023: fugue_create_session MCP tool doesn't spawn shell in default pane

**Priority**: P1
**Component**: mcp
**Severity**: high
**Status**: resolved

## Problem Statement

The `fugue_create_session` MCP tool creates a session with a window and pane, but the pane has no running process. The PTY exists but no shell is spawned, leaving the pane completely non-functional.

## Evidence

- **Reproduction**: Call `fugue_create_session` via MCP, attach to the session - pane is blank and unresponsive
- **Comparison**: `fugue_create_pane` with explicit `command` parameter works correctly

```json
// This creates a broken session:
{ "name": "dev" }

// Result: pane has PTY but no shell process
// has_pty: true, but read_pane returns empty, send_input has no effect
```

## Steps to Reproduce

1. Call `fugue_create_session` via MCP with just a name
2. Attach to the session via session picker (`Ctrl+b s`)
3. Observe blank pane - no shell prompt
4. `fugue_send_input` has no effect
5. `fugue_read_pane` returns empty

## Expected Behavior

`fugue_create_session` should:
1. Accept an optional `command` parameter to spawn in the default pane
2. If no command specified, spawn the user's default shell (`$SHELL` or `/bin/sh`)

## Actual Behavior

Session/window/pane structure is created but no process is spawned in the default pane, making it useless.

## Root Cause

The MCP `fugue_create_session` handler creates the session structure but doesn't spawn a process in the initial pane. Compare with `fugue_create_pane` which correctly handles the `command` parameter.

## Implementation Tasks

### Section 1: Investigation
- [x] Reproduce the bug consistently
- [x] Identify root cause location
- [ ] Document affected code paths in MCP server

### Section 2: Fix Implementation
- [ ] Add optional `command` parameter to `fugue_create_session` schema
- [ ] Update handler to spawn command (or default shell) in initial pane
- [ ] Use `$SHELL` env var with fallback to `/bin/sh`

### Section 3: Testing
- [ ] Test session creation with explicit command
- [ ] Test session creation without command (should spawn default shell)
- [ ] Verify pane is functional after creation

### Section 4: Verification
- [ ] Confirm expected behavior is restored
- [ ] Verify all acceptance criteria met

## Acceptance Criteria

- [ ] `fugue_create_session` spawns a working shell by default
- [ ] Optional `command` parameter allows custom process
- [ ] Pane is immediately usable after session creation
- [ ] No regression in existing session creation flows

## Resolution

**Root Cause Found**: Two issues were identified:
1. **Missing output poller**: The `handle_create_session_with_options` handler spawned the PTY but didn't start an output poller, so PTY output was never read/routed to the pane's scrollback
2. **Missing command parameter**: No way to specify a custom command for the session

**Fix Applied** (2026-01-10):

### 1. Added `command` parameter to protocol message
**File**: `fugue-protocol/src/messages.rs`
```rust
CreateSessionWithOptions {
    name: Option<String>,
    command: Option<String>,  // NEW
}
```

### 2. Added `command` to MCP tool schema
**File**: `fugue-server/src/mcp/tools.rs`

### 3. Updated MCP bridge dispatch
**File**: `fugue-server/src/mcp/bridge.rs`

### 4. Fixed handler to start output poller
**File**: `fugue-server/src/handlers/mcp_bridge.rs`
- Now calls `PtyOutputPoller::spawn_with_sideband()` after spawning PTY
- Uses command parameter with fallback to `$SHELL` or `/bin/sh`

### 5. Also fixed `handle_create_window_with_options`
Same issue - wasn't starting output poller. Fixed with same pattern.

**Tests**: All 135 tests pass.

## Notes

- ~~Workaround: After creating session, call `fugue_create_pane` with a command, then close the empty pane~~
- ~~Priority is P1 because this breaks the primary MCP workflow for session management~~
- **Requires server restart** to pick up changes
