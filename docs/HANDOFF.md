# ccmux Project Handoff

> **LIVING DOCUMENT**: This handoff file is the interface between sessions. Update it constantly as you work—mark completed items, add discoveries, note blockers, revise plans.

## Context

**ccmux** is a Claude Code-aware terminal multiplexer in Rust.
**Current Stage**: Stage 8 (Multi-Agent Orchestration Enhancement)
**Status**: Production-ready core with new orchestration primitives.

## Current State (2026-01-17)

**Orchestration Tools Shipped**: Core orchestration primitives now available:
- `ccmux_expect` - Wait for regex patterns in pane output (FEAT-096)
- `ccmux_run_parallel` - Execute commands in parallel across panes (FEAT-094)
- `ccmux_run_pipeline` - Execute commands sequentially in a single pane (FEAT-095)

**New**: FEAT-098 Gemini Agent Detection shipped - ccmux now detects both Claude and Gemini CLI.

### Active Bugs (2)

| Bug | Priority | Description | Status |
|-----|----------|-------------|--------|
| BUG-047 | P3 | 51+ compiler warnings need cleanup | Agent working |
| BUG-042 | P3 | Excessive Result nesting (Ok(Ok(...))) | ✅ Ready for merge |

### Active Features (7)

| Priority | Features | Status |
|----------|----------|--------|
| **P1** | FEAT-097 (message receive) | Agent working |
| P1 | FEAT-098 (Gemini detection) | ✅ Merged |
| P2 | FEAT-064, FEAT-065 (MCP bridge refactoring) | Backlog |
| P3 | FEAT-069, FEAT-072, FEAT-087-092 (infra + refactoring) | Backlog |

### Latest Session (2026-01-17, Session 5)

**Multi-Agent Orchestration via ccmux:**

Successfully running 8 parallel agents across ccmux sessions:

| Session | Agent | Task | Status |
|---------|-------|------|--------|
| session-0 | Claude | Orchestrator | Active |
| feat-094-parallel | Claude | PR creation | Working |
| feat-095-pipeline | Gemini | Pipeline tool | Working |
| feat-096-expect | Gemini | Expect tool | Working |
| feat-097-gemini | Gemini | Message receive (FEAT-097) | Working |
| bug-042-gemini | Gemini | Result nesting fix | ✅ Complete |
| bug-047-claude | Claude | Compiler warnings | Working |
| bug-053-codex | Claude | Testing | Working |
| bug-054-tui | Gemini | TUI fix | ✅ Complete |

**Completed This Session:**
- **FEAT-098**: Gemini Agent Detection (`d684034`) - GeminiAgentDetector implementing AgentDetector trait
- **BUG-042**: Result nesting fix - regression test added, branch ready for merge
- **BUG-054**: TUI fix - already committed (`3ce77dc`)

**Key Discovery:** Gemini CLI menus require sending digit keys ("1", "2") rather than Enter to select options. Enter cycles through menu items.

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

```
Phase 1: Complete Orchestration (Next)
  1. FEAT-097 (message receive) ← orchestrator polling

Phase 2: Optional Refactoring
  2. FEAT-064, 065 (MCP bridge cleanup)
  3. BUG-047 (compiler warnings)
  4. Other P3 items as time permits
```

### Why This Order?

**FEAT-097 first**: Demonstrated need during parallel agent session - orchestrators have no way to receive worker status updates without polling raw pane output.

## Backlog Summary

### Bugs (2 open)

| Bug | Priority | Severity | Description |
|-----|----------|----------|-------------|
| BUG-047 | P3 | low | 51+ compiler warnings |
| BUG-042 | P3 | low | Result nesting code smell |

### Features (7 backlog)

| Priority | ID | Title | Effort |
|----------|----|-------|--------|
| **P1** | FEAT-097 | ccmux_get_worker_status / ccmux_poll_messages | Small |
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

### 2026-01-17 (Session 5)
| ID | Description | Commit |
|----|-------------|--------|
| FEAT-098 | Gemini Agent Detection | d684034 |
| BUG-042 | Result nesting regression test | (branch ready) |

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
| Total Bugs | 56 |
| Open Bugs | 1 |
| Resolution Rate | 98% |
| Total Features | 98 |
| Completed Features | 90 |
| Completion Rate | 92% |
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
