# FEAT-108: Rename Project to Fugue

**Priority**: P1
**Component**: all
**Effort**: Large
**Status**: new

## Summary

Rename the project from "fugue" to "fugue" - a musical term meaning a contrapuntal composition where multiple independent voices enter successively and interweave around a common theme. The metaphor fits agent orchestration perfectly: multiple agents (voices) working independently but in harmony.

## Etymology

From Latin *fuga* meaning "flight" or "chase." In a fugue, each voice "chases" the previous one, entering with the same theme at different pitches while maintaining its own independent line.

## Scope

### 1. Crate Directory Renames (6 directories)

```
fugue-client/   → fugue-client/
fugue-server/   → fugue-server/
fugue-protocol/ → fugue-protocol/
fugue-utils/    → fugue-utils/
fugue-sandbox/  → fugue-sandbox/
fugue-compat/   → fugue-compat/
```

### 2. Cargo.toml Updates (7 files)

- Workspace root: member names, repository URL
- Each crate: package name, binary name, dependency references

### 3. MCP Tool Renames (47 tools)

All `fugue_*` tools become `fugue_*`:

```
fugue_list_panes      → fugue_list_panes
fugue_create_session  → fugue_create_session
fugue_send_input      → fugue_send_input
fugue_report_status   → fugue_report_status
... (47 total)
```

**Key file**: `fugue-server/src/mcp/tools.rs`

### 4. Core Constant

```rust
// fugue-utils/src/paths.rs:10
const APP_NAME: &str = "fugue";  →  const APP_NAME: &str = "fugue";
```

This drives path construction for config, sockets, logs, etc.

### 5. Environment Variables

```
FUGUE_PANE_ID  → FUGUE_PANE_ID
FUGUE_ADDR     → FUGUE_ADDR
FUGUE_LOG      → FUGUE_LOG
```

### 6. Exit Markers

```
___FUGUE_EXIT_<code>___ → ___FUGUE_EXIT_<code>___
```

### 7. Config/Socket Paths

```
~/.config/fugue/     → ~/.config/fugue/
~/.fugue/            → ~/.fugue/
fugue.sock           → fugue.sock
fugue.pid            → fugue.pid
fugue.log            → fugue.log
```

### 8. Binary Names

```
fugue        → fugue
fugue-server → fugue-server
fugue-compat → fugue-compat
```

### 9. Import Statements (~100+ files)

```rust
use fugue_protocol:: → use fugue_protocol::
use fugue_utils::    → use fugue_utils::
use fugue_server::   → use fugue_server::
use fugue_client::   → use fugue_client::
```

### 10. Documentation

- README.md
- docs/*.md
- CLAUDE.md / AGENTS.md
- feature-management/ PROMPT.md files

### 11. GitHub Repository

- Rename repo: `brendanbecker/fugue` → `brendanbecker/fugue`
- Update all repository URL references

## Implementation Order

1. **Rename directories** (git mv for history preservation)
2. **Update APP_NAME constant** in paths.rs
3. **Update all Cargo.toml** files (package names, deps, binaries)
4. **Bulk find/replace** in .rs files:
   - `fugue_` → `fugue_` (MCP tools, env vars, markers)
   - `fugue-` → `fugue-` (crate names in use statements)
   - `fugue::` → `fugue::` (if any)
5. **Update documentation**
6. **Build and test**
7. **Rename GitHub repo** (after merge)

## Acceptance Criteria

- [ ] All crate directories renamed
- [ ] `cargo build` succeeds
- [ ] `cargo test` passes
- [ ] Binary produces `fugue` and `fugue-server`
- [ ] MCP tools register as `fugue_*`
- [ ] Config reads from `~/.config/fugue/`
- [ ] Socket created as `fugue.sock`
- [ ] All docs updated
- [ ] No remaining "fugue" references (except git history)

## Migration Notes

Existing users will need to:
1. Move config: `mv ~/.config/fugue ~/.config/fugue`
2. Update MCP config to reference `fugue-server`
3. Update any scripts referencing `fugue` binary

## Related

- This is a breaking change; consider major version bump
