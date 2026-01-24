# Task Breakdown: BUG-047

**Work Item**: [BUG-047: Clean up compiler warnings across fugue crates](PROMPT.md)
**Status**: Completed
**Last Updated**: 2026-01-18

## Prerequisites

- [x] Read and understand PROMPT.md
- [x] Review PLAN.md and update if needed
- [x] Run `cargo check 2>&1 | grep warning | wc -l` to get baseline warning count

## Phase 1: Auto-fixable Warnings (Unused Imports)

- [x] Run `cargo fix --allow-dirty` to auto-remove unused imports
- [x] Review the changes made by cargo fix
- [x] Run `cargo check` to verify warnings reduced
- [x] Run `cargo test` to ensure no breakage
- [x] Commit: "fix: remove unused imports (cargo fix)"

## Phase 2: Deprecated Usage (PaneState::Claude)

- [x] Find all uses: `rg "PaneState::Claude" --type rust`
- [x] Replace with `PaneState::Agent` at each location:
  - [x] fugue-protocol/src/types.rs:527
  - [x] fugue-client/src/ui/app.rs:1882
  - [x] fugue-server/src/handlers/mcp_bridge.rs:57
  - [x] fugue-server/src/persistence/restoration.rs:308
  - [x] fugue-server/src/persistence/restoration.rs:354
- [x] Run `cargo check` to verify deprecation warnings gone
- [x] Run `cargo test` to ensure no breakage
- [x] Commit: "fix: replace deprecated PaneState::Claude with PaneState::Agent"

## Phase 3: Dead Code Triage

### Triage Categories
For each dead code item, decide: REMOVE, KEEP (with allow), or WIRE UP

### beads.rs
- [x] Review `is_beads_tracked` - REMOVE/KEEP/WIRE?
- [x] Review `SocketNotFound` - REMOVE/KEEP/WIRE?
- [x] Review `BEADS_*` constants - REMOVE/KEEP/WIRE?
- [x] Review `repo_root` field - REMOVE/KEEP/WIRE?
- [x] Apply decision and add `#[allow(dead_code)]` with comment if keeping

### handlers/mod.rs
- [x] Review `GlobalBroadcast` variant - REMOVE/KEEP/WIRE?
- [x] Review `resolve_active_pane` method - REMOVE/KEEP/WIRE?

### mcp/handlers.rs
- [x] Review `mirror_pane` method - REMOVE/KEEP/WIRE?

### mcp/server.rs
- [x] Review `with_managers` function - REMOVE/KEEP/WIRE?

### orchestration/router.rs
- [x] Review `MessageRouter` and all methods - REMOVE/KEEP/WIRE?
- [x] Review `RouterError` - REMOVE/KEEP/WIRE?

### orchestration/worktree.rs
- [x] Review `is_git_repo` - REMOVE/KEEP/WIRE?

### persistence/ module
- [x] Determine if persistence scaffolding is for planned feature
- [x] Review checkpoint.rs: `extract_sequence`, `validate`
- [x] Review replay.rs: `range`, `clear`
- [x] Review restoration.rs: `without_pty_spawn`
- [x] Review scrollback.rs: `ScrollbackConfig`, `ScrollbackCapture`, `ScrollbackRestore`
- [x] Review types.rs: `Checkpoint::new`, `WalSegmentHeader`
- [x] Review wal.rs: `WalConfig`, `Wal` methods, `WalReader`

### agents/claude/mod.rs
- [x] Review `ClaudeAgentDetector` methods - REMOVE/KEEP/WIRE?

### observability/metrics.rs
- [x] Review `record_replay_failed` - REMOVE/KEEP/WIRE?

### Apply Decisions
- [x] Remove truly dead code
- [x] Add `#[allow(dead_code)]` with justification for scaffolding
- [x] Run `cargo check` to verify dead code warnings resolved
- [x] Run `cargo test` to ensure no breakage
- [x] Commit: "refactor: clean up dead code (BUG-047)"

## Phase 4: Unused Variables

- [x] Fix agents/claude/mod.rs:67: `text` -> `_text` or remove
- [x] Fix handlers/pane.rs:469: `pane` -> `_pane` or remove
- [x] Fix mcp/handlers.rs:1112: `split_direction` -> `_split_direction` or remove
- [x] Fix fugue-client/src/ui/app.rs:2213: `ui_pane` -> `_ui_pane` or remove
- [x] Run `cargo check` to verify unused variable warnings resolved
- [x] Commit: "fix: prefix unused variables with underscore"

## Verification Tasks

- [x] Run `cargo check 2>&1 | grep warning | wc -l` - compare to baseline
- [x] Run `cargo test` - all tests pass
- [x] Run `cargo clippy` - check for any new clippy warnings
- [x] Update bug_report.json status to "fixed"
- [x] Document any intentionally remaining warnings

## Completion Checklist

- [x] All unused import warnings eliminated
- [x] No deprecated PaneState::Claude usage
- [x] Dead code handled (removed or explicitly allowed)
- [x] Unused variables addressed
- [x] Warning count near zero
- [x] All tests passing
- [x] PLAN.md updated with final approach
- [x] Ready for review/merge

---
*Check off tasks as you complete them. Update status field above.*
