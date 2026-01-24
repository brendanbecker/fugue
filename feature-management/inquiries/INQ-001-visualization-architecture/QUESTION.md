# INQ-001: Visualization Architecture Review

## Core Question

**How should fugue handle terminal escape sequences to properly render TUI applications like Claude Code without visual artifacts?**

## Context

fugue's screen rendering produces visual artifacts when displaying Claude Code's TUI output. Multiple stacked "thinking" messages, ghost content, and overlapping UI elements create an unpleasant and confusing viewing experience.

### Observed Issues

From screenshot analysis:
- Multiple horizontal divider lines stacked vertically
- Stacked "Thinking...", "Nucleating..." status messages that should replace each other
- Tool use sections appearing multiple times
- Ghost content that should have been overwritten by cursor movements
- Overall cluttered appearance instead of clean Claude Code interface

### Technical Background

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

## Why This Matters

- User experience is degraded - confusing visual output
- May affect adoption of fugue for AI agent orchestration
- Fundamental architecture issue, not a simple bug fix
- Multiple related bugs have been addressed but core issue persists

## Constraints

1. **No regression** - Must not break existing TUI app support (vim, htop, etc.)
2. **Performance** - Must maintain reasonable rendering performance
3. **Mirror panes** - Must support mirror pane rendering correctly
4. **Compatibility** - Should follow VT100/xterm behavior standards

## Scope of Investigation

1. **Audit existing rendering code** - Understand current PTY output handling
2. **Review previous bugs/features** - Learn from past rendering issues
3. **Analyze terminal emulation** - Compare to proper VT100/xterm behavior
4. **Design improvements** - May need fundamental changes to screen buffer handling
5. **Test with multiple TUI apps** - Claude Code, vim, htop, etc.

## Key Files to Investigate

- `fugue-server/src/pty/` - PTY handling
- `fugue-client/src/render.rs` - Client rendering
- `fugue-client/src/app.rs` - Client app logic
- Terminal emulation/screen buffer code

## Related History

- BUG-048: TUI flicker during spinner
- BUG-041: Claude Code crashes on paste (bracketed paste)
- BUG-053: DSR cursor position handling
- FEAT-062: Mirror pane implementation
- FEAT-103: Original feature request (converted to this inquiry)

## Visual Reference

Screenshot showing artifacts: `/mnt/c/Users/Brend/Pictures/Screenshots/SSfugue.png`

## Research Agent Assignments

| Agent | Focus Area |
|-------|------------|
| Agent 1 | Audit existing fugue rendering code and catalog rendering-related bugs |
| Agent 2 | Analyze proper VT100/xterm escape sequence handling standards |
| Agent 3 | Research terminal multiplexer approaches (tmux, screen, zellij) |

## Expected Outcome

This inquiry should produce:
1. Clear understanding of the root cause
2. Architectural recommendations
3. One or more FEAT work items for implementation
