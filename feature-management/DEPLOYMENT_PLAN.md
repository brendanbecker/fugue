# fugue Deployment Plan - Comprehensive Work Item Analysis

**Generated**: 2026-01-10
**Existing Worktrees**: bug-010, bug-011, bug-013, feat-043, bug-009

---

## Executive Summary

- **Total Active Items**: 41 (33 features + 8 bugs)
- **Completed Items**: 8 (6 features + 2 bugs completed, 1 deprecated)
- **Remaining Items**: 33 (27 features + 6 bugs)
- **Items with Existing Worktrees**: 5

---

## Work Item Status Overview

### Completed (Do NOT create worktrees)

| ID | Title | Status |
|----|-------|--------|
| FEAT-007 | Protocol Layer - IPC Message Types and Codec | completed |
| FEAT-008 | Utilities - Error Types, Logging, Path Helpers | completed |
| FEAT-011 | Client Connection - Unix Socket Client | completed |
| FEAT-012 | Session Management - Session/Window/Pane Hierarchy | completed |
| FEAT-013 | PTY Management - Process Spawning and Lifecycle | completed |
| FEAT-017 | Configuration - TOML Config with Hot Reload | completed |
| FEAT-029 | MCP Natural Language Terminal Control | implemented |
| FEAT-038 | Split Pane Rendering - Layout Manager | implemented |
| BUG-002 | Flaky test ensure_dir_nested | resolved |
| BUG-004 | Client hangs when reattaching to session with dead pane | resolved |
| BUG-005 | Sideband parsing not integrated into PTY output flow | resolved |
| BUG-006 | Viewport not sizing to terminal dimensions | resolved |
| BUG-007 | Shift+Tab not passed through to PTY | resolved |
| BUG-012 | Shift+click text selection | deprecated |

### Items with Existing Worktrees (Already in progress)

| ID | Title | Worktree | Priority |
|----|-------|----------|----------|
| BUG-009 | Flaky persistence tests | bug-009 | P2 |
| BUG-010 | MCP pane broadcast not received by TUI | bug-010 | P1 |
| BUG-011 | Large paste crashes session | bug-011 | P2 |
| BUG-013 | Mouse scroll wheel not working | bug-013 | P2 |
| FEAT-043 | MCP Session Rename Tool | feat-043 | P1 |

---

## Remaining Active Work Items (Need Worktrees)

### P0 - Critical (5 items)

| ID | Title | Component | Dependencies | Notes |
|----|-------|-----------|--------------|-------|
| FEAT-021 | Server Socket Listen Loop | fugue-server | None | Foundational - unblocks everything |
| FEAT-022 | Client Message Routing and Handlers | fugue-server | FEAT-021 | Core message handling |
| FEAT-023 | PTY Output Polling and Broadcasting | fugue-server | FEAT-021, FEAT-022 | Real-time output |
| FEAT-025 | Pane Output Rendering | fugue-client | FEAT-022, FEAT-023 | Display output |
| FEAT-027 | Client Connection Registry | fugue-server | FEAT-021 | Tracks clients per session |

### P1 - High Priority (20 items)

| ID | Title | Component | Dependencies | Notes |
|----|-------|-----------|--------------|-------|
| FEAT-001 | Pane Content Abstraction | session/pane | None | Terminal vs Canvas |
| FEAT-002 | Per-Session-Type Scrollback | config | None | Memory management |
| FEAT-005 | Response Channel Orchestrator-Worker | orchestration | FEAT-028 | Cross-session messaging |
| FEAT-009 | Client UI - Ratatui Terminal Interface | fugue-client | FEAT-007 | TUI framework |
| FEAT-010 | Client Input - Keyboard/Mouse | fugue-client | FEAT-009 | Input handling |
| FEAT-014 | Terminal Parsing - ANSI/VT100 | fugue-server | FEAT-013 | VT100 emulation |
| FEAT-015 | Claude Detection - State from PTY | fugue-server | FEAT-014 | AI state tracking |
| FEAT-020 | Session Isolation - Per-Pane Config | fugue-server | FEAT-013, FEAT-015 | Claude config isolation |
| FEAT-024 | Session Selection UI | fugue-client | FEAT-021 | Session picker |
| FEAT-026 | Input Handling Integration | fugue-client | FEAT-022, FEAT-023 | End-to-end testing |
| FEAT-028 | Orchestration Flexibility Refactor | fugue-protocol | FEAT-007 | Generic primitives |
| FEAT-030 | Sideband Pane Splitting | fugue-server | FEAT-019, FEAT-012, FEAT-013 | XML spawn commands |
| FEAT-032 | Integrated MCP Server | fugue-server | FEAT-018, FEAT-029 | Shared session state |
| FEAT-033 | tmux-like Auto-Start Behavior | fugue-client | FEAT-011 | Seamless UX |
| FEAT-036 | Session-aware MCP Commands | fugue-server (MCP) | FEAT-029 | Smart defaults |
| FEAT-039 | MCP Pane Creation Broadcast | fugue-server | FEAT-038 | TUI client sync |
| FEAT-044 | Claude Session Persistence | fugue-server | FEAT-016 | Auto-resume |
| FEAT-045 | MCP Declarative Layout Tools | fugue-server (MCP) | FEAT-029, FEAT-039 | Complex layouts |
| BUG-010 | MCP pane broadcast not received | fugue-server | None | **Has worktree** |
| BUG-014 | Large output buffer overflow | terminal-buffer | None | Input unresponsive |

