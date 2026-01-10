# FEAT-038: Split Pane Rendering - Layout Manager for Multi-Pane Display

**Priority**: P1
**Component**: ccmux-client
**Type**: new_feature
**Estimated Effort**: large
**Business Value**: high
**Status**: new

## Overview

Implement split pane rendering in the ccmux client so that when a user splits a pane (`Ctrl+B %` for vertical, `Ctrl+B "` for horizontal), both panes are displayed side-by-side (or stacked) rather than requiring manual pane switching with `Ctrl+B o`.

Currently:
- The server correctly creates panes and spawns PTY processes
- The client receives `PaneCreated` messages and stores pane data in `self.panes`
- BUT: `draw_attached()` in `ccmux-client/src/ui/app.rs` only renders ONE pane - the `active_pane_id`
- There is no layout manager tracking pane arrangement to divide the screen

## Current State

### What Works

1. **Server-side pane creation**: When `CreatePane` is sent with a `SplitDirection`, the server creates a new pane with a PTY.

2. **Client pane tracking**: The client's `App` struct has:
   - `panes: HashMap<Uuid, PaneInfo>` - stores all panes
   - `active_pane_id: Option<Uuid>` - the currently focused pane
   - `pane_manager: PaneManager` - manages terminal emulators for each pane

3. **Layout module exists**: `ccmux-client/src/ui/layout.rs` contains a full `LayoutManager` with:
   - `LayoutNode` enum for tree-based layouts
   - `add_pane()`, `remove_pane()`, `calculate_rects()` methods
   - Support for horizontal/vertical splits
   - Active pane tracking

### What's Missing

1. **Layout manager not used**: `App` doesn't use `LayoutManager` - it was written but never integrated.

2. **`draw_attached()` renders single pane**: The function only renders `active_pane_id`:
   ```rust
   fn draw_attached(&self, frame: &mut ratatui::Frame, area: Rect) {
       // ... layout setup ...

       // Only renders ONE pane
       if let Some(pane_id) = self.active_pane_id {
           if let Some(ui_pane) = self.pane_manager.get(pane_id) {
               render_pane(ui_pane, pane_area, frame.buffer_mut(), self.tick_count);
           }
       }
   }
   ```

3. **Split direction not used**: When `PaneCreated` is received, the direction is not used to update any layout structure.

4. **No pane borders**: Multiple panes need visual separation with borders.

5. **No active pane highlighting**: The focused pane should have a distinct border color.

## Requirements

### 1. Integrate LayoutManager with App

Add `LayoutManager` to the `App` struct and initialize it with the first pane:

```rust
pub struct App {
    // ... existing fields ...
    /// Layout manager for pane arrangement
    layout: LayoutManager,
}
```

### 2. Update Layout on Pane Creation

When `PaneCreated` message is received:

1. Determine which pane was split (the source pane)
2. Add the new pane to `LayoutManager` with the correct split direction
3. The layout tree should reflect the spatial relationship

Challenge: The `PaneCreated` message doesn't include the source pane ID or split direction. May need to:
- Track "pending splits" from `CreatePane` commands
- OR: Enhance the protocol to include source pane and direction in `PaneCreated`
- OR: Always add to the active pane's position

### 3. Render All Visible Panes

Update `draw_attached()` to:

1. Call `layout.calculate_rects(pane_area)` to get rectangles for all panes
2. Iterate through all pane rectangles
3. Render each pane's terminal content in its designated rectangle
4. Draw borders around each pane
5. Highlight the active pane's border

```rust
fn draw_attached(&self, frame: &mut ratatui::Frame, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(1)])
        .split(area);

    let pane_area = chunks[0];
    let pane_rects = self.layout.calculate_rects(pane_area);

    for (pane_id, rect) in pane_rects {
        let is_active = Some(pane_id) == self.active_pane_id;
        if let Some(ui_pane) = self.pane_manager.get(pane_id) {
            render_pane_with_border(ui_pane, rect, frame.buffer_mut(),
                                    self.tick_count, is_active);
        }
    }

    // Status bar
    let status = self.build_status_bar();
    let status_widget = Paragraph::new(status).style(Style::default().bg(Color::DarkGray));
    frame.render_widget(status_widget, chunks[1]);
}
```

### 4. Handle Split Direction from CreatePane

When handling `ClientCommand::SplitVertical` or `ClientCommand::SplitHorizontal`:

1. Store the intended split direction
2. When `PaneCreated` is received, use that direction to update the layout

```rust
// In handle_client_command:
ClientCommand::SplitVertical => {
    self.pending_split_direction = Some(SplitDirection::Vertical);
    // ... send CreatePane ...
}

// In handle_server_message for PaneCreated:
ServerMessage::PaneCreated { pane } => {
    let direction = self.pending_split_direction.take()
        .unwrap_or(SplitDirection::Vertical);

    if let Some(active_id) = self.active_pane_id {
        self.layout.root_mut().add_pane(active_id, pane.id, direction.into());
    }
    // ... existing pane setup ...
}
```

### 5. Resize Panes to Their Layout Rectangles

When pane rects are calculated, resize each pane's terminal emulator:

