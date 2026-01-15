# ccmux Project Handoff

> **LIVING DOCUMENT**: This handoff file is the interface between sessions. Update it constantly as you work—mark completed items, add discoveries, note blockers, revise plans. The next session (or a resumed session) relies on this being current.

## Context

**ccmux** is a Claude Code-aware terminal multiplexer in Rust. Development follows the [Context Engineering Methodology](./CONTEXT_ENGINEERING_METHODOLOGY.md).

**Current Stage**: Stage 6 (Implementation) - PRODUCTION READY
**Completed**: All Wave 4 core features + all known bug fixes
**Status**: Production-ready terminal multiplexer with comprehensive MCP integration

## Current State (2026-01-11)

**All QA Demo bugs fixed.** Production ready.

**Key Metrics:**
- 37 bugs tracked, 36 resolved, 1 deprecated
- 60 features tracked, 60 completed
- 1,526+ tests passing
- Clean main branch
- No active worktrees

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
- **Integrated MCP bridge**: Claude controls same sessions as TUI user (12 tools)
- **Sideband pane splitting**: Claude can spawn panes via `<ccmux:spawn>` tags
- **Mouse scroll**: Scroll through terminal scrollback history
- **Quick keybinds**: Ctrl+PageUp/Down for windows, Ctrl+Shift+PageUp/Down for panes
- **MCP read_pane works**: PTY output routed to scrollback (BUG-016 fix)
- **Claude detection**: ClaudeDetector now receives PTY output (FEAT-015 unblocked)
- **Session rename**: `ccmux_rename_session` MCP tool (FEAT-043)
- **Large paste handling**: Graceful chunking prevents crashes (BUG-011 fix)
- **Text selection**: Pane-bounded copy mode with vim-style selection (FEAT-054)
- **Environment tools**: `ccmux_set_environment`, `ccmux_get_environment`, `ccmux_kill_session` (FEAT-047/051/052)
- **Context env vars**: CCMUX_PANE_ID, CCMUX_SESSION_ID auto-injected (FEAT-053)
- **tmux CLI wrapper**: `ccmux-compat` for tmux command compatibility (FEAT-049)
- **Full tmux keybinding parity**: Complete tmux keybind compatibility in TUI (FEAT-055)

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

## Bug Status: All Resolved

**Open Bugs: 0** - All 37 bugs resolved (36 fixed, 1 deprecated).

### QA Demo Bugs (All Fixed)

| Bug | Priority | Root Cause | Fix |
|-----|----------|------------|-----|
| BUG-033 | P1 | Layout passed as string not object | Parse strings in bridge.rs |
| BUG-034 | P2 | active_session_id() ignored explicit selection | Check explicit ID first |
| BUG-035 | P1 | Select/focus tools didn't consume responses | Add response handling loop |
| BUG-036 | P0 | TUI handlers didn't switch sessions | Send AttachSession on focus |
| BUG-037 | P2 | No timeout in recv_response_from_daemon | Add 25s deadline |

### Recently Fixed (This Session)
| Bug | Priority | Description | Resolution |
|-----|----------|-------------|------------|
| BUG-032 | P0 | MCP handlers missing TUI broadcasts | Fixed - add ResponseWithBroadcast to 4 handlers |
| BUG-031 | P2 | Session metadata not persisting | Fixed - persist metadata in WAL/checkpoint |
| BUG-030 | P0 | Daemon unresponsive after operations | Fixed - wrap serde_json::Value for bincode compatibility |
| BUG-029 | P0 | MCP response synchronization lag | Fixed - filter broadcast messages properly |

### Previous Critical Fixes
| Bug | Priority | Description | Resolution |
|-----|----------|-------------|------------|
| BUG-028 | P0 | Daemon crash on nested create_layout | Fixed - two-phase pane creation to avoid lock contention |
| BUG-027 | P0 | MCP response routing swapped | Fixed - filter broadcast messages in recv_response_from_daemon |
| BUG-026 | P1 | Focus management broken | Fixed - broadcast focus changes to TUI clients |
| BUG-025 | P2 | Direction response mismatch | Fixed - return user's requested direction |
| BUG-024 | P0 | Runaway pane spawning via sideband | Fixed - OSC escape sequences (5da1697) |
| BUG-023 | P1 | create_session no shell in pane | Fixed - command param + output poller (1ac7e60) |
| BUG-022 | P2 | Viewport stuck above bottom | Fixed - scroll position reset on resize (b567daf) |