### P2 - Medium Priority (8 items)

| ID | Title | Component | Dependencies | Notes |
|----|-------|-----------|--------------|-------|
| FEAT-003 | Viewport Pinning with Indicator | tui | None | New content indicator |
| FEAT-004 | Worktree-Aware Orchestration | orchestration | FEAT-028 | Parallel development |
| FEAT-006 | Per-Session Log Levels | logging | None | Fine-grained logging |
| FEAT-016 | Persistence - Checkpoint and WAL | fugue-server | FEAT-012 | Crash recovery |
| FEAT-018 | MCP Server Integration | fugue-server | FEAT-012, FEAT-015 | Claude interaction |
| FEAT-019 | Sideband Protocol - XML Parsing | fugue-server | FEAT-014 | Command parsing |
| FEAT-031 | Session Delete/Kill Keybind | fugue-client | FEAT-024, FEAT-012 | Session cleanup |
| FEAT-034 | Mouse Scroll Support | fugue-client | FEAT-010 | Scrollback navigation |
| FEAT-035 | Configurable Tab/Pane Switching | fugue-client | FEAT-010, FEAT-017 | Quick navigation |
| BUG-009 | Flaky persistence tests | fugue-server | None | **Has worktree** |
| BUG-011 | Large paste crashes session | fugue-client | None | **Has worktree** |
| BUG-013 | Mouse scroll wheel not working | fugue-client | None | **Has worktree** |
| BUG-015 | Layout not recalculated on pane close | fugue-client | None | Layout recalculation |

---

## Dependency Graph

```
                    ┌─────────────────────────────────────────┐
                    │           FOUNDATION (Wave 0)            │
                    │         FEAT-007, FEAT-008, FEAT-011     │
                    │         FEAT-012, FEAT-013, FEAT-017     │
                    │              (ALL COMPLETED)              │
                    └─────────────────────────────────────────┘
                                        │
          ┌─────────────────────────────┼─────────────────────────────┐
          │                             │                             │
          ▼                             ▼                             ▼
┌───────────────────┐      ┌───────────────────────┐      ┌───────────────────┐
│   FEAT-021 (P0)   │      │    FEAT-028 (P1)      │      │  Independent (P1) │
│ Server Socket     │      │ Orchestration Refactor│      │   FEAT-001        │
│                   │      │                       │      │   FEAT-002        │
└─────────┬─────────┘      └───────────┬───────────┘      │   FEAT-003        │
          │                            │                   │   FEAT-006        │
    ┌─────┴─────┐                ┌─────┴─────┐            └───────────────────┘
    │           │                │           │
    ▼           ▼                ▼           ▼
┌───────┐   ┌───────┐      ┌───────┐   ┌───────┐
│FEAT-022│   │FEAT-027│      │FEAT-004│   │FEAT-005│
│Msg Rout│   │Registry│      │Worktree│   │RespChan│
└───┬───┘   └───┬───┘      └───────┘   └───────┘
    │           │
    └─────┬─────┘
          │
          ▼
┌───────────────────┐
│   FEAT-023 (P0)   │
│ PTY Output Poll   │
└─────────┬─────────┘
          │
    ┌─────┴─────┐
    │           │
    ▼           ▼
┌───────┐   ┌───────┐
│FEAT-025│   │FEAT-026│
│Output  │   │Input   │
│Render  │   │Test    │
└───────┘   └───────┘

Terminal Parsing Chain:
┌───────────────────┐
│   FEAT-014 (P1)   │
│ ANSI/VT100 Parse  │
└─────────┬─────────┘
          │
    ┌─────┴─────┐
    │           │
    ▼           ▼
┌───────┐   ┌───────┐
│FEAT-015│   │FEAT-019│
│Claude  │   │Sideband│
│Detect  │   │XML     │
└───┬───┘   └───┬───┘
    │           │
    ▼           │
┌───────┐       │
│FEAT-020│       │
│Session │       │
│Isolate │       │
└───────┘       │
                │
                ▼
          ┌───────────┐
          │ FEAT-030  │
          │ Sideband  │
          │ Splitting │
          └───────────┘

MCP Chain:
┌───────────────────┐
│   FEAT-029 (Done) │
│ MCP Natural Lang  │
└─────────┬─────────┘
          │
    ┌─────┼─────┬─────────┐
    │     │     │         │
    ▼     ▼     ▼         ▼
┌─────┐┌─────┐┌─────┐  ┌─────┐
│F-032││F-036││F-039│  │F-045│
│Integ││Sess ││Bcast│  │Decl │
│MCP  ││Aware││     │  │Layou│
└─────┘└─────┘└─────┘  └─────┘

Persistence Chain:
┌───────────────────┐
│   FEAT-016 (P2)   │
│ WAL/Checkpoint    │
└─────────┬─────────┘
          │
          ▼
┌───────────────────┐
│   FEAT-044 (P1)   │
│ Claude Session    │
│ Persistence       │
└───────────────────┘
```

