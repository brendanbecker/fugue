# WAVES.md - Parallel Feature Rollout Plan

**Generated**: 2026-01-08
**Updated**: 2026-01-09
**Total Features**: 27
**Completed**: 20 (Waves 0-3)
**Remaining**: 7 (Wave 4 - Integration)

This document defines feature waves for parallel development. Features within the same wave
can be developed concurrently in isolated git worktrees. Each wave must complete before
the next wave can begin (dependencies are satisfied).

---

## Dependency Graph

```
╔═══════════════════════════════════════════════════════════════════════════════╗
║                    WAVES 0-3: COMPLETED (20 features)                         ║
╠═══════════════════════════════════════════════════════════════════════════════╣
║  Wave 0: FEAT-007, 008, 011, 012, 013, 017 (Foundation)                       ║
║  Wave 1: FEAT-001, 002, 003, 004, 005, 006, 009, 014, 016 (Core)              ║
║  Wave 2: FEAT-010, 015, 019 (Dependent)                                       ║
║  Wave 3: FEAT-018, 020 (Final Integration)                                    ║
╚═══════════════════════════════════════════════════════════════════════════════╝
                                    |
                                    v
╔═══════════════════════════════════════════════════════════════════════════════╗
║                    WAVE 4: INTEGRATION (7 features)                           ║
╠═══════════════════════════════════════════════════════════════════════════════╣
║                                                                               ║
║     FEAT-021 (Server Socket Listen Loop)                                      ║
║         |                                                                     ║
║         v                                                                     ║
║     FEAT-027 (Client Connection Registry)                                     ║
║         |                                                                     ║
║         +------------------+------------------+                               ║
║         |                  |                  |                               ║
║         v                  v                  v                               ║
║     FEAT-022           FEAT-024          FEAT-023                             ║
║     (Message           (Session          (PTY Output                          ║
║     Routing)           Select UI)        Broadcasting)                        ║
║         |                                    |                                ║
║         +------------------+-----------------+                                ║
║                            |                                                  ║
║                            v                                                  ║
║                        FEAT-025                                               ║
║                        (Pane Output                                           ║
║                        Rendering)                                             ║
║                            |                                                  ║
║                            v                                                  ║
║                        FEAT-026                                               ║
║                        (Input Testing)                                        ║
║                                                                               ║
╚═══════════════════════════════════════════════════════════════════════════════╝
```

---

## Wave 0: Foundation ✅ COMPLETED

| ID | Title | Component | Tests |
|----|-------|-----------|-------|
| FEAT-007 | Protocol Layer - IPC Message Types and Codec | fugue-protocol | 86 |
| FEAT-008 | Utilities - Error Types, Logging, Path Helpers | fugue-utils | 108 |
| FEAT-011 | Client Connection - Unix Socket Client | fugue-client | 31 |
| FEAT-012 | Session Management - Session/Window/Pane Hierarchy | fugue-server | 88 |
| FEAT-013 | PTY Management - Process Spawning and Lifecycle | fugue-server | 17 |
| FEAT-017 | Configuration - TOML Config with Hot Reload | fugue-server | 38 |

**Status**: ✅ All 6 features completed (368 tests).

---

## Wave 1: Core Implementation ✅ COMPLETED

| ID | Title | Component | Tests |
|----|-------|-----------|-------|
| FEAT-001 | vt100 Parser Integration | session/pane | 23 |
| FEAT-002 | Per-Session Scrollback Config | config | 47 |
| FEAT-003 | Viewport Pinning | tui | 23 |
| FEAT-004a | Worktree Detection | orchestration | 12 |
| FEAT-004b | Session-Worktree Binding | orchestration | 8 |
| FEAT-004c | Cross-Session Messaging | orchestration | 45 |
| FEAT-005 | Response Channel | orchestration | 72 |
| FEAT-006 | Per-Session Log Levels | logging | 40 |
| FEAT-009 | Client UI | fugue-client | 97 |
| FEAT-016 | Persistence | fugue-server | 85 |

**Status**: ✅ All 10 features completed (452 tests).

> Note: FEAT-004 was decomposed into 004a/b/c. FEAT-014 merged into FEAT-001.

---

## Wave 2: Dependent Features ✅ COMPLETED

| ID | Title | Component | Tests |
|----|-------|-----------|-------|
| FEAT-010 | Client Input - Keyboard and Mouse Handling | fugue-client | 87 |
| FEAT-015 | Claude Detection - State from PTY Output | fugue-server | 45 |
| FEAT-019 | Sideband Protocol - XML Command Parsing | fugue-server | 92 |

**Status**: ✅ All 3 features completed (224 tests).

---

## Wave 3: Final Integration ✅ COMPLETED

| ID | Title | Component | Tests |
|----|-------|-----------|-------|
| FEAT-018 | MCP Server - Model Context Protocol | fugue-server | 32 |
| FEAT-020 | Session Isolation - Per-Pane CLAUDE_CONFIG_DIR | fugue-server | 17 |

**Status**: ✅ All 2 features completed (49 tests).

---

## Wave 4: Client-Server Integration 🚧 IN PROGRESS

All component features are complete (Waves 0-3). Wave 4 wires them together into a working application.

### Critical Path

```
FEAT-021 → FEAT-027 → FEAT-022 → FEAT-023 → FEAT-025 → FEAT-026
                    ↘ FEAT-024 (parallel)
```

### Feature List

