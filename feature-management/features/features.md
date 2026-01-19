# Feature Tracking

**Project**: ccmux
**Last Updated**: 2026-01-18

## Summary Statistics

- **Total Features**: 104
- **Completed**: 102
- **Backlog**: 2

## Current Status

Core terminal multiplexer fully functional with MCP integration, multi-agent orchestration, remote access, and observability. Agent detection supports Claude and Gemini. Ten features remain in backlog: agent status pane (FEAT-102), Codex detection, orchestration improvements, and refactoring tasks.

## Active Backlog

### High Priority (P1)

| ID | Title | Component | Priority | Status |
|----|-------|-----------|----------|--------|
| FEAT-104 | Watchdog Orchestration Skill | skill/orchestration | P1 | new |
| FEAT-103 | Visualization Architecture Review | ccmux-client/rendering | P1 | new |
| FEAT-094 | ccmux_run_parallel - Parallel Command Execution | ccmux-server/mcp | P1 | done |
| FEAT-095 | ccmux_run_pipeline - Sequential Command Pipeline | ccmux-server/mcp | P1 | done |
| FEAT-096 | ccmux_expect - Pattern-Based Wait | ccmux-server/mcp | P1 | done |
| FEAT-097 | ccmux_get_worker_status / ccmux_poll_messages | ccmux-server/mcp | P1 | done |

### Medium Priority (P2)

| ID | Title | Component | Priority | Status |
|----|-------|-----------|----------|--------|
| FEAT-102 | Agent Status Pane | ccmux-client | P2 | new |
| FEAT-101 | Codex CLI agent detection | ccmux-server/agents | P2 | new |
| FEAT-100 | OrchestrationContext abstraction | ccmux-server/mcp | P2 | new |
| FEAT-099 | Dynamic orchestration session naming | ccmux-server/mcp | P2 | blocked (FEAT-100) |
| FEAT-064 | Refactor MCP bridge.rs into modular components | ccmux-server | P2 | ready |
| FEAT-065 | Refactor handlers in MCP bridge modules | ccmux-server | P2 | ready |

### Lower Priority (P3)

| ID | Title | Component | Priority | Status |
|----|-------|-----------|----------|--------|
| FEAT-069 | TLS/auth for direct TCP connections | ccmux-server | P3 | backlog |
| FEAT-072 | Per-pane MCP mode control | ccmux-server | P3 | backlog |
| FEAT-087 | Refactor client app.rs | ccmux-client | P3 | ready |
| FEAT-088 | Refactor handlers/mcp_bridge.rs | ccmux-server | P3 | done |
| FEAT-089 | Refactor protocol types.rs | ccmux-protocol | P3 | done |
| FEAT-090 | Refactor server main.rs | ccmux-server | P3 | ready |
| FEAT-091 | Refactor mcp_handlers.rs | ccmux-server | P3 | ready |
| FEAT-092 | Refactor protocol messages.rs | ccmux-protocol | P3 | ready |

## Recommended Work Order

### Phase 1: Complete Orchestration (Next)
1. **FEAT-097** (ccmux_get_worker_status) - Orchestrator message polling (Completed)

### Phase 2: Refactoring (Optional)
2. FEAT-064, FEAT-065 - MCP bridge cleanup
3. Other P3 refactoring as time permits

### Completed Orchestration Tools
- **FEAT-096** (ccmux_expect) - Foundation primitive ✓
- **FEAT-094** (ccmux_run_parallel) - Parallel execution ✓
- **FEAT-095** (ccmux_run_pipeline) - Sequential pipelines ✓
- **FEAT-097** (ccmux_get_worker_status) - Message polling ✓

These tools reduce orchestrator context consumption by 70-90%.

## Parallel Workstream Candidates

These have no interdependencies:

**Workstream A - Orchestration Completion:**
- Orchestration tooling complete.

**Workstream B - MCP Bridge Refactoring:**
- FEAT-064, FEAT-065, FEAT-088, FEAT-091

**Workstream C - Protocol Refactoring:**
- FEAT-089, FEAT-092

**Workstream D - Client Refactoring:**
- FEAT-087, FEAT-090

**Workstream E - New Capabilities:**
- FEAT-072 (per-pane MCP mode)

## Recent Completions (2026-01-18)

| ID | Title | Commit |
|----|-------|--------|
| FEAT-089 | Refactor protocol types.rs | 2d4f1db |

## Recent Completions (2026-01-17)

| ID | Title | Commit |
|----|-------|--------|
| BUG-051 | Split pane direction parameter | e3d83f0 |
| FEAT-097 | Orchestration message receive | 8e4ed1d |

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
