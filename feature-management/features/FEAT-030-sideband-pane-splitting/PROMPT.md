# FEAT-030: Sideband Pane Splitting - Execute spawn Commands via Sideband Protocol

**Priority**: P1
**Component**: ccmux-server
**Type**: new_feature
**Estimated Effort**: medium
**Business Value**: high
**Status**: new

## Overview

Complete the execute_spawn function in the sideband executor to actually split panes when Claude outputs `<ccmux:spawn />` commands. The sideband parser (FEAT-019) already handles command parsing and stripping from display output, but the executor currently just logs a warning and returns Ok(()).

This feature enables Claude to dynamically create new terminal panes for parallel task execution.

## Current State

The sideband infrastructure is in place:

1. **Parser** (`ccmux-server/src/sideband/parser.rs`): Parses XML tags like `<ccmux:spawn direction="vertical" command="cargo build" cwd="/project" />` and extracts them from display output.

2. **Commands** (`ccmux-server/src/sideband/commands.rs`): Defines `SidebandCommand::Spawn` with `direction`, `command`, and `cwd` fields. `SplitDirection` enum has `Horizontal` and `Vertical` variants.

3. **Executor** (`ccmux-server/src/sideband/executor.rs`): Has `execute_spawn()` method that receives parsed commands but only logs a warning:
   ```rust
   fn execute_spawn(
       &self,
       source_pane: Uuid,
       direction: SplitDirection,
       command: Option<String>,
       cwd: Option<String>,
   ) -> ExecuteResult<()> {
       // TODO: Implement actual pane splitting in SessionManager
       warn!("Spawn command not yet implemented - pane: {}, direction: {:?}", source_pane, direction);
       Ok(())
   }
   ```

4. **SessionManager** (`ccmux-server/src/session/manager.rs`): Has `find_pane()` to locate panes and session/window hierarchy, but no `split_pane` method.

## Requirements

### 1. SessionManager.split_pane() Method

Add a new method to SessionManager:

```rust
pub fn split_pane(
    &mut self,
    source_pane_id: Uuid,
    direction: SplitDirection,
    command: Option<String>,
    cwd: Option<PathBuf>,
) -> Result<Uuid>  // Returns new pane ID
```

This method should:
- Find the window containing `source_pane_id`
- Create a new pane in that window
- Return the new pane's UUID
- NOT spawn PTY (that's handled separately by PtyManager)

### 2. PTY Spawning Integration

The executor needs to:
- Call `split_pane` to create the pane structure
- Call PtyManager to spawn a PTY for the new pane
- Optionally run the specified command in the new pane
- Set working directory if `cwd` is specified

### 3. Client Notification

After creating a new pane:
- Broadcast a `PaneCreated` event to connected clients
- Include window ID, pane ID, and pane info

### 4. Supported Sideband Commands

The following sideband commands should work after this feature:

```xml
<!-- Split vertically (new pane to the right) -->
<ccmux:spawn direction="vertical" />

<!-- Split horizontally (new pane below) -->
<ccmux:spawn direction="horizontal" />

<!-- Split with specific command -->
<ccmux:spawn direction="vertical" command="cargo build" />

<!-- Split with command and working directory -->
<ccmux:spawn direction="horizontal" command="npm test" cwd="/home/user/project" />

<!-- Shorthand directions -->
<ccmux:spawn direction="v" />
<ccmux:spawn direction="h" />
```

## Affected Files

| File | Change |
|------|--------|
| `ccmux-server/src/session/manager.rs` | Add `split_pane()` method |
| `ccmux-server/src/session/window.rs` | May need helper for split position |
| `ccmux-server/src/sideband/executor.rs` | Complete `execute_spawn()` implementation |
| `ccmux-server/src/sideband/commands.rs` | Already complete (SplitDirection enum) |
| `ccmux-server/src/pty/manager.rs` | May need spawn_for_pane() integration |

## Implementation Tasks

### Section 1: SessionManager Extension
- [ ] Add `split_pane()` method to SessionManager
- [ ] Find window containing source pane
- [ ] Create new pane in the same window
- [ ] Return new pane UUID
- [ ] Add unit tests for split_pane

### Section 2: Executor Implementation
- [ ] Complete `execute_spawn()` in executor.rs
- [ ] Call SessionManager.split_pane()
- [ ] Integrate with PtyManager for PTY creation
- [ ] Handle command execution in new pane
- [ ] Handle cwd setting for new pane
- [ ] Add error handling for invalid source pane

### Section 3: PTY Integration
- [ ] Ensure PtyManager can spawn PTY for new panes
- [ ] Pass command to PTY spawn if specified
- [ ] Pass cwd to PTY spawn if specified
- [ ] Handle spawn failures gracefully

### Section 4: Client Broadcasting
- [ ] Broadcast pane creation to connected clients
- [ ] Include necessary pane/window info in message
- [ ] Ensure clients can handle new pane notifications

### Section 5: Testing
- [ ] Unit test: split_pane creates pane in correct window
- [ ] Unit test: split_pane with valid/invalid source pane
- [ ] Integration test: execute_spawn creates working pane
- [ ] Integration test: spawn with command runs command
- [ ] Integration test: spawn with cwd uses correct directory
- [ ] E2E test: Claude output with spawn command creates pane

## Acceptance Criteria

- [ ] `<ccmux:spawn direction="vertical" />` creates a new pane to the right
- [ ] `<ccmux:spawn direction="horizontal" />` creates a new pane below
- [ ] `command` attribute causes specified command to run in new pane
- [ ] `cwd` attribute sets working directory for new pane
- [ ] Invalid source pane returns appropriate error
- [ ] Connected clients receive notification of new pane
- [ ] No regressions in existing sideband command handling
- [ ] All tests passing

## Dependencies

- **FEAT-019**: Sideband Protocol XML Parsing (complete - provides parser)
- **FEAT-012**: Session Management Hierarchy (provides SessionManager, Window, Pane)
- **FEAT-013**: PTY Management (provides PtyManager for spawning)

## Notes

- The SplitDirection enum is already defined with Vertical and Horizontal variants
- The parser already handles the `direction`, `command`, and `cwd` attributes
- Consider whether to track split relationships between panes (for layout purposes)
- Future: Layout manager to handle pane positioning/sizing (not in this feature)
- The executor holds `Arc<Mutex<SessionManager>>` so thread safety is handled
