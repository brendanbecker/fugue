# Task Breakdown: FEAT-004

**Work Item**: [FEAT-004: Worktree-Aware Orchestration](PROMPT.md)
**Status**: Not Started
**Last Updated**: 2026-01-08

## Prerequisites

- [ ] Read and understand PROMPT.md
- [ ] Review PLAN.md and update if needed
- [ ] Verify no conflicting changes in session management

## Design Tasks

- [ ] Finalize git library choice (git2 vs CLI)
- [ ] Define worktree naming convention
- [ ] Specify WAVES.md format (document in PLAN.md)
- [ ] Design merge queue algorithm
- [ ] Define protocol message schemas
- [ ] Review and update PLAN.md with decisions

## Git Module Tasks

- [ ] Create `fugue-server/src/git/mod.rs` module structure
- [ ] Implement git repository detection
- [ ] Add git2 crate dependency to Cargo.toml
- [ ] Create `GitRepo` struct for repository operations
- [ ] Implement `is_git_repo()` detection
- [ ] Implement `list_worktrees()` function
- [ ] Implement `create_worktree(name, branch)` function
- [ ] Implement `remove_worktree(name)` function
- [ ] Add worktree status checking (clean/dirty)
- [ ] Implement orphaned worktree detection
- [ ] Add unit tests for git module

## WAVES.md Parser Tasks

- [ ] Create `fugue-server/src/waves/mod.rs` module
- [ ] Define `Wave` struct (id, tasks, dependencies)
- [ ] Define `Task` struct (id, description, metadata)
- [ ] Implement markdown parser for WAVES.md
- [ ] Parse wave headers and task lists
- [ ] Extract task metadata from list items
- [ ] Validate wave dependencies (no cycles)
- [ ] Add error types for parse failures
- [ ] Add unit tests for parser

## Protocol Extension Tasks

- [ ] Add `WorktreeCreate` message to protocol
- [ ] Add `WorktreeDestroy` message to protocol
- [ ] Add `WorktreeStatus` message for queries
- [ ] Add `WaveStart` notification message
- [ ] Add `WaveComplete` notification message
- [ ] Add `MergeRequest` message
- [ ] Add `MergeComplete` notification
- [ ] Add `TestTrigger` message
- [ ] Add `TestResult` notification
- [ ] Update protocol documentation

## Session Integration Tasks

- [ ] Add worktree tracking to session state
- [ ] Add `worktree_path` field to Pane struct
- [ ] Implement worktree assignment on pane spawn
- [ ] Track worker-to-worktree mapping
- [ ] Add wave progress tracking to session
- [ ] Persist worktree state for crash recovery
- [ ] Implement worktree restoration on resume

## Orchestrator Tasks

- [ ] Add WAVES.md file detection
- [ ] Integrate WAVES.md parser into orchestrator
- [ ] Implement wave worker spawning logic
- [ ] Assign worktrees to spawned workers
- [ ] Implement wave completion detection
- [ ] Trigger next wave on completion
- [ ] Handle wave failure scenarios

## Merge Queue Tasks

- [ ] Create merge queue data structure
- [ ] Implement FIFO ordering with priority support
- [ ] Add conflict detection before merge
- [ ] Implement sequential merge execution
- [ ] Handle merge conflicts (notify orchestrator)
- [ ] Track merge status per worktree
- [ ] Add merge rollback capability

## Test Runner Integration Tasks

- [ ] Define test trigger interface
- [ ] Implement post-merge test execution
- [ ] Capture test output and status
- [ ] Report results to orchestrator
- [ ] Implement wave blocking on test failure
- [ ] Add configurable test failure behavior

## Retrospective Integration Tasks

- [ ] Define retrospective trigger conditions
- [ ] Generate wave completion summary
- [ ] Trigger retrospective on wave complete
- [ ] Pass wave metadata to retrospective agent

## Testing Tasks

- [ ] Unit tests for git module
- [ ] Unit tests for WAVES.md parser
- [ ] Unit tests for merge queue
- [ ] Integration test: worktree lifecycle
- [ ] Integration test: wave execution
- [ ] Integration test: merge queue flow
- [ ] End-to-end test: multi-wave scenario
- [ ] Test crash recovery with worktrees

## Documentation Tasks

- [ ] Document WAVES.md format specification
- [ ] Document worktree naming conventions
- [ ] Update architecture documentation
- [ ] Add configuration options documentation
- [ ] Document merge conflict handling

## Completion Checklist

- [ ] All implementation tasks complete
- [ ] All tests passing
- [ ] Documentation updated
- [ ] PLAN.md reflects final implementation
- [ ] feature_request.json status updated
- [ ] Ready for review/merge

---
*Check off tasks as you complete them. Update status field above.*
