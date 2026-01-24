# FEAT-087: Refactor fugue-client/src/ui/app.rs

**Priority**: P2
**Component**: fugue-client
**Type**: refactor
**Estimated Effort**: medium
**Current Size**: 31.8k tokens (3021 lines)
**Target Size**: <10k tokens per module

## Overview

The TUI client's main application file has grown to 31.8k tokens, making it difficult to navigate, test, and maintain. This refactoring will extract logical components into separate modules.

## Current Structure Analysis

The file likely contains:
- Main event loop
- Input handling dispatch
- Render loop and frame building
- State management
- Keybind processing
- Pane focus management
- Window/session switching
- Status bar rendering
- Modal dialogs (help, session picker, etc.)

## Proposed Module Structure

```
fugue-client/src/ui/
├── app.rs              # Main App struct, event loop coordination (<5k)
├── state.rs            # AppState, focus management, selection
├── render.rs           # Frame building, layout coordination
├── events.rs           # Event dispatch, tick handling
├── modals/
│   ├── mod.rs
│   ├── help.rs         # Help modal
│   ├── session.rs      # Session picker
│   └── command.rs      # Command palette (if exists)
└── keybinds.rs         # Keybind handling (may already exist)
```

## Refactoring Steps

1. **Identify logical boundaries** - Read through app.rs and categorize functions
2. **Extract state management** - AppState, focus tracking, selection
3. **Extract rendering** - Frame building, widget composition
4. **Extract event handling** - Tick processing, input dispatch
5. **Extract modals** - Each modal to its own file
6. **Update imports** - Fix all references across crate

## Acceptance Criteria

- [ ] `app.rs` reduced to <10k tokens
- [ ] Each extracted module is <10k tokens
- [ ] All existing tests pass
- [ ] No functionality changes
- [ ] No new warnings or clippy lints
- [ ] TUI behaves identically before and after

## Testing

- Run full test suite
- Manual testing of all TUI features
- Verify keybinds, modals, rendering all work

## Notes

- This is a pure refactor - no behavior changes
- Consider adding module-level documentation during refactor
- May reveal opportunities for better abstraction
