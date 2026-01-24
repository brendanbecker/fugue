# Task Breakdown: FEAT-038

**Work Item**: [FEAT-038: Split Pane Rendering - Layout Manager for Multi-Pane Display](PROMPT.md)
**Status**: Not Started
**Last Updated**: 2026-01-09

## Prerequisites

- [ ] Read and understand PROMPT.md
- [ ] Review PLAN.md and update if needed
- [ ] Review existing layout code in `fugue-client/src/ui/layout.rs`
- [ ] Review current `draw_attached()` in `fugue-client/src/ui/app.rs`
- [ ] Understand how `PaneManager` and `render_pane` work in `fugue-client/src/ui/pane.rs`
- [ ] Review how split commands are handled (`SplitVertical`, `SplitHorizontal`)
- [ ] Review how `PaneCreated` messages are processed

## Design Tasks

- [ ] Decide on border rendering approach (Block widget vs custom)
- [ ] Decide on minimum pane size handling
- [ ] Plan active pane highlighting colors
- [ ] Document final approach in PLAN.md

## Implementation Tasks

### Phase 1: Add LayoutManager to App (app.rs)

- [ ] Add `use super::layout::{LayoutManager, SplitDirection as LayoutSplitDirection};`
- [ ] Add `layout: LayoutManager` field to `App` struct
- [ ] Add `pending_split_direction: Option<SplitDirection>` field
- [ ] Initialize `layout` in `App::new()` with a placeholder UUID
- [ ] In `Attached` handler, reinitialize layout with first pane ID
- [ ] Verify existing functionality still works (single pane)

### Phase 2: Track Split Direction (app.rs)

- [ ] In `handle_client_command` for `SplitVertical`:
  - Store `Some(SplitDirection::Vertical)` in `pending_split_direction`
- [ ] In `handle_client_command` for `SplitHorizontal`:
  - Store `Some(SplitDirection::Horizontal)` in `pending_split_direction`
- [ ] In `handle_server_message` for `PaneCreated`:
  - Take `pending_split_direction` with `.take()`
  - Default to `Vertical` if None
  - Call `layout.root_mut().add_pane(active_id, new_id, direction)`
- [ ] Handle case where `active_pane_id` is None

### Phase 3: Add root_mut() to LayoutManager (layout.rs)

- [ ] Add `pub fn root_mut(&mut self) -> &mut LayoutNode` method
- [ ] Ensure method is accessible from app.rs

### Phase 4: Multi-Pane Rendering (app.rs)

- [ ] Rewrite `draw_attached()`:
  - [ ] Get pane rects: `let pane_rects = self.layout.calculate_rects(pane_area);`
  - [ ] Iterate: `for (pane_id, rect) in pane_rects`
  - [ ] Check if active: `let is_active = Some(pane_id) == self.active_pane_id;`
  - [ ] Get ui_pane: `if let Some(ui_pane) = self.pane_manager.get(pane_id)`
  - [ ] Render border with appropriate style
  - [ ] Calculate inner rect (subtract border)
  - [ ] Call `render_pane()` with inner rect
- [ ] Handle empty layout case (no panes)

### Phase 5: Border Rendering (app.rs or pane.rs)

- [ ] Create helper function `render_pane_with_border()`
- [ ] Active pane border: `Style::default().fg(Color::Cyan)`
- [ ] Inactive pane border: `Style::default().fg(Color::DarkGray)`
- [ ] Include pane title or index in border
- [ ] Ensure border doesn't overlap content

### Phase 6: Resize All Panes

- [ ] In `AppEvent::Resize` handler:
  - [ ] Calculate all pane rects with new size
  - [ ] For each pane, resize terminal emulator
  - [ ] Send Resize message to server for each pane
- [ ] Create helper: `async fn resize_all_panes(&mut self) -> Result<()>`
- [ ] Call after layout changes (add/remove pane)
- [ ] Call in `Attached` handler after setting up layout

### Phase 7: Pane Navigation Integration

- [ ] Update `cycle_pane()` to use `layout.next_pane()` / `layout.prev_pane()`
- [ ] Sync `active_pane_id` with `layout.active_pane_id()`
- [ ] Update `set_active_pane()` calls to also update layout
- [ ] Test navigation updates border highlighting

### Phase 8: Pane Closure Handling

- [ ] In `PaneClosed` handler:
  - [ ] Call `layout.remove_pane(pane_id)`
  - [ ] If layout is now empty, handle appropriately
  - [ ] Trigger resize for remaining panes
- [ ] Ensure active pane updates if closed pane was active

## Testing Tasks

### Unit Tests

- [ ] Test layout integration doesn't break existing tests
- [ ] Test `pending_split_direction` state management
- [ ] Test border color selection logic

### Integration Tests

- [ ] Test: Attach to session initializes layout correctly
- [ ] Test: SplitVertical command sets pending direction
- [ ] Test: PaneCreated adds pane to layout
- [ ] Test: PaneClosed removes pane from layout
- [ ] Test: Multiple splits create correct tree structure

### Manual Testing

- [ ] Start fugue with single pane - verify rendering
- [ ] Press `Ctrl+B %` - verify vertical split appears
- [ ] Press `Ctrl+B "` - verify horizontal split appears
- [ ] Press `Ctrl+B o` - verify focus switches and borders update
- [ ] Close a pane - verify sibling expands
- [ ] Resize terminal - verify all panes resize proportionally
- [ ] Create nested splits (split a split) - verify correct layout

## Documentation Tasks

- [ ] Update PLAN.md with final implementation details
- [ ] Add code comments for new layout integration
- [ ] Document border rendering approach
- [ ] Add any gotchas or known limitations

## Verification Tasks

- [ ] All acceptance criteria from PROMPT.md met:
  - [ ] `Ctrl+B %` splits vertically, shows both panes
  - [ ] `Ctrl+B "` splits horizontally, shows both panes
  - [ ] Active pane has highlighted border
  - [ ] Inactive panes have dim border
  - [ ] `Ctrl+B o` switches focus correctly
  - [ ] Pane titles shown in borders
  - [ ] Closing pane causes sibling to expand
  - [ ] Terminal resize updates all panes
- [ ] No regressions in single-pane functionality
- [ ] All tests passing
- [ ] Update feature_request.json status

## Completion Checklist

- [ ] All implementation tasks complete
- [ ] All tests passing
- [ ] Code reviewed (self-review)
- [ ] PLAN.md updated with implementation notes
- [ ] Manual testing completed
- [ ] Ready for merge

---
*Check off tasks as you complete them. Update status field above.*
