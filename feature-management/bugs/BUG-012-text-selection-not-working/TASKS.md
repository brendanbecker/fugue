# Task Breakdown: BUG-012

**Work Item**: [BUG-012: Text Selection Not Working in TUI](PROMPT.md)
**Status**: Not Started
**Last Updated**: 2026-01-10

## Prerequisites

- [ ] Read and understand PROMPT.md
- [ ] Review PLAN.md and update if needed
- [ ] Understand ccmux mouse event handling (review FEAT-034)

## Investigation Tasks

### Understand Current Mouse Handling

- [ ] Read `ccmux-client/src/input/mod.rs` - event handling
- [ ] Find where mouse capture is enabled (crossterm)
- [ ] Find where mouse events are processed
- [ ] Review FEAT-034 implementation for mouse scroll
- [ ] Identify what mouse events are currently handled
- [ ] Check if any selection infrastructure exists

### Document Findings

- [ ] Document current mouse event flow in PLAN.md
- [ ] Identify gaps (what's missing for selection)
- [ ] List required changes for each solution approach
- [ ] Update PLAN.md with chosen solution

### Solution Decision

- [ ] Evaluate Copy Mode approach
- [ ] Evaluate Native Passthrough approach
- [ ] Evaluate Hybrid approach
- [ ] Choose solution and document rationale in PLAN.md

## Implementation Tasks

### If Implementing Copy Mode (Recommended)

#### State Management

- [ ] Add `CopyMode` enum variant to UI mode state
- [ ] Add `CopyModeState` struct (cursor position, selection start/end)
- [ ] Add `copy_buffer: Option<String>` to app state
- [ ] Implement state transitions (enter/exit copy mode)

#### Keybindings

- [ ] Add `Prefix+[` binding to enter copy mode
- [ ] Add navigation bindings in copy mode:
  - [ ] `h/j/k/l` - cursor movement
  - [ ] `w/b` - word movement
  - [ ] `0/$` - line start/end
  - [ ] `g/G` - document start/end
  - [ ] `Ctrl+u/d` - half page up/down
- [ ] Add selection bindings:
  - [ ] `v` or `Space` - start visual selection
  - [ ] `V` - line selection mode
- [ ] Add copy binding:
  - [ ] `y` or `Enter` - copy selection and exit
- [ ] Add exit bindings:
  - [ ] `q` or `Escape` - exit copy mode
- [ ] Add paste binding:
  - [ ] `Prefix+]` - paste from copy buffer

#### Clipboard Integration

- [ ] Add `arboard` or `copypasta` dependency
- [ ] Implement `copy_to_clipboard(text: &str)` function
- [ ] Implement `paste_from_clipboard() -> Option<String>` function
- [ ] Handle clipboard errors gracefully

#### Visual Rendering

- [ ] Implement copy mode cursor rendering
- [ ] Implement selection highlight rendering
- [ ] Add copy mode indicator to status bar
- [ ] Ensure selection highlight is visible (contrasting colors)

#### Buffer Access

- [ ] Implement method to get text from terminal buffer
- [ ] Implement coordinate to buffer offset mapping
- [ ] Handle scrollback buffer access
- [ ] Implement selection region text extraction

### If Implementing Native Passthrough

- [ ] Detect Shift modifier on mouse events
- [ ] Disable mouse capture when Shift held
- [ ] Re-enable mouse capture when Shift released
- [ ] Test native selection works with Shift held

### General Implementation

- [ ] Implement chosen solution
- [ ] Add help text for new keybindings
- [ ] Update documentation
- [ ] Self-review changes

## Testing Tasks

### Unit Tests

- [ ] Test copy mode state transitions
- [ ] Test navigation in copy mode
- [ ] Test selection region calculation
- [ ] Test text extraction from buffer
- [ ] Test clipboard integration (mock)

### Integration Tests

- [ ] Test entering/exiting copy mode
- [ ] Test selection workflow end-to-end
- [ ] Test copy/paste cycle
- [ ] Test copy mode across pane switches

### Manual Testing

- [ ] Enter copy mode with Prefix+[
- [ ] Navigate with vi keys
- [ ] Start visual selection with v
- [ ] Copy selection with y
- [ ] Verify text is in system clipboard
- [ ] Paste with Prefix+] (internal buffer)
- [ ] Paste with Ctrl+Shift+V (system clipboard)
- [ ] Exit copy mode with q/Escape
- [ ] Test selection across scrollback
- [ ] Test selection in split panes

### Regression Testing

- [ ] Verify mouse scroll still works (FEAT-034)
- [ ] Verify normal keyboard input unaffected
- [ ] Verify pane switching works
- [ ] Run full test suite

## Verification Tasks

- [ ] Confirm text can be selected
- [ ] Confirm selection is visually highlighted
- [ ] Confirm text can be copied to clipboard
- [ ] Confirm paste works (both internal and system)
- [ ] All acceptance criteria from PROMPT.md met
- [ ] Update bug_report.json status
- [ ] Document resolution in PLAN.md

## Completion Checklist

- [ ] All investigation tasks complete
- [ ] Root cause documented
- [ ] All implementation tasks complete
- [ ] All tests passing
- [ ] Documentation updated
- [ ] PLAN.md updated with final approach
- [ ] Ready for review/merge

---
*Check off tasks as you complete them. Update status field above.*
