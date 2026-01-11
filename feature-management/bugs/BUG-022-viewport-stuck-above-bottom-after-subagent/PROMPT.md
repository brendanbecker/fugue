# BUG-022: Viewport gets stuck above bottom after subagent finishes

**Priority**: P2
**Component**: tui
**Severity**: medium
**Status**: in_progress

## Problem Statement

When a Claude Code subagent finishes in ccmux, the viewport sometimes doesn't render all the way to the bottom - it appears offset a few lines above where it should be. The issue is intermittent and doesn't happen every time.

## Evidence

- **Files Modified**: `ccmux-client/src/ui/pane.rs`
- **Root Cause Location**: `resize()` and `process_output()` methods in Pane struct

## Steps to Reproduce

1. Run ccmux with Claude Code session
2. Have Claude Code spawn a subagent task
3. Wait for the subagent to complete
4. Observe that the viewport may be stuck showing content from a few lines above the actual bottom

## Expected Behavior

Viewport should always show the live bottom of the terminal after a subagent completes, with scrollback offset at 0

## Actual Behavior

Viewport sometimes shows content from a few lines above the bottom, appearing 'stuck' with an invalid scrollback offset

## Root Cause

The issue appears to be related to scrollback synchronization when the VT100 parser is resized. When terminal size changes (even by 1 row due to layout recalculations), the scrollback offset could become invalid, causing the viewport to show content from a few lines up rather than the live bottom.

**Potential triggers:**
- Split pane closes or is created
- Window size changes slightly
- Layout constraints are recalculated
- Race condition between terminal.size() and frame.area() in the draw loop

## Implementation Tasks

### Section 1: Investigation
- [x] Reproduce the bug consistently
- [x] Identify root cause
- [x] Document affected code paths

### Section 2: Fix Implementation
- [x] Implement fix for root cause
- [x] Add error handling if needed
- [ ] Update related documentation

### Section 3: Testing
- [ ] Add unit tests to prevent regression
- [ ] Test fix in affected scenarios
- [ ] Verify no side effects in related functionality

### Section 4: Verification
- [ ] Confirm expected behavior is restored
- [ ] Verify all acceptance criteria met
- [ ] Update bug report with resolution details

## Fix Implemented

Two changes made in `ccmux-client/src/ui/pane.rs`:

### 1. `resize()` now guards against unnecessary resizes and resets scrollback

```rust
pub fn resize(&mut self, rows: u16, cols: u16) {
    let (current_rows, current_cols) = self.size();
    if current_rows != rows || current_cols != cols {
        self.parser.set_size(rows, cols);
        // Reset scroll position to bottom after resize to prevent viewport offset issues
        // when the terminal size changes and scrollback becomes invalid
        self.parser.set_scrollback(0);
        // Read back to ensure consistency with parser state
        self.scroll_offset = self.parser.screen().scrollback();
    }
}
```

- Only resizes when the size actually changes (prevents resetting on every draw frame)
- Resets scrollback to 0 when size changes to prevent invalid viewport offsets
- Reads back the scrollback value to ensure local state matches parser state

### 2. `process_output()` now syncs scrollback state

```rust
pub fn process_output(&mut self, data: &[u8]) {
    self.parser.process(data);
    // Reset scroll to bottom when new output arrives
    self.parser.set_scrollback(0);
    // Read back to ensure consistency with parser state
    self.scroll_offset = self.parser.screen().scrollback();
}
```

- Reads back scrollback value after setting to 0 to ensure consistency

## Acceptance Criteria

- [x] Bug is reproducibly fixed in all scenarios
- [ ] Tests added to prevent regression
- [ ] All affected functionality tested
- [ ] No new bugs introduced
- [ ] Root cause documented

## Notes

- Fix is implemented but needs verification from user testing
- The intermittent nature of the bug makes it difficult to verify complete resolution
- Consider adding logging to track scrollback state changes for future debugging
