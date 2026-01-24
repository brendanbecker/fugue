# AGENTS.md - fugue Project Instructions

## Build & Test

```bash
cargo build           # Build all crates
cargo test            # Run all tests
cargo clippy          # Lint
```

## Architecture

fugue is a Rust terminal multiplexer with MCP integration for AI agent orchestration.

**Key crates:**
- `fugue-server/` - Daemon with MCP bridge
- `fugue-client/` - TUI client
- `fugue-protocol/` - Shared types
- `fugue-utils/` - Utilities

**Key locations:**
- MCP handlers: `fugue-server/src/mcp/bridge/handlers.rs`
- MCP tool schemas: `fugue-server/src/mcp/tools.rs`
- PTY handling: `fugue-server/src/pty/`
- Session management: `fugue-server/src/session/`

## Status Reporting (IMPORTANT)

When running inside fugue with MCP tools available, **report your status** to enable orchestration awareness:

### Required Status Reports

| When | Call |
|------|------|
| Starting work | `fugue_report_status` with `status: "working"` |
| Need user input | `fugue_report_status` with `status: "waiting_for_input"` |
| Blocked on approval | `fugue_report_status` with `status: "blocked"` |
| Task complete | `fugue_report_status` with `status: "complete"` |
| Error encountered | `fugue_report_status` with `status: "error"` |

### Example

```json
// When starting a task
{"tool": "fugue_report_status", "input": {"status": "working", "message": "Implementing FEAT-096"}}

// When blocked and need help
{"tool": "fugue_request_help", "input": {"context": "Cannot find the module to modify"}}

// When done
{"tool": "fugue_report_status", "input": {"status": "complete", "message": "FEAT-096 implemented and tested"}}
```

### Why This Matters

Orchestrators monitor worker sessions via status. Without status reports, the orchestrator cannot:
- Know when you're done
- Know when you need input
- Route help requests
- Aggregate progress

## Orchestration Tags

Sessions can be tagged for message routing:

```json
// Mark yourself as a worker
{"tool": "fugue_set_tags", "input": {"add": ["worker", "feat-096"]}}

// Send message to orchestrator
{"tool": "fugue_send_orchestration", "input": {
  "target": {"tag": "orchestrator"},
  "msg_type": "task.progress",
  "payload": {"percent": 50, "current_step": "implementing polling loop"}
}}
```

## Working in Worktrees

This project uses git worktrees for parallel development. Check your branch:

```bash
git branch --show-current
```

Ensure you're on the correct feature/bug branch before committing.

### Cross-Device Link Error in Worktrees

When building in worktrees, you may encounter:
```
error: could not write to ... Invalid cross-device link (os error 18)
```

**Cause:** Cargo uses hard links for incremental compilation. Hard links can't span filesystems. If `/tmp` (cargo's temp location) is on a different filesystem than your worktree, builds fail.

**Workarounds:**
```bash
# Option 1: Use a target dir on the same filesystem
CARGO_TARGET_DIR=./target cargo build

# Option 2: Disable incremental compilation
CARGO_INCREMENTAL=0 cargo build

# Option 3: Set TMPDIR to same filesystem
TMPDIR=./tmp cargo build
```

This is an environment issue, not a fugue bug.

## Feature Management

- Features: `feature-management/features/`
- Bugs: `feature-management/bugs/`
- Each has a `PROMPT.md` with implementation spec

## Commit Convention

```
<type>: <description>
```

Types: `feat`, `fix`, `refactor`, `docs`, `test`, `chore`
