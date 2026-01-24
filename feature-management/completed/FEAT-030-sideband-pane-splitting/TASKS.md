# Task Breakdown: FEAT-030

**Work Item**: [FEAT-030: Sideband Pane Splitting - Execute spawn Commands via Sideband Protocol](PROMPT.md)
**Status**: Not Started
**Last Updated**: 2026-01-09

## Prerequisites

- [ ] Read and understand PROMPT.md
- [ ] Review PLAN.md and update if needed
- [ ] Review existing sideband executor code in `fugue-server/src/sideband/executor.rs`
- [ ] Review SessionManager API in `fugue-server/src/session/manager.rs`
- [ ] Review PtyManager API in `fugue-server/src/pty/manager.rs`
- [ ] Understand how PTYs are currently spawned (session creation flow)

## Design Tasks

- [ ] Decide on PTY integration approach (executor coordinates vs SessionManager)
- [ ] Determine how executor accesses PtyManager
- [ ] Plan client broadcast mechanism
- [ ] Document method signatures in PLAN.md

## Implementation Tasks

### SessionManager Extension (manager.rs)

- [ ] Add `split_pane()` method signature
- [ ] Implement find source pane by UUID
- [ ] Get window containing source pane
- [ ] Create new pane in that window via `window.create_pane()`
- [ ] Return tuple of (session_id, window_id, new_pane_id)
- [ ] Handle case where source pane doesn't exist
- [ ] Add appropriate error types if needed

### Executor Implementation (executor.rs)

- [ ] Remove the TODO comment and warning from `execute_spawn()`
- [ ] Lock SessionManager and call `split_pane()`
- [ ] Extract session/window/pane IDs from result
- [ ] Handle split_pane errors with appropriate ExecuteError
- [ ] Add info-level logging for successful splits
- [ ] Convert cwd from Option<String> to Option<PathBuf>

### PTY Integration

- [ ] Determine how executor gets PtyManager access
- [ ] Add PtyManager reference to CommandExecutor if needed
- [ ] Call PtyManager.spawn_pty() (or equivalent) for new pane
- [ ] Pass command to PTY spawn if specified
- [ ] Pass cwd to PTY spawn if specified
- [ ] Handle PTY spawn failures (clean up pane on failure?)
- [ ] Ensure PTY output polling starts for new pane

### Client Broadcasting

- [ ] Identify existing broadcast mechanism in server
- [ ] Add broadcast channel to CommandExecutor if needed
- [ ] Create PaneCreated message/event type if not exists
- [ ] Broadcast new pane info after successful creation
- [ ] Include: session_id, window_id, pane_id, dimensions

### Error Handling

- [ ] Add ExecuteError variant for split failure if needed
- [ ] Handle source pane not found
- [ ] Handle window not found (shouldn't happen)
- [ ] Handle PTY spawn failure
- [ ] Log all errors with context

## Testing Tasks

### Unit Tests (manager.rs)

- [ ] Test split_pane with valid source pane
- [ ] Test split_pane with invalid/nonexistent source pane
- [ ] Test split_pane creates pane in correct window
- [ ] Test split_pane returns correct IDs
- [ ] Test split_pane with multiple windows (finds correct one)

### Unit Tests (executor.rs)

- [ ] Test execute_spawn with valid source pane
- [ ] Test execute_spawn with invalid source pane returns error
- [ ] Test execute_spawn with all parameters (direction, command, cwd)
- [ ] Test execute_spawn with minimal parameters (just direction)
- [ ] Test execute_spawn horizontal and vertical directions

### Integration Tests

- [ ] Test full spawn flow: parse command -> execute -> pane created
- [ ] Test spawn with command runs the command
- [ ] Test spawn with cwd uses correct directory
- [ ] Test multiple spawns from same source pane
- [ ] Test spawn from newly created pane (chain spawning)

### Manual Testing

- [ ] Start fugue server
- [ ] Output spawn command in terminal
- [ ] Verify command is stripped from display
- [ ] Verify new pane is created
- [ ] Verify command runs in new pane (if specified)
- [ ] Verify cwd is correct (if specified)

## Documentation Tasks

- [ ] Update PLAN.md with final architecture decisions
- [ ] Add code comments for split_pane method
- [ ] Document any new error types
- [ ] Update module-level docs if needed

## Verification Tasks

- [ ] All acceptance criteria from PROMPT.md met
- [ ] `<fugue:spawn direction="vertical" />` creates pane
- [ ] `<fugue:spawn direction="horizontal" />` creates pane
- [ ] `command` attribute works
- [ ] `cwd` attribute works
- [ ] Tests passing
- [ ] No warnings from removed TODO
- [ ] Update feature_request.json status to "in-progress" or "complete"

## Completion Checklist

- [ ] All implementation tasks complete
- [ ] All tests passing
- [ ] Code reviewed (self-review)
- [ ] PLAN.md updated with implementation notes
- [ ] Ready for merge

---
*Check off tasks as you complete them. Update status field above.*
