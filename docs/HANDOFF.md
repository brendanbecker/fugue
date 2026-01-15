# ccmux Project Handoff

> **LIVING DOCUMENT**: This handoff file is the interface between sessions. Update it constantly as you work—mark completed items, add discoveries, note blockers, revise plans.

## Context

**ccmux** is a Claude Code-aware terminal multiplexer in Rust.
**Current Stage**: Stage 7 (Post-MVP Stabilization & Enhancement)
**Status**: Production-ready core, currently stabilizing extended MCP capabilities and adding agent-specific features.

## Current State (2026-01-14)

**Parallel Execution Mode**: Development is split into 3 streams to isolate risk and maximize throughput.

### Active Streams

| Stream | Focus | Worktree | Objective | Status |
|--------|-------|----------|-----------|--------|
| **Stream A** | Core Stability | `../ccmux-stream-a` | Modularize MCP bridge, fix state drift bugs. | Active (Refactor done) |
| **Stream B** | UX / Safety | `../ccmux-stream-b` | Fix client crashes, implement Human Arbitration. | Active (Fixing BUG-041) |
| **Stream C** | Features | `../ccmux-stream-c` | Sideband Config, Sandboxing, Remote Peering. | Ready |

### Workflow: Continuous Integration

We use a "CI-in-worktree" pattern to keep branches short-lived and history clean.

1.  **Work in Worktree**: Implement task in `../ccmux-stream-X`.
2.  **Commit**: `git add . && git commit -m "..."`.
3.  **Merge**: From the **Root** directory, `git merge stream-x-branch`.
4.  **Refresh Worktree**:
    *   In worktree: `git fetch .. main` (or `git pull origin main` if remote exists).
    *   Merge `main` into your stream branch to sync.
5.  **Update Session**: Update `SESSION.md` in the worktree with the next task.

## Recent Activity (2026-01-14)

### Completed
- **FEAT-064**: Refactor MCP bridge.rs into modular components (Stream A).
- **Retro**: Conducted comprehensive retrospective, categorized backlog into streams.

### In Progress
- **BUG-041 (P0)**: Claude Code crashes on paste inside ccmux (Stream B).
- **BUG-035 (P1)**: MCP handlers return wrong response types (Stream A).
- **FEAT-080 (P2)**: Per-Pane/Session Configuration via Sideband (Stream C).

## Backlog Highlights

### High Priority (P0/P1)
- **BUG-041**: Client crash on paste.
- **BUG-036**: Selection tools don't switch TUI view.
- **BUG-039**: MCP tools hang intermittently.
- **BUG-033**: Layout validation too strict.

### Strategic Features
- **FEAT-079**: Human-Control Arbitration (Safety).
- **FEAT-081**: Landlock Sandboxing (Security).
- **FEAT-066+**: Remote Peering (Connectivity).

## Reference

- **Features**: `feature-management/features/features.md`
- **Bugs**: `feature-management/bugs/bugs.md`
- **Retrospective**: `feature-management/RETROSPECTIVE_2026_01_14.md`