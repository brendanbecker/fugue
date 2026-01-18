# ccmux Project Handoff

> **LIVING DOCUMENT**: This handoff file is the interface between sessions. Update it constantly as you work—mark completed items, add discoveries, note blockers, revise plans.

## Context

**ccmux** is a Claude Code-aware terminal multiplexer in Rust.
**Current Stage**: Stage 8 (Multi-Agent Orchestration Enhancement)
**Status**: Production-ready core with new orchestration primitives.

## Current State (2026-01-18)

**All P1 Features Complete!** Orchestration primitives fully shipped:
- `ccmux_expect` - Wait for regex patterns in pane output (FEAT-096)
- `ccmux_run_parallel` - Execute commands in parallel across panes (FEAT-094)
- `ccmux_run_pipeline` - Execute commands sequentially in a single pane (FEAT-095)
- `ccmux_get_worker_status` - Get worker's last reported status (FEAT-097)
- `ccmux_poll_messages` - Poll messages from worker inbox (FEAT-097)

**Agent Detection**: Both Claude and Gemini CLI detected (FEAT-098).

### Active Bugs (6)

| ID | Description | Priority | Component |
|----|-------------|----------|-----------|
| BUG-058 | `ccmux_kill_session` causes client hang | P2 | client |
| BUG-060 | Orchestration MCP tools require session attachment | P2 | mcp |
| BUG-059 | `ccmux_mirror_pane` tool aborts | P3 | mcp |
| BUG-057 | Agent detection cross-contamination | P3 | agents |
| BUG-047 | Compiler warnings cleanup | P3 | build |
| BUG-042 | Result nesting code smell | P3 | mcp |

### Active Features (5 backlog)

| Priority | Features | Status |
|----------|----------|--------|
| P2 | FEAT-064, FEAT-065 (MCP bridge refactoring) | Backlog |
| P3 | FEAT-069, FEAT-072, FEAT-087-092 (infra + refactoring) | Backlog |

### Latest Session (2026-01-18, Session 7)

**Multi-Agent Orchestration Demo - Retrospective**

Ran the full `DEMO-MULTI-AGENT.md` script to validate orchestration capabilities.

**What Worked:**
| Capability | Status | Notes |
|------------|--------|-------|
| Session creation/tagging | ✅ | Created 3 worker sessions, tagged by specialty |
| Agent detection | ✅ | Claude instances detected (`is_claude: true`) |
| `ccmux_expect` | ✅ | Pattern matching for "Claude Code" startup |
| `ccmux_run_parallel` | ✅ | 3 commands in ~2.5s with structured results |
| `ccmux_run_pipeline` | ✅ | Sequential execution, stops on error |
| `ccmux_list_panes` / `ccmux_get_status` | ✅ | Real-time cognitive state monitoring |
| Beads integration | ✅ | assign, find_pane, release, pane_history all work |

**What Failed (New Bugs Filed):**
| Tool | Error | Bug |
|------|-------|-----|
| `ccmux_kill_session` | Client hangs (TUI keybindings still work) | BUG-058 |
| `ccmux_mirror_pane` | AbortError - feature incomplete | BUG-059 |
| `ccmux_send_orchestration` | "Must be attached to session" | BUG-060 |
| `ccmux_broadcast` | "Must be attached to session" | BUG-060 |
| `ccmux_report_status` | "Must be attached to session" | BUG-060 |

**Key Insight:** The orchestration message-passing tools (`send_orchestration`, `broadcast`, `report_status`, `request_help`) require session context that the MCP bridge doesn't have. This blocks the Act 7 message-passing demo entirely. Architecture decision needed.

**Demo Script Assessment:**
- Acts 1-6: Work fully (session creation, agent spawning, parallel execution, status monitoring, beads tracking)
- Act 7: Blocked by BUG-060 (message passing)
- Act 8: Blocked by BUG-059 (mirror panes)
- Acts 9-10: Work with workaround (pipeline works; cleanup works but causes hang)

### Previous Session (2026-01-17, Session 6)

**Merged remaining work items and cleanup:**

| Item | Description | Commit |
|------|-------------|--------|
| BUG-042 | Result nesting regression test | 9cb0263 |
| BUG-047 | 51+ compiler warnings fixed | 354e4d1 |
| FEAT-097 | Orchestration message receive | 382f376 |

**Other accomplishments:**
- Created `DEMO-MULTI-AGENT.md` showcasing orchestration workflows
- Removed obsolete `DEMO.md` and `DEMO-QA.md`
- Added `Makefile` for build/install convenience
- Closed 8 parallel agent sessions, cleaned up all worktrees

**Key Discovery:** Gemini CLI menus require sending digit keys ("1", "2") rather than Enter to select options. Enter cycles through menu items.

### Previous Session (2026-01-17, Session 5)

**Multi-Agent Orchestration via ccmux:**

Successfully ran 8 parallel agents across ccmux sessions for FEAT-097, BUG-042, BUG-047.

### Previous Session (2026-01-17, Session 4)

**Background Agent Experiment - Aborted:**

