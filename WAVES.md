# WAVES.md - Parallel Feature Rollout Plan

**Generated**: 2026-01-08
**Total Features**: 20
**Completed**: 6
**Remaining**: 14

This document defines feature waves for parallel development. Features within the same wave
can be developed concurrently in isolated git worktrees. Each wave must complete before
the next wave can begin (dependencies are satisfied).

---

## Dependency Graph

```
                    [Wave 0 - Foundation Layer (Completed)]
                    ========================================
    FEAT-007         FEAT-008         FEAT-013
    Protocol         Utilities        PTY Mgmt
       |                |                |
       +------+    +----+          +-----+-----+
       |      |    |               |           |
       v      v    v               v           |
    FEAT-011  FEAT-012  FEAT-017   FEAT-014    |
    Client    Session   Config     Terminal    |
    Connect   Mgmt      (done)     Parsing     |
    (done)    (done)                  |        |
       |         |                    |        |
       |         |         +----------+--------+-----+
       |         |         |          |        |     |
       v         |         v          v        |     |
    FEAT-009     |      FEAT-019   FEAT-015    |     |
    Client UI    |      Sideband   Claude      |     |
       |         |      Protocol   Detection   |     |
       |         |                    |        |     |
       v         |         +----------+--------+     |
    FEAT-010     |         |          |              |
    Client       |         v          v              |
    Input        +----> FEAT-018   FEAT-020 <--------+
                        MCP Server Session Isolation

    [Standalone features - no dependencies, can start immediately]
    FEAT-001 (Pane Content)    FEAT-002 (Scrollback)    FEAT-003 (Viewport)
    FEAT-004 (Worktree)        FEAT-005 (Response)      FEAT-006 (Logging)
```

---

## Wave 0: Completed Foundation

These features are already implemented and provide the foundation for subsequent waves.

| ID | Title | Component | Effort | Notes |
|----|-------|-----------|--------|-------|
| FEAT-007 | Protocol Layer - IPC Message Types and Codec | ccmux-protocol | medium | Foundation for all IPC |
| FEAT-008 | Utilities - Error Types, Logging, Path Helpers | ccmux-utils | small | Foundation utilities |
| FEAT-011 | Client Connection - Unix Socket Client | ccmux-client | medium | Depends on FEAT-007 |
| FEAT-012 | Session Management - Session/Window/Pane Hierarchy | ccmux-server | medium | Depends on FEAT-007 |
| FEAT-013 | PTY Management - Process Spawning and Lifecycle | ccmux-server | medium | No dependencies |
| FEAT-017 | Configuration - TOML Config with Hot Reload | ccmux-server | medium | Depends on FEAT-008 |

**Status**: All 6 features completed. Wave 1 can begin immediately.

---

## Wave 1: Core Implementation

All dependencies satisfied by Wave 0. These features can be developed **in parallel**.

### Parallel Group 1A: Architecture Requirements (No Server Dependencies)

These are standalone features that don't depend on any other new features.

| ID | Title | Component | Priority | Effort | Dependencies | Parallel |
|----|-------|-----------|----------|--------|--------------|----------|
| FEAT-001 | Pane Content Abstraction (Terminal vs Canvas) | session/pane | P1 | large | none | Yes |
| FEAT-002 | Per-Session-Type Scrollback Configuration | config | P1 | medium | none | Yes |
| FEAT-003 | Viewport Pinning with New Content Indicator | tui | P2 | medium | none | Yes |
| FEAT-004 | Worktree-Aware Orchestration | orchestration | P2 | xl | none | Yes |
| FEAT-005 | Response Channel for Orchestrator-Worker | orchestration | P1 | medium | none | Yes |
| FEAT-006 | Per-Session Log Levels and Storage | logging | P2 | medium | none | Yes |

### Parallel Group 1B: Core Server & Client Features

These depend on Wave 0 completions.

| ID | Title | Component | Priority | Effort | Dependencies | Parallel |
|----|-------|-----------|----------|--------|--------------|----------|
| FEAT-009 | Client UI - Ratatui Terminal Interface | ccmux-client | P1 | large | FEAT-007, FEAT-011 | Yes |
| FEAT-014 | Terminal Parsing - ANSI/VT100 State Machine | ccmux-server | P1 | medium | FEAT-013 | Yes |
| FEAT-016 | Persistence - Checkpoint and WAL | ccmux-server | P2 | large | FEAT-012 | Yes |

