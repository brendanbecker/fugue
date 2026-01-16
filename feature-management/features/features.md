# Feature Tracking

**Project**: ccmux
**Last Updated**: 2026-01-15

## Summary Statistics

- **Total Features**: 82
- **Completed**: 72
- **Backlog**: 10

## Current Status

Core terminal multiplexer features complete. Ten features remain in backlog.

**Recent Completions (2026-01-15)**:
- FEAT-082 (aka 073): Multi-tier routing logic via target aliases
- FEAT-075: Snapshot + replay resync API (event retention)
- FEAT-070: gastown remote pane support via CCMUX_ADDR
- FEAT-068: SSH tunnel integration and documentation
- FEAT-067: Client TCP connection support
- FEAT-066: TCP listener support in daemon
- FEAT-071: Per-pane Claude configuration on spawn
- FEAT-080: Per-Pane/Session Configuration via Sideband Commands
- FEAT-081: Optional Landlock Integration for Per-Pane Sandboxing

**Recent Completions (2026-01-14)**:
- FEAT-079: Comprehensive Human-Control Arbitration
- FEAT-077: Human-control mode UX indicator and MCP error details

**Recent Completions (2026-01-13)**:
- FEAT-060: MCP daemon auto-recovery (connection monitoring, reconnection, structured errors)
- FEAT-048: MCP orchestration protocol tools (tag-based agent communication)
- FEAT-057: Beads passive awareness (auto-detect .beads/, env var injection)
- FEAT-028: Tag-based routing (replaces binary orchestrator/worker model)
- FEAT-036: Session-aware MCP commands (auto-defaults, pane/window naming)
- FEAT-050: Session metadata storage (arbitrary key-value metadata)
- FEAT-056: User priority lockout for MCP focus control

### Backlog Features

| ID | Title | Component | Priority | Status | Notes |
|----|-------|-----------|----------|--------|-------|
| FEAT-063 | Add file-based logging to MCP bridge mode | ccmux-server | P1 | **Ready** | Enable file logging for mcp-bridge to debug BUG-039. Small change. |
| FEAT-061 | Screen Redraw Command | ccmux-client | P2 | **Ready** | Keybind to force full screen redraw for display corruption recovery. |
| FEAT-074 | Observability instrumentation (metrics, tracing, status) | ccmux-server | P2 | **Ready** | Structured logging, tracing, metrics, and status fields per OBSERVABILITY.md. |
| FEAT-073 | Visibility dashboard (stuck detection, mailbox, graph pane) | ccmux-client | P2 | **Ready** | Terminal-native visibility dashboard with activity feed/intent log and mailbox. |
| FEAT-064 | Refactor MCP bridge.rs into modular components | ccmux-server | P2 | **Ready** | Extract connection, health, and tool modules from 33k+ token bridge.rs. |
| FEAT-065 | Refactor handlers in MCP bridge modules | ccmux-server | P2 | **Ready** | Continue modularization of MCP bridge after FEAT-064. |
| FEAT-062 | Mirror Pane (Picture-in-Picture View) | ccmux-server, ccmux-client | P3 | **Ready** | Read-only pane that mirrors another pane's output for multi-agent monitoring. |
| FEAT-076 | Capability signaling protocol (sideband metadata) | ccmux-server, ccmux-protocol | P3 | **Ready** | Explicit sideband capability signals stored as pane metadata. |
| FEAT-058 | Beads Query Integration | ccmux-server, ccmux-client | P3 | **Ready** | TUI visibility into work queue. |
| FEAT-059 | Beads Workflow Integration | ccmux-server, ccmux-protocol | P3 | Blocked | Depends on FEAT-058. Scope reduced by FEAT-050. |

### Backlog Dependencies

```
FEAT-063 (Ready)
  |-- No dependencies
  |-- Blocks: BUG-039 investigation

FEAT-061 (Ready)
  |-- No dependencies

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
3. **FEAT-064** - P2, ready now (no dependencies, improves maintainability)
4. **FEAT-065** - P2, ready now (parallel with FEAT-064)
5. **FEAT-062** - P3, ready now (no dependencies, improves multi-agent workflows)
6. **FEAT-058** - P3, ready now (all dependencies complete)
7. **FEAT-059** - P3, after FEAT-058

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

### Post-MVP Features (37 features)
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
| FEAT-079 | Comprehensive Human-Control Arbitration | Complete |
| FEAT-077 | Human-control mode UX indicator and MCP error details | Complete |
| FEAT-071 | Per-pane Claude configuration on spawn | Complete |
| FEAT-066 | TCP listener support in daemon | Complete |
| FEAT-067 | Client TCP connection support | Complete |
| FEAT-068 | SSH tunnel integration and documentation | Complete |
| FEAT-070 | gastown remote pane support via CCMUX_ADDR | Complete |
| FEAT-075 | Snapshot + replay resync API (event retention) | Complete |
| FEAT-080 | Per-Pane/Session Configuration via Sideband Commands | Complete |
| FEAT-081 | Optional Landlock Integration for Per-Pane Sandboxing | Complete |
| FEAT-082 | Multi-tier routing logic via target aliases | Complete |

## Test Coverage

| Wave | Features | Tests |
|------|----------|-------|
| Wave 0 | 6 | 368 |
| Wave 1 | 9 | 452 |
| Wave 2 | 3 | 224 |
| Wave 3 | 2 | 49 |
| Wave 4 | 7 | 126 |
| Post-MVP | 45 | 457 |
| **Total** | **72** | **1,676** |

## Architecture

See [WAVES.md](/WAVES.md) for the complete parallel development plan with dependency analysis.

The project follows a multi-crate workspace structure:
- `ccmux-client/` - TUI client (ratatui + crossterm)
- `ccmux-server/` - Daemon with PTY management + MCP bridge
- `ccmux-protocol/` - Message types and codec (bincode)
- `ccmux-session/` - Session/window/pane hierarchy
- `ccmux-utils/` - Shared utilities
- `ccmux-persistence/` - WAL-based crash recovery
- `ccmux-sandbox/` - Landlock sandboxing helper| FEAT-083 | Protocol Generalization: Generic Widget System | ccmux-protocol | P1 | new | [Link](features/FEAT-083-protocol-generalization-generic-widget-system/) |
| FEAT-084 | Protocol Generalization: Abstract Agent State | ccmux-protocol | P2 | new | [Link](features/FEAT-084-protocol-generalization-abstract-agent-state/) |
| FEAT-085 | ADR: The Dumb Pipe Strategy | docs | P1 | new | [Link](features/FEAT-085-adr-the-dumb-pipe-strategy/) |
