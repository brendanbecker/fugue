# FEAT-034: Mouse Scroll Support

**Priority**: P2
**Component**: fugue-client
**Type**: enhancement
**Estimated Effort**: small
**Business Value**: medium
**Status**: new

## Overview

Scrolling with the scrollwheel/trackpad doesn't work in fugue. Users should be able to scroll through terminal output using mouse scroll events in the pane output view.

## Problem Statement

Currently, fugue has infrastructure for mouse scroll handling but the scroll events are not being applied to the pane viewport. The code path exists:

1. `terminal.rs` enables mouse capture via `EnableMouseCapture`
2. `event.rs` captures `MouseEvent` from crossterm's `EventStream`
3. `mouse.rs` handles `MouseEventKind::ScrollUp/ScrollDown` and returns `InputAction::ScrollUp/ScrollDown`
4. `app.rs` receives these actions in `handle_input_action()`

However, the scroll actions only update the `InputHandler::scroll_offset` internal state and send viewport messages to the server - they don't update the local UI pane's scroll position for immediate visual feedback.

## Current Architecture Analysis

### Mouse Event Flow

```
CrosstermEvent::Mouse(event)
    |
    v
event.rs: InputEvent::Mouse(mouse)
    |
    v
app.rs: handle_input() -> CrosstermEvent::Mouse(mouse)
    |
    v
InputHandler::handle_event() -> handle_mouse()
    |
    v
mouse.rs: handle_mouse_event() -> InputAction::ScrollUp/ScrollDown { lines }
    |
    v
app.rs: handle_input_action()
    |
    v
Currently: Sends ClientMessage::SetViewportOffset to server
Missing: Does NOT update local Pane::scroll_offset
```

### Relevant Files

| File | Purpose |
|------|---------|
| `fugue-client/src/ui/terminal.rs` | Enables `EnableMouseCapture` on init |
| `fugue-client/src/ui/event.rs` | Routes `MouseEvent` to `InputEvent::Mouse` |
| `fugue-client/src/input/mouse.rs` | Converts scroll events to `InputAction::ScrollUp/Down` |
| `fugue-client/src/input/mod.rs` | `InputHandler` with `scroll_offset` state (copy mode only) |
| `fugue-client/src/ui/pane.rs` | `Pane` struct with `scroll_offset` and `scroll_up()/scroll_down()` methods |
| `fugue-client/src/ui/app.rs` | `handle_input_action()` processes scroll actions |

### Key Code Sections

**mouse.rs - Scroll Event Handling** (already works):
```rust
MouseEventKind::ScrollUp => {
    match mode {
        InputMode::Copy => InputAction::ScrollUp { lines: DEFAULT_SCROLL_LINES },
        _ => InputAction::ScrollUp { lines: DEFAULT_SCROLL_LINES }, // Normal mode too!
    }
}
```

**pane.rs - Scroll Methods** (exist but not used for mouse scroll):
```rust
pub fn scroll_up(&mut self, lines: usize) {
    let max_scroll = self.parser.screen().scrollback();
    self.scroll_offset = (self.scroll_offset + lines).min(max_scroll);
}

pub fn scroll_down(&mut self, lines: usize) {
    self.scroll_offset = self.scroll_offset.saturating_sub(lines);
}
```

**app.rs - Current Scroll Action Handler** (missing local pane update):
```rust
InputAction::ScrollUp { lines } => {
    if let Some(pane_id) = self.active_pane_id {
        let new_offset = self.input_handler.scroll_offset();
        self.connection
            .send(ClientMessage::SetViewportOffset { pane_id, offset: new_offset })
            .await?;
        let _ = lines; // Not used!
    }
}
```

## Solution

The fix is straightforward: update the local UI pane's scroll position in `handle_input_action()` when processing scroll events.

### Implementation

Modify `app.rs` `handle_input_action()` to update the local pane:

```rust
InputAction::ScrollUp { lines } => {
    if let Some(pane_id) = self.active_pane_id {
        // Update LOCAL UI pane for immediate visual feedback
        if let Some(pane) = self.pane_manager.get_mut(pane_id) {
            pane.scroll_up(lines);
        }

        // Optionally notify server (for state sync with other clients)
        let new_offset = self.pane_manager.get(pane_id)
            .map(|p| p.scroll_offset)
            .unwrap_or(0);
        self.connection
            .send(ClientMessage::SetViewportOffset { pane_id, offset: new_offset })
            .await?;
    }
}

InputAction::ScrollDown { lines } => {
    if let Some(pane_id) = self.active_pane_id {
        // Update LOCAL UI pane for immediate visual feedback
        if let Some(pane) = self.pane_manager.get_mut(pane_id) {
            pane.scroll_down(lines);
        }

        // Notify server
        let new_offset = self.pane_manager.get(pane_id)
            .map(|p| p.scroll_offset)
            .unwrap_or(0);
        if new_offset == 0 {
            self.connection
                .send(ClientMessage::JumpToBottom { pane_id })
                .await?;
        } else {
            self.connection
                .send(ClientMessage::SetViewportOffset { pane_id, offset: new_offset })
                .await?;
        }
    }
}
```

### tui-term Integration

