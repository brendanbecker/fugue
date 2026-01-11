# FEAT-054: Pane-bounded text selection in copy mode

**Priority**: P2
**Component**: ccmux-client (UI/Input)
**Type**: new_feature
**Estimated Effort**: large
**Business Value**: high

## Overview

Currently, shift+click text selection is handled by the user's terminal emulator, which doesn't know about ccmux pane boundaries. This results in selection spanning across panes or including border characters.

Implement proper text selection within copy mode that respects pane boundaries.

## Current State

- Copy mode exists (`Prefix+[`) but only supports scrolling (j/k)
- `FocusState::Selecting` is defined in `ccmux-client/src/ui/pane.rs` but unused
- No selection tracking or rendering implemented

## Core Features

### 1. Keyboard Selection (vim-style)

| Key | Action |
|-----|--------|
| `v` | Enter character visual mode, select with hjkl/arrows |
| `V` | Enter line visual mode, select whole lines |
| `Space` or `Enter` | Copy selection and exit copy mode |
| `y` | Yank (copy) selection |
| `Escape` or `q` | Cancel selection and exit copy mode |

### 2. Mouse Selection

- Click and drag to select text within the pane
- Selection automatically clipped to pane boundaries
- Double-click to select word
- Triple-click to select line

### 3. Clipboard Integration

- Use OSC 52 escape sequence to copy to system clipboard
- OSC 52 format: `\x1b]52;c;BASE64_TEXT\x07`
- Works across SSH sessions (terminal must support OSC 52)
- Fallback: store in internal buffer for paste with `Prefix+]`

### 4. Visual Feedback

- Highlight selected text with inverted colors or distinct background
- Show selection mode indicator in status bar (e.g., "-- VISUAL --" or "-- VISUAL LINE --")

## Benefits

- Proper text selection that respects pane boundaries
- Familiar vim-style keybindings for power users
- Mouse support for intuitive selection
- Clipboard integration that works across SSH
- Consistent with tmux copy mode behavior

## Implementation Tasks

### Section 1: Design
- [ ] Review requirements and acceptance criteria
- [ ] Audit existing copy mode implementation in pane.rs and input handlers
- [ ] Design selection state structure (start pos, end pos, mode)
- [ ] Design text extraction from vt100 parser's screen buffer
- [ ] Document implementation approach in PLAN.md

### Section 2: Selection State
- [ ] Add selection state to pane (start row/col, end row/col, mode)
- [ ] Selection positions should be relative to scrollback buffer, not screen
- [ ] Implement `FocusState::Selecting` with visual mode tracking
- [ ] Track whether in character or line visual mode

### Section 3: Keyboard Input
- [ ] Handle `v` to enter character visual mode
- [ ] Handle `V` to enter line visual mode
- [ ] Extend hjkl/arrow handling to update selection end position
- [ ] Handle `y`, `Space`, `Enter` to yank and exit
- [ ] Handle `Escape`, `q` to cancel and exit
- [ ] Update cursor position display during selection

### Section 4: Mouse Input
- [ ] Handle mouse drag events in copy mode
- [ ] Convert mouse coordinates to pane-relative positions
- [ ] Clip selection to pane boundaries
- [ ] Handle double-click for word selection
- [ ] Handle triple-click for line selection
- [ ] Start selection on mouse down, update on drag, finalize on release

### Section 5: Text Extraction
- [ ] Extract selected text from vt100 parser screen buffer
- [ ] Handle line visual mode (select full lines)
- [ ] Handle character visual mode (select character range)
- [ ] Handle multi-line selections correctly
- [ ] Strip trailing whitespace from lines (optional, configurable)

### Section 6: Clipboard Integration
- [ ] Implement OSC 52 escape sequence output
- [ ] Base64 encode selected text
- [ ] Write OSC 52 sequence to stdout (terminal)
- [ ] Implement fallback internal paste buffer
- [ ] Handle paste with `Prefix+]` from internal buffer

### Section 7: Visual Rendering
- [ ] Modify pane rendering to highlight selected text
- [ ] Use inverted colors or distinct background style
- [ ] Update status bar to show visual mode indicator
- [ ] Ensure highlighting works with scrollback

### Section 8: Testing
- [ ] Add unit tests for selection state management
- [ ] Add unit tests for text extraction
- [ ] Add integration tests for keyboard selection
- [ ] Add integration tests for mouse selection
- [ ] Test with scrollback history
- [ ] Test selection doesn't cross pane boundaries
- [ ] Run full test suite

### Section 9: Documentation
- [ ] Document copy mode keybindings in README/help
- [ ] Document OSC 52 requirements for clipboard
- [ ] Add code comments
- [ ] Update CHANGELOG

## Acceptance Criteria

- [ ] Can enter copy mode and select text with keyboard (v + movement)
- [ ] Can enter line visual mode with V
- [ ] Can select text with mouse drag, clipped to pane bounds
- [ ] Selected text is visually highlighted during selection
- [ ] Copied text goes to system clipboard via OSC 52
- [ ] Selection does not cross pane boundaries
- [ ] Works correctly with scrollback history
- [ ] Double-click selects word, triple-click selects line
- [ ] Status bar shows visual mode indicator
- [ ] `Prefix+]` pastes from internal buffer (fallback)
- [ ] All tests passing
- [ ] No regressions in existing copy mode scrolling

## Files to Modify

| File | Changes |
|------|---------|
| `ccmux-client/src/ui/pane.rs` | Add selection state, rendering |
| `ccmux-client/src/input/mod.rs` | Handle selection keys in copy mode |
| `ccmux-client/src/input/mouse.rs` | Handle mouse drag for selection |
| `ccmux-client/src/ui/app.rs` | Coordinate selection and clipboard |

## Dependencies

- FEAT-010 (Client Input - Keyboard and Mouse Event Handling) - already complete

## Related Work

- BUG-012 (deprecated) - Text selection not working in TUI (shift+click)
- FEAT-034 (complete) - Mouse scroll support

## Technical Notes

### Selection State Structure

```rust
pub struct Selection {
    /// Start position (row, col) relative to scrollback buffer
    start: (usize, usize),
    /// End position (row, col) relative to scrollback buffer
    end: (usize, usize),
    /// Visual mode: character or line
    mode: VisualMode,
}

pub enum VisualMode {
    Character,
    Line,
}
```

### OSC 52 Clipboard

```rust
fn copy_to_clipboard(text: &str) {
    let encoded = base64::encode(text);
    // Write to stdout to send to terminal
    print!("\x1b]52;c;{}\x07", encoded);
    std::io::stdout().flush().unwrap();
}
```

### Text Extraction from vt100

The vt100 parser provides `screen.contents()` or per-cell access. Need to:
1. Determine visible rows from scrollback position
2. Extract cells in selection range
3. Convert to string, handling line breaks appropriately

## Notes

- OSC 52 clipboard requires terminal emulator support (most modern terminals do)
- Selection coordinates need careful handling when scrollback is involved
- Line visual mode should include trailing newlines for proper paste behavior
- Consider adding configuration for selection highlight style
