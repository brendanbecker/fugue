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
- **tmux-like auto-start**: Just run `ccmux`, server starts automatically if not running
- Create new sessions with `n` key, delete with `Ctrl+D`
- Sessions auto-create default window/pane/PTY
- **Configurable default command**: Set `default_command = "claude"` in config to auto-launch Claude in new sessions
- **CLI command override**: `ccmux claude --resume` runs custom command instead of default
- Full terminal I/O (shell prompt, commands, output)
- PTY output broadcasting to connected clients
- Session persistence and recovery (sessions survive server restart)
- Output pollers for restored sessions
- Pane close detection and cleanup
- Return to session selection when last pane closes
- Comprehensive modifier key support (Shift+Tab, Alt+key, Ctrl+Arrow, etc.)
- New panes inherit server's working directory
- **Integrated MCP bridge**: Claude controls same sessions as TUI user (11 tools)
- **Sideband pane splitting**: Claude can spawn panes via `<ccmux:spawn>` tags
- **Mouse scroll**: Scroll through terminal scrollback history
- **Quick keybinds**: Ctrl+PageUp/Down for windows, Ctrl+Shift+PageUp/Down for panes

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
| BUG-005 | Sideband parsing not integrated | P0 | ✅ Fixed |
| BUG-006 | Viewport not sizing to terminal | P1 | ✅ Fixed |
| BUG-007 | Shift+Tab not passed through | P1 | ✅ Fixed |
| BUG-008 | Pane/window creation no PTY | P0 | ✅ Fixed |
| BUG-010 | MCP pane broadcast not received by TUI | P1 | 🔍 Investigating |
| BUG-011 | Large paste crashes session | P2 | 📋 New |
| BUG-012 | Text selection not working in TUI | P2 | 📋 New |

## Post-MVP Features

| ID | Feature | Priority | Status |
|----|---------|----------|--------|
| FEAT-029 | MCP Natural Language Control | P1 | ✅ Merged |
| FEAT-030 | Sideband Pane Splitting | P1 | ✅ Merged |
| FEAT-031 | Session Delete Keybind (Ctrl+D) | P2 | ✅ Merged |
| FEAT-032 | Integrated MCP Server | P1 | ✅ Merged |
| FEAT-033 | tmux-like Auto-Start | P1 | ✅ Merged |
| FEAT-017 | Hot-Reload Config | P1 | ✅ Complete |
| FEAT-034 | Mouse Scroll Support | P2 | ✅ Merged |
| FEAT-035 | Configurable Tab/Pane Switching | P2 | ✅ Merged |
| FEAT-037 | tmux-Compatible Keybinds | P1 | ✅ Merged |
| FEAT-038 | Split Pane Rendering | P1 | ✅ Merged |
| FEAT-039 | MCP Pane Creation Broadcast | P2 | ✅ Merged |
| FEAT-040 | MCP Pane Reliability Improvements | P1 | ✅ Merged |
| FEAT-041 | MCP Session/Window Targeting | P1 | ✅ Merged |
| FEAT-042 | MCP Debug Logging | P1 | ✅ Merged |

### FEAT-040: MCP Pane Reliability (BUG-010 Investigation)
- Added debug logging to broadcast_to_session_except() for diagnosis
- Added comprehensive tests for MCP-to-TUI broadcast scenarios
- BUG-010 root cause still under investigation

### FEAT-034/035: Navigation Improvements
- **FEAT-034**: Mouse scroll navigates terminal scrollback with visual offset indicator
- **FEAT-035**: Configurable quick keybinds without prefix (Ctrl+PageUp/Down for windows, Ctrl+Shift for panes)

### FEAT-037: tmux-Compatible Keybinds
All prefix keybinds now match tmux defaults for muscle-memory compatibility.

### FEAT-032/033: UX Improvements
- **FEAT-032**: MCP bridge connects to main daemon, Claude controls same sessions as user
- **FEAT-033**: `ccmux` auto-starts server if not running (like tmux)

## Active Worktrees

| Worktree | Branch | Purpose |
|----------|--------|---------|
| `ccmux-bug-010` | `bug-010-mcp-pane-broadcast` | BUG-010 investigation |
| `ccmux-wt-bug-009` | `bug-009-flaky-persistence-tests` | BUG-009 investigation |

### BUG-010 Investigation Status

MCP pane creation broadcast not reaching TUI. FEAT-041 and FEAT-042 now merged - ready to debug with comprehensive logging.

## Session Log (2026-01-10)

### Work Completed This Session
1. **FEAT-041** merged - MCP session/window targeting for `ccmux_create_pane`
2. **FEAT-042** merged - Comprehensive MCP broadcast path logging
3. **BUG-011** created - Large paste crashes session
4. **BUG-012** created - Text selection not working in TUI
5. Cleaned up FEAT-041/042 worktrees after merge
6. Rebuilt release binaries with new features

### Commits Made
- `4bd48ff` - docs: add BUG-011 large paste crashes session work item
- Merge FEAT-041 and FEAT-042

