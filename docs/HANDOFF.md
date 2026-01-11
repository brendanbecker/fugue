# ccmux Project Handoff

> **LIVING DOCUMENT**: This handoff file is the interface between sessions. Update it constantly as you work—mark completed items, add discoveries, note blockers, revise plans. The next session (or a resumed session) relies on this being current.

## Context

**ccmux** is a Claude Code-aware terminal multiplexer in Rust. Development follows the [Context Engineering Methodology](./CONTEXT_ENGINEERING_METHODOLOGY.md).

**Current Stage**: Stage 6 (Implementation) - PRODUCTION READY
**Completed**: All Wave 4 core features + all known bug fixes
**Status**: Production-ready terminal multiplexer with comprehensive MCP integration

## Current State (2026-01-11)

**All bugs fixed, all features complete.** Production ready.

**Key Metrics:**
- 32 bugs tracked, 31 resolved, 1 deprecated
- 60 features tracked, 60 completed
- 1,526+ tests passing
- Clean git working tree on main branch
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

**Open Bugs: 0** - All bugs fixed.

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

### Next Session Checklist
- [x] ~~Fix BUG-029 (P0): MCP response synchronization~~ - MERGED
- [x] ~~Fix BUG-030 (P0): Daemon unresponsive~~ - MERGED (bincode fix)
- [x] ~~Fix BUG-031 (P2): Metadata persistence~~ - MERGED
- [x] ~~Fix BUG-032 (P0): MCP handlers missing TUI broadcasts~~ - MERGED
- [x] ~~Implement FEAT-059: Beads Workflow Integration~~ - MERGED
- [ ] Update README with new MCP tools
- [ ] Create release build and test full workflow

## Session Log (2026-01-11) - Bug Fixes & Feature Completion

### Work Completed This Session
1. **BUG-029 fixed & merged** - MCP response synchronization (filter broadcast messages)
2. **BUG-030 fixed & merged** - Daemon unresponsive (serde_json::Value bincode wrapper)
3. **BUG-031 fixed & merged** - Metadata persistence across restarts
4. **BUG-032 discovered & fixed** - MCP handlers missing TUI broadcasts
5. **FEAT-059 implemented & merged** - Beads workflow integration (4 new MCP tools)
6. **All worktrees cleaned up** - No active worktrees
7. **All bugs resolved** - 31/32 fixed, 1 deprecated
8. **All features complete** - 60/60 features implemented

### Key Fixes
- **BUG-029**: `is_broadcast_message()` now filters `SessionFocused`, `WindowFocused`, `PaneFocused`
- **BUG-030**: Wrap `serde_json::Value` in newtype for bincode serialization compatibility
- **BUG-031**: WAL entries for metadata set/remove, checkpoint includes metadata
- **BUG-032**: 4 handlers changed from `Response` to `ResponseWithBroadcast`
- **FEAT-059**: New tools: `ccmux_beads_assign`, `ccmux_beads_release`, `ccmux_beads_find_pane`, `ccmux_beads_pane_history`

### Commits Made
- `4e64391` - fix(mcp): correct response synchronization (BUG-029)
- `1305744` - fix(protocol): wrap serde_json::Value for bincode compatibility (BUG-030)
- `f055f56` - fix(persistence): persist session metadata across restarts (BUG-031)
- `c9d916d` - fix(mcp): add TUI broadcasts to state-modifying handlers (BUG-032)
- `3b08b8e` - feat(beads): add workflow integration tools (FEAT-059)
- `c7de119` - docs: add BUG-032 MCP handlers missing TUI broadcasts
- `38e6ff4` - chore: remove unused HashMap imports in persistence module

---

## Session Log (2026-01-11) - QA Demo & Stream C Merge

### Work Completed This Session (Earlier)
1. **Comprehensive QA demo of MCP tools** - Tested 15+ operations
2. **FEAT-058 merged** - Beads query integration (stream-c)
3. **3 new bugs filed**:
   - BUG-029 (P0): MCP response synchronization - responses lag by one call
   - BUG-030 (P0): Daemon unresponsive after create_window/kill_session/create_layout
   - BUG-031 (P2): Session metadata not persisting (FEAT-050 persistence incomplete)
4. **All worktrees cleaned up** - No active streams remaining

