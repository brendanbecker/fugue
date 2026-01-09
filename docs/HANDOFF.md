# ccmux Project Handoff

> **LIVING DOCUMENT**: This handoff file is the interface between sessions. Update it constantly as you work—mark completed items, add discoveries, note blockers, revise plans. The next session (or a resumed session) relies on this being current.

## Context

**ccmux** is a Claude Code-aware terminal multiplexer in Rust. Development follows the [Context Engineering Methodology](./CONTEXT_ENGINEERING_METHODOLOGY.md).

**Current Stage**: Stage 6 (Implementation) - MVP FUNCTIONAL
**Completed**: All Wave 4 core features + critical bug fixes
**Status**: Working terminal multiplexer with Claude Code integration

## Current State

**MVP IS FUNCTIONAL!** Terminal multiplexer works end-to-end.

### What Works
- Server accepts client connections via Unix socket
- Client connects and displays session selection UI
- Create new sessions with `n` key
- Sessions auto-create default window/pane/PTY
- Full terminal I/O (shell prompt, commands, output)
- PTY output broadcasting to connected clients
- Session persistence and recovery (sessions survive server restart)
- Output pollers for restored sessions
- Pane close detection and cleanup
- Return to session selection when last pane closes
- Comprehensive modifier key support (Shift+Tab, Alt+key, Ctrl+Arrow, etc.)
- New panes inherit server's working directory
- MCP server for Claude integration (7 tools)

