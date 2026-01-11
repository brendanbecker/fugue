# Implementation Plan: FEAT-057

**Work Item**: [FEAT-057: Beads Passive Awareness - Auto-Detection and Environment Setup](PROMPT.md)
**Component**: ccmux-server
**Priority**: P2
**Created**: 2026-01-11

## Overview

Add passive beads awareness to ccmux that automatically detects when panes are operating in beads-tracked repositories and configures environment variables appropriately.

## Architecture Decisions

<!-- Document key design choices and rationale -->

- **Approach**: Detect `.beads/` directory on pane spawn, store in pane metadata, propagate to PTY environment
- **Trade-offs**:
  - Sync vs async detection (choosing sync with fast path optimization for simplicity)
  - Pane-level vs session-level beads config (choosing pane-level for per-worktree flexibility)
  - Eager detection vs lazy detection (choosing eager at spawn time)

## Affected Components

<!-- List files and modules that will be modified -->

| Component | Type of Change | Risk Level |
|-----------|----------------|------------|
| ccmux-server/src/config/schema.rs | Add BeadsConfig struct | Low |
| ccmux-server/src/pty/manager.rs | Add beads detection logic | Medium |
| ccmux-server/src/session/pane.rs | Add beads_root field | Low |
| ccmux-client/src/ui/status.rs | Add beads indicator | Low |
| ccmux-server/src/pty/spawn.rs | Merge beads env vars | Medium |

## Implementation Details

### 1. Configuration Schema

```rust
// In ccmux-server/src/config/schema.rs
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct BeadsConfig {
    /// Enable auto-detection of .beads/ directories
    pub auto_detect: bool,

    /// Auto-set BEADS_DIR when .beads/ is detected
    pub auto_set_beads_dir: bool,

    /// Set BEADS_NO_DAEMON=true for new panes
    pub no_daemon_default: bool,
}

impl Default for BeadsConfig {
    fn default() -> Self {
        Self {
            auto_detect: true,
            auto_set_beads_dir: true,
            no_daemon_default: false,
        }
    }
}
```

### 2. Beads Detection Function

```rust
// In ccmux-server/src/pty/manager.rs
use std::path::{Path, PathBuf};

/// Detect .beads/ directory by walking up the directory tree
pub fn detect_beads_root(cwd: &Path) -> Option<PathBuf> {
    let mut current = cwd.to_path_buf();

    // Fast path: check current directory first
    let beads_dir = current.join(".beads");
    if beads_dir.is_dir() {
        return Some(beads_dir);
    }

    // Walk up the directory tree
    while current.pop() {
        let beads_dir = current.join(".beads");
        if beads_dir.is_dir() {
            return Some(beads_dir);
        }
    }

    None
}
```

### 3. Pane Metadata Extension

```rust
// In ccmux-server/src/session/pane.rs
pub struct Pane {
    // existing fields...

    /// Path to .beads/ directory if detected
    pub beads_root: Option<PathBuf>,
}
```

### 4. Environment Variable Integration

When spawning a PTY, if beads is detected:

```rust
// In spawn logic
if let Some(beads_root) = &pane.beads_root {
    if config.beads.auto_set_beads_dir {
        env.insert("BEADS_DIR".to_string(), beads_root.display().to_string());
    }
    if config.beads.no_daemon_default {
        env.insert("BEADS_NO_DAEMON".to_string(), "true".to_string());
    }
}
```

### 5. Status Line Indicator

```rust
// In ccmux-client/src/ui/status.rs
fn render_pane_status(pane: &Pane) -> String {
    let mut status = String::new();

    if pane.beads_root.is_some() {
        status.push_str("[bd] ");
    }

    // existing status rendering...
    status
}
```

## Dependencies

- FEAT-047 (ccmux_set_environment) - Provides session environment infrastructure
- FEAT-053 (auto-inject context env vars) - Similar pattern for env var injection

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Performance impact on pane creation | Low | Medium | Fast path optimization, caching |
| False positive detection | Low | Low | Only check for `.beads/` directory existence |
| Breaking existing workflows | Low | Medium | Feature is opt-in, disabled by default |
| Regression in PTY spawning | Low | High | Comprehensive testing |

## Rollback Strategy

If implementation causes issues:
1. Revert commits associated with this work item
2. Verify system returns to previous state
3. Document what went wrong in comments.md

## Implementation Notes

<!-- Add notes during implementation -->

---
*This plan should be updated as implementation progresses.*
