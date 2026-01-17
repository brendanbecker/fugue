# Feature Tracking

**Project**: ccmux
**Last Updated**: 2026-01-16

## Summary Statistics

- **Total Features**: 93
- **Completed**: 86
- **Backlog**: 7

## Current Status

Core terminal multiplexer fully functional with MCP integration, multi-agent orchestration, remote access, and observability. Eight features remain in backlog, primarily code refactoring tasks.

## Active Backlog

| ID | Title | Component | Priority | Status |
|----|-------|-----------|----------|--------|
| FEAT-064 | Refactor MCP bridge.rs into modular components | ccmux-server | P2 | ready |
| FEAT-065 | Refactor handlers in MCP bridge modules | ccmux-server | P2 | ready |
| FEAT-069 | TLS/auth for direct TCP connections | ccmux-server | P3 | backlog |
| FEAT-072 | Per-pane MCP mode control | ccmux-server | P3 | backlog |
| FEAT-087 | Refactor client app.rs | ccmux-client | P3 | ready |
| FEAT-088 | Refactor handlers/mcp_bridge.rs | ccmux-server | P3 | ready |
| FEAT-089 | Refactor protocol types.rs | ccmux-protocol | P3 | ready |
| FEAT-090 | Refactor server main.rs | ccmux-server | P3 | ready |
| FEAT-091 | Refactor mcp_handlers.rs | ccmux-server | P3 | ready |
| FEAT-092 | Refactor protocol messages.rs | ccmux-protocol | P3 | ready |

## Parallel Workstream Candidates

These have no interdependencies:

**Workstream A - MCP Bridge Refactoring:**
- FEAT-064, FEAT-065, FEAT-088, FEAT-091

**Workstream B - Protocol Refactoring:**
- FEAT-089, FEAT-092

**Workstream C - Client Refactoring:**
- FEAT-087, FEAT-090

**Workstream D - New Capabilities:**
- FEAT-072 (per-pane MCP mode)

## Recent Completions (2026-01-16)

| ID | Title | Commit |
|----|-------|--------|
| FEAT-093 | Special keys support (Escape, Ctrl, function keys) | 7b9cd2c |
| FEAT-062 | Mirror Pane (picture-in-picture) | 4325e86 |
| FEAT-073 | Visibility dashboard | b6d43c0 |
| FEAT-074 | Observability instrumentation | 40c3b1b |
| FEAT-082 | Multi-tier routing logic | 4eb45dd |
| FEAT-083 | Generic widget system | f421cad |
| FEAT-084 | Abstract agent state | d5ef0f8 |
| FEAT-085 | ADR: Dumb Pipe Strategy | e9e84e2 |
| FEAT-086 | Environment variable persistence | 06055a8 |
| FEAT-066 | TCP listener support | 6b977a5 |
| FEAT-067 | Client TCP connection | 83fa28a |
| FEAT-068 | SSH tunnel documentation | 0525712 |
| FEAT-070 | Gastown remote pane support | 64387fa |
| FEAT-071 | Per-pane Claude configuration | 8497e08 |
| FEAT-080 | Per-pane/session config via sideband | 9086333 |
| FEAT-081 | Landlock sandboxing integration | 8497e08 |

## Architecture

Multi-crate workspace:
- `ccmux-client/` - TUI client (ratatui + crossterm)
- `ccmux-server/` - Daemon with PTY management + MCP bridge
- `ccmux-protocol/` - Message types and codec (bincode)
- `ccmux-session/` - Session/window/pane hierarchy
- `ccmux-utils/` - Shared utilities
- `ccmux-persistence/` - WAL-based crash recovery
- `ccmux-sandbox/` - Landlock sandboxing helper

## Test Coverage

**Total**: 1,676+ tests across all crates