---

## Deployment Waves

### Wave 1 - Foundation Server (Can Start Immediately)

**Items with NO dependencies on incomplete work:**

| ID | Title | Branch Name | Worktree Path | Est. Effort |
|----|-------|-------------|---------------|-------------|
| FEAT-021 | Server Socket Listen Loop | feat-021-server-socket | ~/projects/tools/fugue-worktrees/feat-021 | large |
| FEAT-027 | Client Connection Registry | feat-027-client-registry | ~/projects/tools/fugue-worktrees/feat-027 | small |
| FEAT-028 | Orchestration Flexibility Refactor | feat-028-orchestration-refactor | ~/projects/tools/fugue-worktrees/feat-028 | medium |
| FEAT-001 | Pane Content Abstraction | feat-001-pane-content | ~/projects/tools/fugue-worktrees/feat-001 | large |
| FEAT-002 | Per-Session-Type Scrollback | feat-002-scrollback-config | ~/projects/tools/fugue-worktrees/feat-002 | medium |
| FEAT-003 | Viewport Pinning | feat-003-viewport-pinning | ~/projects/tools/fugue-worktrees/feat-003 | medium |
| FEAT-006 | Per-Session Log Levels | feat-006-session-logging | ~/projects/tools/fugue-worktrees/feat-006 | medium |
| FEAT-014 | Terminal Parsing ANSI/VT100 | feat-014-terminal-parsing | ~/projects/tools/fugue-worktrees/feat-014 | medium |
| FEAT-016 | Persistence WAL/Checkpoint | feat-016-persistence | ~/projects/tools/fugue-worktrees/feat-016 | large |
| FEAT-009 | Client UI Ratatui | feat-009-client-ui | ~/projects/tools/fugue-worktrees/feat-009 | large |
| FEAT-033 | tmux-like Auto-Start | feat-033-auto-start | ~/projects/tools/fugue-worktrees/feat-033 | small |
| BUG-014 | Large output buffer overflow | bug-014-buffer-overflow | ~/projects/tools/fugue-worktrees/bug-014 | medium |
| BUG-015 | Layout not recalculated on pane close | bug-015-layout-recalc | ~/projects/tools/fugue-worktrees/bug-015 | medium |

**Parallel Capacity**: All 13 items can run in parallel

### Wave 2 - Blocked on Wave 1

**Items that depend on Wave 1 completions:**

