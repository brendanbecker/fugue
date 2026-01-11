# Task Breakdown: FEAT-054

**Work Item**: [FEAT-054: Pane-bounded text selection in copy mode](PROMPT.md)
**Status**: Not Started
**Last Updated**: 2026-01-11

## Prerequisites

- [ ] Read and understand PROMPT.md
- [ ] Review PLAN.md and update if needed
- [ ] Audit existing copy mode implementation
- [ ] Review FocusState::Selecting in pane.rs

## Design Tasks

- [ ] Review requirements and acceptance criteria
- [ ] Understand vt100 parser screen buffer API
- [ ] Design Selection struct and VisualMode enum
- [ ] Plan integration with existing FocusState
- [ ] Update PLAN.md with findings

## Implementation Tasks

### Selection State (pane.rs)
- [ ] Add Selection struct with anchor/cursor positions
- [ ] Add SelectionPos struct for row/col
- [ ] Add VisualMode enum (Character, Line)
- [ ] Implement Selection::normalized() for ordered start/end
- [ ] Implement Selection::contains() for hit testing
- [ ] Extend FocusState::Selecting to hold Selection
- [ ] Add internal paste buffer field

### Keyboard Input (input/mod.rs)
- [ ] Handle 'v' to enter character visual mode
- [ ] Handle 'V' to enter line visual mode
- [ ] Extend hjkl handling to update selection cursor
- [ ] Extend arrow key handling for selection
- [ ] Handle 'y' to yank and exit
- [ ] Handle 'Enter'/'Space' to copy and exit
- [ ] Handle 'Escape'/'q' to cancel and exit
- [ ] Handle '0'/'$' for line start/end movement

### Mouse Input (input/mouse.rs)
- [ ] Handle MouseDown to start selection
- [ ] Handle MouseDrag to update selection cursor
- [ ] Handle MouseUp to finalize selection
- [ ] Implement double-click word selection
- [ ] Implement triple-click line selection
- [ ] Clamp coordinates to pane boundaries
- [ ] Convert screen coords to scrollback-relative coords

### Text Extraction
- [ ] Implement extract_selection() function
- [ ] Handle character visual mode extraction
- [ ] Handle line visual mode extraction
- [ ] Access vt100 screen buffer correctly
- [ ] Handle scrollback offset in coordinates
- [ ] Handle multi-line selections
- [ ] Strip trailing whitespace (if configured)

### Clipboard Integration (app.rs)
- [ ] Add base64 dependency if needed
- [ ] Implement copy_to_system_clipboard() with OSC 52
- [ ] Store copy in internal paste buffer
- [ ] Implement paste from internal buffer (Prefix+])
- [ ] Handle clipboard errors gracefully

### Visual Rendering (pane.rs)
- [ ] Modify cell rendering to check selection
- [ ] Apply inverted/highlight style to selected cells
- [ ] Ensure highlighting works with scrollback view
- [ ] Update status bar with mode indicator
- [ ] Show "-- VISUAL --" or "-- VISUAL LINE --"

## Testing Tasks

- [ ] Unit test Selection::normalized()
- [ ] Unit test Selection::contains() for character mode
- [ ] Unit test Selection::contains() for line mode
- [ ] Unit test text extraction for single line
- [ ] Unit test text extraction for multi-line
- [ ] Integration test: v + movement + y
- [ ] Integration test: V + movement + y
- [ ] Integration test: mouse drag selection
- [ ] Integration test: double-click word selection
- [ ] Integration test: triple-click line selection
- [ ] Test selection with scrollback history
- [ ] Test selection doesn't cross pane bounds
- [ ] Test OSC 52 output format
- [ ] Manual test: verify clipboard works in terminal
- [ ] Run full test suite

## Documentation Tasks

- [ ] Document copy mode keybindings in README
- [ ] Document OSC 52 terminal requirements
- [ ] Add code comments to Selection struct
- [ ] Add code comments to extraction logic
- [ ] Update CHANGELOG

## Verification Tasks

- [ ] All acceptance criteria from PROMPT.md met
- [ ] Tests passing
- [ ] Update feature_request.json status
- [ ] Document completion in comments.md

## Completion Checklist

- [ ] All implementation tasks complete
- [ ] All tests passing
- [ ] Documentation updated
- [ ] PLAN.md reflects final implementation
- [ ] Ready for review/merge

---
*Check off tasks as you complete them. Update status field above.*
