# FEAT-020: Session Isolation - Per-Pane CLAUDE_CONFIG_DIR

**Priority**: P1
**Component**: ccmux-server
**Type**: new_feature
**Estimated Effort**: small
**Business Value**: high
**Status**: new

## Overview

CLAUDE_CONFIG_DIR per pane for concurrent Claude instances, preventing config file conflicts.

## Requirements

- Unique CLAUDE_CONFIG_DIR per pane running Claude
- Directory structure: ~/.ccmux/claude/{pane-id}/
- Environment variable injection at PTY spawn
- Cleanup of config directories on pane close
- Prevent concurrent Claude instances from conflicting
- Session ID isolation for --resume support

## Affected Files

- `ccmux-server/src/pty/config.rs`
- `ccmux-server/src/claude/isolation.rs`
- `ccmux-server/src/session/pane.rs`

## Implementation Tasks

### Section 1: Design
- [ ] Design isolation directory structure
- [ ] Plan environment variable injection strategy
- [ ] Design cleanup lifecycle hooks
- [ ] Document session ID isolation approach

### Section 2: Directory Management
- [ ] Implement isolation directory creation (~/.ccmux/claude/{pane-id}/)
- [ ] Handle directory permissions (700 for security)
- [ ] Implement directory existence checks
- [ ] Add error handling for filesystem operations

### Section 3: Environment Variable Injection
- [ ] Extend PtyConfig to support CLAUDE_CONFIG_DIR
- [ ] Inject CLAUDE_CONFIG_DIR at PTY spawn time
- [ ] Ensure variable is set before shell execution
- [ ] Handle existing CLAUDE_CONFIG_DIR (override vs preserve)

### Section 4: Pane Lifecycle Integration
- [ ] Create isolation directory on pane creation
- [ ] Clean up directory on pane close
- [ ] Handle orphaned directories on server restart
- [ ] Implement graceful cleanup (wait for Claude exit)

### Section 5: Session ID Isolation
- [ ] Generate unique session IDs per pane
- [ ] Store session metadata in isolation directory
- [ ] Support --resume with isolated session
- [ ] Prevent session ID collisions

### Section 6: Testing
- [ ] Unit tests for isolation directory management
- [ ] Integration tests for concurrent Claude instances
- [ ] Test cleanup on normal pane close
- [ ] Test cleanup on abnormal termination
- [ ] Test --resume functionality with isolation

## Acceptance Criteria

- [ ] Each pane running Claude has unique CLAUDE_CONFIG_DIR
- [ ] Multiple concurrent Claude instances do not conflict
- [ ] Isolation directories are cleaned up on pane close
- [ ] Server restart cleans up orphaned directories
- [ ] --resume works correctly with isolated sessions
- [ ] All tests passing

## Dependencies

- FEAT-013: PTY Management - Process Spawning and Lifecycle (provides PTY spawning infrastructure)
- FEAT-015: (referenced dependency for session management)

## Notes

- Directory structure follows XDG conventions (~/.ccmux/)
- Permissions should be restrictive (700) for security
- Consider cleanup strategies for crash scenarios
- Session ID should be stable for --resume support
