# ccmux Project Handoff

> **LIVING DOCUMENT**: This handoff file is the interface between sessions. Update it constantly as you work—mark completed items, add discoveries, note blockers, revise plans. The next session (or a resumed session) relies on this being current.

## Context

**ccmux** is a Claude Code-aware terminal multiplexer in Rust. Development follows the [Context Engineering Methodology](./CONTEXT_ENGINEERING_METHODOLOGY.md).

**Current Stage**: Stage 6 (Implementation) - Complete
**Completed**: All waves (20 of 20 features implemented)

## Implementation Progress

### Wave Status

| Wave | Features | Status |
|------|----------|--------|
| 0 | Protocol, Utilities, Connection, Session, PTY, Config | ✅ Complete |
| 1 | Parser, Scrollback, Viewport, Worktree (3), Response, Logging, UI, Persistence | ✅ Complete |
| 2 | Client Input, Claude Detection, Sideband Protocol | ✅ Complete |
| 3 | MCP Server, Session Isolation | ✅ Complete |

### Feature Implementation Status

| ID | Feature | Component | Status | Tests | Priority |
|----|---------|-----------|--------|-------|----------|
| FEAT-001 | vt100 Parser Integration | session/pane | ✅ Done | 23 | P1 |
| FEAT-002 | Per-Session Scrollback Config | config | ✅ Done | 47 | P1 |
| FEAT-003 | Viewport Pinning | tui | ✅ Done | 23 | P2 |
| FEAT-004a | Worktree Detection | orchestration | ✅ Done | 12 | P2 |
| FEAT-004b | Session-Worktree Binding | orchestration | ✅ Done | 8 | P2 |
| FEAT-004c | Cross-Session Messaging | orchestration | ✅ Done | 45 | P2 |
| FEAT-005 | Response Channel | orchestration | ✅ Done | 72 | P1 |
| FEAT-006 | Per-Session Log Levels | logging | ✅ Done | 40 | P2 |
| FEAT-007 | Protocol Layer | ccmux-protocol | ✅ Done | 86 | P1 |
| FEAT-008 | Utilities | ccmux-utils | ✅ Done | 108 | P1 |
| FEAT-009 | Client UI | ccmux-client | ✅ Done | 97 | P1 |
| FEAT-010 | Client Input | ccmux-client | ✅ Done | 87 | P1 |
| FEAT-011 | Client Connection | ccmux-client | ✅ Done | 31 | P1 |
| FEAT-012 | Session Management | ccmux-server | ✅ Done | 88 | P1 |
| FEAT-013 | PTY Management | ccmux-server | ✅ Done | 17 | P1 |
| FEAT-014 | ~~Terminal Parsing~~ | - | 🔀 Merged | - | - |
| FEAT-015 | Claude Detection | ccmux-server | ✅ Done | 45 | P1 |
| FEAT-016 | Persistence | ccmux-server | ✅ Done | 85 | P2 |
| FEAT-017 | Configuration | ccmux-server | ✅ Done | 38 | P1 |
| FEAT-018 | MCP Server | ccmux-server | ✅ Done | 32 | P2 |
| FEAT-019 | Sideband Protocol | ccmux-server | ✅ Done | 92 | P2 |
| FEAT-020 | Session Isolation | ccmux-server | ✅ Done | 17 | P1 |

> **Note:** FEAT-004 decomposed into 004a/b/c. FEAT-014 merged into FEAT-001 (both add vt100 parsing).

**Total Tests**: 1089 passing

## Orchestration Pattern

This project uses **git worktrees** for parallel feature development:

1. Create worktree per feature: `git worktree add -b feature/FEAT-XXX-name ../ccmux-wt-name main`
2. Write `SESSION_PROMPT.md` in each worktree with implementation instructions
3. Launch parallel Claude Code sessions, one per worktree
4. Merge branches back to main after wave completion
5. Run test-runner agent to validate and add tests
6. Repeat for next wave

## All Features Complete

All 20 features have been implemented across 4 waves. The project is feature-complete.

## Completed Work

### Stage 1-4: Research & Architecture
- Deep research from 3 LLMs (Claude, Gemini, ChatGPT)
- Parsed into `docs/research/parsed/` and `SYNTHESIS.md`
- Architecture docs in `docs/architecture/`
- 3 ADRs for key decisions

### Stage 6: Implementation