```rust
for (pane_id, rect) in &pane_rects {
    // Account for border (1 cell on each side)
    let inner_width = rect.width.saturating_sub(2);
    let inner_height = rect.height.saturating_sub(2);

    self.pane_manager.resize_pane(*pane_id, inner_height, inner_width);

    // Notify server of resize
    self.connection.send(ClientMessage::Resize {
        pane_id: *pane_id,
        cols: inner_width,
        rows: inner_height,
    }).await?;
}
```

### 6. Visual Borders and Active Indicator

Pane borders should:
- Use single-line borders (Box-drawing characters)
- Show pane index or title in border
- Active pane: bright/bold border color (e.g., Cyan)
- Inactive panes: dim border color (e.g., DarkGray)

### 7. Handle Pane Closure

When a pane is closed:
1. Remove from `LayoutManager`
2. Remaining sibling should expand to fill the space
3. If active pane was closed, select another pane

## Affected Files

| File | Change |
|------|--------|
| `ccmux-client/src/ui/app.rs` | Add LayoutManager, update draw_attached(), handle splits |
| `ccmux-client/src/ui/layout.rs` | May need adjustments based on integration |
| `ccmux-client/src/ui/pane.rs` | Add border rendering variant |
| `ccmux-client/src/ui/mod.rs` | Ensure layout module is properly exported |

## Existing Code Reference

### layout.rs Key APIs

```rust
impl LayoutManager {
    pub fn new(pane_id: Uuid) -> Self;
    pub fn calculate_rects(&self, area: Rect) -> Vec<(Uuid, Rect)>;
    pub fn active_pane_id(&self) -> Option<Uuid>;
    pub fn set_active_pane(&mut self, pane_id: Uuid);
    pub fn next_pane(&mut self);
    pub fn prev_pane(&mut self);
}

impl LayoutNode {
    pub fn add_pane(&mut self, target_pane_id: Uuid, new_pane_id: Uuid,
                    direction: SplitDirection) -> bool;
    pub fn remove_pane(&mut self, pane_id: Uuid) -> bool;
}
```

### SplitDirection Conversion

The layout module already has a `From` impl:
```rust
impl From<ccmux_protocol::SplitDirection> for SplitDirection {
    fn from(dir: ccmux_protocol::SplitDirection) -> Self {
        match dir {
            ccmux_protocol::SplitDirection::Horizontal => SplitDirection::Horizontal,
            ccmux_protocol::SplitDirection::Vertical => SplitDirection::Vertical,
        }
    }
}
```

## Implementation Tasks

### Section 1: LayoutManager Integration
- [ ] Add `layout: LayoutManager` field to App struct
- [ ] Initialize layout with first pane in `Attached` handler
- [ ] Add `pending_split_direction: Option<SplitDirection>` field
- [ ] Store direction when handling split commands
- [ ] Update layout when PaneCreated is received
- [ ] Handle pane removal in layout

### Section 2: Multi-Pane Rendering
- [ ] Update `draw_attached()` to use `layout.calculate_rects()`
- [ ] Render all panes from the rect list
- [ ] Add border rendering around each pane
- [ ] Implement active pane highlighting (different border color)
- [ ] Ensure proper clipping to pane boundaries

### Section 3: Resize Handling
- [ ] Calculate inner dimensions (accounting for borders)
- [ ] Resize terminal emulators when layout changes
- [ ] Send resize messages to server for each pane
- [ ] Handle terminal resize event for all panes

### Section 4: Pane Navigation Integration
- [ ] Use `layout.next_pane()` for `NextPane` command
- [ ] Use `layout.prev_pane()` for `PreviousPane` command
- [ ] Sync `active_pane_id` with `layout.active_pane_id()`
- [ ] Visual feedback when switching panes

### Section 5: Testing
- [ ] Test single pane rendering (no regression)
- [ ] Test vertical split creates side-by-side panes
- [ ] Test horizontal split creates stacked panes
- [ ] Test nested splits (split a split)
- [ ] Test pane closure collapses layout
- [ ] Test resize distributes space correctly
- [ ] Test active pane indicator updates on switch

## Acceptance Criteria

- [ ] When user presses `Ctrl+B %`, screen splits vertically showing both panes
- [ ] When user presses `Ctrl+B "`, screen splits horizontally showing both panes
- [ ] Active pane has a highlighted border (e.g., cyan)
- [ ] Inactive panes have a dim border (e.g., dark gray)
- [ ] `Ctrl+B o` switches focus and updates border highlighting
- [ ] Pane titles/indices shown in borders
- [ ] Closing a pane causes sibling to expand
- [ ] Terminal resize updates all pane sizes proportionally
- [ ] No regressions in single-pane functionality
- [ ] All tests passing

## Dependencies

- **FEAT-025**: Pane Output Rendering (provides render_pane and PaneManager)
- **FEAT-030**: Sideband Pane Splitting (server-side split implementation)

## Notes

- The LayoutManager in layout.rs is already quite complete - this feature is primarily about integration
- Consider whether to track split relationships in the protocol (source pane, direction)
- Future enhancement: mouse click on pane to focus
- Future enhancement: resize panes with mouse drag on border
- The status bar should show total pane count (already does)
