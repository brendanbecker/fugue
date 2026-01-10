# Implementation Plan: BUG-013

**Work Item**: [BUG-013: Mouse Scroll Wheel Not Working for Scrollback](PROMPT.md)
**Component**: ccmux-client
**Priority**: P2
**Created**: 2026-01-10

## Overview

Mouse scroll wheel does not scroll through terminal scrollback history despite FEAT-034 being marked as complete. This is either a bug in FEAT-034's implementation, a regression from subsequent changes, or an integration issue.

## Architecture Decisions

### Approach: To Be Determined

After investigation, the fix will depend on where the failure occurs in the event flow:

```
+------------------+     +------------------+     +------------------+     +------------------+
| Terminal sends   | --> | crossterm        | --> | ccmux-client     | --> | tui-term widget  |
| scroll events    |     | captures events  |     | handles events   |     | scrolls viewport |
+------------------+     +------------------+     +------------------+     +------------------+
```

Possible failure points:
1. Terminal not sending scroll events (unlikely, terminal-specific)
2. crossterm not capturing scroll events (mouse capture config)
3. ccmux-client not handling scroll events (missing handler, regression)
4. tui-term widget not scrolling (API usage, state management)

### Trade-offs

The fix approach depends entirely on investigation findings. No trade-offs to evaluate until root cause is identified.

## Current Expected Event Flow

Based on FEAT-034's design, scroll events should flow as:

```
1. User scrolls mouse wheel
2. Terminal emulator sends escape sequences for scroll
3. crossterm (with EnableMouseCapture) receives MouseEvent::ScrollUp/ScrollDown
4. ccmux-client event loop receives the event
5. Event dispatcher routes to scroll handler
6. Scroll handler updates viewport offset
7. tui-term widget re-renders with new offset
8. User sees scrolled content
```

Key questions to answer during investigation:
- Is step 3 happening? (crossterm receiving events)
- Is step 4 happening? (event loop processing them)
- Is step 5 happening? (routing to scroll handler)
- Is step 6 happening? (viewport state update)
- Is step 7 happening? (widget re-render)

## Files to Investigate

| File | Purpose | Priority |
|------|---------|----------|
| `ccmux-client/src/main.rs` | Mouse capture enable | High |
| `ccmux-client/src/input/mod.rs` | Event handling | High |
| `ccmux-client/src/input/mouse.rs` | Mouse events (if exists) | High |
| `ccmux-client/src/ui/app.rs` | UI state, viewport | High |
| `completed/features/FEAT-034-*/` | Original implementation | High |
| `ccmux-client/src/ui/pane.rs` | Pane scroll state | Medium |

## FEAT-034 Implementation Review

*To be filled during investigation*

### Original Implementation Location
- Check `completed/features/FEAT-034-mouse-scroll-support/`

### Key Code Paths
*Document the intended code path from FEAT-034*

### Current State of That Code
*Does the code still exist? Was it modified?*

## Dependencies

No new dependencies expected. This is a bug fix in existing functionality.

| Dependency | Purpose | Already Present |
|------------|---------|-----------------|
| crossterm | Mouse event capture | Yes |
| ratatui | TUI rendering | Yes |
| tui-term | Terminal widget | Yes |

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Fix breaks other mouse functionality | Medium | Medium | Test all mouse features |
| Root cause is in crossterm/terminal | Low | High | Test crossterm in isolation |
| Multiple bugs causing same symptom | Medium | Medium | Thorough investigation |
| Regression in FEAT-035 code | Medium | Low | Review FEAT-035 changes |

## Relationship to BUG-012

BUG-012 (Text selection not working) is a related mouse issue. They may share a root cause:
- Both involve mouse event handling
- Both suggest mouse events aren't being processed correctly
- Fixing one may fix or inform the other

Should investigate both together to identify any common root cause in mouse event handling infrastructure.

## Rollback Strategy

If fix causes issues:
1. Revert commits associated with this bug fix
2. Mouse scroll will remain non-functional (current state)
3. Document issues and consider alternative approach

## Implementation Notes

<!-- Add notes during implementation -->

### Investigation Findings

*To be filled during investigation*

### Root Cause

*To be determined*

### Chosen Solution

*To be determined after investigation*

### Relationship to Other Bugs

*Document any connections to BUG-012 or other issues*

---
*This plan should be updated as implementation progresses.*
