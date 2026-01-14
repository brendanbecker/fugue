# Feature Tracking

**Project**: ccmux
**Last Updated**: 2026-01-13

## Summary Statistics

- **Total Features**: 73
- **Completed**: 58
- **Backlog**: 15

## Current Status

Core terminal multiplexer features complete. Thirteen features remain in backlog.

**Recent Completions (2026-01-11)**:
- FEAT-060: MCP daemon auto-recovery (connection monitoring, reconnection, structured errors)
- FEAT-048: MCP orchestration protocol tools (tag-based agent communication)
- FEAT-057: Beads passive awareness (auto-detect .beads/, env var injection)
- FEAT-028: Tag-based routing (replaces binary orchestrator/worker model)
- FEAT-036: Session-aware MCP commands (auto-defaults, pane/window naming)
- FEAT-050: Session metadata storage (arbitrary key-value metadata)
- FEAT-056: User priority lockout for MCP focus control

**Recent Additions (2026-01-13)**:
- FEAT-075: Snapshot + replay resync API (event retention)
- FEAT-074: Observability instrumentation (metrics, tracing, status)
- FEAT-071: Per-pane Claude configuration on spawn (model selection, context limits, presets)
- FEAT-070: gastown remote pane support via CCMUX_ADDR (enables hybrid Mayor/polecat orchestration)
- FEAT-069: TLS/auth for direct TCP connections (Phase 4 of remote peering - future enhancement)
- FEAT-068: SSH tunnel integration and documentation (Phase 3 of remote peering - MVP completion)
- FEAT-067: Client TCP connection support (Phase 2 of remote peering support)
- FEAT-066: TCP listener support in daemon (Phase 1 of remote peering support)
- FEAT-064: Refactor MCP bridge.rs into modular components (maintainability)
- FEAT-063: Add file-based logging to MCP bridge mode (unblocks BUG-039)

**Recent Additions (2026-01-11)**:
- FEAT-062: Mirror pane (picture-in-picture view) for multi-agent monitoring
- FEAT-061: Screen redraw command to fix display corruption

### Backlog Features

| ID | Title | Component | Priority | Status | Notes |
|----|-------|-----------|----------|--------|-------|
| FEAT-075 | Snapshot + replay resync API (event retention) | ccmux-server, ccmux-protocol, ccmux-client | P2 | **Ready** | Resync after gaps via snapshot fallback + event replay. |
| FEAT-063 | Add file-based logging to MCP bridge mode | ccmux-server | P1 | **Ready** | Enable file logging for mcp-bridge to debug BUG-039. Small change. |
| FEAT-061 | Screen Redraw Command | ccmux-client | P2 | **Ready** | Keybind to force full screen redraw for display corruption recovery. |
| FEAT-074 | Observability instrumentation (metrics, tracing, status) | ccmux-server | P2 | **Ready** | Structured logging, tracing, metrics, and status fields per OBSERVABILITY.md. |
| FEAT-073 | Visibility dashboard (stuck detection, mailbox, graph pane) | ccmux-client | P2 | **Ready** | Terminal-native visibility dashboard for multi-agent oversight. |
| FEAT-071 | Per-pane Claude configuration on spawn | ccmux-server, ccmux-protocol | P2 | **Ready** | Configure model, context limits per pane. Blocks FEAT-072. |
| FEAT-066 | TCP listener support in daemon | ccmux-server | P2 | **Ready** | Optional TCP listener for remote workflows. Phase 1 of remote peering. Blocks FEAT-067. |
| FEAT-067 | Client TCP connection support | ccmux-client | P2 | **Ready** | TCP client connection capability. Phase 2 of remote peering. Blocks FEAT-068. |
| FEAT-068 | SSH tunnel integration and documentation | documentation | P2 | **Ready** | Document and validate SSH tunnel workflow. Phase 3 of remote peering (MVP completion). |
| FEAT-070 | gastown remote pane support via CCMUX_ADDR | external (gastown fork) | P2 | **Ready** | Extend gastown to support remote pane execution via ccmux TCP connections. Enables hybrid Mayor/polecat workflows. |
| FEAT-064 | Refactor MCP bridge.rs into modular components | ccmux-server | P2 | **Ready** | Extract connection, health, and tool modules from 33k+ token bridge.rs. |
| FEAT-065 | Refactor handlers in MCP bridge modules | ccmux-server | P2 | **Ready** | Continue modularization of MCP bridge after FEAT-064. |
| FEAT-062 | Mirror Pane (Picture-in-Picture View) | ccmux-server, ccmux-client | P3 | **Ready** | Read-only pane that mirrors another pane's output for multi-agent monitoring. |
| FEAT-069 | TLS/auth for direct TCP connections | ccmux-server, ccmux-client | P3 | **Ready** | Native TLS and authentication for direct TCP. Phase 4 of remote peering (future enhancement). |
| FEAT-058 | Beads Query Integration | ccmux-server, ccmux-client | P3 | **Ready** | TUI visibility into work queue. |
| FEAT-059 | Beads Workflow Integration | ccmux-server, ccmux-protocol | P3 | Blocked | Depends on FEAT-058. Scope reduced by FEAT-050. |

