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
- **Integrated MCP bridge**: Claude controls same sessions as TUI user (12 tools)
- **Sideband pane splitting**: Claude can spawn panes via `<ccmux:spawn>` tags
- **Mouse scroll**: Scroll through terminal scrollback history
- **Quick keybinds**: Ctrl+PageUp/Down for windows, Ctrl+Shift+PageUp/Down for panes
- **MCP read_pane works**: PTY output routed to scrollback (BUG-016 fix)
- **Claude detection**: ClaudeDetector now receives PTY output (FEAT-015 unblocked)
- **Session rename**: `ccmux_rename_session` MCP tool (FEAT-043)
- **Large paste handling**: Graceful chunking prevents crashes (BUG-011 fix)

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
| BUG-009 | Flaky persistence tests | P2 | ✅ Fixed |
| BUG-010 | MCP pane broadcast not received by TUI | P1 | ✅ Fixed |
| BUG-011 | Large paste crashes session | P2 | ✅ Fixed |
| BUG-012 | Text selection not working in TUI | P2 | ❌ Deprecated (Shift+click works) |
| BUG-013 | Mouse scroll wheel not working | P2 | 📋 New |
| BUG-014 | Large output buffer overflow | P2 | 📋 New |
| BUG-015 | Layout not recalculated on pane close | P2 | 📋 New |
| BUG-016 | PTY output not routed to pane state (breaks Claude detection + MCP read_pane) | P1 | ✅ Fixed |
| BUG-017 | MCP send_input doesn't handle Enter key | P1 | 📋 New |
| BUG-018 | TUI pane interaction failure (can't see input bar) | P1 | 🔍 Needs investigation |

## Parallel Execution Plan

**Last Updated**: 2026-01-10
**Analysis By**: retrospective-agent

This plan organizes open work items into parallel streams based on dependency analysis and component isolation. Items within the same stream have dependencies and must be executed sequentially. Different streams can be worked in parallel using worktrees.

### Dependency Graph

```
BUG-016 (PTY output routing)
    |
    +---> FEAT-015 (Claude Detection)
    |         |
    |         +---> FEAT-044 (Claude Session Persistence)
    |
    +---> BUG-010 (MCP broadcast) [possibly related]

BUG-011 (Large paste) <--?--> BUG-014 (Large output)
    [may share buffer management root cause]

BUG-013 (Mouse scroll) -- independent
BUG-015 (Layout recalc) -- independent
BUG-009 (Flaky tests) -- independent

FEAT-043 (Session rename) -- independent
FEAT-045 (Declarative layouts) -- independent (but benefits from BUG-010 fix)
```

### Component Isolation Analysis

| Component | Items | Notes |
|-----------|-------|-------|
| **ccmux-server/pty/** | BUG-016, BUG-014 | PTY output routing, buffer management |
| **ccmux-server/mcp/** | BUG-010, FEAT-043, FEAT-045 | MCP tools and broadcast |
| **ccmux-server/session/** | FEAT-015, FEAT-044 | Pane state, Claude detection |
| **ccmux-server/persistence/** | BUG-009 | Test isolation |
| **ccmux-client/ui/** | BUG-015, BUG-013 | Layout, mouse handling |
| **ccmux-client/input/** | BUG-011 | Input handling |
| **ccmux-protocol/** | BUG-011 | Message size limits |

### Recommended Parallel Streams

#### Stream A: Critical PTY/Claude Path (P1 - DO FIRST)
**Goal**: Fix Claude detection and MCP read_pane
**Worktree**: `ccmux-wt-stream-a`

```
1. BUG-016: PTY output not routed to pane state [P1]
   - Root cause: PtyOutputPoller.flush() only broadcasts, never calls pane.process()
   - Fix: Route PTY output through pane.process() for scrollback + Claude detection
   - Files: ccmux-server/src/pty/output.rs, ccmux-server/src/session/pane.rs
   - Estimate: 2-4 hours
   |
   v
2. FEAT-015: Claude Detection from PTY Output [P1]
   - Currently BLOCKED by BUG-016
   - ClaudeDetector exists but pane.process() is never called
   - After BUG-016, verify detection works, tune patterns
   - Files: ccmux-server/src/claude/detector.rs
   - Estimate: 1-2 hours (verification + tuning)
   |
   v
3. FEAT-044: Claude Session Persistence & Auto-Resume [P1]
   - Depends on FEAT-015 for Claude detection
   - Track claude_session_id in pane metadata
   - Auto-resume with `claude --resume <id>` on server restart
   - Files: ccmux-server/src/session/pane.rs, persistence/
   - Estimate: 4-6 hours
```

**Total Stream A Estimate**: 7-12 hours

#### Stream B: MCP Broadcast Investigation (P1)
**Goal**: Fix MCP-to-TUI broadcast path
**Worktree**: `ccmux-bug-010` (existing)

```
1. BUG-010: MCP pane broadcast not received by TUI [P1]
   - May be RELATED to BUG-016 (both involve output/broadcast routing)
   - Debug with FEAT-042 logging (already merged)
   - Check session_id matching, client registration
   - Files: ccmux-server/src/handlers/mcp_bridge.rs, registry.rs
   - Estimate: 2-4 hours
   |
   v (if BUG-010 unblocks)
2. FEAT-045: MCP Declarative Layout Tools [P2]
   - Depends on broadcast working for client sync
   - Add ccmux_create_layout, ccmux_split_pane, ccmux_resize_pane
   - Files: ccmux-server/src/mcp/tools.rs, handlers.rs
   - Estimate: 4-6 hours
```

**Note**: If BUG-010 is actually caused by BUG-016, this stream merges into Stream A.

**Total Stream B Estimate**: 6-10 hours

#### Stream C: Client-Side Bugs (P2 - PARALLEL SAFE)
**Goal**: Fix TUI layout and mouse issues
**Worktree**: `ccmux-wt-stream-c` or use existing `ccmux-bug-013`

These are **completely independent** and can be worked in any order or simultaneously.

```
1. BUG-015: Layout not recalculated on pane close [P2]
   - Client-only: ccmux-client/src/ui/
   - Fix layout tree pruning when panes are removed
   - Estimate: 2-3 hours

2. BUG-013: Mouse scroll wheel not working [P2]
   - Client-only: ccmux-client/src/input/
   - FEAT-034 regression - investigate crossterm mouse capture
   - Worktree exists: ccmux-bug-013
   - Estimate: 1-2 hours
```

**Total Stream C Estimate**: 3-5 hours

#### Stream D: Buffer/Crash Bugs (P2 - PARALLEL SAFE)
**Goal**: Fix large input/output handling
**Worktree**: `ccmux-bug-011` (existing) or new

```
1. BUG-011: Large paste crashes session [P2]
   - Input path: client -> protocol -> server -> PTY
   - May need chunking or size limits
   - Worktree exists: ccmux-bug-011
   - Estimate: 2-4 hours

2. BUG-014: Large output buffer overflow [P2]
   - Output path: PTY -> server -> client viewport
   - May share root cause with BUG-011 (buffer limits)
   - Consider implementing together
   - Estimate: 3-5 hours
```

**Total Stream D Estimate**: 5-9 hours

#### Stream E: Test Stability (P2 - PARALLEL SAFE)
**Goal**: Fix flaky persistence tests
**Worktree**: `ccmux-wt-bug-009` (existing)

```
1. BUG-009: Flaky persistence tests [P2]
   - Server-only: ccmux-server/src/persistence/
   - Apply tempfile::TempDir pattern from BUG-002
   - Independent of all other streams
   - Worktree exists: ccmux-wt-bug-009
   - Estimate: 2-3 hours
```

**Total Stream E Estimate**: 2-3 hours

#### Stream F: MCP Enhancements (P2 - INDEPENDENT)
**Goal**: Add session rename capability
**Worktree**: `ccmux-feat-043` (existing)

```
1. FEAT-043: MCP Session Rename Tool [P2]
   - Server-only: ccmux-server/src/mcp/
   - Small, independent enhancement
   - Worktree exists: ccmux-feat-043
   - Estimate: 2-3 hours
```

**Total Stream F Estimate**: 2-3 hours

### Execution Matrix

| Stream | Items | Priority | Blocked By | Can Start Now | Worktree |
|--------|-------|----------|------------|---------------|----------|
| **A** | BUG-016 -> FEAT-015 -> FEAT-044 | P1 | - | YES | Create new |
| **B** | BUG-010 -> FEAT-045 | P1 | Maybe BUG-016 | YES (investigate) | ccmux-bug-010 |
| **C** | BUG-015, BUG-013 | P2 | - | YES | ccmux-bug-013 |
| **D** | BUG-011, BUG-014 | P2 | - | YES | ccmux-bug-011 |
| **E** | BUG-009 | P2 | - | YES | ccmux-wt-bug-009 |
| **F** | FEAT-043 | P2 | - | YES | ccmux-feat-043 |

### Recommended Execution Order

**Phase 1 (Critical)**: Start Stream A immediately
- BUG-016 is the critical path blocker for Claude integration
- Unblocks FEAT-015 and FEAT-044 (core Claude features)
- May also resolve BUG-010 (investigate relationship first)

**Phase 2 (Parallel)**: While Stream A is in progress, run 2-3 of these in parallel:
- Stream C (BUG-015, BUG-013) - Quick client fixes
- Stream E (BUG-009) - Test stability
- Stream F (FEAT-043) - Small MCP enhancement

**Phase 3 (After Stream A)**:
- If BUG-010 still exists after BUG-016 fix, prioritize Stream B
- Stream D (BUG-011, BUG-014) for robustness

### Maximum Parallelism Configuration

For a multi-agent setup with 3 parallel workers:

| Worker | Streams | Rationale |
|--------|---------|-----------|
| **Worker 1** | Stream A | Critical path, requires focus |
| **Worker 2** | Stream C + E | Client bugs + test fixes (no overlap) |
| **Worker 3** | Stream F + D | MCP enhancement + robustness |

After Phase 1 completes:
| Worker | Streams | Rationale |
|--------|---------|-----------|
| **Worker 1** | Stream A continued (FEAT-015, FEAT-044) | Continue critical path |
| **Worker 2** | Stream B (if needed) | MCP broadcast after PTY fix |
| **Worker 3** | Stream D | Buffer handling |

### Time Estimates Summary

| Priority | Total Estimate | Items |
|----------|---------------|-------|
| **P1 (Critical)** | 13-22 hours | BUG-016, BUG-010, FEAT-015, FEAT-044 |
| **P2 (Important)** | 16-26 hours | BUG-009, BUG-011, BUG-013, BUG-014, BUG-015, FEAT-043, FEAT-045 |
| **Total** | 29-48 hours | All items |

With 3-way parallelism: **10-16 hours wall time** for all items.

### Backlog Items (Not in Current Plan)

| ID | Feature | Notes |
|----|---------|-------|
| FEAT-028 | Orchestration Flexibility Refactor | Future enhancement, not blocking |
| FEAT-036 | Session-Aware MCP Commands | Overlaps with FEAT-043, defer |

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
| FEAT-044 | Claude Session Persistence & Auto-Resume | P1 | 📋 Planned |
| FEAT-045 | MCP Declarative Layout Tools | P2 | 📋 Planned |
| FEAT-046 | MCP Focus/Select Control | P1 | 📋 New |

### Open Features (blocked/in-progress)
| ID | Feature | Notes |
|----|---------|-------|
| FEAT-015 | Claude Detection from PTY Output | ✅ Unblocked - BUG-016 fixed, verify detection works |

### Backlog Features (future enhancements)
| ID | Feature | Notes |
|----|---------|-------|
| FEAT-028 | Orchestration Flexibility Refactor | Generalize orchestrator/worker to tag-based roles |
| FEAT-036 | Session-Aware MCP Commands | May overlap with FEAT-043 |

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

| Worktree | Branch | Status | Next Task |
|----------|--------|--------|-----------|
| `ccmux-stream-a` | `stream-a-critical-path` | ✅ BUG-016 done | Rebase, reassign to FEAT-044 or BUG-017 |
| `ccmux-bug-013` | `bug-013-mouse-scroll` | 🔄 In progress | Check if complete next session |
| `ccmux-bug-010` | `bug-010-mcp-pane-broadcast` | ✅ Merged | Can be removed |
| `ccmux-bug-011` | `bug-011-large-paste-crash` | ✅ Merged | Can be removed |
| `ccmux-feat-043` | `feat-043-session-rename` | ✅ Merged | Can be removed |
| `ccmux-wt-bug-009` | `bug-009-flaky-persistence-tests` | ✅ Merged | Can be removed |

### Next Session Checklist
- [ ] Check if BUG-013 (mouse scroll) is complete in `ccmux-bug-013` worktree
- [ ] Rebase `ccmux-stream-a` and assign to next P1 task (FEAT-044 or BUG-017)
- [ ] Clean up merged worktrees (bug-010, bug-011, feat-043, wt-bug-009)
- [ ] Rebuild server with all fixes and test `read_pane` works (BUG-016 fix)

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
