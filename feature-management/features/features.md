# Feature Tracking

**Project**: fugue
**Last Updated**: 2026-01-28

## Summary Statistics

- **Total Features**: 126
- **Completed**: 118
- **Backlog**: 8

## Current Status

Core terminal multiplexer fully functional with MCP integration, multi-agent orchestration, remote access, and observability. Agent detection supports Claude, Gemini, and Codex. Watchdog system implemented with named watchdogs, auto-clear, and orchestration skill. Mail system implemented with filesystem-based storage and MCP commands.

## Active Backlog

### Medium Priority (P2)

| ID | Title | Component | Status |
|----|-------|-----------|--------|
| FEAT-112 | Orchestrator Context Handoff | orchestration | new |
| FEAT-115 | Hierarchical Session List | mcp/session | new |

### Lower Priority (P3)

| ID | Title | Component | Status |
|----|-------|-----------|--------|
| FEAT-064 | Refactor MCP bridge.rs | fugue-server | ready |
| FEAT-065 | Refactor handlers in MCP bridge | fugue-server | ready |
| FEAT-087 | Refactor client app.rs | fugue-client | ready |
| FEAT-090 | Refactor server main.rs | fugue-server | ready |
| FEAT-091 | Refactor mcp_handlers.rs | fugue-server | ready |
| FEAT-113-116 | Web Interface Features | web | new |

## Recently Completed (2026-01-28)

| ID | Title | Commit |
|----|-------|--------|
| FEAT-126 | Watchdog Mail Checking | a1e3ca9 |
| FEAT-125 | MCP Mail Commands | 43896e6 |
| FEAT-124 | Mail Storage Format | 1de3624 |
| FEAT-114 | Named/Multiple Watchdogs | 01d290f |
| FEAT-111 | Watchdog Auto-Clear Cycle | 13c872e |
| FEAT-110 | Watchdog Monitor Agent | 3a9ddf0 |
| FEAT-104 | Watchdog Orchestration Skill | 5b0e86a |

## Completed Features (by Category)

### Mail System
| ID | Title | Commit |
|----|-------|--------|
| FEAT-126 | Watchdog Mail Checking | a1e3ca9 |
| FEAT-125 | MCP Mail Commands | 43896e6 |
| FEAT-124 | Mail Storage Format | 1de3624 |

### Orchestration & Multi-Agent
| ID | Title | Commit |
|----|-------|--------|
| FEAT-114 | Named/Multiple Watchdogs | 01d290f |
| FEAT-111 | Watchdog Auto-Clear Cycle | 13c872e |
| FEAT-110 | Watchdog Monitor Agent | 3a9ddf0 |
| FEAT-109 | drain_messages MCP tool | ee5b2b3 |
| FEAT-106 | Session Creation Tags | feb8e72 |
| FEAT-105 | Universal Agent Presets | 578654d |
| FEAT-104 | Watchdog Orchestration Skill | f19363b |
| FEAT-102 | Agent Status Pane | 1d5cfa2 |
| FEAT-101 | Codex CLI Agent Detection | 2d602a6 |
| FEAT-100 | OrchestrationContext Abstraction | 1f2eb29 |
| FEAT-097 | get_worker_status/poll_messages | 8e4ed1d |
| FEAT-096 | fugue_expect Pattern Wait | done |
| FEAT-095 | fugue_run_pipeline | done |
| FEAT-094 | fugue_run_parallel | done |

### MCP Tools
| ID | Title | Commit |
|----|-------|--------|
| FEAT-121 | cwd in Pane Title | 1da9bd4 |
| FEAT-117 | strip_escapes for read_pane | cabd0d9 |
| FEAT-093 | Special Keys Support | 7b9cd2c |
| FEAT-062 | Mirror Pane | 4325e86 |

### Infrastructure
| ID | Title | Commit |
|----|-------|--------|
| FEAT-089 | Refactor protocol types.rs | 2d4f1db |
| FEAT-088 | Refactor handlers/mcp_bridge.rs | done |
| FEAT-086 | Environment Variable Persistence | 06055a8 |
| FEAT-081 | Landlock Sandboxing | 8497e08 |
| FEAT-080 | Per-pane/session Config | 9086333 |
| FEAT-073 | Visibility Dashboard | b6d43c0 |
| FEAT-074 | Observability Instrumentation | 40c3b1b |

### Networking
| ID | Title | Commit |
|----|-------|--------|
| FEAT-070 | Remote Pane Support | 64387fa |
| FEAT-068 | SSH Tunnel Documentation | 0525712 |
| FEAT-067 | Client TCP Connection | 83fa28a |
| FEAT-066 | TCP Listener Support | 6b977a5 |

## Recommended Work Order

### Phase 1: Quality of Life (Next)
1. **FEAT-115** - Hierarchical session list view
2. **FEAT-112** - Orchestrator context handoff

### Phase 2: Refactoring (Optional)
- FEAT-064, FEAT-065 - MCP bridge cleanup
- Other P3 refactoring as time permits

## Architecture

Multi-crate workspace:
- `fugue-client/` - TUI client (ratatui + crossterm)
- `fugue-server/` - Daemon with PTY management + MCP bridge
- `fugue-protocol/` - Message types and codec (bincode)
- `fugue-session/` - Session/window/pane hierarchy
- `fugue-utils/` - Shared utilities
- `fugue-persistence/` - WAL-based crash recovery
- `fugue-sandbox/` - Landlock sandboxing helper

## Test Coverage

**Total**: 1,700+ tests across all crates
