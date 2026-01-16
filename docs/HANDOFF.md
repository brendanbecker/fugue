# ccmux Project Handoff

> **LIVING DOCUMENT**: This handoff file is the interface between sessions. Update it constantly as you work—mark completed items, add discoveries, note blockers, revise plans.

## Context

**ccmux** is a Claude Code-aware terminal multiplexer in Rust.
**Current Stage**: Stage 7 (Post-MVP Stabilization & Enhancement)
**Status**: Production-ready core, currently stabilizing extended MCP capabilities and adding agent-specific features.

## Current State (2026-01-15)

**Parallel Execution Mode**: Development is split into 3 streams to isolate risk and maximize throughput.

### Active Streams

| Stream | Focus | Worktree | Objective | Status |
|--------|-------|----------|-----------|--------|
| **Stream A** | User Interface | `../ccmux-stream-a` | Implement TUI visibility dashboard. | Active (FEAT-073) |
| **Stream B** | UX / Safety | `../ccmux-stream-b` | Fix client crashes, implement Adaptive Layout. | In Progress (FEAT-082) |
| **Stream C** | Observability | `../ccmux-stream-c` | Backend metrics and structured tracing. | Active (FEAT-074) |

### Workflow: Continuous Integration

We use a "CI-in-worktree" pattern to keep branches short-lived and history clean.

1.  **Work in Worktree**: Implement task in `../ccmux-stream-X`.
2.  **Commit**: `git add . && git commit -m "..."`.
3.  **Merge**: From the **Root** directory, `git merge stream-x-branch`.
4.  **Refresh Worktree**:
    *   In worktree: `git fetch .. main` (or `git pull origin main` if remote exists).
    *   Merge `main` into your stream branch to sync.
5.  **Update Session**: Update `SESSION.md` in the worktree with the next task.

## Recent Activity (2026-01-15)

### Completed
- **BUG-032**: MCP handlers missing TUI broadcasts (Stream A).
- **FEAT-064**: Refactor MCP bridge.rs into modular components (Stream A).
- **BUG-033, BUG-034, BUG-035, BUG-039, BUG-040**: Core stability and validation fixes (Stream A).
- **BUG-036**: Fix Selection tools not switching TUI view (Stream A).
- **BUG-041**: Fix Claude Code crash on paste via bracketed paste (Stream B).
- **FEAT-079**: Comprehensive Human-Control Arbitration (Stream B).
- **FEAT-077**: Human-Control UX Indicators (Stream B).
- **FEAT-078**: Per-client focus state support (Stream B).
- **FEAT-080, FEAT-081, FEAT-071**: Sideband Config, Landlock Sandboxing, Per-pane Claude config (Stream C).
- **FEAT-066**: TCP listener support in daemon (Stream C).
- **FEAT-067**: Client TCP connection support (Stream C).
- **FEAT-068**: SSH tunnel integration and documentation (Stream C).
- **FEAT-070**: Gastown remote pane support (Stream C).
- **FEAT-075**: Snapshot + replay resync API (Stream B).
- **FEAT-082 (aka 073)**: Multi-tier routing logic via target aliases (Stream C).
- **FEAT-063**: File-based logging for MCP bridge (Stream B).
- **BUG-042**: Flatten Result nesting code smell (Stream A).
- **Retro**: Conducted comprehensive retrospective, categorized backlog into streams.

### In Progress
- **FEAT-073 (P2)**: Visibility dashboard (Stream A).
- **FEAT-082 (P2)**: Adaptive layout engine (Stream B).
- **FEAT-074 (P2)**: Telemetry and observability dashboard (Stream C).

## Backlog Highlights

### High Priority (P0/P1)
- **FEAT-061**: Screen Redraw Command (UX).
- **BUG-039**: MCP tools hang intermittently (Logging added, monitor).
- **BUG-033**: Layout validation too strict.

### Strategic Features
- **FEAT-079**: Human-Control Arbitration (Safety).
- **FEAT-081**: Landlock Sandboxing (Security).
- **FEAT-066+**: Remote Peering (Connectivity).

## Reference

- **Features**: `feature-management/features/features.md`
- **Bugs**: `feature-management/bugs/bugs.md`
- **Retrospective**: `feature-management/RETROSPECTIVE_2026_01_14.md`

---

## Session Log (2026-01-15) - Task Reassignment

### Work Completed This Session
1. **Stream Reassignment**
   - Stream A (completed stability) assigned **FEAT-073** (Visibility Dashboard).
   - Stream C (completed remote peering) assigned **FEAT-074** (Observability Instrumentation).
   - Main branch updated to reflect new stream objectives.

### Next Steps
- Monitor Stream B progress on Adaptive Layout.
- Prepare for FEAT-073/074 integration.