### Backlog Dependencies

```
FEAT-063 (Ready)
  |-- No dependencies
  |-- Blocks: BUG-039 investigation

FEAT-061 (Ready)
  |-- No dependencies

FEAT-071 (Ready)
  |-- FEAT-020 (complete) - Per-pane config directories
  |-- Blocks: FEAT-072 (Per-pane MCP mode control)

FEAT-066 (Ready)
  |-- No dependencies
  |-- Blocks: FEAT-067 (Client TCP connection support)

FEAT-067 (Ready)
  |-- FEAT-066 (complete or ready)
  |-- Blocks: FEAT-068 (SSH tunnel documentation)

FEAT-068 (Ready)
  |-- FEAT-066 (TCP listener in daemon)
  |-- FEAT-067 (TCP connection in client)
  |-- Completes: Remote peering MVP

FEAT-070 (Ready)
  |-- FEAT-066 (TCP listener in daemon)
  |-- FEAT-067 (TCP connection in client)
  |-- FEAT-068 (SSH tunnel documentation)
  |-- Enables: Remote gastown workflows

FEAT-069 (Ready)
  |-- FEAT-066 (TCP listener in daemon)
  |-- FEAT-067 (TCP connection in client)
  |-- FEAT-068 (SSH tunnel baseline)
  |-- Phase 4: Future enhancement (optional)

FEAT-064 (Ready)
  |-- No dependencies

FEAT-065 (Ready)
  |-- No dependencies (can run in parallel with FEAT-064)

FEAT-062 (Ready)
  |-- No dependencies

FEAT-058 (Ready)
  |-- FEAT-057 (complete) - Beads detection
  |-- FEAT-050 (complete) - Metadata caching
  |-- FEAT-028 (complete) - Orchestration notifications

FEAT-059 (Blocked)
  |-- FEAT-058 (not started) - Daemon communication
  |-- FEAT-057 (complete) - Beads detection
  |-- FEAT-050 (complete) - Issue tracking infrastructure
```

### Implementation Priority

1. **FEAT-063** - P1, ready now (no dependencies, unblocks BUG-039 debugging)
2. **FEAT-061** - P2, ready now (no dependencies, improves UX)
3. **FEAT-071** - P2, ready now (FEAT-020 complete, enables heterogeneous agent configs)
4. **FEAT-066** - P2, ready now (no dependencies, Phase 1 for remote workflows)
5. **FEAT-067** - P2, after FEAT-066 (Phase 2 for remote workflows)
6. **FEAT-068** - P2, after FEAT-066+067 (completes remote MVP)
7. **FEAT-070** - P2, after FEAT-066+067+068 (enables remote gastown workflows)
8. **FEAT-064** - P2, ready now (no dependencies, improves maintainability)
9. **FEAT-065** - P2, ready now (parallel with FEAT-064)
10. **FEAT-062** - P3, ready now (no dependencies, improves multi-agent workflows)
11. **FEAT-058** - P3, ready now (all dependencies complete)
12. **FEAT-059** - P3, after FEAT-058
13. **FEAT-069** - P3, after FEAT-066+067+068 (optional future enhancement)

## Completed Features

All completed features are in `feature-management/completed/`. Here's a summary by wave:

### Wave 0: Foundation (6 features)
| ID | Title | Status |
|----|-------|--------|
| FEAT-007 | Protocol Layer - IPC Message Types and Codec | Complete |
| FEAT-008 | Utilities - Error Types, Logging, Path Helpers | Complete |
| FEAT-011 | Client Connection - Unix Socket Client | Complete |
| FEAT-012 | Session Management - Session/Window/Pane Hierarchy | Complete |
| FEAT-013 | PTY Management - Process Spawning and Lifecycle | Complete |
| FEAT-017 | Configuration - TOML Config with Hot Reload | Complete |

### Wave 1: Core Components (9 features)
| ID | Title | Status |
|----|-------|--------|
| FEAT-009 | Client UI - Ratatui Terminal Interface | Complete |
| FEAT-014 | Terminal Parsing - ANSI/VT100 State Machine | Complete |
| FEAT-016 | Persistence - Checkpoint and WAL for Crash Recovery | Complete |
| FEAT-001 | Pane Content Abstraction (Terminal vs Canvas) | Complete |
| FEAT-002 | Per-Session-Type Scrollback Configuration | Complete |
| FEAT-003 | Viewport Pinning with New Content Indicator | Complete |
| FEAT-005 | Response Channel for Orchestrator-Worker | Complete |
| FEAT-006 | Per-Session Log Levels and Storage | Complete |
| FEAT-004 | Worktree-Aware Orchestration | Complete |

