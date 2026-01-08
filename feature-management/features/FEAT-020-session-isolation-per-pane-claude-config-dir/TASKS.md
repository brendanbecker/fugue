# Task Breakdown: FEAT-020

**Work Item**: [FEAT-020: Session Isolation - Per-Pane CLAUDE_CONFIG_DIR](PROMPT.md)
**Status**: Not Started
**Last Updated**: 2026-01-08

## Prerequisites

- [ ] Read and understand PROMPT.md
- [ ] Review PLAN.md and update if needed
- [ ] Verify FEAT-013 (PTY Management) is complete
- [ ] Review existing PtyConfig implementation
- [ ] Understand pane lifecycle in session management

## Design Tasks

- [ ] Finalize isolation directory structure
- [ ] Document pane ID format requirements
- [ ] Design cleanup grace period strategy
- [ ] Plan orphan directory detection

## Implementation Tasks

### Isolation Module (ccmux-server/src/claude/isolation.rs)

- [ ] Create claude module directory structure
- [ ] Implement `IsolationDir` struct
- [ ] Implement `create_isolation_dir(pane_id)` function
- [ ] Implement `remove_isolation_dir(pane_id)` function
- [ ] Implement `cleanup_orphaned_dirs()` function
- [ ] Add pane metadata file creation/reading
- [ ] Set proper permissions (700) on directories
- [ ] Add error types for isolation operations

### PTY Configuration (ccmux-server/src/pty/config.rs)

- [ ] Add `isolation_dir: Option<PathBuf>` to PtyConfig
- [ ] Add CLAUDE_CONFIG_DIR to environment in `build_env()` method
- [ ] Update PtyConfig builder/constructor
- [ ] Document new configuration field

### Pane Integration (ccmux-server/src/session/pane.rs)

- [ ] Import isolation module
- [ ] Call `create_isolation_dir()` in pane creation
- [ ] Pass isolation_dir to PtyConfig
- [ ] Call `remove_isolation_dir()` in pane close
- [ ] Add grace period before cleanup (wait for process exit)
- [ ] Handle cleanup errors gracefully

### Server Startup

- [ ] Call `cleanup_orphaned_dirs()` on server initialization
- [ ] Log cleaned up orphan directories
- [ ] Handle cleanup errors (log but continue)

## Testing Tasks

- [ ] Unit test: IsolationDir creation
- [ ] Unit test: IsolationDir removal
- [ ] Unit test: Permission validation
- [ ] Unit test: Orphan detection
- [ ] Integration test: Single pane isolation
- [ ] Integration test: Multiple concurrent panes
- [ ] Integration test: Pane close cleanup
- [ ] Integration test: Server restart orphan cleanup
- [ ] Integration test: --resume with isolated session

## Documentation Tasks

- [ ] Document CLAUDE_CONFIG_DIR behavior
- [ ] Document cleanup lifecycle
- [ ] Add troubleshooting section for orphan directories
- [ ] Update PtyConfig documentation

## Verification Tasks

- [ ] All acceptance criteria from PROMPT.md met
- [ ] Tests passing
- [ ] Update feature_request.json status
- [ ] Document completion in PLAN.md

## Completion Checklist

- [ ] All implementation tasks complete
- [ ] All tests passing
- [ ] Documentation updated
- [ ] PLAN.md reflects final implementation
- [ ] Ready for review/merge

---
*Check off tasks as you complete them. Update status field above.*