### Wave 1 Summary

- **Total Features**: 9
- **Estimated Effort**: 3 large + 5 medium + 1 xl = significant
- **P1 Features**: 5 (FEAT-001, 002, 005, 009, 014)
- **P2 Features**: 4 (FEAT-003, 004, 006, 016)
- **Maximum Parallelism**: 9 (all can run concurrently)

### Recommended Wave 1 Worktree Assignments

For optimal parallelism with limited resources, prioritize P1 features:

| Worktree | Feature | Rationale |
|----------|---------|-----------|
| worktree-1 | FEAT-009 (Client UI) | Critical path - blocks FEAT-010 |
| worktree-2 | FEAT-014 (Terminal Parsing) | Critical path - blocks FEAT-015, 019, 020 |
| worktree-3 | FEAT-001 (Pane Content) | P1, large effort, standalone |
| worktree-4 | FEAT-002 (Scrollback) | P1, medium effort, standalone |
| worktree-5 | FEAT-005 (Response Channel) | P1, medium effort, standalone |

**Deferred to later in Wave 1** (if resources limited):
- FEAT-003, FEAT-004, FEAT-006, FEAT-016 (all P2)

---

## Wave 2: Dependent Features

Requires Wave 1 completion (specifically FEAT-009 and FEAT-014).

| ID | Title | Component | Priority | Effort | Dependencies | Parallel |
|----|-------|-----------|----------|--------|--------------|----------|
| FEAT-010 | Client Input - Keyboard and Mouse Handling | ccmux-client | P1 | medium | FEAT-009 | Yes |
| FEAT-015 | Claude Detection - State from PTY Output | ccmux-server | P1 | large | FEAT-014 | Yes |
| FEAT-019 | Sideband Protocol - XML Command Parsing | ccmux-server | P2 | medium | FEAT-014 | Yes |

### Wave 2 Summary

- **Total Features**: 3
- **Estimated Effort**: 1 large + 2 medium
- **P1 Features**: 2 (FEAT-010, FEAT-015)
- **P2 Features**: 1 (FEAT-019)
- **Maximum Parallelism**: 3 (all can run concurrently)
- **Blocking Dependencies**:
  - FEAT-010 blocked by: FEAT-009
  - FEAT-015 blocked by: FEAT-014
  - FEAT-019 blocked by: FEAT-014

### Recommended Wave 2 Worktree Assignments

| Worktree | Feature | Rationale |
|----------|---------|-----------|
| worktree-1 | FEAT-015 (Claude Detection) | Critical path - blocks FEAT-018, 020 |
| worktree-2 | FEAT-010 (Client Input) | Completes client functionality |
| worktree-3 | FEAT-019 (Sideband Protocol) | P2 but enables Claude integration |

---

## Wave 3: Final Integration

Requires Wave 2 completion (specifically FEAT-015).

| ID | Title | Component | Priority | Effort | Dependencies | Parallel |
|----|-------|-----------|----------|--------|--------------|----------|
| FEAT-018 | MCP Server - Model Context Protocol | ccmux-server | P2 | large | FEAT-012, FEAT-015 | Yes |
| FEAT-020 | Session Isolation - Per-Pane CLAUDE_CONFIG_DIR | ccmux-server | P1 | small | FEAT-013, FEAT-015 | Yes |

### Wave 3 Summary

- **Total Features**: 2
- **Estimated Effort**: 1 large + 1 small
- **P1 Features**: 1 (FEAT-020)
- **P2 Features**: 1 (FEAT-018)
- **Maximum Parallelism**: 2 (both can run concurrently)
- **Blocking Dependencies**:
  - FEAT-018 blocked by: FEAT-012 (Wave 0), FEAT-015 (Wave 2)
  - FEAT-020 blocked by: FEAT-013 (Wave 0), FEAT-015 (Wave 2)

### Recommended Wave 3 Worktree Assignments

