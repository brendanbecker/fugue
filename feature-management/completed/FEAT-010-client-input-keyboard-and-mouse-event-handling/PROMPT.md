# FEAT-010: Client Input - Keyboard and Mouse Event Handling

**Priority**: P1
**Component**: fugue-client
**Type**: new_feature
**Estimated Effort**: medium
**Business Value**: high

## Overview

Keyboard and mouse event handling via crossterm, prefix key support (tmux-style), and input routing to active pane. This is a core client feature that enables user interaction with the terminal multiplexer.

## Requirements

### Crossterm Event Handling
- Use crossterm for cross-platform keyboard and mouse event capture
- Handle raw mode for terminal input
- Support for all standard key events (letters, numbers, modifiers)
- Handle special keys (arrows, function keys, home/end, etc.)
- Mouse event capture (clicks, scroll, movement)

### Prefix Key Support (tmux-style)
- Configurable prefix key (default: Ctrl+B, like tmux)
- Command mode activation after prefix key
- Timeout for command mode (configurable, e.g., 500ms)
- Visual indicator when in command mode
- Escape from command mode

### Command Mode Actions
- Pane navigation (arrow keys or hjkl)
- Pane splitting (| for vertical, - for horizontal)
- Pane closing (x)
- Window management (c for create, n/p for next/prev)
- Session detach (d)
- Copy mode activation ([)
- Custom command bindings

### Input Routing to Active Pane
- Forward non-command input to active pane's PTY
- Handle paste operations
- Support bracketed paste mode
- Maintain input buffer for performance

### Key Binding Configuration
- TOML/JSON configuration for keybindings
- Support for modifier combinations (Ctrl, Alt, Shift)
- Chord support (prefix + key sequences)
- Binding categories (navigation, window, session, custom)

### Mouse Support
- Click to select pane
- Click to focus window
- Scroll wheel for scrollback navigation
- Optional: drag to resize panes
- Configurable mouse enable/disable

## Affected Files

- `fugue-client/src/input.rs` - Core input handling and event loop
- `fugue-client/src/keybindings.rs` - Key binding configuration and dispatch

## Implementation Tasks

### Section 1: Design
- [ ] Review crossterm event API and capabilities
- [ ] Design input event handling architecture
- [ ] Design key binding configuration schema
- [ ] Plan command mode state machine
- [ ] Design input routing mechanism

### Section 2: Core Event Loop
- [ ] Set up crossterm raw mode
- [ ] Implement event polling loop
- [ ] Handle keyboard events
- [ ] Handle mouse events
- [ ] Implement event debouncing if needed

### Section 3: Prefix Key and Command Mode
- [ ] Implement prefix key detection
- [ ] Create command mode state machine
- [ ] Add command mode timeout
- [ ] Implement escape from command mode
- [ ] Add visual indicator for command mode

### Section 4: Key Binding System
- [ ] Define key binding configuration schema
- [ ] Implement key binding parser
- [ ] Create action dispatcher
- [ ] Support modifier combinations
- [ ] Implement chord sequences

### Section 5: Input Routing
- [ ] Route input to active pane PTY
- [ ] Handle paste operations
- [ ] Implement bracketed paste support
- [ ] Add input buffering for performance

### Section 6: Mouse Support
- [ ] Implement click for pane selection
- [ ] Implement scroll wheel handling
- [ ] Add mouse enable/disable toggle
- [ ] Optional: pane resize via drag

### Section 7: Testing
- [ ] Unit tests for key binding parsing
- [ ] Unit tests for command mode state machine
- [ ] Integration tests for input routing
- [ ] Test modifier key combinations
- [ ] Test mouse event handling

### Section 8: Documentation
- [ ] Document default keybindings
- [ ] Document configuration options
- [ ] Add keybinding reference card
- [ ] Update CHANGELOG

## Acceptance Criteria

- [ ] Keyboard events captured via crossterm
- [ ] Prefix key activates command mode
- [ ] Command mode times out appropriately
- [ ] Key bindings are configurable
- [ ] Input routes correctly to active pane
- [ ] Mouse clicks select panes
- [ ] Scroll wheel navigates scrollback
- [ ] No input lag or dropped keystrokes
- [ ] Works on Linux, macOS, Windows

## Dependencies

- FEAT-009 (must be completed first)

## Notes

- Consider using crossterm's event stream for async handling
- May need to coordinate with TUI rendering to avoid conflicts
- Bracketed paste mode is important for multi-line input
- Mouse capture may interfere with terminal selection - provide toggle
- Test with various terminal emulators for compatibility