### Historical Bug Summary
| ID | Description | Status |
|----|-------------|--------|
| BUG-001 to BUG-021 | Various integration bugs | ✅ All Fixed |
| BUG-012 | Text selection not working | ❌ Deprecated (Shift+click works) |
| BUG-022 to BUG-027 | QA demo bugs + recent fixes | ✅ All Fixed |

## Feature Backlog (Prioritized)

**Last Updated**: 2026-01-11

**ALL FEATURES COMPLETE** - 60/60 features implemented.

### Just Merged (This Session)
| ID | Title | Status |
|----|-------|--------|
| ✅ FEAT-059 | Beads Workflow Integration | Merged |
| ✅ BUG-032 | MCP handlers missing TUI broadcasts | Fixed |
| ✅ BUG-031 | Metadata persistence | Fixed |
| ✅ BUG-029 | MCP response synchronization | Fixed |
| ✅ FEAT-058 | Beads Query Integration | Merged |
| ✅ BUG-028 | Daemon crash on nested create_layout | Fixed |
| ✅ FEAT-060 | MCP Daemon Auto-Recovery | Merged |
| ✅ FEAT-048 | MCP Orchestration Protocol Tools | Merged |
| ✅ FEAT-057 | Beads Passive Awareness | Merged |
| ✅ FEAT-056 | User Priority Lockout for MCP Focus Control | Merged |
| ✅ FEAT-028 | Orchestration Flexibility Refactor (Tag-based Routing) | Merged |
| ✅ FEAT-036 | Session-Aware MCP Commands | Merged |
| ✅ FEAT-050 | Session Metadata Storage | Merged |

### P3 - Low Priority (Nice to Have)

All P3 features complete.

### Gas Town Integration: COMPLETE ✅

All Gas Town integration features have been implemented:
- ✅ FEAT-052: `ccmux_kill_session` - Worker cleanup
- ✅ FEAT-047: `ccmux_set_environment` - Env propagation
- ✅ FEAT-051: `ccmux_get_environment` - Env retrieval
- ✅ FEAT-053: Context env vars - Claude self-identification
- ✅ FEAT-049: `ccmux-compat` - tmux CLI compatibility

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
| FEAT-043 | MCP Session Rename Tool | P2 | ✅ Merged |
| FEAT-044 | Claude Session Persistence & Auto-Resume | P1 | ✅ Done (stream-d) |
| FEAT-045 | MCP Declarative Layout Tools | P2 | ✅ Merged |
| FEAT-046 | MCP Focus/Select Control | P1 | ✅ Done (stream-e) |

### Open Features (blocked/in-progress)
| ID | Feature | Notes |
|----|---------|-------|
| FEAT-015 | Claude Detection from PTY Output | ✅ Merged |

### Backlog Features (updated after retrospective)
| ID | Feature | Status | Notes |
|----|---------|--------|-------|
| FEAT-048 | MCP Orchestration Protocol Tools | ✅ Merged | Agent-to-agent messaging via MCP |
| FEAT-057 | Beads Passive Awareness | ✅ Merged | Auto-detect .beads/, set env vars |
| FEAT-058 | Beads Query Integration | ✅ Merged | TUI visibility into work queue |
| FEAT-059 | Beads Workflow Integration | Ready | Pane-issue correlation (unblocked) |

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

**None** - All work complete and merged.

### Recently Merged (2026-01-11) - Latest Session
- ✅ **BUG-030**: Daemon unresponsive (serde_json::Value bincode fix)
- ✅ **FEAT-059**: Beads Workflow Integration (4 new MCP tools)
- ✅ **BUG-032**: MCP handlers missing TUI broadcasts
- ✅ **BUG-031**: Metadata persistence across restarts
- ✅ **BUG-029**: MCP response synchronization

