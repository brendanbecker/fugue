# FEAT-004: Worktree-Aware Orchestration

**Priority**: P2
**Component**: orchestration
**Type**: new_feature
**Estimated Effort**: xl
**Business Value**: high

## Overview

Workers spawn in isolated git worktrees for parallel development:

- Orchestrator reads `WAVES.md` from scan-prioritize agent
- Spawns wave 1 workers in parallel, each in own worktree
- On completion: merge queue -> tests -> retro
- Then spawns wave 2

## Requirements

- Git integration subsystem (detect repo, create worktrees)
- Parse WAVES.md format for task waves
- Worktree lifecycle management (create, cleanup)
- Merge queue coordination
- Test runner integration post-merge
- Retrospective trigger on wave completion

## Current State

- Zero git/worktree integration in codebase
- Panes track optional `cwd` field
- No branch tracking or git status detection
- No workspace-aware features

## Affected Files

- New: `fugue-server/src/git/` module
- `fugue-server/src/session/manager.rs`
- `fugue-protocol/src/lib.rs` (worktree messages)
- New: WAVES.md parser

## Benefits

- True parallel development with isolated workspaces
- No merge conflicts during parallel agent work
- Concurrent feature development across multiple Claude Code instances
- Automated merge queue prevents integration issues
- Wave-based execution ensures dependencies are respected

## Implementation Tasks

### Section 1: Git Integration Foundation
- [ ] Create `fugue-server/src/git/` module structure
- [ ] Implement git repository detection (find .git, check worktree support)
- [ ] Add git2 crate dependency for Rust git operations
- [ ] Create GitRepo abstraction for repository operations
- [ ] Implement worktree status detection (list existing worktrees)

### Section 2: Worktree Lifecycle Management
- [ ] Implement worktree creation with branch naming convention
- [ ] Add worktree cleanup on completion/abort
- [ ] Handle worktree locking and concurrent access
- [ ] Implement orphaned worktree detection and cleanup
- [ ] Add worktree path management (configurable base directory)

### Section 3: WAVES.md Parser
- [ ] Define WAVES.md format specification
- [ ] Implement markdown parser for wave structure
- [ ] Extract task metadata (ID, description, dependencies)
- [ ] Validate wave dependencies (no cycles, proper ordering)
- [ ] Create Wave and Task domain models

### Section 4: Protocol Extensions
- [ ] Add worktree-related messages to fugue-protocol
- [ ] Define WorktreeCreate, WorktreeDestroy messages
- [ ] Add wave status messages (WaveStart, WaveComplete)
- [ ] Implement merge queue messages (MergeRequest, MergeComplete)
- [ ] Add test trigger messages

### Section 5: Orchestrator Integration
- [ ] Integrate WAVES.md reading into orchestrator flow
- [ ] Spawn wave workers with worktree assignment
- [ ] Track worker-to-worktree mapping in session state
- [ ] Implement wave completion detection
- [ ] Trigger next wave on previous wave completion

### Section 6: Merge Queue Coordination
- [ ] Implement merge queue data structure
- [ ] Add merge ordering logic (FIFO with conflict detection)
- [ ] Integrate with git merge operations
- [ ] Handle merge conflicts (notify, block, or auto-resolve)
- [ ] Track merge status per worktree

### Section 7: Test Runner Integration
- [ ] Define test trigger protocol
- [ ] Implement post-merge test execution
- [ ] Capture test results and status
- [ ] Block next wave on test failure (configurable)
- [ ] Report test status to orchestrator

### Section 8: Retrospective Trigger
- [ ] Define retrospective trigger conditions
- [ ] Implement wave completion summary generation
- [ ] Trigger retrospective agent on wave complete
- [ ] Pass wave metadata to retrospective

### Section 9: Session State Extensions
- [ ] Add worktree tracking to session state
- [ ] Persist worktree assignments for crash recovery
- [ ] Update pane model with worktree reference
- [ ] Add wave progress tracking

### Section 10: Testing
- [ ] Unit tests for git module
- [ ] Unit tests for WAVES.md parser
- [ ] Integration tests for worktree lifecycle
- [ ] Integration tests for merge queue
- [ ] End-to-end test for multi-wave execution

## Acceptance Criteria

- [ ] Git worktrees can be created and destroyed programmatically
- [ ] WAVES.md files are parsed correctly with wave/task extraction
- [ ] Workers spawn in isolated worktrees with unique branches
- [ ] Merge queue processes completed work in order
- [ ] Tests run automatically after merges
- [ ] Retrospective triggers on wave completion
- [ ] Crash recovery restores worktree state
- [ ] No regressions in existing session functionality
- [ ] All tests passing

## Notes

This is a foundational feature for enabling true parallel agent orchestration. The git worktree model allows multiple Claude Code instances to work on different features simultaneously without stepping on each other's changes.

Key design decisions to make during implementation:
- Worktree naming convention (e.g., `fugue-wave1-task3`)
- Branch naming strategy (feature branches vs. temporary branches)
- Merge conflict resolution strategy (block, notify, or attempt auto-resolve)
- WAVES.md format specification (likely markdown with frontmatter)
