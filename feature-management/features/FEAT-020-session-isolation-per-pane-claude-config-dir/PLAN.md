# Implementation Plan: FEAT-020

**Work Item**: [FEAT-020: Session Isolation - Per-Pane CLAUDE_CONFIG_DIR](PROMPT.md)
**Component**: ccmux-server
**Priority**: P1
**Created**: 2026-01-08

## Overview

CLAUDE_CONFIG_DIR per pane for concurrent Claude instances, preventing config file conflicts.

## Architecture Decisions

### Isolation Directory Structure

```
~/.ccmux/
  claude/
    {pane-id}/           # Isolation directory per pane
      config/            # Claude configuration files
      sessions/          # Session data for --resume
      .pane-metadata     # Pane ID, session ID, creation time
```

### Environment Variable Strategy

At PTY spawn, inject:
- `CLAUDE_CONFIG_DIR=~/.ccmux/claude/{pane-id}`

This overrides any existing CLAUDE_CONFIG_DIR to ensure isolation.

### Pane ID Format

Use the existing pane UUID from session management:
- Format: `{session-id}_{window-id}_{pane-index}` or raw UUID
- Must be filesystem-safe (no special characters)
- Must be stable for --resume support

### Cleanup Lifecycle

```
Pane Created -> Create Isolation Directory
     |
     v
PTY Spawned -> Inject CLAUDE_CONFIG_DIR
     |
     v
Pane Running -> Claude uses isolated config
     |
     v
Pane Closing -> Wait for Claude exit (grace period)
     |
     v
Pane Closed -> Remove Isolation Directory
```

## Affected Components

| Component | Type of Change | Risk Level |
|-----------|----------------|------------|
| ccmux-server/src/pty/config.rs | Modify - Add CLAUDE_CONFIG_DIR support | Low |
| ccmux-server/src/claude/isolation.rs | New - Isolation directory management | Low |
| ccmux-server/src/session/pane.rs | Modify - Lifecycle hooks for isolation | Low |

## Dependencies

- FEAT-013: PTY Management - Provides PtyConfig and spawn infrastructure
- FEAT-015: Additional session management features (if applicable)

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Directory not cleaned up on crash | Medium | Low | Orphan cleanup on server start |
| Permission issues | Low | Medium | Explicit chmod 700 |
| Race condition on pane close | Low | Low | Grace period before cleanup |
| Disk space accumulation | Low | Low | Periodic cleanup of old directories |

## Implementation Phases

### Phase 1: Isolation Module
- Create `ccmux-server/src/claude/isolation.rs`
- Implement directory creation/removal
- Add pane metadata file handling

### Phase 2: PTY Integration
- Extend PtyConfig with isolation_dir field
- Inject CLAUDE_CONFIG_DIR in environment
- Wire up at spawn time

### Phase 3: Lifecycle Integration
- Hook into pane creation for directory setup
- Hook into pane close for cleanup
- Implement orphan cleanup on server start

### Phase 4: Testing
- Unit tests for isolation module
- Integration tests for concurrent instances
- Crash recovery testing

## Rollback Strategy

If implementation causes issues:
1. Revert commits associated with this work item
2. Remove claude/isolation.rs module
3. Remove isolation directory injection from PtyConfig
4. Verify normal PTY spawning still works

## Implementation Notes

<!-- Add notes during implementation -->

---
*This plan should be updated as implementation progresses.*
