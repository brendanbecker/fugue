# Implementation Plan: FEAT-003

**Work Item**: [FEAT-003: Viewport Pinning with New Content Indicator](PROMPT.md)
**Component**: tui
**Priority**: P2
**Created**: 2026-01-08

## Overview

Implement viewport pinning that allows users to scroll up and review previous output without being yanked to the bottom when new content arrives. A visual indicator shows the number of new lines below, with keyboard and click navigation to jump back to the bottom.

## Architecture Decisions

### Client-Authoritative Scroll Position

**Decision**: Scroll position is managed client-side, not synchronized to server.

**Rationale**:
- Scroll position is a UI concern, not session state
- Reduces protocol overhead for a high-frequency operation
- Server only needs to know content bounds, not viewport position
- Simpler implementation with better responsiveness

**Trade-offs**:
- Multiple clients viewing same pane will have independent scroll positions (acceptable)
- Server cannot optimize output buffering based on client viewport (minor)

### Scrollback Buffer Strategy

**Decision**: Use ring buffer for scrollback with configurable line limit.

**Rationale**:
- Bounded memory usage regardless of output volume
- O(1) append and index operations
- Natural fit for terminal semantics

**Trade-offs**:
- Very old content is lost (configurable limit, default 10000 lines)
- Must handle offset invalidation when content wraps

### Indicator Overlay Approach

**Decision**: Render indicator as floating overlay, not inline content.

**Rationale**:
- Does not consume pane content space
- Can be styled independently
- Clear visual separation from actual output

**Trade-offs**:
- Requires z-ordering in render pipeline
- May obscure last line of content (acceptable with styling)

## Affected Components

| Component | Type of Change | Risk Level |
|-----------|----------------|------------|
| `fugue-protocol/src/lib.rs` | Protocol extension | Low |
| `fugue-client/src/ui.rs` | Major implementation | Medium |
| `fugue-client/src/state.rs` | New viewport state | Low |
| `fugue-server/src/session/pane.rs` | Minor updates | Low |
| `fugue-common/src/config.rs` | Config options | Low |

## Implementation Approach

### Phase 1: Viewport State Foundation
1. Define `ViewportState` struct in client
2. Add viewport tracking to pane UI state
3. Implement basic scroll offset calculation
4. Wire up mouse wheel events

### Phase 2: Content Rendering with Offset
1. Modify pane content renderer to respect offset
2. Calculate visible line range from offset
3. Handle partial line visibility at edges
4. Test with static content

### Phase 3: Pinning Logic
1. Detect scroll direction (up vs down)
2. Set `is_pinned` flag on scroll up
3. Track new lines received while pinned
4. Auto-unpin when scrolled to bottom

### Phase 4: Indicator Widget
1. Create `NewContentIndicator` widget
2. Position at bottom of pane viewport
3. Display line count with arrow icon
4. Style with attention-grabbing but subtle color

### Phase 5: Navigation Actions
1. Implement jump-to-bottom action
2. Add keyboard bindings (G, Ctrl+End)
3. Add click handler to indicator
4. Implement smooth scroll animation (optional)

### Phase 6: Configuration
1. Add `scroll_behavior` to config schema
2. Add `scrollback_lines` limit option
3. Wire configuration to viewport behavior

## Dependencies

No external dependencies. This feature builds on existing infrastructure:
- Pane dimensions already tracked
- PaneOutput messages already broadcast
- Ratatui widget system available

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Performance with rapid output | Medium | High | Debounce indicator updates, batch line counting |
| Scroll position drift on resize | Low | Medium | Recalculate offset on resize events |
| Memory growth from scrollback | Low | Medium | Ring buffer with configurable limit |
| Click detection accuracy | Low | Low | Use generous hit area for indicator |

## Rollback Strategy

If implementation causes issues:
1. Revert commits associated with FEAT-003
2. Protocol changes are additive, no backward compatibility issues
3. Config options have sensible defaults
4. Verify terminal output still functions without viewport features

## Testing Strategy

### Unit Tests
- ViewportState offset calculations
- Pinning state transitions
- Line counting accuracy
- Ring buffer behavior at wrap boundary

### Integration Tests
- Scroll up pins viewport
- New content increments counter
- Jump to bottom clears indicator
- Resize maintains relative position

### Manual Testing
- Rapid output (e.g., `yes | head -10000`)
- Mixed output rates (slow then fast)
- Multiple panes with different scroll states
- Configuration option changes

## Implementation Notes

<!-- Add notes during implementation -->

---
*This plan should be updated as implementation progresses.*
