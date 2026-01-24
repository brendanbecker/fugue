# Task Breakdown: FEAT-010

**Work Item**: [FEAT-010: Client Input - Keyboard and Mouse Event Handling](PROMPT.md)
**Status**: Not Started
**Last Updated**: 2026-01-08

## Prerequisites

- [ ] Read and understand PROMPT.md
- [ ] Review PLAN.md and update if needed
- [ ] Verify FEAT-009 is complete (dependency)
- [ ] Review crossterm documentation and examples
- [ ] Review existing fugue-client structure

## Design Tasks

- [ ] Design input event handling architecture
- [ ] Design command mode state machine
- [ ] Define key binding configuration schema (TOML)
- [ ] Design action dispatch mechanism
- [ ] Plan input routing to PTY
- [ ] Document mouse interaction model

## Implementation Tasks

### Core Event Loop (input.rs)
- [ ] Add crossterm dependency to Cargo.toml
- [ ] Set up terminal raw mode on client start
- [ ] Implement async event stream with tokio
- [ ] Handle Event::Key variants
- [ ] Handle Event::Mouse variants
- [ ] Handle Event::Resize for terminal size changes
- [ ] Implement graceful cleanup on exit (restore terminal)

### Input State Machine (input.rs)
- [ ] Create InputMode enum (Normal, Command, etc.)
- [ ] Implement prefix key detection
- [ ] Create command mode state with timeout
- [ ] Handle escape from command mode
- [ ] Track current input mode
- [ ] Emit mode change events for UI indicator

### Key Binding System (keybindings.rs)
- [ ] Define KeySpec struct for key representation
- [ ] Implement key spec parser ("C-b", "M-x", "S-Tab")
- [ ] Define Action enum for all possible actions
- [ ] Create KeyBindings configuration struct
- [ ] Implement TOML deserialization
- [ ] Create default keybinding set
- [ ] Implement binding lookup by key

### Action Dispatcher (input.rs)
- [ ] Create ActionDispatcher trait/struct
- [ ] Implement navigation actions (focus_left/right/up/down)
- [ ] Implement pane actions (split, close, zoom)
- [ ] Implement window actions (create, next, prev, select)
- [ ] Implement session actions (detach, copy_mode)
- [ ] Wire dispatcher to event loop

### Input Routing
- [ ] Route normal mode input to active pane PTY
- [ ] Implement input buffering for performance
- [ ] Handle paste detection
- [ ] Implement bracketed paste mode support
- [ ] Handle special sequences (Ctrl+C, Ctrl+D, etc.)

### Mouse Support
- [ ] Enable mouse capture via crossterm
- [ ] Implement click to select pane
- [ ] Implement click to focus window tab
- [ ] Implement scroll wheel for scrollback
- [ ] Add mouse enable/disable toggle (prefix + m)
- [ ] Handle mouse coordinate translation to panes

### Configuration Integration
- [ ] Load keybindings from config file
- [ ] Support config hot-reload for keybindings
- [ ] Validate keybinding configuration
- [ ] Merge user bindings with defaults

## Testing Tasks

- [ ] Unit test: key spec parsing
- [ ] Unit test: binding lookup
- [ ] Unit test: state machine transitions
- [ ] Unit test: action dispatch
- [ ] Integration test: prefix -> command flow
- [ ] Integration test: input routing to PTY
- [ ] Integration test: mouse pane selection
- [ ] Manual test: Linux terminal emulators
- [ ] Manual test: macOS Terminal.app and iTerm2
- [ ] Manual test: Windows Terminal

## Documentation Tasks

- [ ] Document default keybindings in README
- [ ] Create keybinding reference card
- [ ] Document configuration options
- [ ] Add examples for custom bindings
- [ ] Update CHANGELOG

## Verification Tasks

- [ ] All acceptance criteria from PROMPT.md met
- [ ] Tests passing on all platforms
- [ ] No input lag or dropped keystrokes
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