| ID | Title | Component | Priority | Effort | Dependencies |
|----|-------|-----------|----------|--------|--------------|
| FEAT-021 | Server Socket Listen Loop | fugue-server | P0 | large (4-6h) | None |
| FEAT-027 | Client Connection Registry | fugue-server | P0 | small (1-2h) | FEAT-021 |
| FEAT-022 | Client Message Routing | fugue-server | P0 | large (6-8h) | FEAT-027 |
| FEAT-023 | PTY Output Broadcasting | fugue-server | P0 | medium (2-3h) | FEAT-027 |
| FEAT-024 | Session Selection UI | fugue-client | P1 | small (2h) | FEAT-022 |
| FEAT-025 | Pane Output Rendering | fugue-client | P0 | medium (3-4h) | FEAT-022, FEAT-023 |
| FEAT-026 | Input Testing | fugue-client | P1 | small (1-2h) | FEAT-025 |

### Wave 4 Summary

- **Total Features**: 7
- **Estimated Effort**: 20-27 hours
- **P0 Features**: 5 (FEAT-021, 022, 023, 025, 027)
- **P1 Features**: 2 (FEAT-024, 026)
- **Maximum Parallelism**: 3 (after FEAT-027: 022, 023, 024 can run in parallel)

### Recommended Wave 4 Worktree Assignments

| Phase | Worktree | Feature | Rationale |
|-------|----------|---------|-----------|
| 1 | wt-socket | FEAT-021 (Socket) | Unblocks everything |
| 2 | wt-registry | FEAT-027 (Registry) | Small, unblocks routing |
| 3a | wt-routing | FEAT-022 (Routing) | Core server logic |
| 3b | wt-output | FEAT-023 (Output) | Can parallel with 022 |
| 3c | wt-session-ui | FEAT-024 (Session UI) | Can parallel with 022/023 |
| 4 | wt-render | FEAT-025 (Rendering) | Needs 022+023 |
| 5 | wt-input-test | FEAT-026 (Testing) | Final verification |

### What Wave 4 Enables

After Wave 4 completion, fugue will be a **fully functional terminal multiplexer**:
- Start server daemon
- Connect client
- Create/attach sessions
- See shell output in panes
- Type commands
- Multiple concurrent clients

---

## Critical Path Analysis

### Waves 0-3: ✅ COMPLETED

All component features implemented with 1,093 tests passing.

### Wave 4 Critical Path

```
FEAT-021 (4-6h) → FEAT-027 (1-2h) → FEAT-022 (6-8h) → FEAT-025 (3-4h) → FEAT-026 (1-2h)
                                  ↘ FEAT-023 (2-3h) ↗
                                  ↘ FEAT-024 (2h) [parallel]
```

**Minimum Sequential Time**: ~18-24 hours (critical path)
**With Parallelism**: ~15-20 hours (022/023/024 in parallel)

---

## Execution Summary

| Wave | Features | Status | Tests | Effort |
|------|----------|--------|-------|--------|
| 0 | 6 | ✅ Done | 368 | Foundation |
| 1 | 10 | ✅ Done | 452 | Core Implementation |
| 2 | 3 | ✅ Done | 224 | Dependent Features |
| 3 | 2 | ✅ Done | 49 | Final Integration |
| 4 | 7 | 🚧 Pending | - | Client-Server Integration |
| **Total** | **27** | **20 done** | **1,093** | **~20-27h remaining** |

---

## Machine-Readable Format

For consumption by worktree orchestration (FEAT-004):

```json
{
  "waves": {
    "0": {
      "status": "completed",
      "features": ["FEAT-007", "FEAT-008", "FEAT-011", "FEAT-012", "FEAT-013", "FEAT-017"],
      "tests": 368
    },
    "1": {
      "status": "completed",
      "features": ["FEAT-001", "FEAT-002", "FEAT-003", "FEAT-004a", "FEAT-004b", "FEAT-004c", "FEAT-005", "FEAT-006", "FEAT-009", "FEAT-016"],
      "tests": 452
    },
    "2": {
      "status": "completed",
      "features": ["FEAT-010", "FEAT-015", "FEAT-019"],
      "tests": 224
    },
    "3": {
      "status": "completed",
      "features": ["FEAT-018", "FEAT-020"],
      "tests": 49
    },
    "4": {
      "status": "pending",
      "features": ["FEAT-021", "FEAT-022", "FEAT-023", "FEAT-024", "FEAT-025", "FEAT-026", "FEAT-027"],
      "parallel": true,
      "blocking": {
        "FEAT-021": [],
        "FEAT-027": ["FEAT-021"],
        "FEAT-022": ["FEAT-027"],
        "FEAT-023": ["FEAT-027"],
        "FEAT-024": ["FEAT-022"],
        "FEAT-025": ["FEAT-022", "FEAT-023"],
        "FEAT-026": ["FEAT-025"]
      }
    }
  },
  "critical_path": ["FEAT-021", "FEAT-027", "FEAT-022", "FEAT-025", "FEAT-026"],
  "p0_priority_order": [
    "FEAT-021", "FEAT-027", "FEAT-022", "FEAT-023", "FEAT-025"
  ],
  "total_tests": 1093
}
```

---

## Notes for Orchestration System

1. **Waves 0-3 Complete**: All 20 component features are implemented and tested.

2. **Wave 4 Focus**: Integration features wire existing components together.

3. **P0 Prioritization**: Wave 4 has 5 P0 features that form the critical path for MVP.

4. **Critical Path Focus**: FEAT-021 → FEAT-027 → FEAT-022 → FEAT-025 is the critical path.

5. **Parallel Opportunities**: After FEAT-027, features 022/023/024 can run in parallel.

6. **Test Requirements**: Each feature merge should trigger:
   - Unit tests for the feature
   - Integration tests with completed dependencies
   - End-to-end smoke test (after FEAT-025)

7. **Worktree Naming Convention**: `fugue-wt-{name}` (e.g., `fugue-wt-socket`)