### Recently Merged (2026-01-11) - Earlier
- ✅ **BUG-028**: Daemon crash on nested create_layout (stream-e)
- ✅ **FEAT-060**: MCP Daemon Auto-Recovery (stream-f)
- ✅ **FEAT-048**: MCP Orchestration Protocol Tools (stream-a)
- ✅ **FEAT-057**: Beads Passive Awareness (stream-b)
- ✅ **FEAT-058**: Beads Query Integration (stream-c)
- ✅ **FEAT-056**: User Priority Lockout for MCP Focus Control
- ✅ **FEAT-028**: Orchestration Flexibility Refactor (Tag-based Routing)
- ✅ **FEAT-036**: Session-Aware MCP Commands
- ✅ **FEAT-050**: Session Metadata Storage
- ✅ **FEAT-055**: Full tmux keybinding parity in TUI
- ✅ **BUG-027**: MCP response routing (P0 critical)
- ✅ **BUG-026**: Focus management broken
- ✅ **BUG-025**: Direction response mismatch
- ✅ FEAT-054: Pane-bounded text selection in copy mode
- ✅ FEAT-052: ccmux_kill_session MCP tool
- ✅ FEAT-047: ccmux_set_environment MCP tool
- ✅ FEAT-051: ccmux_get_environment MCP tool
- ✅ FEAT-053: Auto-inject CCMUX context env vars
- ✅ FEAT-049: tmux-compatible CLI wrapper (ccmux-compat)

### Recently Merged (2026-01-10)
- ✅ BUG-014: Large output buffer overflow
- ✅ BUG-015: Layout not recalculated on pane close
- ✅ BUG-018/020: Session reattach - client has no PTY
- ✅ BUG-021: ccmux_rename_session missing from standalone MCP server
- ✅ FEAT-044: Claude session persistence & auto-resume
- ✅ FEAT-045: MCP declarative layout tools
- ✅ FEAT-046: MCP focus/select control

---

## Session Log (2026-01-14) - Advanced Features & Remote Peering (Stream C)

### Work Completed This Session
1. **FEAT-080: Per-Pane/Session Configuration via Sideband Commands**
   - Extended `SidebandParser` to support `config` attribute with JSON payload.
   - Updated `AsyncCommandExecutor` to parse config, apply `env`, `cwd`, and `timeout_secs`.
   - Implemented auto-kill logic for `timeout_secs`.
   - Fixed `SidebandParser` regex to support mixed quotes (allowing JSON in attributes).

2. **FEAT-081: Landlock Integration (Sandboxing)**
   - Created `ccmux-sandbox` helper binary using `landlock` crate.
   - Implemented RO system, RW CWD/tmp/dev policy.
   - Updated `ccmux-server` to wrap PTY commands when sandboxing requested.

3. **FEAT-071: Per-pane Claude configuration**
   - Added `presets` support to `AppConfig`.
   - Implemented resolved config writing to `.claude.json` in isolation dirs.
   - Updated `ccmux_create_pane` MCP tool schema.

4. **FEAT-066: TCP listener support in daemon (Phase 1)**
   - Added `listen_tcp` to config and `--listen-tcp` CLI flag.
   - Refactored server to support concurrent Unix and TCP listeners.

5. **FEAT-067: Client TCP connection support (Phase 2)**
   - Added `--addr` and `CCMUX_ADDR` support to `ccmux-client`.
   - Refactored client connection to support `tcp://` and `unix://` URLs.

6. **FEAT-068: SSH tunnel integration and documentation**
   - Created `docs/REMOTE_ACCESS.md` guide.
   - Updated `README.md` with Remote Access section.

7. **FEAT-070: Gastown remote pane support**
   - Updated `ccmux-compat` CLI wrapper to support `--addr` flag and `CCMUX_ADDR` env var.
   - Implemented `set-environment` and `show-environment` commands in `ccmux-compat`.
   - Refactored `ccmux-compat` client to support generic `StreamTrait` (Unix/TCP).
   - Documented Gas Town integration and remote Claude presets in `docs/REMOTE_ACCESS.md`.

### Commits Made
- `9086333` - feat(sideband): implement FEAT-080 per-pane config and timeouts
- `8497e08` - feat(advanced): implement FEAT-081 sandboxing and FEAT-071 per-pane claude config
- `6b977a5` - feat(remote): implement FEAT-066 TCP listener support in daemon
- `83fa28a` - feat(client): implement FEAT-067 client TCP connection support
- `0525712` - docs: implement FEAT-068 SSH tunnel documentation
- `6b977a5` - feat(remote): implement FEAT-070 Gastown remote pane support

### Next Steps
- **FEAT-073**: Multi-tier routing logic.