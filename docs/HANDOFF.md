# ccmux Project Handoff

> **LIVING DOCUMENT**: This handoff file is the interface between sessions. Update it constantly as you work—mark completed items, add discoveries, note blockers, revise plans. The next session (or a resumed session) relies on this being current.

## Context

**ccmux** is a Claude Code-aware terminal multiplexer in Rust. Development follows the [Context Engineering Methodology](./CONTEXT_ENGINEERING_METHODOLOGY.md).

**Current Stage**: Stage 6 (Implementation) - Wave 4 Integration
**Completed**: 20 component features (Waves 0-3)
**Remaining**: 7 integration features (Wave 4) to wire components into working MVP

## Current State

All component features are implemented and tested (1,093 tests). The project needs **client-server integration** to become a usable terminal multiplexer.

### What Works
- All protocol types defined
- Session/Window/Pane hierarchy
- PTY spawning and management
- Client UI framework (ratatui)
- Input handling
- Persistence/recovery framework
- MCP server (for Claude integration)
- Sideband protocol parsing

### What's Missing
- Server doesn't listen for connections (TODO in main.rs:316)
- No message routing between client and server
- PTY output not broadcast to clients
- Pane rendering not wired to output

## Wave 4: Integration Features

**Goal**: Wire existing components into a working terminal multiplexer.

| ID | Feature | Priority | Effort | Status |
|----|---------|----------|--------|--------|
| FEAT-021 | Server Socket Listen Loop | P0 | 4-6h | 🔲 Pending |
| FEAT-027 | Client Connection Registry | P0 | 1-2h | 🔲 Pending |
| FEAT-022 | Client Message Routing | P0 | 6-8h | 🔲 Pending |
| FEAT-023 | PTY Output Broadcasting | P0 | 2-3h | 🔲 Pending |
| FEAT-024 | Session Selection UI | P1 | 2h | 🔲 Pending |
| FEAT-025 | Pane Output Rendering | P0 | 3-4h | 🔲 Pending |
| FEAT-026 | Input Testing | P1 | 1-2h | 🔲 Pending |

**Critical Path**: FEAT-021 → FEAT-027 → FEAT-022 → FEAT-025 → FEAT-026

**Total Estimated Effort**: 20-27 hours

### Feature Work Items

Each feature has full documentation in `feature-management/features/FEAT-XXX-*/`:
- `PROMPT.md` - Implementation instructions
- `PLAN.md` - Architecture decisions
- `TASKS.md` - Checkbox task breakdown
- `feature_request.json` - Metadata

## Next Steps

1. **Start with FEAT-021** (Server Socket Listen Loop)
   - Location: `ccmux-server/src/main.rs` line 316
   - Implement Unix socket listener with tokio
   - Accept loop with per-client task spawning
   - See `feature-management/features/FEAT-021-*/PROMPT.md`

2. **Then FEAT-027** (Client Connection Registry)
   - Track connected clients and session associations
   - Enable broadcasting to session members

3. **Then parallel**: FEAT-022, FEAT-023, FEAT-024
   - Message routing, PTY output, session UI

4. **Finally**: FEAT-025, FEAT-026
   - Wire output to rendering, verify input works

## Implementation Progress

### Wave Status

| Wave | Features | Status | Tests |
|------|----------|--------|-------|
| 0 | Protocol, Utilities, Connection, Session, PTY, Config | ✅ Complete | 368 |
| 1 | Parser, Scrollback, Viewport, Worktree (3), Response, Logging, UI, Persistence | ✅ Complete | 452 |
| 2 | Client Input, Claude Detection, Sideband Protocol | ✅ Complete | 224 |
| 3 | MCP Server, Session Isolation | ✅ Complete | 49 |
| 4 | Client-Server Integration (7 features) | 🚧 Pending | - |

**Total Tests**: 1,093 passing

### Component Features (Waves 0-3) - All Complete

| ID | Feature | Component | Tests |
|----|---------|-----------|-------|
| FEAT-001 | vt100 Parser Integration | session/pane | 23 |
| FEAT-002 | Per-Session Scrollback Config | config | 47 |
| FEAT-003 | Viewport Pinning | tui | 23 |
| FEAT-004a | Worktree Detection | orchestration | 12 |
| FEAT-004b | Session-Worktree Binding | orchestration | 8 |
| FEAT-004c | Cross-Session Messaging | orchestration | 45 |
| FEAT-005 | Response Channel | orchestration | 72 |
| FEAT-006 | Per-Session Log Levels | logging | 40 |
| FEAT-007 | Protocol Layer | ccmux-protocol | 86 |
| FEAT-008 | Utilities | ccmux-utils | 108 |
| FEAT-009 | Client UI | ccmux-client | 97 |
| FEAT-010 | Client Input | ccmux-client | 87 |
| FEAT-011 | Client Connection | ccmux-client | 31 |
| FEAT-012 | Session Management | ccmux-server | 88 |
| FEAT-013 | PTY Management | ccmux-server | 17 |
| FEAT-015 | Claude Detection | ccmux-server | 45 |
| FEAT-016 | Persistence | ccmux-server | 85 |
| FEAT-017 | Configuration | ccmux-server | 38 |
| FEAT-018 | MCP Server | ccmux-server | 32 |
| FEAT-019 | Sideband Protocol | ccmux-server | 92 |
| FEAT-020 | Session Isolation | ccmux-server | 17 |

## Key Documents

| Document | Purpose |
|----------|---------|
| `WAVES.md` | Canonical wave plan with dependency graph |
| `feature-management/features/` | Wave 4 feature work items |
| `docs/architecture/ARCHITECTURE.md` | System overview |
| `docs/architecture/CRATE_STRUCTURE.md` | Workspace layout |

## Technology Stack

- **PTY**: portable-pty 0.9
- **Parser**: vt100 0.15
- **TUI**: ratatui 0.29 + crossterm 0.28
- **Async**: tokio 1.x
- **Persistence**: okaywal (WAL) + bincode
- **Config**: notify + arc-swap

## Recent Session (2026-01-09)

### Work Completed
1. Fixed MCP error handling per spec (protocol vs tool errors)
2. Added `.gitignore` for Rust project
3. Cleaned git history (922MB → 1.1MB via filter-repo)
4. Pushed clean repo to GitHub
5. Scoped Wave 4 integration work (7 features, 20-27h)
6. Created feature work items (FEAT-021 through FEAT-027)
7. Updated WAVES.md with Wave 4
8. Ran retrospective agent to validate features

### Key Decisions
- FEAT-027 (Connection Registry) split out as own feature
- FEAT-022 estimate raised to 6-8h (17 message types)
- P0 features form critical path for MVP

## Build & Run

```bash
# Build
cargo build --release

# Run server (currently exits immediately - needs FEAT-021)
./target/release/ccmux-server

# Run MCP server mode (works)
./target/release/ccmux-server mcp-server

# Run client (fails to connect - needs server)
./target/release/ccmux

# Run tests
cargo test --workspace
```
