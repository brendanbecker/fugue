# Implementation Plan: FEAT-038

**Work Item**: [FEAT-038: Split Pane Rendering - Layout Manager for Multi-Pane Display](PROMPT.md)
**Component**: ccmux-client
**Priority**: P1
**Created**: 2026-01-09

## Overview

Integrate the existing `LayoutManager` from `layout.rs` into the `App` struct and update `draw_attached()` to render multiple panes based on their layout positions. Currently the layout code exists but is unused - all pane rendering goes through a single `active_pane_id`.

## Architecture Decisions

### Approach: Integrate Existing LayoutManager

The `LayoutManager` in `ccmux-client/src/ui/layout.rs` already provides:
- Tree-based layout with `LayoutNode::Pane` and `LayoutNode::Split`
- `add_pane(target, new, direction)` for splitting
- `remove_pane()` for closure
- `calculate_rects(area)` returning `Vec<(Uuid, Rect)>`
- Active pane tracking and navigation

The implementation focuses on **integration**, not new layout logic.

### State Tracking for Splits

**Problem**: `PaneCreated` message doesn't include the source pane or split direction.

**Solution**: Track pending splits in App:
```rust
struct App {
    // ...
    pending_split_direction: Option<SplitDirection>,
}
```

When `SplitVertical` or `SplitHorizontal` command is handled, store the direction.
When `PaneCreated` arrives, use that direction to add the pane relative to `active_pane_id`.

### Rendering Flow

```
draw_attached(frame, area)
    |
    v
Split area into [pane_area, status_bar]
    |
    v
layout.calculate_rects(pane_area) -> Vec<(Uuid, Rect)>
    |
    v
For each (pane_id, rect):
    |
    +-> Get ui_pane from pane_manager
    |
    +-> Calculate inner_rect (subtract border)
    |
    +-> Render border with active indicator
    |
    +-> render_pane(ui_pane, inner_rect, ...)
    |
    v
Render status bar
```

### Border Rendering

Each pane needs a border. Options:

**Option A: Block widget per pane**
- Wrap each pane in a `Block::default().borders(Borders::ALL)`
- Simple but creates slight overhead

**Option B: Custom border drawing**
- Draw border characters directly to buffer
- More control over shared edges

**Recommendation**: Option A for simplicity. Ratatui handles overlapping borders reasonably.

### Resize Strategy

When terminal size changes:
1. Recalculate all pane rects via `layout.calculate_rects(new_area)`
2. For each pane, resize the terminal emulator to inner dimensions
3. Send `Resize` messages to server for each pane

This happens in both:
- `AppEvent::Resize` handler
- After layout changes (add/remove pane)

## Affected Components

| Component | Type of Change | Risk Level |
|-----------|----------------|------------|
| `ccmux-client/src/ui/app.rs` | Major - add layout, rewrite draw_attached | Medium |
| `ccmux-client/src/ui/pane.rs` | Minor - may add border variant | Low |
| `ccmux-client/src/ui/layout.rs` | Minor - may need root_mut() accessor | Low |

## Dependencies

- FEAT-025 (Pane Output Rendering) - provides PaneManager and render_pane
- FEAT-030 (Sideband Pane Splitting) - server creates panes on split

Both are marked as "new" but the client already handles `PaneCreated` messages, so this feature can proceed.

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Layout calculation performance | Low | Low | LayoutManager is already efficient |
| Border overlap issues | Medium | Low | Use standard Ratatui Block |
| Resize race conditions | Low | Medium | Debounce resize events |
| Active pane sync issues | Medium | Medium | Single source of truth in layout |
| Protocol limitations (no source pane) | High | Medium | Track pending splits locally |

## Implementation Phases

### Phase 1: Add LayoutManager to App
1. Add `layout: LayoutManager` field
2. Add `pending_split_direction: Option<SplitDirection>` field
3. Initialize layout with first pane when attached
4. Keep existing single-pane rendering working initially

### Phase 2: Track Splits
1. Store direction in `handle_client_command` for split commands
2. Use stored direction in `PaneCreated` handler
3. Call `layout.root.add_pane()` with correct parameters
4. Handle pane closure with `layout.remove_pane()`

### Phase 3: Multi-Pane Rendering
1. Rewrite `draw_attached()` to iterate over layout rects
2. Add border rendering for each pane
3. Differentiate active vs inactive pane borders
4. Ensure proper inner area calculation

### Phase 4: Resize and Navigation
1. Update resize handler to resize all panes
2. Integrate `layout.next_pane()` / `prev_pane()` with navigation commands
3. Sync `active_pane_id` with layout's active tracking
4. Test pane focus switching

### Phase 5: Polish and Testing
1. Add pane titles/indices to borders
2. Test edge cases (single pane, many splits, rapid resizes)
3. Performance testing with multiple panes
4. Fix any visual artifacts

## Open Questions

1. **Shared borders**: Should adjacent panes share a border line or have separate borders?
   - Separate borders (Option A) is simpler
   - Shared borders look cleaner but require custom drawing

2. **Minimum pane size**: What happens when terminal is too small?
   - Could hide panes below minimum
   - Could show "terminal too small" message
   - Recommend: allow tiny panes, let user resize terminal

3. **Pane index in border**: Show index like tmux?
   - Useful for `select-pane -t N` commands
   - Requires tracking stable indices

## Testing Strategy

1. **Unit Tests** (layout.rs already has good coverage):
   - Verify add_pane produces correct tree
   - Verify calculate_rects divides space correctly
   - Verify remove_pane collapses correctly

2. **Integration Tests**:
   - App creates layout on attach
   - Split commands update layout
   - PaneCreated adds to layout
   - PaneClosed removes from layout

3. **Manual Testing**:
   - Visual verification of split rendering
   - Border colors correct
   - Resize behavior
   - Navigation between panes

## Rollback Strategy

If implementation causes issues:
1. Keep `LayoutManager` code but don't use it
2. Revert `draw_attached()` to single-pane rendering
3. Keep `pending_split_direction` for future use
4. Document issues in comments.md

## Implementation Notes

<!-- Add notes during implementation -->

---
*This plan should be updated as implementation progresses.*
