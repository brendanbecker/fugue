# Implementation Plan: FEAT-030

**Work Item**: [FEAT-030: Sideband Pane Splitting - Execute spawn Commands via Sideband Protocol](PROMPT.md)
**Component**: ccmux-server
**Priority**: P1
**Created**: 2026-01-09

## Overview

Complete the execute_spawn function in the sideband executor to actually split panes when Claude outputs `<ccmux:spawn />` commands. Currently the parser and command types exist but execution is a TODO stub.

## Architecture Decisions

### Approach: Minimal Viable Split

For the initial implementation, focus on creating functional panes without complex layout management:

1. **Pane Creation**: Add pane to same window as source pane
2. **PTY Spawning**: Create PTY for new pane with optional command/cwd
3. **No Layout Logic**: Defer visual layout to client-side (panes are logical, not positioned)

### Method Signature

```rust
// In SessionManager
pub fn split_pane(
    &mut self,
    source_pane_id: Uuid,
    _direction: SplitDirection,  // Reserved for future layout
    command: Option<String>,
    cwd: Option<PathBuf>,
) -> Result<(Uuid, Uuid, Uuid)>  // (session_id, window_id, new_pane_id)
```

Direction is captured but not used for layout in this phase - it's reserved for future layout manager integration.

### Executor Flow

```
execute_spawn(source_pane, direction, command, cwd)
    |
    v
SessionManager.split_pane(source_pane, direction, command, cwd)
    |
    +--> find_pane(source_pane) -> (session, window, _)
    |
    +--> window.create_pane() -> new_pane
    |
    v
Return (session_id, window_id, new_pane_id)
    |
    v
PtyManager.spawn_pty(new_pane_id, command, cwd)
    |
    v
Broadcast PaneCreated to clients
```

### PTY Integration Options

Two approaches for PTY creation:

**Option A: Executor Coordinates (Recommended)**
- Executor calls SessionManager.split_pane()
- Executor calls PtyManager.spawn_pty()
- Cleaner separation of concerns

**Option B: SessionManager Coordinates**
- SessionManager takes PtyManager reference
- split_pane() calls PtyManager internally
- Tighter coupling but simpler executor

Recommendation: **Option A** - keeps SessionManager focused on session hierarchy.

## Affected Components

| Component | Type of Change | Risk Level |
|-----------|----------------|------------|
| `ccmux-server/src/session/manager.rs` | Add split_pane() method | Low |
| `ccmux-server/src/sideband/executor.rs` | Complete execute_spawn() | Medium |
| `ccmux-server/src/pty/manager.rs` | May need spawn API adjustments | Low |

## Dependencies

- FEAT-019 (Sideband Protocol) - Complete, provides parser and commands
- FEAT-012 (Session Management) - Complete, provides SessionManager
- FEAT-013 (PTY Management) - Complete, provides PtyManager

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| PTY spawn failure | Medium | Medium | Return error, don't create orphan pane |
| Invalid source pane | Low | Low | Validate and return clear error |
| Race conditions | Low | Medium | SessionManager is behind Mutex |
| Client notification failure | Low | Low | Log warning, don't fail split |

## Implementation Phases

### Phase 1: SessionManager.split_pane()
1. Add method to find source pane's window
2. Create new pane in that window
3. Return necessary IDs for PTY spawning
4. Add unit tests

### Phase 2: Executor Implementation
1. Remove TODO warning from execute_spawn
2. Lock SessionManager and call split_pane
3. Handle errors appropriately
4. Add logging for successful splits

### Phase 3: PTY Integration
1. Get PtyManager reference in executor (or access through Server)
2. Spawn PTY with command and cwd
3. Associate PTY with new pane ID
4. Handle spawn failures

### Phase 4: Client Broadcasting
1. After successful split, broadcast event
2. Use existing broadcast infrastructure
3. Include session/window/pane info

## Open Questions

1. **Executor PTY Access**: How does executor get PtyManager reference?
   - Option: Pass to CommandExecutor::new()
   - Option: Access through shared Server state

2. **Broadcast Channel**: Where is the client broadcast channel?
   - Need to review server connection handling

3. **Default Shell**: If no command specified, use what?
   - Probably: User's default shell from $SHELL

## Testing Strategy

1. **Unit Tests**:
   - split_pane with valid source pane
   - split_pane with invalid source pane
   - split_pane creates pane in correct window

2. **Integration Tests**:
   - execute_spawn end-to-end
   - spawn with command
   - spawn with cwd

3. **Manual Testing**:
   - Run ccmux, have Claude output spawn command
   - Verify new pane appears and is functional

## Rollback Strategy

If implementation causes issues:
1. Revert changes to executor.rs (restore TODO stub)
2. Revert changes to manager.rs (remove split_pane)
3. Spawn commands will be parsed and stripped but not executed
4. Document what went wrong in comments.md

## Implementation Notes

<!-- Add notes during implementation -->

---
*This plan should be updated as implementation progresses.*