| ID | Title | Blocked By | Branch Name | Worktree Path |
|----|-------|------------|-------------|---------------|
| FEAT-022 | Client Message Routing | FEAT-021 | feat-022-message-routing | ~/projects/tools/fugue-worktrees/feat-022 |
| FEAT-015 | Claude Detection | FEAT-014 | feat-015-claude-detection | ~/projects/tools/fugue-worktrees/feat-015 |
| FEAT-019 | Sideband Protocol XML | FEAT-014 | feat-019-sideband-protocol | ~/projects/tools/fugue-worktrees/feat-019 |
| FEAT-010 | Client Input Keyboard/Mouse | FEAT-009 | feat-010-client-input | ~/projects/tools/fugue-worktrees/feat-010 |
| FEAT-004 | Worktree-Aware Orchestration | FEAT-028 | feat-004-worktree-orchestration | ~/projects/tools/fugue-worktrees/feat-004 |
| FEAT-005 | Response Channel Orchestrator | FEAT-028 | feat-005-response-channel | ~/projects/tools/fugue-worktrees/feat-005 |
| FEAT-024 | Session Selection UI | FEAT-021 | feat-024-session-selection | ~/projects/tools/fugue-worktrees/feat-024 |
| FEAT-036 | Session-aware MCP Commands | FEAT-029 | feat-036-session-mcp | ~/projects/tools/fugue-worktrees/feat-036 |
| FEAT-039 | MCP Pane Creation Broadcast | FEAT-038 | feat-039-mcp-broadcast | ~/projects/tools/fugue-worktrees/feat-039 |
| FEAT-044 | Claude Session Persistence | FEAT-016 | feat-044-claude-persistence | ~/projects/tools/fugue-worktrees/feat-044 |

### Wave 3 - Blocked on Wave 2

| ID | Title | Blocked By | Branch Name | Worktree Path |
|----|-------|------------|-------------|---------------|
| FEAT-023 | PTY Output Polling | FEAT-021, FEAT-022 | feat-023-pty-output | ~/projects/tools/fugue-worktrees/feat-023 |
| FEAT-018 | MCP Server Integration | FEAT-012, FEAT-015 | feat-018-mcp-server | ~/projects/tools/fugue-worktrees/feat-018 |
| FEAT-020 | Session Isolation | FEAT-013, FEAT-015 | feat-020-session-isolation | ~/projects/tools/fugue-worktrees/feat-020 |
| FEAT-030 | Sideband Pane Splitting | FEAT-019, FEAT-012, FEAT-013 | feat-030-sideband-split | ~/projects/tools/fugue-worktrees/feat-030 |
| FEAT-034 | Mouse Scroll Support | FEAT-010 | feat-034-mouse-scroll | ~/projects/tools/fugue-worktrees/feat-034 |
| FEAT-035 | Configurable Tab Switching | FEAT-010, FEAT-017 | feat-035-tab-switching | ~/projects/tools/fugue-worktrees/feat-035 |

### Wave 4 - Blocked on Wave 3

| ID | Title | Blocked By | Branch Name | Worktree Path |
|----|-------|------------|-------------|---------------|
| FEAT-025 | Pane Output Rendering | FEAT-022, FEAT-023 | feat-025-pane-output | ~/projects/tools/fugue-worktrees/feat-025 |
| FEAT-026 | Input Handling Integration | FEAT-022, FEAT-023 | feat-026-input-integration | ~/projects/tools/fugue-worktrees/feat-026 |
| FEAT-032 | Integrated MCP Server | FEAT-018, FEAT-029 | feat-032-integrated-mcp | ~/projects/tools/fugue-worktrees/feat-032 |
| FEAT-031 | Session Delete Keybind | FEAT-024, FEAT-012 | feat-031-session-delete | ~/projects/tools/fugue-worktrees/feat-031 |
| FEAT-045 | MCP Declarative Layout | FEAT-029, FEAT-039 | feat-045-mcp-layout | ~/projects/tools/fugue-worktrees/feat-045 |

---

## Critical Path Analysis

The critical path for core functionality is:

```
FEAT-021 (Server Socket) -> FEAT-027 (Registry) -> FEAT-022 (Message Routing)
-> FEAT-023 (PTY Output) -> FEAT-025 (Pane Rendering)
```

**Recommendation**: Prioritize Wave 1's FEAT-021 and FEAT-027 first as they unblock the most downstream work.

---

## Worktree Creation Commands

### Wave 1 - Create Immediately

