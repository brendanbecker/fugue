# ccmux Project Handoff

> **LIVING DOCUMENT**: This handoff file is the interface between sessions. Update it constantly as you work—mark completed items, add discoveries, note blockers, revise plans. The next session (or a resumed session) relies on this being current.

## Context

**ccmux** is a Claude Code-aware terminal multiplexer in Rust. Development follows the [Context Engineering Methodology](./CONTEXT_ENGINEERING_METHODOLOGY.md).

**Current Stage**: Stage 6 (Implementation) - Wave 3 Ready
**Completed**: Waves 1-2 (6 of 14 features implemented)

## Implementation Progress

### Wave Status

| Wave | Features | Status |
|------|----------|--------|
| 1 | Protocol Layer, Utilities | ✅ Complete |
| 2 | Connection, Session Mgmt, PTY Mgmt, Config | ✅ Complete |
| 3 | Client UI, Terminal Parsing, Persistence | ⏳ Ready |
| 4 | Client Input, Claude Detection | ⏸️ Pending |
| 5 | Session Isolation | ⏸️ Pending |
| 6 | MCP Server, Sideband Protocol | ⏸️ Pending |

### Feature Implementation Status

| # | Feature | Crate | Status | Tests |
|---|---------|-------|--------|-------|
| 1 | Protocol Layer | ccmux-protocol | ✅ Done | 86 |
| 2 | Utilities | ccmux-utils | ✅ Done | 108 |
| 3 | Client UI | ccmux-client | ⏳ Wave 3 | - |
| 4 | Client Input | ccmux-client | ⏸️ Wave 4 | - |
| 5 | Client Connection | ccmux-client | ✅ Done | 31 |
| 6 | Session Management | ccmux-server | ✅ Done | 88 |
| 7 | PTY Management | ccmux-server | ✅ Done | 17 |
| 8 | Terminal Parsing | ccmux-server | ⏳ Wave 3 | - |
| 9 | Claude Detection | ccmux-server | ⏸️ Wave 4 | - |
| 10 | Persistence | ccmux-server | ⏳ Wave 3 | - |
| 11 | Configuration | ccmux-server | ✅ Done | 38 |
| 12 | MCP Server | ccmux-server | ⏸️ Wave 6 | - |
| 13 | Sideband Protocol | ccmux-server | ⏸️ Wave 6 | - |
| 14 | Session Isolation | ccmux-server | ⏸️ Wave 5 | - |

**Total Tests**: 384 passing

## Orchestration Pattern

This project uses **git worktrees** for parallel feature development:

1. Create worktree per feature: `git worktree add -b feature/N-name ../ccmux-wt-name main`
2. Write `SESSION_PROMPT.md` in each worktree with implementation instructions
3. Launch parallel Claude Code sessions, one per worktree
4. Merge branches back to main after wave completion
5. Run test-runner agent to validate and add tests
6. Repeat for next wave

## Wave 3 Features (Next)

| Feature | Crate | Key Dependencies |
|---------|-------|------------------|
| Client UI (3) | ccmux-client | ratatui, tui-term, connection |
| Terminal Parsing (8) | ccmux-server | vt100, session |
| Persistence (10) | ccmux-server | okaywal, bincode, session |

## Completed Work

### Stage 1-4: Research & Architecture
- Deep research from 3 LLMs (Claude, Gemini, ChatGPT)
- Parsed into `docs/research/parsed/` and `SYNTHESIS.md`
- Architecture docs in `docs/architecture/`
- 3 ADRs for key decisions

### Stage 6: Implementation

**2026-01-08 - Wave 2 Complete**
- Merged 4 feature branches:
  - `feature/5-client-connection`: Unix socket client with async I/O
  - `feature/6-session-mgmt`: Session/Window/Pane hierarchy
  - `feature/7-pty-mgmt`: portable-pty integration
  - `feature/11-config`: Hot-reload config with ArcSwap
- Added 190 tests (194 → 384 total)
- All tests passing, no clippy warnings

**2026-01-08 - Wave 1 Complete**
- Merged 2 feature branches:
  - `feature/1-protocol-layer`: IPC messages and codec
  - `feature/2-utilities`: Error types, logging, XDG paths
- Added 177 tests (17 → 194 total)
- Initialized 4-crate workspace structure

## Key Documents

| Document | Purpose |
|----------|---------|
| `docs/architecture/ARCHITECTURE.md` | System overview |
| `docs/architecture/CRATE_STRUCTURE.md` | Workspace layout |
| `docs/FEATURE_HANDOFF.md` | Parallel task: featmgmt backfill |

## Technology Stack

- **PTY**: portable-pty 0.9
- **Parser**: vt100 0.15
- **TUI**: ratatui 0.29 + crossterm 0.28
- **Async**: tokio 1.x
- **Persistence**: okaywal (WAL) + bincode
- **Config**: notify + arc-swap

## Note on Feature Management

Features are tracked informally in this file. A parallel effort is backfilling
the formal `feature-management/` system - see `docs/FEATURE_HANDOFF.md`.