The `Pane` struct uses `tui_term::vt100::Parser` for terminal emulation. When we call `pane.scroll_up()` or `pane.scroll_down()`, we're updating `self.scroll_offset` which affects what the `PseudoTerminal` widget renders.

**Important**: The current `render_pane()` function in `pane.rs` creates a `PseudoTerminal` but may not be passing the scroll offset. Need to verify `tui-term` supports scrollback rendering with offset.

Looking at `pane.rs`:
```rust
let pseudo_term = PseudoTerminal::new(pane.screen())
    .style(Style::default().fg(Color::White).bg(Color::Black));
```

The `PseudoTerminal` widget from `tui-term` renders the `Screen` state. The scroll offset is managed by the `vt100::Parser` which has a scrollback buffer. We need to verify if:
1. The Parser stores scrollback correctly (it does - 1000 lines configured)
2. The PseudoTerminal can render with a scroll offset (need to check tui-term API)

### tui-term Scrollback API

Looking at `tui-term` crate, the `Screen` struct from `vt100` has:
- `scrollback()` - returns number of scrollback lines
- We can get scrollback content via the Parser

The `PseudoTerminal` widget may need to be told to render at a specific scroll offset. If the current tui-term doesn't support this, we have alternatives:

1. **If tui-term supports scroll offset**: Pass offset to widget
2. **If not**: May need to manually render scrollback lines

Based on current code, the `Pane::scroll_offset` field exists but isn't being used in rendering. The `is_scrolled()` method is only used for UI indicators (showing "scrolled up" in title).

## Investigation Needed

1. **tui-term scroll API**: Does `PseudoTerminal` support rendering with scroll offset?
   - Check tui-term docs/source for scroll-related methods
   - May need to use `Screen::contents_between()` or similar

2. **vt100 scrollback access**: How to get scrollback lines from Parser?
   - `parser.screen().scrollback()` returns count
   - Need method to get actual scrollback content

3. **Alternative approach**: If tui-term doesn't support scroll offset rendering:
   - Manually extract rows from screen + scrollback
   - Render using raw ratatui widgets instead of PseudoTerminal

## Files to Modify

| File | Changes |
|------|---------|
| `fugue-client/src/ui/app.rs` | Update `handle_input_action()` to scroll local pane |
| `fugue-client/src/ui/pane.rs` | Possibly update `render_pane()` to use scroll offset |

## Implementation Tasks

### Section 1: Investigation
- [ ] Review tui-term source/docs for scroll offset rendering support
- [ ] Test if PseudoTerminal respects Parser's scroll state
- [ ] Determine if custom scrollback rendering is needed

### Section 2: Core Implementation
- [ ] Modify `handle_input_action()` in app.rs to call `pane.scroll_up()/scroll_down()`
- [ ] Ensure scroll offset is used in pane rendering
- [ ] Test mouse scroll works with scrollback content

### Section 3: Rendering Integration
- [ ] If needed: Update `render_pane()` to pass scroll offset to renderer
- [ ] If needed: Implement custom scrollback rendering
- [ ] Ensure scroll indicator shows correct offset

### Section 4: Edge Cases
- [ ] Handle scroll at boundaries (top/bottom)
- [ ] Reset scroll to bottom on new output (already done in `process_output`)
- [ ] Handle empty scrollback (no-op)

### Section 5: Testing
- [ ] Manual test: Generate scrollback with output, then scroll up/down
- [ ] Verify scroll indicator updates in title bar
- [ ] Test scroll bounds (can't scroll past top, scrolls to bottom)
- [ ] Test scroll reset on new output

## Acceptance Criteria

- [ ] Mouse scroll up moves viewport up through scrollback history
- [ ] Mouse scroll down moves viewport down (toward live output)
- [ ] Scroll indicator in pane border shows current scroll position
- [ ] Scrolling to bottom returns to live-follow mode
- [ ] New output while scrolled shows "new content" indicator (FEAT-003 enhancement)
- [ ] Scroll works with both scrollwheel and trackpad gestures
- [ ] No visual lag or jank during scrolling

## Dependencies

- **FEAT-010** (Client Input - Keyboard and Mouse Event Handling): Mouse events must be captured

## Related Features

- **FEAT-003** (Viewport Pinning with New Content Indicator): Shows indicator when new content arrives while scrolled
- **FEAT-014** (Terminal Parsing - ANSI/VT100 State Machine): Parser maintains scrollback buffer

## Technical Notes

### Scroll Lines Configuration

Currently hardcoded in `mouse.rs`:
```rust
const DEFAULT_SCROLL_LINES: usize = 3;
```

This could be made configurable via `config.toml` in the future.

### Server vs Client Scroll State

The architecture has both client-side and server-side scroll state:
- **Client**: `Pane::scroll_offset` in UI pane (immediate visual feedback)
- **Server**: `ScrollbackBuffer::viewport_offset` (persistent state, shared across clients)

For mouse scroll, we update client-side first for responsiveness, then sync to server. This matches how tmux handles scroll mode.

### Copy Mode Integration

The `InputHandler` has its own `scroll_offset` for copy mode. This should be kept in sync with the pane's scroll offset, or we should use only one source of truth. Current recommendation: use `Pane::scroll_offset` as the single source of truth for UI rendering.
