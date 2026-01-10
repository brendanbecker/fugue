# BUG-013: Mouse Scroll Wheel Not Working for Scrollback

**Priority**: P2 (Medium)
**Component**: ccmux-client
**Status**: new
**Created**: 2026-01-10

## Summary

Mouse scroll wheel does not scroll through terminal scrollback history. FEAT-034 (Mouse Scroll Support) was supposedly implemented and merged, but scrolling with the mouse wheel is not functioning. This appears to be a regression or incomplete implementation.

## Symptoms

- Scroll wheel does nothing in the TUI
- Cannot scroll back through terminal output using mouse
- FEAT-034 claims this feature works but it doesn't
- No visual feedback when scrolling (no scroll indicator change)

## Background

This is a Rust terminal multiplexer project using:
- **ratatui**: TUI framework
- **crossterm**: Terminal backend (handles input/output)
- **tui-term**: Terminal emulation widget

FEAT-034 was marked as completed, claiming to add mouse scroll support. However, the feature is not functioning. This could indicate:
1. The feature was never fully integrated
2. A regression was introduced by later changes
3. The feature has a bug that wasn't caught in testing
4. There's a configuration or mode issue preventing scroll events from being processed

## Related Context

- **FEAT-034**: Mouse Scroll Support - marked as merged but not working
- **FEAT-035**: Added Ctrl+PageUp/Down for window navigation (may have conflicts)
- **BUG-012**: Text selection not working (related mouse issues - may share root cause)
- Uses ratatui + crossterm + tui-term stack

## Likely Causes to Investigate

### 1. Mouse Scroll Events Not Being Captured After Recent Changes

Recent changes (FEAT-035 window navigation, other features) may have altered mouse event handling in a way that breaks scroll wheel capture.

**Files to check:**
- `ccmux-client/src/input/mod.rs` - Input event handling
- `ccmux-client/src/input/mouse.rs` (if exists) - Mouse event handling
- Check for any changes that might intercept or drop scroll events

### 2. FEAT-034 Implementation May Have a Bug or Regression

The original FEAT-034 implementation may have a bug, or a subsequent change may have broken it.

**Files to check:**
- Review FEAT-034 implementation files
- Check `completed/features/FEAT-034-mouse-scroll-support/` for implementation details
- Look for any conditionals that might prevent scroll handling

### 3. Mouse Capture Mode May Be Interfering

crossterm's mouse capture may be configured in a way that captures scroll events but doesn't process them, or may be disabled entirely.

**Files to check:**
- `ccmux-client/src/main.rs` - Look for `EnableMouseCapture` usage
- Check if mouse capture is enabled at startup
- Check if mouse capture is disabled in certain modes

### 4. Scroll Events Captured But Not Translated to Viewport Scroll

Events may be received but the translation to actual viewport scrolling may be broken.

**Files to check:**
- Viewport/scroll state management
- tui-term widget scroll handling
- Event dispatch from scroll events to viewport updates

### 5. Configuration Issue or Mouse Mode Not Enabled

There may be a configuration requirement or mouse mode toggle that isn't enabled by default.

**Files to check:**
- Configuration loading code
- Any mouse-related config options
- Default mode settings on startup

## Investigation Steps

1. **Verify mouse capture is enabled**
   - Check if `crossterm::event::EnableMouseCapture` is called on startup
   - Verify it's not disabled by any subsequent code

2. **Add debug logging for mouse events**
   - Add temporary logging to see if scroll events are received at all
   - Log the event type, delta, and coordinates

3. **Review FEAT-034 implementation**
   - Read the original FEAT-034 files in completed/
   - Trace the code path from mouse event to scroll action

4. **Check for recent regressions**
   - Review git history for changes to input handling since FEAT-034
   - Look for changes that might affect mouse event routing

5. **Test mouse capture in isolation**
   - Create a minimal test to verify crossterm receives scroll events
   - Verify the issue is in ccmux, not crossterm or the terminal

## Acceptance Criteria

- [ ] Root cause identified and documented
- [ ] Mouse scroll wheel scrolls terminal output up/down
- [ ] Scroll works in all panes (not just active)
- [ ] Scroll speed is reasonable (not too fast/slow)
- [ ] Scroll direction is correct (wheel up = scroll up in history)
- [ ] No regression in other mouse functionality
- [ ] Add test case to prevent future regression

## Implementation Tasks

### Section 1: Investigation

- [ ] Review FEAT-034 implementation in completed/
- [ ] Add debug logging to mouse event handler
- [ ] Verify mouse capture is enabled on startup
- [ ] Test if scroll events are received by crossterm
- [ ] Trace code path from scroll event to viewport update
- [ ] Identify exact point of failure
- [ ] Document findings in PLAN.md

### Section 2: Fix Implementation

Based on investigation findings:

#### If events not captured:
- [ ] Ensure `EnableMouseCapture` includes scroll events
- [ ] Check crossterm version supports scroll events

#### If events captured but not routed:
- [ ] Fix event routing to scroll handler
- [ ] Ensure scroll handler is registered

#### If events routed but scroll logic broken:
- [ ] Fix scroll calculation/viewport update logic
- [ ] Ensure tui-term widget receives scroll commands

#### If configuration issue:
- [ ] Enable mouse mode by default
- [ ] Document any required configuration

### Section 3: Testing

- [ ] Manual test scroll wheel in single pane
- [ ] Manual test scroll wheel in split panes
- [ ] Test scroll wheel after pane switch
- [ ] Test scroll in different terminal emulators
- [ ] Verify no regression in keyboard shortcuts (Ctrl+PageUp/Down)
- [ ] Verify no regression in text selection (if BUG-012 is fixed)

### Section 4: Verification

- [ ] Confirm scroll wheel works as expected
- [ ] All acceptance criteria met
- [ ] Update bug_report.json with resolution details
- [ ] Consider if BUG-012 shares root cause

## Notes

This is a P2 bug because:
- Users can still navigate scrollback using keyboard (copy mode, Ctrl+PageUp/Down)
- It does not block core terminal functionality
- However, it is a regression from expected behavior (FEAT-034 claimed to implement this)
- Mouse scroll is a common, expected feature in terminal multiplexers

Since FEAT-034 was marked as complete, this is likely either:
1. A bug in the original implementation that wasn't caught
2. A regression introduced by later changes
3. An integration issue where the feature code exists but isn't wired up correctly

Investigating FEAT-034's implementation files should be the first step.

## References

- FEAT-034: Mouse scroll support (completed feature that isn't working)
- FEAT-035: Window navigation with Ctrl+PageUp/Down
- BUG-012: Text selection not working (related mouse issues)
- crossterm mouse event documentation
- tui-term widget scrolling documentation