| Worktree | Feature | Rationale |
|----------|---------|-----------|
| worktree-1 | FEAT-020 (Session Isolation) | P1, small effort, quick win |
| worktree-2 | FEAT-018 (MCP Server) | P2, large effort, enables AI automation |

---

## Critical Path Analysis

The longest dependency chain determines minimum completion time:

```
FEAT-013 (Wave 0, done)
    |
    v
FEAT-014 (Wave 1)  -----> Total: ~medium effort
    |
    v
FEAT-015 (Wave 2)  -----> Total: ~large effort
    |
    +---> FEAT-018 (Wave 3)  -----> Total: ~large effort
    |
    +---> FEAT-020 (Wave 3)  -----> Total: ~small effort
```

**Critical Path**: FEAT-013 -> FEAT-014 -> FEAT-015 -> FEAT-020

**Estimated Timeline** (assuming 1 feature per developer):
- Wave 1: 2-3 weeks (large effort for FEAT-009, FEAT-001)
- Wave 2: 1-2 weeks (large effort for FEAT-015)
- Wave 3: 1 week (small effort for FEAT-020, parallel with FEAT-018)

**Total Estimated Duration**: 4-6 weeks with parallelization

---

## Execution Summary

| Wave | Features | Parallelism | P1 Count | P2 Count | Effort |
|------|----------|-------------|----------|----------|--------|
| 0 | 6 | N/A | 5 | 1 | Completed |
| 1 | 9 | 9 | 5 | 4 | 3L + 5M + 1XL |
| 2 | 3 | 3 | 2 | 1 | 1L + 2M |
| 3 | 2 | 2 | 1 | 1 | 1L + 1S |
| **Total** | **20** | - | **13** | **7** | - |

---

## Machine-Readable Format

For consumption by worktree orchestration (FEAT-004):

```json
{
  "waves": {
    "0": {
      "status": "completed",
      "features": ["FEAT-007", "FEAT-008", "FEAT-011", "FEAT-012", "FEAT-013", "FEAT-017"]
    },
    "1": {
      "status": "pending",
      "features": ["FEAT-001", "FEAT-002", "FEAT-003", "FEAT-004", "FEAT-005", "FEAT-006", "FEAT-009", "FEAT-014", "FEAT-016"],
      "parallel": true,
      "blocking": {
        "FEAT-009": ["FEAT-007", "FEAT-011"],
        "FEAT-014": ["FEAT-013"],
        "FEAT-016": ["FEAT-012"]
      }
    },
    "2": {
      "status": "blocked",
      "features": ["FEAT-010", "FEAT-015", "FEAT-019"],
      "parallel": true,
      "blocking": {
        "FEAT-010": ["FEAT-009"],
        "FEAT-015": ["FEAT-014"],
        "FEAT-019": ["FEAT-014"]
      }
    },
    "3": {
      "status": "blocked",
      "features": ["FEAT-018", "FEAT-020"],
      "parallel": true,
      "blocking": {
        "FEAT-018": ["FEAT-012", "FEAT-015"],
        "FEAT-020": ["FEAT-013", "FEAT-015"]
      }
    }
  },
  "critical_path": ["FEAT-013", "FEAT-014", "FEAT-015", "FEAT-020"],
  "p1_priority_order": [
    "FEAT-009", "FEAT-014", "FEAT-001", "FEAT-002", "FEAT-005",
    "FEAT-010", "FEAT-015", "FEAT-020"
  ]
}
```

---

## Notes for Orchestration System

1. **Wave Gating**: Do not start Wave N+1 until all features in Wave N are merged and tests pass.

2. **Merge Strategy**: Features within a wave should be merged in dependency order when possible
   to allow late-wave features to start before all wave features complete.

3. **P1 Prioritization**: When resources are limited, prioritize P1 features within each wave.

4. **Critical Path Focus**: FEAT-014 and FEAT-015 are on the critical path - prioritize these
   to minimize total completion time.

5. **Test Requirements**: Each feature merge should trigger:
   - Unit tests for the feature
   - Integration tests with completed dependencies
   - Retrospective analysis before next wave

6. **Worktree Naming Convention**: `worktree-{wave}-{feature_id}` (e.g., `worktree-1-feat-009`)