### Wave 2: Input & Detection (3 features)
| ID | Title | Status |
|----|-------|--------|
| FEAT-010 | Client Input - Keyboard and Mouse Event Handling | Complete |
| FEAT-015 | Claude Detection - State Detection from PTY Output | Complete |
| FEAT-019 | Sideband Protocol - XML Command Parsing | Complete |

### Wave 3: Integration (2 features)
| ID | Title | Status |
|----|-------|--------|
| FEAT-018 | MCP Server - Model Context Protocol Integration | Complete |
| FEAT-020 | Session Isolation - Per-Pane CLAUDE_CONFIG_DIR | Complete |

### Wave 4: Client-Server Integration (7 features)
| ID | Title | Status |
|----|-------|--------|
| FEAT-021 | Server Socket Listen Loop | Complete |
| FEAT-022 | Client Message Routing and Handlers | Complete |
| FEAT-023 | PTY Output Polling and Broadcasting | Complete |
| FEAT-024 | Session Selection UI | Complete |
| FEAT-025 | Pane Output Rendering | Complete |
| FEAT-026 | Input Handling Integration and Testing | Complete |
| FEAT-027 | Client Connection Registry | Complete |

### Post-MVP Features (27 features)
| ID | Title | Status |
|----|-------|--------|
| FEAT-029 | MCP Natural Language Terminal Control | Complete |
| FEAT-030 | Sideband Pane Splitting | Complete |
| FEAT-031 | Session Delete Keybind (Ctrl+D) | Complete |
| FEAT-032 | Integrated MCP Server | Complete |
| FEAT-033 | tmux-like Auto-Start Behavior | Complete |
| FEAT-034 | Mouse Scroll Support | Complete |
| FEAT-035 | Configurable Tab/Pane Switching | Complete |
| FEAT-037 | tmux-Compatible Keybinds | Complete |
| FEAT-038 | Split Pane Rendering | Complete |
| FEAT-039 | MCP Pane Creation Broadcast | Complete |
| FEAT-040 | MCP Pane Reliability Improvements | Complete |
| FEAT-041 | MCP Session/Window Targeting | Complete |
| FEAT-042 | MCP Debug Logging | Complete |
| FEAT-043 | MCP Session Rename Tool | Complete |
| FEAT-044 | Claude Session Persistence & Auto-Resume | Complete |
| FEAT-045 | MCP Declarative Layout Tools | Complete |
| FEAT-046 | MCP Focus/Select Control | Complete |
| FEAT-047 | ccmux_set_environment MCP tool | Complete |
| FEAT-048 | MCP Orchestration Protocol Tools | Complete |
| FEAT-049 | tmux-compatible CLI wrapper (ccmux-compat) | Complete |
| FEAT-051 | ccmux_get_environment MCP tool | Complete |
| FEAT-052 | ccmux_kill_session MCP tool | Complete |
| FEAT-053 | Auto-inject CCMUX Context Environment Variables | Complete |
| FEAT-054 | Pane-bounded text selection in copy mode | Complete |
| FEAT-055 | Full tmux keybinding parity in TUI | Complete |
| FEAT-056 | User Priority Lockout for MCP Focus Control | Complete |
| FEAT-057 | Beads Passive Awareness | Complete |
| FEAT-060 | MCP Daemon Auto-Recovery | Complete |
| FEAT-028 | Orchestration Flexibility Refactor (Tag-based Routing) | Complete |
| FEAT-036 | Session-Aware MCP Commands | Complete |
| FEAT-050 | Session Metadata Storage | Complete |

## Test Coverage

| Wave | Features | Tests |
|------|----------|-------|
| Wave 0 | 6 | 368 |
| Wave 1 | 9 | 452 |
| Wave 2 | 3 | 224 |
| Wave 3 | 2 | 49 |
| Wave 4 | 7 | 126 |
| Post-MVP | 31 | 347 |
| **Total** | **58** | **1,566** |

## Architecture

See [WAVES.md](/WAVES.md) for the complete parallel development plan with dependency analysis.

The project follows a multi-crate workspace structure:
- `ccmux-client/` - TUI client (ratatui + crossterm)
- `ccmux-server/` - Daemon with PTY management + MCP bridge
- `ccmux-protocol/` - Message types and codec (bincode)
- `ccmux-session/` - Session/window/pane hierarchy
- `ccmux-utils/` - Shared utilities
- `ccmux-persistence/` - WAL-based crash recovery
