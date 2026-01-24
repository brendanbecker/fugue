# FEAT-057: Beads Passive Awareness - Auto-Detection and Environment Setup

**Priority**: P2
**Component**: fugue-server
**Type**: new_feature
**Estimated Effort**: medium
**Business Value**: medium

## Overview

Add passive beads awareness to fugue that automatically detects when panes are operating in beads-tracked repositories and configures them appropriately. Currently, using beads with fugue requires manual configuration of environment variables (BEADS_DIR, BEADS_NO_DAEMON) for each session/pane. This is error-prone and tedious, especially in multi-agent workflows where many panes are created dynamically.

## Problem Statement

When working with beads-tracked repositories in fugue:

1. Each pane must have BEADS_DIR manually configured to point to the `.beads/` directory
2. Worktree panes often need BEADS_NO_DAEMON=true to avoid daemon conflicts
3. Multi-agent workflows create many panes dynamically, making manual setup impractical
4. No visual feedback indicates whether a pane is in a beads-tracked repo

## Proposed Solution

### 1. Auto-detect `.beads/` Directory

On pane creation, check if the working directory or any parent directory contains a `.beads/` directory:

```rust
fn detect_beads_root(cwd: &Path) -> Option<PathBuf> {
    let mut current = cwd.to_path_buf();
    loop {
        let beads_dir = current.join(".beads");
        if beads_dir.is_dir() {
            return Some(beads_dir);
        }
        if !current.pop() {
            return None;
        }
    }
}
```

Store the detected beads root path in pane metadata for later use.

### 2. Auto-set Environment Variables

When beads is detected and auto-configuration is enabled:

- Set `BEADS_DIR` to the path of the `.beads/` directory
- Optionally set `BEADS_NO_DAEMON=true` for worktree panes (configurable)

These environment variables are passed to the PTY spawn, leveraging the existing session environment infrastructure from FEAT-047.

### 3. Status Line Indicator

Add visual feedback in the pane status area showing beads awareness:

- Display a "bd" badge or icon when pane is in a beads-tracked repo
- Could integrate with existing status line rendering

### 4. Configuration Options

Add a new `[beads]` section to the fugue config:

```toml
[beads]
# Enable auto-detection (default: true)
auto_detect = true

# Auto-set BEADS_DIR when detected
auto_set_beads_dir = true

# Set BEADS_NO_DAEMON for new panes (useful for worktrees)
no_daemon_default = false
```

## Benefits

- Zero-configuration beads integration for new panes
- Visual feedback that pane is in beads-tracked repo
- Eliminates manual env var setup errors
- Seamless multi-agent workflows
- No performance impact on pane creation (async detection)

## Implementation Tasks

### Section 1: Design
- [ ] Review requirements and acceptance criteria
- [ ] Design solution architecture
- [ ] Identify affected components
- [ ] Document implementation approach

### Section 2: Implementation
- [ ] Add `BeadsConfig` struct to config schema
- [ ] Implement `detect_beads_root()` function in PTY manager
- [ ] Add `beads_root: Option<PathBuf>` to Pane metadata
- [ ] Integrate beads detection into pane spawn flow
- [ ] Auto-set BEADS_DIR when detected and enabled
- [ ] Add beads indicator to status line rendering
- [ ] Add configuration parsing for `[beads]` section

### Section 3: Testing
- [ ] Add unit tests for beads directory detection
- [ ] Add unit tests for config parsing
- [ ] Add integration tests for env var propagation
- [ ] Test nested directory detection (finds `.beads/` in parent dirs)
- [ ] Test performance impact on pane creation
- [ ] Manual testing with actual beads repositories

### Section 4: Documentation
- [ ] Update configuration documentation
- [ ] Add beads integration section to README
- [ ] Add code comments
- [ ] Update CHANGELOG

### Section 5: Verification
- [ ] All acceptance criteria met
- [ ] Tests passing
- [ ] Code review completed
- [ ] Ready for deployment

## Acceptance Criteria

- [ ] Panes in beads repos auto-detect `.beads/` directory
- [ ] BEADS_DIR is automatically set when auto_detect is enabled
- [ ] Status line shows indicator for beads-aware panes
- [ ] Configuration allows disabling auto-detection
- [ ] Works with nested directories (finds `.beads/` in parent dirs)
- [ ] No performance impact on pane creation (async detection)
- [ ] All tests passing
- [ ] Documentation updated
- [ ] No regressions in existing functionality

## Files to Modify

| File | Change |
|------|--------|
| `fugue-server/src/pty/manager.rs` | Add detection on pane spawn |
| `fugue-server/src/session/pane.rs` | Store beads metadata |
| `fugue-server/src/config/schema.rs` | Add beads config section |
| `fugue-client/src/ui/status.rs` | Add beads indicator |

## Dependencies

- **FEAT-047** (fugue_set_environment): COMPLETED - Environment variable infrastructure
- **FEAT-050** (Session Metadata Storage): COMPLETED - Can store beads detection state in session metadata

## Leveraging Completed Features

### FEAT-050 Session Metadata (Completed)

The `fugue_set_metadata` and `fugue_get_metadata` tools can be used to store beads detection state:

```rust
// When beads is detected, store in session metadata
session.set_metadata("beads.root", beads_dir.to_string_lossy());
session.set_metadata("beads.detected", "true");

// Query beads state
if let Some(root) = session.get_metadata("beads.root") {
    // Pane is in beads-tracked repo
}
```

This enables:
- Persistent beads state across session restarts
- MCP tools to query beads awareness without file system checks
- Integration with FEAT-058/059 beads features

## Notes

- Detection should be async to avoid blocking pane creation
- Consider caching beads root detection results for performance
- Future enhancement: could expose beads awareness via MCP tool
- Beads metadata can persist in session state via FEAT-050
