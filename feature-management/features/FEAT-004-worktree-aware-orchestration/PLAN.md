# Implementation Plan: FEAT-004

**Work Item**: [FEAT-004: Worktree-Aware Orchestration](PROMPT.md)
**Component**: orchestration
**Priority**: P2
**Created**: 2026-01-08

## Overview

Workers spawn in isolated git worktrees for parallel development. Orchestrator reads WAVES.md from scan-prioritize agent, spawns wave 1 workers in parallel (each in own worktree), on completion triggers merge queue followed by tests and retrospective, then spawns wave 2.

## Architecture Decisions

<!-- Document key design choices and rationale -->

- **Approach**: [To be determined during implementation]
- **Trade-offs**: [To be evaluated]

### Key Design Questions

1. **Git Library Choice**: git2 (libgit2 bindings) vs. shelling out to git CLI
   - git2: Better type safety, no subprocess overhead
   - CLI: More familiar, handles edge cases git2 might miss

2. **Worktree Naming Convention**: How to name worktrees and branches
   - Option A: `ccmux-wave{N}-task{M}` (predictable, sortable)
   - Option B: `ccmux-{task-slug}` (more readable)
   - Option C: UUID-based (collision-free)

3. **WAVES.md Format**: Structure for wave/task specification
   - Likely markdown with YAML frontmatter per task
   - Need to define required vs. optional fields

4. **Merge Strategy**: How to handle merge queue
   - Sequential merges to main branch
   - Conflict detection before merge attempt
   - Rollback strategy on failure

## Affected Components

<!-- List files and modules that will be modified -->

| Component | Type of Change | Risk Level |
|-----------|----------------|------------|
| ccmux-server/src/git/ (new) | New module | Medium |
| ccmux-server/src/session/manager.rs | Extension | Medium |
| ccmux-protocol/src/lib.rs | Extension | Low |
| ccmux-server/src/waves/ (new) | New module | Medium |

## Dependencies

No feature dependencies. This is a standalone new feature.

External dependencies to add:
- `git2` crate for git operations
- Possibly `pulldown-cmark` for WAVES.md parsing

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Worktree conflicts with user's git state | Medium | High | Check for dirty state, warn before operations |
| Merge conflicts between workers | Medium | Medium | Conflict detection, notification system |
| Orphaned worktrees on crash | Medium | Low | Cleanup on startup, periodic garbage collection |
| Performance with many worktrees | Low | Medium | Limit concurrent worktrees, cleanup policy |
| git2 crate limitations | Low | Medium | Fallback to CLI for edge cases |

## Rollback Strategy

If implementation causes issues:
1. Revert commits associated with this work item
2. Verify system returns to previous state
3. Document what went wrong in comments.md

Worktree cleanup on rollback:
- All ccmux-created worktrees should be removable via `git worktree remove`
- Worktrees are named with `ccmux-` prefix for easy identification

## Implementation Notes

<!-- Add notes during implementation -->

### Phase 1: Git Foundation
Focus on basic git detection and worktree operations without orchestration integration.

### Phase 2: WAVES.md Parser
Standalone parser that can be tested independently.

### Phase 3: Integration
Wire git and parser into session management and orchestrator.

### Phase 4: Merge Queue & Testing
Add coordination features for merge and test execution.

---
*This plan should be updated as implementation progresses.*
