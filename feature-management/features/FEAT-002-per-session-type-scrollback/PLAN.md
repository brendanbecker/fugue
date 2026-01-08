# Implementation Plan: FEAT-002

**Work Item**: [FEAT-002: Per-Session-Type Scrollback Configuration](PROMPT.md)
**Component**: config
**Priority**: P1
**Created**: 2026-01-08

## Overview

Support per-session-type buffer sizes instead of global-only scrollback. Enable configuration like `scrollback: { orchestrator: 50000, worker: 500, default: 1000 }`. Also support runtime override at spawn time via `<ccmux:spawn scrollback="10000">` directive.

## Architecture Decisions

### AD-1: Scrollback Configuration Structure

**Decision**: Use nested TOML table `[terminal.scrollback]` with named session types.

**Rationale**:
- Keeps scrollback config grouped under terminal settings
- Allows arbitrary session type names via serde flatten
- Backwards compatible (single number can be interpreted as default)

**Trade-offs**:
- Slightly more complex config parsing
- Need to handle both legacy `scrollback_lines: N` and new `scrollback.default: N`

### AD-2: Buffer Data Structure

**Decision**: Use `VecDeque<String>` circular buffer with line-based storage.

**Rationale**:
- O(1) push and pop from both ends
- Natural fit for scrollback (add at end, trim from start)
- Line-based storage matches terminal semantics
- String storage preserves escape sequences for faithful replay

**Alternative Considered**: `bytes::Bytes` with custom line indexing
- More memory efficient for large buffers
- Added complexity not justified for initial implementation
- Can optimize later if profiling shows need

### AD-3: Per-Pane Override Storage

**Decision**: Store optional `scrollback_override: Option<usize>` in `Pane` struct.

**Rationale**:
- Simple and explicit
- Override takes precedence over session-type lookup
- Set at spawn time from `<ccmux:spawn scrollback="N">` attribute

### AD-4: Hot-Reload Behavior

**Decision**: New config applies only to newly spawned panes; existing panes retain their buffer size.

**Rationale**:
- Resizing existing buffers mid-session is complex (truncation decisions)
- Simpler implementation with predictable behavior
- Users can restart panes to apply new settings

## Affected Components

| Component | Type of Change | Risk Level |
|-----------|----------------|------------|
| `ccmux-server/src/config/schema.rs` | Modify existing | Low |
| `ccmux-server/src/pty/buffer.rs` | New file | Low |
| `ccmux-server/src/pty/mod.rs` | Modify existing | Low |
| `ccmux-server/src/session/pane.rs` | Modify existing | Medium |
| `ccmux-server/src/pty/handle.rs` | Modify existing | Medium |

## Dependencies

None - this feature builds on existing config and PTY infrastructure.

## Implementation Phases

### Phase 1: Config Schema (Low Risk)

1. Add `ScrollbackConfig` struct to `schema.rs`
2. Update `TerminalConfig` to use new structure
3. Add backwards compatibility for legacy `scrollback_lines` field
4. Add unit tests for config parsing

### Phase 2: Scrollback Buffer Module (Low Risk)

1. Create `ccmux-server/src/pty/buffer.rs`
2. Implement `ScrollbackBuffer` struct with `VecDeque<String>`
3. Add push, get_lines, resize operations
4. Add comprehensive unit tests

### Phase 3: Pane Integration (Medium Risk)

1. Add `scrollback_override: Option<usize>` to `Pane`
2. Add `scrollback_buffer: ScrollbackBuffer` to `Pane`
3. Update `Pane::new()` to accept scrollback size parameter
4. Add method to resolve effective scrollback size

### Phase 4: PTY Output Integration (Medium Risk)

1. Update `PtyHandle` to feed output to scrollback buffer
2. Parse output into lines (handle partial lines)
3. Integrate with existing PTY read loop

### Phase 5: Spawn Directive Parsing (Low Risk)

1. Update spawn directive parser to extract `scrollback` attribute
2. Pass scrollback override through pane creation path
3. Add integration tests

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Memory pressure from large buffers | Medium | High | Add memory tracking, warnings |
| Performance impact from buffer operations | Low | Medium | Use efficient data structures |
| Hot-reload edge cases | Low | Low | Document behavior clearly |
| Regression in existing functionality | Low | Medium | Comprehensive testing |

## Rollback Strategy

If implementation causes issues:

1. Revert commits associated with FEAT-002
2. Restore legacy `scrollback_lines` field in config
3. Remove `buffer.rs` module
4. Verify system returns to previous state
5. Document issues in comments.md

## Testing Strategy

### Unit Tests

- `ScrollbackConfig` parsing from TOML
- `ScrollbackBuffer` operations (push, get, resize, wrap-around)
- Backwards compatibility with legacy config
- Session type resolution logic

### Integration Tests

- Spawn pane with default scrollback
- Spawn pane with session-type scrollback
- Spawn pane with override scrollback
- Hot-reload config and verify new panes use new values
- Memory tracking accuracy

## Implementation Notes

<!-- Add notes during implementation -->

---
*This plan should be updated as implementation progresses.*
