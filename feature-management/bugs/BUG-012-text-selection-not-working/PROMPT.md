# BUG-012: Text Selection Not Working in TUI

**Priority**: P2 (Medium)
**Component**: ccmux-client
**Status**: new
**Created**: 2026-01-10

## Summary

Text selection does not work in the ccmux TUI client. When attempting to click and drag to select text, nothing happens - there is no selection, no highlight, and no ability to copy text. This is not just a visual issue; selection is completely non-functional.

## Symptoms

- Click and drag does not select text
- No text can be copied from the terminal output
- Selection is completely non-functional, not just invisible
- No copy mode available (like tmux's Prefix+[)

## Background

This is a Rust terminal multiplexer project using:
- **ratatui**: TUI framework
- **crossterm**: Terminal backend (handles input/output)
- **tui-term**: Terminal emulation widget

Mouse events ARE being captured - mouse scroll support was added in FEAT-034. This suggests the mouse capture infrastructure is in place but selection events are either not being handled or are being consumed without enabling selection functionality.

## Related Context

In tmux, text selection is handled via "copy mode" which is entered with `Prefix+[`. This allows:
- Vi or emacs-style navigation
- Visual selection of text
- Copying to tmux paste buffer
- Pasting with `Prefix+]`

ccmux may need a similar mechanism, or it may need to allow native terminal selection to work by releasing mouse capture in certain contexts.

## Likely Causes to Investigate

### 1. Mouse Selection Events Not Being Captured or Handled

The client may be receiving mouse drag events but not processing them for selection purposes.

**Files to check:**
- `ccmux-client/src/input/mod.rs` - Input event handling
- `ccmux-client/src/input/mouse.rs` (if exists) - Mouse event handling

### 2. No Selection State Management Implemented

There may simply be no code to track selection state (start position, end position, selected text).

**Files to check:**
- `ccmux-client/src/ui/` - UI state management
- Look for any `Selection` structs or selection-related code

### 3. Mouse Events Being Consumed for Other Purposes

FEAT-034 added mouse scroll support. The mouse event handler may be routing drag events to scroll logic instead of selection logic.

**Files to check:**
- Mouse event dispatch code
- FEAT-034 implementation

### 4. Copy Mode (Prefix+[) Not Implemented

If ccmux intends to use a tmux-style copy mode, it may simply not be implemented yet.

**Files to check:**
- Key binding handlers
- Look for any copy mode references

### 5. Crossterm Mouse Capture Intercepting Selection

When crossterm enables mouse capture, the terminal cannot perform native selection. If ccmux captures mouse events but doesn't implement selection, the user gets no selection at all.

**Files to check:**
- `crossterm::event::EnableMouseCapture` usage
- Mouse capture enable/disable logic

## Potential Solutions

### Option A: Implement Copy Mode (Recommended)

Implement tmux-style copy mode:
- `Prefix+[` enters copy mode
- Vi-style navigation (h/j/k/l, w/b, 0/$, etc.)
- `v` or `Space` starts visual selection
- `y` or `Enter` copies selection
- `q` or `Escape` exits copy mode
- `Prefix+]` pastes from buffer

**Pros:**
- Full control over selection behavior
- Consistent with tmux users' expectations
- Works with any terminal

**Cons:**
- More complex to implement
- Learning curve for users unfamiliar with tmux

### Option B: Allow Native Terminal Selection

Disable mouse capture in certain contexts (e.g., when Shift is held) to allow the native terminal selection to work.

**Pros:**
- Simple to implement
- Users get familiar terminal selection behavior

**Cons:**
- Loses mouse functionality while Shift is held
- Inconsistent behavior between native and ccmux

### Option C: Hybrid Approach

- Implement copy mode for power users
- Allow Shift+click to use native selection as fallback

## Acceptance Criteria

- [ ] Root cause identified and documented
- [ ] Text can be selected from terminal output
- [ ] Selected text can be copied to system clipboard
- [ ] User has clear way to enter selection/copy mode
- [ ] Selection is visually highlighted
- [ ] Copying works reliably
- [ ] Add test case to prevent regression
- [ ] Document the selection/copy feature for users

## Implementation Tasks

### Section 1: Investigation

- [ ] Review current mouse event handling code
- [ ] Identify where mouse drag events are processed
- [ ] Check if any selection infrastructure exists
- [ ] Review FEAT-034 implementation for mouse handling patterns
- [ ] Document findings in PLAN.md
- [ ] Decide on solution approach (copy mode vs native vs hybrid)

### Section 2: Fix Implementation

#### If implementing Copy Mode:

- [ ] Add `CopyMode` state to UI state machine
- [ ] Implement `Prefix+[` keybinding to enter copy mode
- [ ] Implement copy mode navigation (vi-style recommended)
- [ ] Implement visual selection rendering
- [ ] Implement `y`/`Enter` to copy to clipboard
- [ ] Implement `Prefix+]` to paste from clipboard
- [ ] Implement `q`/`Escape` to exit copy mode

#### If allowing Native Selection:

- [ ] Detect when Shift is held with mouse events
- [ ] Disable mouse capture when Shift+click/drag
- [ ] Re-enable mouse capture when Shift released

#### General:

- [ ] Add clipboard integration (arboard crate or similar)
- [ ] Add visual feedback for selection
- [ ] Update help text / documentation

### Section 3: Testing

- [ ] Add unit tests for copy mode state management
- [ ] Add integration tests for selection flow
- [ ] Manual test selection in various scenarios
- [ ] Test clipboard integration on Linux/WSL
- [ ] Verify mouse scroll still works (no regression from FEAT-034)

### Section 4: Verification

- [ ] Confirm text can be selected
- [ ] Confirm text can be copied to clipboard
- [ ] Confirm paste works
- [ ] All acceptance criteria met
- [ ] Update bug_report.json with resolution details

## Notes

This is a P2 bug because:
- It does not block core terminal functionality
- Users can still view output, just cannot copy it
- However, it significantly impacts workflow efficiency
- Copy/paste is a fundamental terminal operation

The fix should prioritize usability. Copy mode is recommended as it:
- Gives ccmux feature parity with tmux
- Provides a consistent, predictable experience
- Allows future enhancements (paste buffers, search in copy mode, etc.)

## References

- FEAT-034: Mouse scroll support (shows mouse event handling patterns)
- tmux copy mode documentation
- crossterm mouse capture documentation