Attempted to launch 3 parallel background agents via Task tool, but:
1. Agents worked in main instead of assigned worktrees
2. One agent got blocked on permission issues
3. Reset main to discard untrusted changes

**Lesson:** Background Task agents don't respect worktree assignments - they need external orchestration (e.g., ccmux sessions) for true isolation.

**Remaining P1:** FEAT-097 (message receive) still needs implementation.

### Previous Session (2026-01-17, Session 3)

**Parallel Agent Results - 5/5 Work Items Merged:**

| Work Item | Agent | Status | Commit |
|-----------|-------|--------|--------|
| BUG-054 | Gemini | ✅ Merged | `3ce77dc` - TUI Enter handling fix |
| FEAT-096 | Gemini | ✅ Merged | `ab34d81` - `ccmux_expect` tool |
| FEAT-094 | Claude | ✅ Merged | `bbf060c` - `ccmux_run_parallel` tool |
| BUG-053 | Claude | ✅ Merged | `cb1839c` - DSR cursor position fix |
| FEAT-095 | Claude | ✅ Merged | `3f1d4ff` - `ccmux_run_pipeline` tool |

**Key Accomplishments:**
- Successfully ran 5 parallel agents (3 Gemini, 2 Claude) across worktrees
- Orchestrator approved permissions remotely via `ccmux_send_input`
- Demonstrated "plate spinning" workflow for multi-agent coordination
- Resolved FEAT-095 merge conflicts - integrated PipelineRunner into combined orchestration.rs
- Cleaned up all 5 parallel agent worktrees and branches

## Recommended Work Order

Focus on bug fixes to complete the demo, then optional refactoring:

```
Phase 1: Demo Blockers (P2 Bugs)
  1. BUG-058 - kill_session client hang
  2. BUG-060 - Orchestration session attachment (architecture decision)
  3. BUG-059 - mirror_pane incomplete

Phase 2: New Capabilities (P2 Features)
  4. FEAT-100 - OrchestrationContext abstraction
  5. FEAT-101 - Codex CLI agent detection

Phase 3: Refactoring (Optional)
  6. FEAT-064, 065 (MCP bridge cleanup)
  7. Other P3 items as time permits
```

## Parallel Workstreams

These workstreams are **fully independent** and can run in separate worktrees:

### Workstream A: Client Stability (BUG-058)
**Goal**: Fix client hang after `ccmux_kill_session`

| Item | Description | Effort | Files |
|------|-------------|--------|-------|
| BUG-058 | kill_session client hang | Medium | `ccmux-client/src/ui/app.rs`, `ccmux-server/src/session/` |

**Root Cause Hypothesis**: Client waiting for events from killed session. Need to properly handle session removal notification.

### Workstream B: MCP Session Context (BUG-060)
**Goal**: Enable orchestration tools from MCP clients

| Item | Description | Effort | Files |
|------|-------------|--------|-------|
| BUG-060 | Orchestration session attachment | Medium | `ccmux-server/src/mcp/bridge/` |

**Architecture Options**:
1. Auto-attach MCP bridge to spawning session
2. Add `ccmux_attach_session` tool
3. Add `from_session` parameter to orchestration tools
4. Allow MCP-specific "mcp-client" identity

### Workstream C: Mirror Pane Implementation (BUG-059)
**Goal**: Complete the mirror pane feature for "plate spinning"

| Item | Description | Effort | Files |
|------|-------------|--------|-------|
| BUG-059 | mirror_pane abort | Low-Medium | `ccmux-server/src/mcp/bridge/handlers.rs` |

**Note**: Listed as dead code in BUG-047, suggesting incomplete implementation.

### Workstream D: Agent Detection (FEAT-101)
**Goal**: Detect Codex CLI alongside Claude and Gemini

| Item | Description | Effort | Files |
|------|-------------|--------|-------|
| FEAT-101 | Codex CLI detection | Low | `ccmux-server/src/agents/` |

**Note**: Blocked by BUG-053 (Codex requires DSR cursor position). Spec exists at `feature-management/features/FEAT-101-codex-cli-detection/`.

### Workstream E: Code Quality (P3)
**Goal**: Clean up warnings and code smells

| Item | Description | Effort | Files |
|------|-------------|--------|-------|
| BUG-047 | Compiler warnings | Low | Various |
| BUG-042 | Result nesting | Low | `ccmux-server/src/mcp/bridge/` |
| BUG-057 | Agent cross-contamination | Low | `ccmux-server/src/agents/` |

### Workstream F: Refactoring (Optional)
**Goal**: Improve code organization

| Item | Description | Effort | Files |
|------|-------------|--------|-------|
| FEAT-064 | Refactor bridge.rs | Medium | `ccmux-server/src/mcp/bridge/` |
| FEAT-065 | Refactor handlers | Medium | `ccmux-server/src/mcp/bridge/handlers.rs` |
| FEAT-087-092 | Various refactoring | Medium | Multiple |

## Backlog Summary

### Bugs (6 open)

| Priority | Count | IDs |
|----------|-------|-----|
| P2 | 2 | BUG-058, BUG-060 |
| P3 | 4 | BUG-042, BUG-047, BUG-057, BUG-059 |

### Features (6 backlog)

