# Implementation Plan: FEAT-054

**Work Item**: [FEAT-054: Pane-bounded text selection in copy mode](PROMPT.md)
**Component**: ccmux-client (UI/Input)
**Priority**: P2
**Created**: 2026-01-11

## Overview

Implement text selection within copy mode that respects pane boundaries, with vim-style keyboard controls, mouse drag selection, visual highlighting, and clipboard integration via OSC 52.

## Architecture Decisions

- **Approach**: Extend existing copy mode with selection state tracking, modify pane rendering to show highlights, add OSC 52 output for clipboard
- **Trade-offs**:
  - Vim-style keybindings vs simpler selection (choosing vim-style for power user appeal and tmux parity)
  - OSC 52 vs native clipboard access (choosing OSC 52 for SSH compatibility, with internal buffer fallback)
  - Selection relative to scrollback vs screen (choosing scrollback for consistency with scroll position)

## Affected Components

| Component | Type of Change | Risk Level |
|-----------|----------------|------------|
| ccmux-client/src/ui/pane.rs | Add Selection state, modify rendering | Medium |
| ccmux-client/src/input/mod.rs | Add selection key handlers | Medium |
| ccmux-client/src/input/mouse.rs | Add drag selection handlers | Medium |
| ccmux-client/src/ui/app.rs | Coordinate selection, clipboard output | Low |

## Implementation Details

### 1. Selection State

Add to `pane.rs`:

```rust
/// Tracks text selection state within a pane
#[derive(Debug, Clone)]
pub struct Selection {
    /// Anchor position (where selection started)
    pub anchor: SelectionPos,
    /// Cursor position (current end of selection)
    pub cursor: SelectionPos,
    /// Visual mode type
    pub mode: VisualMode,
}

#[derive(Debug, Clone, Copy)]
pub struct SelectionPos {
    /// Row in scrollback buffer (0 = oldest line)
    pub row: usize,
    /// Column (0-indexed)
    pub col: usize,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VisualMode {
    /// Character-wise selection (v)
    Character,
    /// Line-wise selection (V)
    Line,
}

impl Selection {
    /// Returns (start, end) with start <= end
    pub fn normalized(&self) -> (SelectionPos, SelectionPos) {
        if self.anchor.row < self.cursor.row
            || (self.anchor.row == self.cursor.row && self.anchor.col <= self.cursor.col)
        {
            (self.anchor, self.cursor)
        } else {
            (self.cursor, self.anchor)
        }
    }

    /// Check if a position is within the selection
    pub fn contains(&self, row: usize, col: usize) -> bool {
        let (start, end) = self.normalized();
        match self.mode {
            VisualMode::Line => row >= start.row && row <= end.row,
            VisualMode::Character => {
                if row < start.row || row > end.row {
                    false
                } else if row == start.row && row == end.row {
                    col >= start.col && col <= end.col
                } else if row == start.row {
                    col >= start.col
                } else if row == end.row {
                    col <= end.col
                } else {
                    true
                }
            }
        }
    }
}
```

### 2. Integrate with FocusState

The existing `FocusState::Selecting` can hold the `Selection`:

```rust
pub enum FocusState {
    Normal,
    CopyMode { scroll_offset: usize },
    Selecting { scroll_offset: usize, selection: Selection },
}
```

### 3. Key Handling Flow

In copy mode input handler:

```
'v' -> transition CopyMode -> Selecting(Character) with anchor at cursor
'V' -> transition CopyMode -> Selecting(Line) with anchor at cursor
hjkl/arrows in Selecting -> update selection.cursor
'y'/'Enter'/Space in Selecting -> extract text, copy, exit to Normal
Escape/q in Selecting -> exit to Normal (cancel)
```

### 4. Mouse Handling Flow

```
MouseDown in pane area -> start selection at click position
MouseDrag -> update selection.cursor (clamped to pane bounds)
MouseUp -> finalize selection (optionally auto-copy)
DoubleClick -> select word at position
TripleClick -> select line at position
```

### 5. Text Extraction

```rust
fn extract_selection(
    parser: &vt100::Parser,
    selection: &Selection,
    scrollback_size: usize,
) -> String {
    let (start, end) = selection.normalized();
    let mut result = String::new();

    for row in start.row..=end.row {
        // Get row content from parser
        let row_content = get_row_content(parser, row, scrollback_size);

        let line_start = if row == start.row { start.col } else { 0 };
        let line_end = if row == end.row {
            end.col + 1
        } else {
            row_content.len()
        };

        if selection.mode == VisualMode::Line {
            result.push_str(&row_content);
            result.push('\n');
        } else {
            let slice = &row_content[line_start.min(row_content.len())..line_end.min(row_content.len())];
            result.push_str(slice);
            if row < end.row {
                result.push('\n');
            }
        }
    }

    result
}
```

### 6. Rendering Highlights

In pane rendering, check each cell against selection:

```rust
fn render_cell(row: usize, col: usize, cell: &Cell, selection: Option<&Selection>) -> Style {
    let base_style = cell.style();

    if let Some(sel) = selection {
        if sel.contains(row, col) {
            // Invert colors or use highlight background
            return base_style.reversed();
        }
    }

    base_style
}
```

### 7. OSC 52 Output

```rust
fn copy_to_system_clipboard(text: &str) -> io::Result<()> {
    use base64::{Engine as _, engine::general_purpose};
    let encoded = general_purpose::STANDARD.encode(text);

    // OSC 52: set clipboard
    // c = clipboard (vs p = primary selection)
    let osc = format!("\x1b]52;c;{}\x07", encoded);

    let mut stdout = io::stdout();
    stdout.write_all(osc.as_bytes())?;
    stdout.flush()
}
```

## Dependencies

- FEAT-010 (Input Handling) - already complete
- vt100 crate for screen buffer access
- base64 crate for OSC 52 encoding

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| OSC 52 not supported by terminal | Medium | Low | Provide internal paste buffer fallback |
| Selection coordinates off by one | Medium | Medium | Extensive testing with edge cases |
| Performance with large selections | Low | Medium | Lazy text extraction, efficient contains() check |
| Conflicts with existing mouse handling | Low | Medium | Clear state machine transitions |

## Rollback Strategy

If implementation causes issues:
1. Revert commits associated with this work item
2. Copy mode reverts to scroll-only behavior
3. Document issues in comments.md

## Testing Strategy

1. **Unit tests**: Selection state, contains(), normalized(), text extraction
2. **Integration tests**: Key sequences, mouse events, clipboard output
3. **Manual testing**: Visual verification of highlights, clipboard functionality
4. **Edge cases**: Empty selection, single char, full pane, scrollback boundary

## Implementation Notes

<!-- Add notes during implementation -->

---
*This plan should be updated as implementation progresses.*
