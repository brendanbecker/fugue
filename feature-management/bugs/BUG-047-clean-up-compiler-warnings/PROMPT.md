# BUG-047: Clean up compiler warnings across fugue crates

**Priority**: P3
**Component**: build
**Severity**: low
**Status**: new

## Problem Statement

The build produces 51+ warnings across fugue-server, fugue-client, and fugue-protocol. These should be cleaned up to maintain code quality and prevent important warnings from being buried in noise.

## Evidence

```
cargo check output: 51+ warnings
```

### Warning Categories

#### 1. Unused Imports (9 warnings, auto-fixable)

These can be fixed automatically with `cargo fix`:

- `ClientType` in multiple handlers (input.rs, mcp_bridge.rs, pane.rs, session.rs)
- `messages::ErrorDetails` in handlers
- `error::McpError` in mcp/mod.rs
- `GaugeSnapshot` in observability/mod.rs
- `crate::types::ClientType` in codec.rs

#### 2. Deprecated `PaneState::Claude` (5 warnings)

Should use `PaneState::Agent` instead:

- fugue-protocol/src/types.rs:527
- fugue-client/src/ui/app.rs:1882
- fugue-server/src/handlers/mcp_bridge.rs:57
- fugue-server/src/persistence/restoration.rs:308
- fugue-server/src/persistence/restoration.rs:354

#### 3. Dead Code - Unused Structs/Functions/Methods (~30 warnings)

**beads.rs:**
- `is_beads_tracked`
- `SocketNotFound`
- `BEADS_*` constants
- `repo_root` field

**handlers/mod.rs:**
- `GlobalBroadcast` variant
- `resolve_active_pane` method

**mcp/handlers.rs:**
- `mirror_pane` method

**mcp/server.rs:**
- `with_managers` function

**orchestration/router.rs:**
- `MessageRouter` and all its methods
- `RouterError`

**orchestration/worktree.rs:**
- `is_git_repo`

**persistence/checkpoint.rs:**
- `extract_sequence`
- `validate`

**persistence/replay.rs:**
- `range`
- `clear`

**persistence/restoration.rs:**
- `without_pty_spawn`

**persistence/scrollback.rs:**
- `ScrollbackConfig` fields
- `ScrollbackCapture` methods
- `ScrollbackRestore`

**persistence/types.rs:**
- `Checkpoint::new`
- `WalSegmentHeader`

**persistence/wal.rs:**
- `WalConfig` fields
- `Wal` methods
- `WalReader`

**agents/claude/mod.rs:**
- `ClaudeAgentDetector` methods

**observability/metrics.rs:**
- `record_replay_failed`

#### 4. Unused Variables (4 warnings)

- agents/claude/mod.rs:67: `text`
- handlers/pane.rs:469: `pane`
- mcp/handlers.rs:1112: `split_direction`
- fugue-client/src/ui/app.rs:2213: `ui_pane`

## Steps to Reproduce

1. Run `cargo build` or `cargo check` in the fugue workspace
2. Observe 51+ warnings across the three crates

## Expected Behavior

Clean build with zero warnings (or minimal intentional ones that are explicitly allowed).

## Actual Behavior

Build produces 51+ warnings including unused imports, deprecated usage, dead code, and unused variables.

## Root Cause Analysis

This is accumulated tech debt from rapid development. Some dead code may be scaffolding for planned features (like the persistence WAL system). The deprecation warnings are from an API migration (Claude -> Agent) that was not fully completed.

## Implementation Tasks

### Section 1: Auto-fixable Warnings
- [ ] Run `cargo fix` to auto-remove unused imports
- [ ] Verify the automatic fixes didn't break anything
- [ ] Commit the auto-fixes separately for clean history

### Section 2: Deprecation Fixes
- [ ] Replace `PaneState::Claude` with `PaneState::Agent` at all locations
- [ ] Verify behavior is unchanged after replacement
- [ ] Consider removing the deprecated variant if no longer needed

### Section 3: Dead Code Triage
- [ ] Review each dead code warning and categorize:
  - **Remove**: Truly unused, no plans to use
  - **Keep (allow)**: Scaffolding for future features, add `#[allow(dead_code)]` with comment
  - **Use**: Actually needed, wire it up
- [ ] Remove truly dead code
- [ ] Add explicit `#[allow(dead_code)]` with justification for kept scaffolding
- [ ] Wire up any code that should actually be used

### Section 4: Unused Variables
- [ ] Prefix with underscore (`_text`, `_pane`, etc.) if intentionally unused
- [ ] Remove entirely if not needed
- [ ] Use the variable if it should be used

### Section 5: Verification
- [ ] Run `cargo check` and verify warning count is near zero
- [ ] Run `cargo test` to ensure no regressions
- [ ] Document any intentional remaining warnings

## Acceptance Criteria

- [ ] Unused import warnings eliminated (cargo fix)
- [ ] No deprecated `PaneState::Claude` usage
- [ ] Dead code either removed or explicitly allowed with justification
- [ ] Unused variables prefixed with underscore or removed
- [ ] Build produces minimal/zero warnings
- [ ] All tests pass
- [ ] No functional regressions

## Notes

### Approach Recommendations

1. **Run `cargo fix` first**: This handles the 9 unused import warnings automatically
2. **Replace deprecated usage**: Simple find-and-replace for `PaneState::Claude` -> `PaneState::Agent`
3. **Triage dead code carefully**: Some persistence code (WAL, checkpoint) may be scaffolding for future crash recovery features - don't remove if it's intentional scaffolding
4. **Prefix unused variables**: Use underscore prefix (e.g., `_pane`) for intentionally unused variables

### Dead Code Considerations

Before removing dead code, check if it's:
- Scaffolding for a planned feature (check feature-management)
- Part of an incomplete refactoring
- Test-only code that's not conditionally compiled
- Actually used via macros or reflection (rare in Rust)

If keeping scaffolding, add explicit allow:
```rust
#[allow(dead_code)] // Scaffolding for FEAT-XXX: crash recovery
struct WalReader { ... }
```
