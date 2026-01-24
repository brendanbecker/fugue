# Implementation Plan: FEAT-053

**Work Item**: [FEAT-053: Auto-inject CCMUX context environment variables on pane spawn](PROMPT.md)
**Component**: fugue-server (PTY spawning)
**Priority**: P1
**Created**: 2026-01-11

## Overview

Automatically inject environment variables (FUGUE_PANE_ID, FUGUE_SESSION_ID, FUGUE_WINDOW_ID, FUGUE_SESSION_NAME) when spawning any pane, enabling processes to be self-aware of their fugue context.

## Architecture Decisions

- **Approach**: Create a helper method to centralize environment injection, modify all PtyConfig creation sites to use it
- **Trade-offs**:
  - Helper method vs inline code at each site (choosing helper for DRY and consistency)
  - Environment variables vs alternative discovery mechanism (choosing env vars for simplicity and tmux parity)

## Affected Components

| Component | Type of Change | Risk Level |
|-----------|----------------|------------|
| fugue-server/src/handlers/mcp_bridge.rs | Add env injection to ~10 PtyConfig sites | Medium |
| fugue-server/src/mcp/handlers.rs | Add env injection if PTY spawning present | Low |
| fugue-server/src/session.rs | Add env injection if PTY spawning present | Low |
| fugue-server/src/sideband/async_executor.rs | Add env injection for sideband spawns | Medium |
| fugue-server/src/pty/ (if exists) | Potential helper method location | Low |

## Implementation Details

### 1. Helper Method Design

Create a helper function or extend PtyConfig to inject context:

```rust
// Option A: Free function
fn fugue_context_env(
    session_id: &SessionId,
    session_name: &str,
    window_id: &WindowId,
    pane_id: &PaneId,
) -> Vec<(String, String)> {
    vec![
        ("FUGUE_SESSION_ID".to_string(), session_id.to_string()),
        ("FUGUE_SESSION_NAME".to_string(), session_name.to_string()),
        ("FUGUE_WINDOW_ID".to_string(), window_id.to_string()),
        ("FUGUE_PANE_ID".to_string(), pane_id.to_string()),
    ]
}

// Option B: PtyConfig builder method
impl PtyConfig {
    pub fn with_fugue_context(
        self,
        session_id: &SessionId,
        session_name: &str,
        window_id: &WindowId,
        pane_id: &PaneId,
    ) -> Self {
        self.with_env("FUGUE_SESSION_ID", &session_id.to_string())
            .with_env("FUGUE_SESSION_NAME", session_name)
            .with_env("FUGUE_WINDOW_ID", &window_id.to_string())
            .with_env("FUGUE_PANE_ID", &pane_id.to_string())
    }
}
```

### 2. Spawn Site Modifications

At each location where PtyConfig is created:

```rust
// Before
let config = PtyConfig::default()
    .with_size(cols, rows)
    .with_cwd(cwd);

// After
let config = PtyConfig::default()
    .with_size(cols, rows)
    .with_cwd(cwd)
    .with_fugue_context(&session_id, &session_name, &window_id, &pane_id);
```

### 3. Known Spawn Locations (to audit)

Based on the feature description, these files need modification:
- `fugue-server/src/handlers/mcp_bridge.rs` (~10 locations)
- `fugue-server/src/mcp/handlers.rs`
- `fugue-server/src/session.rs`
- `fugue-server/src/sideband/async_executor.rs`

Each location should already have access to session/window/pane IDs in scope.

## Dependencies

None

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Missing a spawn site | Medium | Low | Audit all PtyConfig usages, add tests |
| Context not available at spawn | Low | Medium | Context should be in scope; verify during implementation |
| Breaking existing env var handling | Low | High | Use existing with_env() method, don't modify its behavior |
| Performance overhead | Very Low | Very Low | Just 4 string insertions per spawn |

## Rollback Strategy

If implementation causes issues:
1. Revert commits associated with this work item
2. Verify system returns to previous state
3. Document what went wrong in comments.md

## Implementation Notes

<!-- Add notes during implementation -->

---
*This plan should be updated as implementation progresses.*