### Known Issues
- `kill -9` corrupts terminal (SIGKILL can't be caught - run `reset` to fix)
- Legacy zombie sessions from before BUG-004 fix need manual cleanup (clear `~/.local/share/.ccmux/state/`)

## Wave 4: Integration Features

**Goal**: Wire existing components into a working terminal multiplexer.

| ID | Feature | Priority | Status |
|----|---------|----------|--------|
| FEAT-021 | Server Socket Listen Loop | P0 | ✅ Merged |
| FEAT-027 | Client Connection Registry | P0 | ✅ Merged |
| FEAT-022 | Client Message Routing | P0 | ✅ Merged |
| FEAT-023 | PTY Output Broadcasting | P0 | ✅ Merged |
| FEAT-024 | Session Selection UI | P1 | ✅ Merged |
| FEAT-025 | Pane Output Rendering | P0 | ✅ Merged |
| FEAT-026 | Input Testing | P1 | ✅ Working (verified manually) |

## Bug Status

| ID | Description | Priority | Status |
|----|-------------|----------|--------|
| BUG-001 | Client input not captured | P0 | ✅ Fixed |
| BUG-002 | Flaky test (shared temp dir) | P2 | ✅ Fixed |
| BUG-003 | Session missing default pane | P0 | ✅ Fixed |
| BUG-004 | Zombie panes hang client on reattach | P1 | ✅ Fixed |

## Post-MVP Features

| ID | Feature | Priority | Status |
|----|---------|----------|--------|
| FEAT-029 | MCP Natural Language Control | P1 | ✅ Merged |
| FEAT-030 | Sideband Pane Splitting | P1 | ✅ Merged |
| FEAT-031 | Session Delete Keybind (Ctrl+D) | P2 | 🔄 In progress |
| FEAT-032 | Integrated MCP Server | P1 | 🔲 Work item created |
| FEAT-033 | tmux-like Auto-Start | P1 | 🔲 Work item created |

### FEAT-032/033: UX Improvements
- **FEAT-032**: MCP bridge connects to main daemon, Claude controls same sessions as user
- **FEAT-033**: `ccmux` auto-starts server if not running (like tmux)

## Active Worktrees

| Worktree | Branch | Status |
|----------|--------|--------|
| `ccmux-wt-feat-030` | feat-030-sideband-pane-splitting | ✅ Merged (cleanup needed) |
| `ccmux-wt-feat-031` | feat-031-session-delete-keybind | 🔄 In progress |

## Session Log (2026-01-09) - Continued

### Work Completed This Session
1. **BUG-003** fixed - Session creation now auto-creates window/pane/PTY
2. **FEAT-025** merged - Pane output rendering with tui-term
3. **Output poller startup** - Fixed missing poller on session creation
4. **Restored session pollers** - Fixed empty panes on attach to restored sessions
5. **CWD inheritance** - New panes start in server's working directory
6. **Pane close notification** - Server broadcasts PaneClosed on PTY EOF
7. **Session select on empty** - Client returns to session select when last pane closes
8. **Modifier key support** - Shift+Tab, Alt+key, Ctrl+Arrow, Shift+Arrow, Alt+Backspace
9. **FEAT-029** work item created - MCP natural language terminal control
10. **FEAT-030** work item created (by Claude in ccmux) - Sideband pane splitting
11. **BUG-004** fixed - Zombie panes/sessions now auto-cleanup when PTY dies
12. **FEAT-031** work item created - Session delete keybind (Ctrl+D) for cleanup

### Key Fixes Made
- `ccmux-server/src/handlers/session.rs` - Start output poller after PTY spawn
- `ccmux-server/src/main.rs` - Start output pollers for restored panes
- `ccmux-server/src/pty/output.rs` - Broadcast PaneClosed on EOF
- `ccmux-client/src/ui/app.rs` - Comprehensive modifier key handling
- `ccmux-client/src/ui/app.rs` - Return to session select when panes empty
- `ccmux-server/src/pty/output.rs` - PaneClosedNotification channel for cleanup
- `ccmux-server/src/main.rs` - run_pane_cleanup_loop() for auto-cleanup of dead panes/sessions

### Commits Made
- `890e924` - fix(server): start output poller on session creation
- `336273b` - fix: inherit cwd for new panes and notify on pane close
- `3eb04e3` - fix: start output pollers for restored panes
- `cec6c73` - fix(client): return to session select when last pane closes

## Build & Run

```bash
# Build
cargo build --release

# Run server (in background or separate terminal)
./target/release/ccmux-server

# Run client (connects to server)
./target/release/ccmux

# In client:
#   n = create new session
#   Enter = attach to selected session
#   q = quit

# Run MCP server mode
./target/release/ccmux-server mcp-server

# Run tests
cargo test --workspace
```

## Implementation Progress

### Wave Status

| Wave | Features | Status | Tests |
|------|----------|--------|-------|
| 0 | Protocol, Utilities, Connection, Session, PTY, Config | ✅ Complete | 368 |
| 1 | Parser, Scrollback, Viewport, Worktree (3), Response, Logging, UI, Persistence | ✅ Complete | 452 |
| 2 | Client Input, Claude Detection, Sideband Protocol | ✅ Complete | 224 |
| 3 | MCP Server, Session Isolation | ✅ Complete | 49 |
| 4 | Client-Server Integration (7 features) | ✅ Complete | 126 |

**Total Tests**: 1,219 passing

## Key Documents

| Document | Purpose |
|----------|---------|
| `WAVES.md` | Canonical wave plan with dependency graph |
| `feature-management/features/` | Feature work items |
| `feature-management/bugs/` | Bug work items |
| `docs/architecture/ARCHITECTURE.md` | System overview |
| `docs/architecture/CRATE_STRUCTURE.md` | Workspace layout |

## Technology Stack

- **PTY**: portable-pty 0.9
- **Parser**: vt100 0.15
- **TUI**: ratatui 0.29 + crossterm 0.28 + tui-term 0.2
- **Async**: tokio 1.x
- **Persistence**: okaywal (WAL) + bincode
- **Config**: notify + arc-swap

## Next Steps

### Priority 1: Polish
1. Merge BUG-002 (flaky test fix)
2. Clean up old worktrees (feat-022, feat-023)

### Priority 2: Enhanced MCP
3. Implement FEAT-029 (MCP natural language control)
   - Enables commands like "create new window", "split pane horizontally"

### Priority 3: Pane Management
4. Implement FEAT-030 (Sideband pane splitting)
   - Claude can spawn new panes via `<ccmux:spawn>` tags

## Future Considerations

**Post-MVP discussion**: The orchestration system (FEAT-004) has methodology-specific coupling (orchestrator/worker concepts). Consider generalizing to tag-based session roles for broader usability.