```bash
# Navigate to fugue directory
cd /home/becker/projects/tools/fugue

# Create worktree base directory
mkdir -p ~/projects/tools/fugue-worktrees

# Critical P0 items (create these first)
git worktree add ~/projects/tools/fugue-worktrees/feat-021 -b feat-021-server-socket
git worktree add ~/projects/tools/fugue-worktrees/feat-027 -b feat-027-client-registry

# P1 items
git worktree add ~/projects/tools/fugue-worktrees/feat-028 -b feat-028-orchestration-refactor
git worktree add ~/projects/tools/fugue-worktrees/feat-001 -b feat-001-pane-content
git worktree add ~/projects/tools/fugue-worktrees/feat-002 -b feat-002-scrollback-config
git worktree add ~/projects/tools/fugue-worktrees/feat-014 -b feat-014-terminal-parsing
git worktree add ~/projects/tools/fugue-worktrees/feat-016 -b feat-016-persistence
git worktree add ~/projects/tools/fugue-worktrees/feat-009 -b feat-009-client-ui
git worktree add ~/projects/tools/fugue-worktrees/feat-033 -b feat-033-auto-start

# P2 items
git worktree add ~/projects/tools/fugue-worktrees/feat-003 -b feat-003-viewport-pinning
git worktree add ~/projects/tools/fugue-worktrees/feat-006 -b feat-006-session-logging

# Bugs (no dependencies)
git worktree add ~/projects/tools/fugue-worktrees/bug-014 -b bug-014-buffer-overflow
git worktree add ~/projects/tools/fugue-worktrees/bug-015 -b bug-015-layout-recalc
```

### Wave 2 - Create After Wave 1 Dependencies Complete

```bash
# After FEAT-021 completes
git worktree add ~/projects/tools/fugue-worktrees/feat-022 -b feat-022-message-routing
git worktree add ~/projects/tools/fugue-worktrees/feat-024 -b feat-024-session-selection

# After FEAT-014 completes
git worktree add ~/projects/tools/fugue-worktrees/feat-015 -b feat-015-claude-detection
git worktree add ~/projects/tools/fugue-worktrees/feat-019 -b feat-019-sideband-protocol

# After FEAT-009 completes
git worktree add ~/projects/tools/fugue-worktrees/feat-010 -b feat-010-client-input

# After FEAT-028 completes
git worktree add ~/projects/tools/fugue-worktrees/feat-004 -b feat-004-worktree-orchestration
git worktree add ~/projects/tools/fugue-worktrees/feat-005 -b feat-005-response-channel

# After FEAT-029 (already complete) + FEAT-038 (implemented)
git worktree add ~/projects/tools/fugue-worktrees/feat-036 -b feat-036-session-mcp
git worktree add ~/projects/tools/fugue-worktrees/feat-039 -b feat-039-mcp-broadcast

# After FEAT-016 completes
git worktree add ~/projects/tools/fugue-worktrees/feat-044 -b feat-044-claude-persistence
```

### Wave 3 - Create After Wave 2 Dependencies Complete

```bash
# After FEAT-021 + FEAT-022 complete
git worktree add ~/projects/tools/fugue-worktrees/feat-023 -b feat-023-pty-output

# After FEAT-015 completes
git worktree add ~/projects/tools/fugue-worktrees/feat-018 -b feat-018-mcp-server
git worktree add ~/projects/tools/fugue-worktrees/feat-020 -b feat-020-session-isolation

# After FEAT-019 completes
git worktree add ~/projects/tools/fugue-worktrees/feat-030 -b feat-030-sideband-split

# After FEAT-010 completes
git worktree add ~/projects/tools/fugue-worktrees/feat-034 -b feat-034-mouse-scroll
git worktree add ~/projects/tools/fugue-worktrees/feat-035 -b feat-035-tab-switching
```

### Wave 4 - Create After Wave 3 Dependencies Complete

```bash
# After FEAT-022 + FEAT-023 complete
git worktree add ~/projects/tools/fugue-worktrees/feat-025 -b feat-025-pane-output
git worktree add ~/projects/tools/fugue-worktrees/feat-026 -b feat-026-input-integration

# After FEAT-018 + FEAT-029 complete
git worktree add ~/projects/tools/fugue-worktrees/feat-032 -b feat-032-integrated-mcp

# After FEAT-024 complete
git worktree add ~/projects/tools/fugue-worktrees/feat-031 -b feat-031-session-delete

# After FEAT-039 completes
git worktree add ~/projects/tools/fugue-worktrees/feat-045 -b feat-045-mcp-layout
```

---

## Summary Table: All Worktrees to Create