### QA Demo Results
**Features Verified Working:**
- Session CRUD (create, list, select, rename)
- Pane operations (split, resize, focus, read)
- Environment variables (set/get)
- Metadata (set/get - but not persisting)
- Input sending to panes

**Bugs Discovered:**
- Response synchronization issue causes "Unexpected response" errors
- Operations succeed but wrong response type returned
- Certain operations (create_window, kill_session, create_layout) crash daemon
- Metadata lost on daemon restart

### Root Cause Analysis
BUG-029 (response sync) appears to be the root cause. Responses are delivered to the wrong MCP calls, offset by one. When error handling triggers on mismatched types, certain operations cause the daemon to enter an unrecoverable state (BUG-030).

### Commits Made
- `6c233e5` - docs: add QA demo bug reports (BUG-029, BUG-030, BUG-031)
- `f9028ad` - Merge branch 'feat/feat-058-beads-query-integration'

---

## Session Log (2026-01-11) - Feature Merge & QA Bugs

### Work Completed This Session
1. **6 features merged from parallel streams**:
   - FEAT-054: Pane-bounded text selection in copy mode
   - FEAT-052: ccmux_kill_session MCP tool
   - FEAT-047: ccmux_set_environment MCP tool
   - FEAT-051: ccmux_get_environment MCP tool
   - FEAT-053: Auto-inject CCMUX context env vars
   - FEAT-049: tmux-compatible CLI wrapper (ccmux-compat)
2. **3 new bugs discovered during QA demo**:
   - BUG-027 (P0): MCP response routing swapped
   - BUG-026 (P1): Focus management broken
   - BUG-025 (P2): Direction response mismatch
3. **Conflict resolution**: Merged stream-c env injection with stream-b MCP tools
4. **Test count increased**: 1219 → 1466 tests passing

### Priority for Next Session
**BUG-027 is P0 critical** - MCP responses going to wrong handlers. Actions work correctly but clients receive wrong response types. This breaks monitoring and orchestration workflows.

---

## Session Log (2026-01-11) - Parallel Development Setup

### Work Completed This Session
1. **FEAT-053 created** - Auto-inject CCMUX context environment variables (enables Claude self-identification)
2. **FEAT-054 created** - Pane-bounded text selection in copy mode (vim-style, OSC 52 clipboard)
3. **Advanced MCP tools merged** - ccmux_split_pane, ccmux_resize_pane, ccmux_create_layout
4. **Bug cleanup** - BUG-022, BUG-023, BUG-024 moved to completed/
5. **Handoff consolidation** - Merged feature-management/HANDOFF.md into docs/HANDOFF.md
6. **Parallel worktrees created** - 4 streams for parallel feature development
7. **SESSION.md files** - Each worktree has worker instructions (gitignored)

### Commits Made
- `dcad991` - fix(logging): client logs to file instead of stderr
- `24a49f7` - chore: add SESSION.md to .gitignore for worker session tracking
- `62a1b5b` - docs: add FEAT-054 pane-bounded text selection in copy mode
- `79e48a3` - chore: move resolved bugs to completed, update HANDOFF.md
- `f0dfb21` - feat(mcp): add advanced pane management tools (split, resize, layout)

### Parallel Streams Ready
| Stream | Features | Est. Time |
|--------|----------|-----------|
| A | FEAT-054 (text selection) | 4-6h |
| B | FEAT-052 → 047 → 051 (MCP tools) | 3h |
| C | FEAT-053 (env injection) | 2h |
| D | FEAT-049 (tmux wrapper) | 3-4h |

### Key Discoveries
- CCMUX_* environment variables will enable Claude Code to identify its own pane context
- Gas Town integration path is clear: FEAT-052 → FEAT-047 → FEAT-053
- Shift+click selection is terminal emulator level; need copy mode for pane-bounded selection

---

## Session Log (2026-01-10) - BUG-019 Fix: UTF-8 Panic in Claude Detector

### Root Cause
After FEAT-015 merge, TUI would hang after some terminal output. Investigation found:
- Claude detector's `output_buffer` truncation used byte slicing without checking UTF-8 char boundaries
- When truncation point landed mid-character (common with box drawing ╭─, spinners ⠋, etc.), it panicked
- Panic killed the output poller task silently, stopping all output to TUI
- Input still worked (Ctrl+b s) because that's handled by separate event loop