| Priority | ID | Title | Effort |
|----------|----|-------|--------|
| P2 | FEAT-064 | Refactor MCP bridge.rs | Medium |
| P2 | FEAT-065 | Refactor handlers in MCP bridge | Medium |
| P3 | FEAT-069 | TLS/auth for TCP connections | Large |
| P3 | FEAT-072 | Per-pane MCP mode control | Small |
| P3 | FEAT-087 | Refactor client app.rs | Medium |
| P3 | FEAT-088-092 | Various refactoring tasks | Medium |

## Architecture Notes

### Orchestration Tools Design

All orchestration tools are **bridge-only implementations**:
- No protocol changes required
- Use existing primitives: `create_pane`, `send_input`, `read_pane`, `close_pane`
- Module: `ccmux-server/src/mcp/bridge/orchestration.rs`

**Available Tools:**
- `ccmux_expect` - Wait for regex pattern match in pane output
- `ccmux_run_parallel` - Execute up to 10 commands in parallel panes
- `ccmux_run_pipeline` - Execute commands sequentially in a single pane

**Completion Detection Pattern:**
```bash
{ <command> ; } ; echo "___CCMUX_EXIT_$?___"
```
Poll `read_pane` for exit marker to detect command completion.

### Key Files

| Component | Location |
|-----------|----------|
| Agent detection | `ccmux-server/src/agents/` (Claude, Gemini) |
| Orchestration tools | `ccmux-server/src/mcp/bridge/orchestration.rs` |
| MCP bridge handlers | `ccmux-server/src/mcp/bridge/handlers.rs` |
| MCP tool schemas | `ccmux-server/src/mcp/tools.rs` |
| PTY output (DSR fix) | `ccmux-server/src/pty/output.rs` |
| Protocol types | `ccmux-protocol/src/types.rs` |

### ADR-001: The Dumb Pipe Strategy

ccmux is agent-agnostic:
- `Widget` type for generic UI elements
- `AgentState` for any AI agent (not just Claude)
- External systems push data via widget protocol
- See: `docs/adr/ADR-001-dumb-pipe-strategy.md`

## Recent Completions

### 2026-01-17 (Session 6)
| ID | Description | Commit |
|----|-------------|--------|
| FEAT-097 | Orchestration message receive | 382f376 |
| BUG-047 | Compiler warnings cleanup | 354e4d1 |
| BUG-042 | Result nesting regression test | 9cb0263 |

### 2026-01-17 (Session 5)
| ID | Description | Commit |
|----|-------------|--------|
| FEAT-098 | Gemini Agent Detection | d684034 |

### 2026-01-17 (Session 3)
| ID | Description | Commit |
|----|-------------|--------|
| FEAT-095 | ccmux_run_pipeline tool | 3f1d4ff |
| FEAT-096 | ccmux_expect tool | ab34d81 |
| FEAT-094 | ccmux_run_parallel tool | bbf060c |
| BUG-054 | TUI Enter handling fix | 3ce77dc |
| BUG-053 | DSR [6n] cursor position | cb1839c |

### 2026-01-17 (Sessions 1-2)
| ID | Description | Commit |
|----|-------------|--------|
| BUG-052 | Nested agents MCP connection | Verified working |
| BUG-051 | Split pane direction parameter | e3d83f0 |
| BUG-049 | send_input submit reliability | 4af3599 |

### 2026-01-16
| ID | Description | Commit |
|----|-------------|--------|
| BUG-050 | cwd inheritance | ca1dcc9 |
| BUG-048 | TUI flicker | 39ad9fc |
| BUG-046 | MCP select commands | 1ccf693 |
| FEAT-093 | Special keys support | 7b9cd2c |
| FEAT-062 | Mirror pane | 4325e86 |

## Reference

- **Features**: `feature-management/features/features.md`
- **Bugs**: `feature-management/bugs/bugs.md`
- **Agent Cooperation**: `docs/AGENT_COOPERATION.md` - Status reporting protocol
- **Agent Instructions**: `AGENTS.md` - Instructions for AI agents
- **Orchestration Tool Specs**:
  - `feature-management/features/FEAT-094-run-parallel-command-execution/PROMPT.md`
  - `feature-management/features/FEAT-095-run-pipeline-sequential-commands/PROMPT.md`
  - `feature-management/features/FEAT-096-expect-pattern-wait/PROMPT.md`
  - `feature-management/features/FEAT-097-orchestration-message-receive/PROMPT.md`

## Metrics

| Metric | Value |
|--------|-------|
| Total Bugs | 60 |
| Open Bugs | 6 |
| Resolution Rate | 90% |
| Total Features | 101 |
| Completed Features | 90 |
| Completion Rate | 89% |
| Test Count | 1,714+ |

---

## Session Log Template

When starting a new session, copy this template:

```markdown
## Session Log (YYYY-MM-DD)

### Goals
- [ ] Goal 1
- [ ] Goal 2

### Completed
- **ITEM-XXX**: Description (commit abc1234)

### Discovered
- **NEW-ITEM**: Description, root cause, impact

### Blockers
- Description of any blockers encountered

### Next Session
- Recommended starting point
```