| Wave | ID | Branch Name | Dependencies | Already Exists? |
|------|-------|-------------|--------------|-----------------|
| 1 | FEAT-021 | feat-021-server-socket | None | No |
| 1 | FEAT-027 | feat-027-client-registry | None | No |
| 1 | FEAT-028 | feat-028-orchestration-refactor | None | No |
| 1 | FEAT-001 | feat-001-pane-content | None | No |
| 1 | FEAT-002 | feat-002-scrollback-config | None | No |
| 1 | FEAT-003 | feat-003-viewport-pinning | None | No |
| 1 | FEAT-006 | feat-006-session-logging | None | No |
| 1 | FEAT-014 | feat-014-terminal-parsing | None | No |
| 1 | FEAT-016 | feat-016-persistence | None | No |
| 1 | FEAT-009 | feat-009-client-ui | None | No |
| 1 | FEAT-033 | feat-033-auto-start | None | No |
| 1 | BUG-014 | bug-014-buffer-overflow | None | No |
| 1 | BUG-015 | bug-015-layout-recalc | None | No |
| - | BUG-009 | bug-009 | None | **Yes** |
| - | BUG-010 | bug-010 | None | **Yes** |
| - | BUG-011 | bug-011 | None | **Yes** |
| - | BUG-013 | bug-013 | None | **Yes** |
| - | FEAT-043 | feat-043 | None | **Yes** |
| 2 | FEAT-022 | feat-022-message-routing | FEAT-021 | No |
| 2 | FEAT-024 | feat-024-session-selection | FEAT-021 | No |
| 2 | FEAT-015 | feat-015-claude-detection | FEAT-014 | No |
| 2 | FEAT-019 | feat-019-sideband-protocol | FEAT-014 | No |
| 2 | FEAT-010 | feat-010-client-input | FEAT-009 | No |
| 2 | FEAT-004 | feat-004-worktree-orchestration | FEAT-028 | No |
| 2 | FEAT-005 | feat-005-response-channel | FEAT-028 | No |
| 2 | FEAT-036 | feat-036-session-mcp | FEAT-029 | No |
| 2 | FEAT-039 | feat-039-mcp-broadcast | FEAT-038 | No |
| 2 | FEAT-044 | feat-044-claude-persistence | FEAT-016 | No |
| 3 | FEAT-023 | feat-023-pty-output | FEAT-021, FEAT-022 | No |
| 3 | FEAT-018 | feat-018-mcp-server | FEAT-012, FEAT-015 | No |
| 3 | FEAT-020 | feat-020-session-isolation | FEAT-013, FEAT-015 | No |
| 3 | FEAT-030 | feat-030-sideband-split | FEAT-019 | No |
| 3 | FEAT-034 | feat-034-mouse-scroll | FEAT-010 | No |
| 3 | FEAT-035 | feat-035-tab-switching | FEAT-010 | No |
| 4 | FEAT-025 | feat-025-pane-output | FEAT-022, FEAT-023 | No |
| 4 | FEAT-026 | feat-026-input-integration | FEAT-022, FEAT-023 | No |
| 4 | FEAT-032 | feat-032-integrated-mcp | FEAT-018, FEAT-029 | No |
| 4 | FEAT-031 | feat-031-session-delete | FEAT-024 | No |
| 4 | FEAT-045 | feat-045-mcp-layout | FEAT-029, FEAT-039 | No |

---

## Recommendations

### Immediate Actions (Wave 1)

1. **Create all Wave 1 worktrees** - 13 items can start in parallel immediately
2. **Prioritize FEAT-021 and FEAT-027** - These are the P0 critical path blockers
3. **Continue work on existing worktrees** - bug-009, bug-010, bug-011, bug-013, feat-043

### Priority Ordering within Wave 1

1. **FEAT-021** (Server Socket) - Unblocks everything
2. **FEAT-027** (Client Registry) - Required by FEAT-022 and FEAT-023
3. **FEAT-014** (Terminal Parsing) - Unblocks FEAT-015 and FEAT-019
4. **FEAT-009** (Client UI) - Unblocks FEAT-010
5. **FEAT-028** (Orchestration Refactor) - Unblocks FEAT-004 and FEAT-005
6. **FEAT-016** (Persistence) - Unblocks FEAT-044
7. **BUG-014** (Buffer Overflow) - P1 user-facing bug
8. Others can be worked in any order

### Worktree Path Convention

All worktrees follow the pattern:
```
~/projects/tools/fugue-worktrees/{item-id-slug}
```

Branch names follow:
```
{type}-{id}-{slug}
```
Where type is `feat`, `bug`, or `fix`.

---

## Notes

- Items marked as "completed" or "implemented" have working code in main
- Items with existing worktrees (bug-009, bug-010, bug-011, bug-013, feat-043) are already in progress
- Wave assignments assume sequential completion - if multiple items complete simultaneously, waves can be collapsed
- FEAT-029 and FEAT-038 are marked as "implemented" and can be treated as completed dependencies
