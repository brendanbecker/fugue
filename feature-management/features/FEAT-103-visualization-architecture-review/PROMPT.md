# FEAT-103: Visualization Architecture Review

**Priority**: P1
**Component**: client/rendering
**Effort**: Large
**Status**: converted

> **Note**: This feature has been converted to **INQ-001** (Visualization Architecture Review).
> The scope requires multi-agent deliberation before implementation.
> See: `../inquiries/INQ-001-visualization-architecture/`

## Problem

fugue's screen rendering produces visual artifacts when displaying Claude Code's TUI output. Multiple stacked "thinking" messages, ghost content, and overlapping UI elements create an unpleasant and confusing viewing experience.

### Observed Issues

From screenshot analysis:
- Multiple horizontal divider lines stacked vertically
- Stacked "Thinking...", "Nucleating..." status messages that should replace each other
- Tool use sections appearing multiple times
- Ghost content that should have been overwritten by cursor movements
- Overall cluttered appearance instead of clean Claude Code interface

### Root Cause

Claude Code's TUI uses ANSI escape sequences for:
- Cursor positioning (`[nA`, `[nB`, `[nC`, `[nD`, `[H`)
- Screen clearing (`[2J`, `[K`, `[0K`, `[1K`, `[2K`)
- Scrolling regions
- Overwriting previous content in-place

fugue's rendering may not be properly handling:
1. In-place content updates (cursor moves + overwrites)
2. Line/screen clear sequences
3. Scrolling regions
4. Partial line updates

## Scope

This is a significant architectural review, not a simple bug fix. Need to:

1. **Audit existing rendering code** - Understand current PTY output handling
2. **Review previous bugs/features** - Learn from past rendering issues
3. **Analyze terminal emulation** - Compare to proper VT100/xterm behavior
4. **Design improvements** - May need fundamental changes to screen buffer handling
5. **Test with multiple TUI apps** - Claude Code, vim, htop, etc.

## Related History

Review these for context:
- BUG-048: TUI flicker during spinner
- BUG-041: Claude Code crashes on paste (bracketed paste)
- BUG-053: DSR cursor position handling
- FEAT-062: Mirror pane implementation
- Any other PTY/rendering related items

## Key Files to Review

- `fugue-server/src/pty/` - PTY handling
- `fugue-client/src/render.rs` - Client rendering (from FEAT-087 refactor)
- `fugue-client/src/app.rs` - Client app logic
- Terminal emulation/screen buffer code

## Proposed Approach

**Agent Council recommended** - Multiple perspectives needed:

1. **Researcher Agent**: Audit existing code, catalog all rendering-related bugs/features
2. **Terminal Expert Agent**: Analyze proper VT100/xterm escape sequence handling
3. **Architect Agent**: Design improved screen buffer architecture
4. **Implementation Agent**: Execute the design changes
5. **QA Agent**: Test with various TUI applications

## Acceptance Criteria

- [ ] Claude Code renders cleanly without ghost/stacked content
- [ ] Status messages update in-place correctly
- [ ] Cursor movement sequences work properly
- [ ] Screen/line clear sequences work properly
- [ ] No regression in other TUI apps (vim, htop, etc.)
- [ ] Mirror panes render source content correctly (relates to BUG-066)

## Visual Reference

Screenshot saved at: `/mnt/c/Users/Brend/Pictures/Screenshots/SSfugue.png`

Shows the rendering artifacts - stacked thinking messages, multiple divider lines, cluttered appearance.

## Notes

- This may reveal deeper issues with the terminal emulation approach
- Consider whether fugue needs a proper screen buffer (like tmux's)
- May need to intercept and process escape sequences more thoroughly
- Performance implications of more sophisticated rendering