### Next Steps
- Restart server to pick up FEAT-041/042 changes
- Use FEAT-042 debug logging to trace BUG-010 root cause
- Consider implementing copy mode (Prefix+[) for BUG-012

---

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

# Run (auto-starts server if needed, like tmux)
./target/release/ccmux

# Run with custom command (overrides default_command from config)
./target/release/ccmux bash
./target/release/ccmux claude --resume
./target/release/ccmux python -m http.server

# Run MCP bridge for Claude Code integration
./target/release/ccmux-server mcp-bridge

# Run tests
cargo test --workspace
```

### Session Selection UI Keys
| Key | Action |
|-----|--------|
| `n` | Create new session |
| `Ctrl+D` | Delete selected session |
| `Enter` | Attach to selected session |
| `j/k` or `↑/↓` | Navigate session list |
| `q` | Quit |

### Prefix Keybinds (Ctrl+b, then...)
| Key | Action |
|-----|--------|
| **Windows** ||
| `c` | Create window |
| `&` | Close window |
| `n/p` | Next/prev window |
| `0-9` | Select window by number |
| `w` | List windows |
| **Panes** ||
| `%` | Split vertical |
| `"` | Split horizontal |
| `x` | Close pane |
| `o` | Next pane (cycle) |
| `;` | Previous pane |
| `h/j/k/l` | Vim-style navigation |
| `←↓↑→` | Arrow navigation |
| `z` | Zoom pane (fullscreen) |
| **Session** ||
| `s` | Session picker |
| `d` | Detach |
| **Modes** ||
| `:` | Command mode |
| `[` | Copy/scroll mode |
| `?` | Help |

## Configuration

Config file: `~/.config/ccmux/config.toml`

```toml
[general]
# Auto-launch Claude in every new session
default_command = "claude"
```

## MCP Integration

Add to `~/.claude/mcp.json`:
```json
{
  "mcpServers": {
    "ccmux": {
      "command": "/path/to/ccmux-server",
      "args": ["mcp-bridge"]
    }
  }
}
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

### Priority 1: Hot-Reload Config
- Wire up existing ConfigWatcher for live config reloads (no server restart needed)

### Priority 2: Multi-Agent Orchestration
- Test orchestrator spawning worker Claude instances via sideband tags
- Example: `<ccmux:spawn direction="vertical" command="claude 'implement feature X'" />`

### Priority 3: Window/Pane Management UI
- Keybinds for splitting panes, switching windows
- Status bar showing session/window/pane info

## Future Considerations

**Post-MVP discussion**: The orchestration system (FEAT-004) has methodology-specific coupling (orchestrator/worker concepts). Consider generalizing to tag-based session roles for broader usability.

## Session Log (2026-01-09) - Latest

### Work Completed This Session
1. Merged BUG-002 (flaky test fix)
2. Merged FEAT-029 (MCP natural language control - 4 new tools)
3. Merged FEAT-030 (sideband pane splitting)
4. Merged FEAT-031 (session delete keybind Ctrl+D)
5. Created FEAT-032 (integrated MCP server) and FEAT-033 (auto-start) work items
6. Merged FEAT-032 (MCP bridge connects to daemon)
7. Merged FEAT-033 (tmux-like auto-start)
8. Added `default_command` config option for auto-launching programs in new sessions
9. Set up MCP config at `~/.claude/mcp.json`
10. Cleaned up 6 merged worktrees

### Commits Made This Session
- `b69a3b3` - Merge BUG-002 (flaky test fix)
- `e245443` - Merge FEAT-029 (MCP natural language control)
- `61048f8` - feat: add FEAT-032 and FEAT-033 work items
- `c02c01f` - Merge FEAT-030 (sideband pane splitting)
- `3a1ad12` - fix(server): add pane cleanup loop for BUG-004
- Merge FEAT-031, FEAT-032, FEAT-033
- `8501844` - feat(config): add default_command for auto-launching programs

## Session Log (2026-01-09) - tmux Keybind Alignment + Bug Fix

### Work Completed
1. **FEAT-037** implemented - tmux-compatible prefix keybinds
2. **Prefix+s** now returns to session picker (was TODO)
3. Created worktrees for FEAT-017, FEAT-034, FEAT-035
4. **FEAT-017** merged (was already complete in worktree)
5. **BUG-008** fixed - Pane/window creation now spawns PTY

### Key Changes
- `c` now creates window (was: pane)
- `n/p` now navigate windows (was: panes)
- `o` cycles panes (tmux default)
- `;` goes to previous pane (tmux default)
- `0-9` select window by number (new)
- `&` closes window (new)
- `s` returns to session picker (implemented)
- `%` and `"` now create working panes with shells (BUG-008 fix)

### Commits Made
- `4a15c13` - feat(client): implement Prefix+s to return to session selection
- `a15a767` - feat(client): align prefix keybinds with tmux defaults
- `025bb36` - docs: add FEAT-037 tmux-compatible keybinds (completed)
- `e630f06` - Merge FEAT-017 (hot-reload config)
- `8f02484` - fix(server): spawn PTY for new panes and windows (BUG-008)
- `9055d9b` - docs: add BUG-008 work item

---

## Session Log (2026-01-09) - Bug Fix & Feature Planning Session

### Work Completed
1. **BUG-005** fixed - Sideband parsing now integrated into PTY output flow
2. **BUG-006** fixed - Viewport now sizes to client terminal on attach
3. **BUG-007** fixed - Shift+Tab (BackTab) now passed through to PTY
4. **FEAT-034** planned - Mouse scroll support (work item created)
5. **FEAT-035** planned - Configurable tab/pane switching with Ctrl+PageUp/Down

### Key Technical Discoveries
- Crossterm sends `KeyCode::BackTab` for Shift+Tab, not `KeyCode::Tab` with SHIFT modifier
- Left/Right Ctrl distinction not possible in terminal protocols for key combinations
- Windows captures Alt+Tab/Ctrl+Tab at OS level; use PageUp/PageDown instead

### Commits Made
- `b3b0d68` - fix(client): use client terminal size on session attach (BUG-006)
- `dca9e36` - docs: add BUG-005, FEAT-034, FEAT-035 work items
- `27af339` - fix(client): handle BackTab keycode for Shift+Tab (BUG-007)
- `afee35a` - Merge BUG-005 (sideband parsing integration)
