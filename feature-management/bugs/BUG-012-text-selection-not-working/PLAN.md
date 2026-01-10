# Implementation Plan: BUG-012

**Work Item**: [BUG-012: Text Selection Not Working in TUI](PROMPT.md)
**Component**: ccmux-client
**Priority**: P2
**Created**: 2026-01-10

## Overview

Text selection does not work in the ccmux TUI. Mouse capture is enabled (for scroll support from FEAT-034), which prevents native terminal selection. No copy mode or selection handling is implemented, leaving users unable to copy any text from the terminal output.

## Architecture Decisions

### Approach: To Be Determined

After investigation, choose from:

1. **Copy Mode (tmux-style)**: Implement `Prefix+[` to enter copy mode with vi-style navigation and selection
2. **Native Selection Passthrough**: Release mouse capture when Shift is held to allow native terminal selection
3. **Hybrid**: Implement copy mode AND allow Shift+click for native selection
4. **Direct Mouse Selection**: Implement click-and-drag selection within ccmux (most complex)

### Trade-offs

| Option | Pros | Cons |
|--------|------|------|
| Copy Mode | Full control, tmux parity, extensible | Learning curve, more implementation work |
| Native Passthrough | Simple, familiar | Loses mouse features while Shift held |
| Hybrid | Best of both worlds | More code paths to maintain |
| Direct Selection | Most intuitive | Complex implementation, coordinate mapping |

**Decision**: TBD after investigation identifies current architecture constraints.

## Current Mouse Handling Architecture

Based on FEAT-034 (mouse scroll), the current architecture likely:

```
+------------------+     +------------------+     +------------------+
| crossterm events | --> | Event dispatcher | --> | Handler(scroll)  |
| (mouse capture)  |     | (ccmux-client)   |     | (ccmux-client)   |
+------------------+     +------------------+     +------------------+
```

Key questions:
- Where does event dispatch happen?
- What mouse events are currently handled?
- How is mouse scroll implemented?
- Is there infrastructure for different mouse modes?

## Files to Investigate

| File | Purpose | Risk Level |
|------|---------|------------|
| `ccmux-client/src/input/mod.rs` | Input event handling | High |
| `ccmux-client/src/input/mouse.rs` | Mouse events (if exists) | High |
| `ccmux-client/src/ui/app.rs` | UI state machine | High |
| `ccmux-client/src/ui/mod.rs` | UI module | Medium |
| `ccmux-client/src/main.rs` | Mouse capture enable | Medium |

## Copy Mode State Machine (if implemented)

```
                                +-------------+
                                |   Normal    |
                                |    Mode     |
                                +------+------+
                                       |
                                Prefix+[
                                       |
                                       v
                                +-------------+
                                |    Copy     |
                                |    Mode     |<----+
                                +------+------+     |
                                       |            |
                    +------------------+--------+   |
                    |                  |        |   |
                 h/j/k/l           v/Space      |   |
                   |                  |         |   |
                   v                  v         |   |
             +-------------+   +-------------+  |   |
             |  Navigate   |   |  Visual     |  |   |
             |  (cursor)   |   |  Select     |--+   |
             +-------------+   +------+------+      |
                                      |            |
                                  y/Enter          |
                                      |            |
                                      v            |
                               +-------------+     |
                               |   Copied    |     |
                               |  (exit)     |-----+
                               +-------------+
                                      |
                                  q/Escape
                                      |
                                      v
                               +-------------+
                               |   Normal    |
                               |    Mode     |
                               +-------------+
```

## Dependencies

| Dependency | Purpose | Crate |
|------------|---------|-------|
| Clipboard | System clipboard access | `arboard` or `copypasta` |
| (existing) | Mouse events | `crossterm` |
| (existing) | UI rendering | `ratatui` |

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Copy mode conflicts with existing keybindings | Medium | Medium | Check existing bindings first |
| Clipboard access fails on some platforms | Medium | Medium | Graceful fallback, error message |
| Selection rendering impacts performance | Low | Medium | Only render selection overlay in copy mode |
| Regression in mouse scroll | Low | High | Test FEAT-034 functionality |

## Rollback Strategy

If implementation causes issues:
1. Revert commits associated with this work item
2. Mouse capture remains enabled (current state)
3. Document issues and consider alternative approach

## Implementation Notes

<!-- Add notes during implementation -->

### Investigation Findings

*To be filled during investigation*

### Root Cause

*Confirmed: Mouse capture enabled, no selection handling implemented*

### Chosen Solution

*To be determined after investigation*

---
*This plan should be updated as implementation progresses.*
