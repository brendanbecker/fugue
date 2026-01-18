# ccmux Project Handoff

> **LIVING DOCUMENT**: This handoff file is the interface between sessions. Update it constantly as you work—mark completed items, add discoveries, note blockers, revise plans.

## Context

**ccmux** is a Claude Code-aware terminal multiplexer in Rust.
**Current Stage**: Stage 8 (Multi-Agent Orchestration Enhancement)
**Status**: Production-ready core. Focus shifting to high-level orchestration primitives for reduced context consumption.

## Current State (2026-01-17)

**Orchestration Focus**: Core multiplexer stable. New work items target orchestrator efficiency—high-level MCP tools that reduce context consumption by 70-90% in multi-agent workflows.

### Active Bugs (4)

| Bug | Priority | Description |
|-----|----------|-------------|
| **BUG-053** | P1 | Codex CLI fails with cursor position error (DSR [6n not handled) |
| BUG-054 | P2 | send_input submit:true doesn't trigger Enter in TUI apps |
| BUG-047 | P3 | 51+ compiler warnings need cleanup |
| BUG-042 | P3 | Excessive Result nesting (Ok(Ok(...))) code smell |

### Active Features (10)

| Priority | Features |
|----------|----------|
| **P1** | FEAT-094 (run_parallel), FEAT-095 (run_pipeline), FEAT-096 (expect) |
| P2 | FEAT-064, FEAT-065 (MCP bridge refactoring) |
| P3 | FEAT-069, FEAT-072, FEAT-087-092 (infra + refactoring) |

### Latest Session (2026-01-17)

**Created:**
- **FEAT-094**: `ccmux_run_parallel` - Execute commands in parallel across panes
- **FEAT-095**: `ccmux_run_pipeline` - Sequential command pipeline in single pane
- **FEAT-096**: `ccmux_expect` - Block until regex pattern appears in output
- **BUG-053**: Codex CLI fails with cursor position error (DSR [6n not handled)
- **BUG-054**: send_input submit:true doesn't trigger Enter in TUI apps

**Archived:**
- **BUG-051**: Split pane direction parameter fixed (commit e3d83f0)
- **BUG-052**: Nested agents MCP connection - verified working (no longer reproduces)

**Analysis:**
- Conducted retrospective, reprioritized backlog
- Orchestration tools prioritized as P1 (70-90% context savings)
- BUG-052 tested and confirmed fixed - Gemini inside ccmux successfully uses ccmux MCP

## Recommended Work Order

```
Phase 1: Orchestration Foundation (Next)
  1. FEAT-096 (ccmux_expect) ← foundation primitive, small effort
  2. FEAT-094 (ccmux_run_parallel) ← uses expect pattern
  3. FEAT-095 (ccmux_run_pipeline) ← uses expect pattern

Phase 2: Terminal Compatibility
  4. BUG-053 (Codex CLI cursor position) ← DSR escape sequence handling

Phase 3: Optional Refactoring
  5. FEAT-064, 065 (MCP bridge cleanup)
  6. Other P3 items as time permits
```

### Why This Order?

**FEAT-096 first**: The `ccmux_expect` tool provides the completion-detection primitive used by both `run_parallel` and `run_pipeline`. Building it first enables the other tools.

**BUG-053 after orchestration**: Codex CLI support is desirable but not blocking - Claude Code and Gemini CLI work. The DSR escape sequence fix can be addressed after the orchestration tools.

## Backlog Summary

### Bugs (4 open)

| Bug | Priority | Severity | Description |
|-----|----------|----------|-------------|
| **BUG-053** | P1 | high | Codex CLI cursor position error (DSR [6n) |
| BUG-054 | P2 | medium | submit:true doesn't trigger Enter in TUI apps |
| BUG-047 | P3 | low | 51+ compiler warnings |
| BUG-042 | P3 | low | Result nesting code smell |

### Features (10 backlog)

| Priority | ID | Title | Effort |
|----------|----|-------|--------|
| **P1** | FEAT-096 | ccmux_expect - Pattern-based wait | Small |
| **P1** | FEAT-094 | ccmux_run_parallel - Parallel execution | Medium |
| **P1** | FEAT-095 | ccmux_run_pipeline - Sequential pipeline | Medium |
| P2 | FEAT-064 | Refactor MCP bridge.rs | Medium |
| P2 | FEAT-065 | Refactor handlers in MCP bridge | Medium |
| P3 | FEAT-069 | TLS/auth for TCP connections | Large |
| P3 | FEAT-072 | Per-pane MCP mode control | Small |
| P3 | FEAT-087 | Refactor client app.rs | Medium |
| P3 | FEAT-088-092 | Various refactoring tasks | Medium |

## Architecture Notes

### Orchestration Tools Design

All three new tools are **bridge-only implementations**:
- No protocol changes required
- Use existing primitives: `create_pane`, `send_input`, `read_pane`, `close_pane`
- New module: `ccmux-server/src/mcp/bridge/orchestration.rs`

**Completion Detection Pattern:**
```bash
{ <command> ; } ; echo "___CCMUX_EXIT_$?___"
```
Poll `read_pane` for exit marker to detect command completion.

### Key Files

| Component | Location |
|-----------|----------|
| MCP bridge handlers | `ccmux-server/src/mcp/bridge/handlers.rs` |
| MCP tool schemas | `ccmux-server/src/mcp/tools.rs` |
| Protocol types | `ccmux-protocol/src/types.rs` |
| Agent detection | `ccmux-server/src/agents/` |
| Persistence | `ccmux-server/src/persistence/` |

### ADR-001: The Dumb Pipe Strategy

ccmux is agent-agnostic:
- `Widget` type for generic UI elements
- `AgentState` for any AI agent (not just Claude)
- External systems push data via widget protocol
- See: `docs/adr/ADR-001-dumb-pipe-strategy.md`

## Recent Completions

### 2026-01-17
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
- **Retrospective**: `feature-management/RETROSPECTIVE_2026_01_17.md`
- **ADR**: `docs/adr/ADR-001-dumb-pipe-strategy.md`
- **Orchestration Tool Specs**:
  - `feature-management/features/FEAT-094-run-parallel-command-execution/PROMPT.md`
  - `feature-management/features/FEAT-095-run-pipeline-sequential-commands/PROMPT.md`
  - `feature-management/features/FEAT-096-expect-pattern-wait/PROMPT.md`

## Metrics

| Metric | Value |
|--------|-------|
| Total Bugs | 54 |
| Open Bugs | 4 |
| Resolution Rate | 93% |
| Total Features | 96 |
| Completed Features | 86 |
| Completion Rate | 90% |
| Test Count | 1,676+ |

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
