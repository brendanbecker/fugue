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
| **Stream A** | Core Stability | `../ccmux-stream-a` | Modularize MCP bridge, fix state drift bugs. | Complete |
| **Stream B** | UX / Safety | `../ccmux-stream-b` | Fix client crashes, implement Human Arbitration. | Ready |
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
- **FEAT-082 (P2)**: Adaptive layout engine (Stream B).
- **FEAT-074 (P2)**: Telemetry and observability dashboard (Stream C).

## Backlog Highlights

### High Priority (P0/P1)
- **BUG-041**: Client crash on paste (Verified Fix, monitor).
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

## Session Log (2026-01-15) - Core Stability & Conflict Resolution

### Work Completed This Session
1. **BUG-032 Resolved (Stream A)**
   - Merged `stream-a-core-stability` which implemented global TUI broadcasts for MCP operations (CreateSession, CreateWindow, CreatePane, etc.).
   - Updated handlers to return `ResponseWithGlobalBroadcast` or `ResponseWithBroadcast` variants.

2. **Merge Conflict Resolution**
   - Resolved complex conflicts in `ccmux-server/src/handlers/mcp_bridge.rs` and `session.rs` arising from parallel changes in Stream A (broadcasts), Stream B (persistence/sequencing), and Stream C (per-pane config).
   - Harmonized logic to ensure:
     - New sessions/panes get custom Claude config (FEAT-071).
     - Events are logged to persistence (FEAT-075).
     - Events are sequenced for replay (FEAT-075).
     - Events are broadcast to all TUI clients (BUG-032).

3. **Compilation & Testing**
   - Fixed compilation errors in `mcp_bridge.rs`, `tcp.rs`, `compat/client.rs`.
   - Updated `test_handle_resize_success` to match new broadcast behavior.
   - Verified 1626+ tests passing across the workspace.

### Next Steps
- **Stream B**: FEAT-082 (Adaptive Layout).
- **Stream C**: FEAT-074 (Telemetry/Dashboard).