### Fix
Changed `detector.rs` lines 193-199:
```rust
// Before: let split_point = len - max/2; buffer[split_point..]  // PANIC!
// After: Find valid UTF-8 boundary before slicing
let split_point = (target_point..len)
    .find(|&i| buffer.is_char_boundary(i))
    .unwrap_or(len);
```

### Commits
- Fix UTF-8 panic in ClaudeDetector buffer truncation (BUG-019)
- Added regression tests for multi-byte UTF-8 handling

---

## Session Log (2026-01-10) - Parallel Execution & Major Bug Fixes

### Work Completed This Session
1. **Feature-management cleanup** - Moved 40+ completed bugs/features to `completed/` folder
2. **BUG-016 fixed** - PTY output now routed to pane state (enables scrollback + Claude detection)
3. **BUG-009 fixed** - Flaky persistence tests resolved
4. **BUG-010 fixed** - MCP pane broadcast now reaches TUI
5. **BUG-011 fixed** - Large paste handled gracefully
6. **FEAT-043 merged** - MCP session rename tool (`ccmux_rename_session`)
7. **BUG-017 filed** - MCP `send_input` doesn't handle Enter key (discovered during dogfooding)
8. **Parallel execution plan** - Created 6-stream parallelization strategy in HANDOFF.md
9. **Worktree setup** - Created `ccmux-stream-a` for critical path work

### Dogfooding ccmux
- Used ccmux MCP tools to spawn Claude in worktree and send it tasks
- Discovered BUG-016 (can't read pane output) and BUG-017 (can't send Enter) in the process
- Successfully orchestrated bug fix via ccmux despite the irony

### Commits Merged
- `af36339` - chore: reorganize feature-management, add BUG-016, update HANDOFF
- BUG-009: 2 commits (flaky persistence fix)
- BUG-016: 2 commits (PTY output routing)
- FEAT-043: 1 commit (session rename MCP tool)
- BUG-011: 1 commit (large paste handling)
- BUG-010: 1 commit (MCP broadcast fix)

### Key Discoveries
- BUG-016 was blocking FEAT-015 (Claude detection) - now unblocked
- MCP orchestration works but needs BUG-017 fixed for full automation

---

## Session Log (2026-01-10) - Split Direction & Active Session Fixes

### Work Completed This Session
1. **Split direction fix** - "vertical" now correctly creates side-by-side panes (was inverted)
2. **Active session tracking** - MCP pane creation now uses the session you're viewing (not oldest)
3. **PaneCreated broadcast direction** - Added direction field so TUI applies correct layout
4. **FEAT-045** created - MCP declarative layout tools (complex layouts, custom ratios)
5. **BUG-014** created - Large output buffer overflow

### Technical Changes
- `ccmux-protocol/src/messages.rs` - Added `direction: SplitDirection` to `PaneCreated` message
- `ccmux-server/src/session/manager.rs` - Added active session tracking (`active_session_id`, `set_active_session()`)
- `ccmux-server/src/handlers/session.rs` - Set active session when TUI attaches
- `ccmux-server/src/handlers/mcp_bridge.rs` - Use active session, include direction in broadcast
- `ccmux-server/src/mcp/bridge.rs` - Fixed direction mapping (vertical = side-by-side)
- `ccmux-client/src/ui/app.rs` - Use direction from broadcast message

### Commits Made
- `d4aeb08` - feat: track active session for MCP commands
- `d598133` - fix: correct split direction mapping for terminal conventions
- `8e81b31` - docs: add FEAT-045 MCP declarative layout tools

### Next Steps
- Test split direction fix after server restart
- Implement FEAT-045 for sophisticated layout control

---

## Session Log (2026-01-10) - Earlier

### Work Completed This Session
1. **FEAT-041** merged - MCP session/window targeting for `ccmux_create_pane`
2. **FEAT-042** merged - Comprehensive MCP broadcast path logging
3. **BUG-011** created - Large paste crashes session
4. **BUG-012** created - Text selection not working in TUI
5. **BUG-013** created - Mouse scroll wheel not working (FEAT-034 regression?)
6. Cleaned up FEAT-041/042 worktrees after merge
7. Rebuilt release binaries with new features

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

**Total Tests**: 1,526 passing

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