**2026-01-08 - Wave 3 Complete (Feature Complete)**
- Merged final 2 feature branches:
  - `feature/FEAT-018-mcp-server`: MCP server for Claude integration (32 tests)
  - `feature/FEAT-020-session-isolation`: Per-pane CLAUDE_CONFIG_DIR isolation (17 tests)
- Resolved merge conflict in main.rs (both features added mod declarations)
- Test count: 1040 → 1089 (+49 tests)
- All 20 features implemented, project feature-complete

**2026-01-08 - Wave 2 Complete**
- Merged 3 feature branches from worktrees:
  - `feature/FEAT-010-client-input`: Keyboard/mouse input handling (87 tests)
  - `feature/FEAT-015-claude-detection`: Claude state detection from PTY output (45 tests)
  - `feature/FEAT-019-sideband`: XML sideband protocol for Claude commands (92 tests)
- Test count: 816 → 1040 (+224 tests)
- Wave 2 complete, Wave 3 (final) unblocked

**2026-01-08 - Wave 1 Complete**
- Merged final 4 feature branches from worktrees:
  - `feature/FEAT-001-pane-content`: vt100 parser integration (23 tests)
  - `feature/FEAT-004a-worktree-detection`: Git worktree discovery (12 tests)
  - `feature/FEAT-004b-session-binding`: Session-worktree association (8 tests)
  - `feature/FEAT-004c-cross-session`: Cross-session messaging with router (45 tests)
- Resolved merge conflicts in orchestration module (combined worktree + router)
- Fixed WorktreeInfo struct compatibility (added `head` field)
- Test count: 748 → 816 (+68 tests)
- Wave 1 fully complete, Wave 2 unblocked

**2026-01-08 - Feature Decomposition**
- Rescoped FEAT-001 from "Pane Content Abstraction" to "vt100 Parser Integration" (small)
- Merged FEAT-014 (Terminal Parsing) into FEAT-001 - same goal, avoid duplication
- Decomposed FEAT-004 (xl effort) into 3 medium features:
  - FEAT-004a: Worktree Detection
  - FEAT-004b: Session-Worktree Binding
  - FEAT-004c: Cross-Session Messaging
- Created fresh worktrees from current main (all 4 at commit 4bc4813)
- Wave 1 now has 10 features total (6 complete, 4 remaining)

**2026-01-08 - Wave 1 Progress (6/9 features)**
- Merged 2 more feature branches:
  - `feature/FEAT-009-client-ui`: Ratatui-based terminal UI (97 tests)
  - `feature/FEAT-016-persistence`: WAL + checkpoint persistence with recovery (85 tests)
- Fixed compile errors from protocol type integration
- Test count: 566 → 748 (+182 tests)
- Identified conflicts in FEAT-001 and FEAT-014 (both need reconciliation with scrollback)
- FEAT-004 has no implementation (only SESSION_PROMPT.md)

**2026-01-08 - Wave 1 Partial (4/9 features)**
- Merged 4 feature branches from worktrees:
  - `feature/FEAT-002-scrollback`: Per-session scrollback configuration (47 tests)
  - `feature/FEAT-003-viewport`: Viewport pinning with ViewportState protocol type (23 tests)
  - `feature/FEAT-005-response`: Response channel with PaneTarget, ReplyMessage, ReplyResult (72 tests)
  - `feature/FEAT-006-logging`: Per-session log levels (40 tests)
- Resolved merge conflicts in protocol types (combined ViewportState + Reply types)
- Test count: 384 → 566 (+182 tests)
- Remaining Wave 1: FEAT-001, FEAT-004, FEAT-009, FEAT-014, FEAT-016

**2026-01-08 - Wave 0 Complete**
- Merged 6 feature branches:
  - `feature/FEAT-007-protocol`: IPC messages and codec (86 tests)
  - `feature/FEAT-008-utilities`: Error types, logging, XDG paths (108 tests)
  - `feature/FEAT-011-connection`: Unix socket client with async I/O (31 tests)
  - `feature/FEAT-012-session`: Session/Window/Pane hierarchy (88 tests)
  - `feature/FEAT-013-pty`: portable-pty integration (17 tests)
  - `feature/FEAT-017-config`: Hot-reload config with ArcSwap (38 tests)
- Initialized 4-crate workspace structure
- All tests passing, no clippy warnings

## Key Documents

| Document | Purpose |
|----------|---------|
| `WAVES.md` | Canonical wave plan with dependency graph |
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

Features are tracked informally in this file. The canonical wave structure is in `WAVES.md`.
A parallel effort is backfilling the formal `feature-management/` system - see `docs/FEATURE_HANDOFF.md`.
