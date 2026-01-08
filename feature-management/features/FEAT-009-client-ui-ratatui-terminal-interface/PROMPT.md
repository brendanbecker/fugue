# FEAT-009: Client UI - Ratatui Terminal Interface

**Priority**: P1
**Component**: ccmux-client
**Type**: new_feature
**Estimated Effort**: large
**Business Value**: high

## Overview

Ratatui-based terminal UI with pane rendering using tui-term, status bar, borders, and Claude state indicators. This is the primary user interface for ccmux, providing visual management of multiple Claude Code sessions.

## Requirements

### Ratatui-based Terminal UI Framework
- Use Ratatui as the TUI framework
- Implement main application loop with event handling
- Support crossterm backend for cross-platform compatibility
- Handle terminal resize events gracefully

### Pane Rendering with tui-term
- Integrate tui-term crate for terminal emulation rendering
- Render PTY output in pane widgets
- Support ANSI escape sequences and colors
- Handle scrollback buffer display

### Status Bar
- Display current session name and ID
- Show active pane indicator
- Display connection status to server
- Show keybinding hints
- Display timestamp or uptime

### Border Rendering
- Draw borders between panes
- Support different border styles (single, double, rounded)
- Highlight active pane border
- Show pane titles in border

### Claude State Indicators
- Idle indicator (waiting for input)
- Thinking indicator (processing)
- Tool Use indicator (executing tools)
- Error indicator (something went wrong)
- Completion indicator (task finished)

Suggested visual indicators:
- Idle: `[ ] Idle` or dim status
- Thinking: `[...] Thinking` with animation
- Tool Use: `[>] Tool Use` or spinner
- Error: `[!] Error` in red
- Complete: `[v] Done` in green

### Layout Management
- Support horizontal and vertical splits
- Flexible pane sizing (percentage or fixed)
- Nested layouts for complex arrangements
- Layout presets (single, split, grid)

### Resize Handling
- Respond to terminal resize signals (SIGWINCH)
- Recalculate layout on resize
- Maintain pane proportions where possible
- Handle minimum size constraints

## Current State

- `ccmux-client/src/ui.rs` exists as a stub
- Client can connect to server via Unix socket
- No actual UI rendering implemented yet

## Affected Files

- `ccmux-client/src/ui.rs` - Main UI module (currently stub)
- `ccmux-client/src/ui/layout.rs` - Layout management
- `ccmux-client/src/ui/pane.rs` - Pane widget
- `ccmux-client/src/ui/status.rs` - Status bar widget
- `ccmux-client/src/ui/borders.rs` - Border rendering
- `ccmux-client/src/ui/indicators.rs` - Claude state indicators
- `ccmux-client/Cargo.toml` - Add ratatui, tui-term dependencies

## Implementation Tasks

### Section 1: Design
- [ ] Review Ratatui architecture and best practices
- [ ] Design component hierarchy (App, Layout, Pane, Status)
- [ ] Define state model for UI
- [ ] Design event handling flow
- [ ] Plan integration with server connection

### Section 2: Core Framework
- [ ] Add Ratatui and tui-term to Cargo.toml
- [ ] Implement terminal initialization/cleanup
- [ ] Create main application struct
- [ ] Implement event loop (input + server events)
- [ ] Handle terminal resize events

### Section 3: Layout System
- [ ] Implement layout constraint system
- [ ] Create horizontal/vertical split logic
- [ ] Implement pane sizing calculations
- [ ] Support nested layouts
- [ ] Add layout preset configurations

### Section 4: Pane Widget
- [ ] Create Pane widget struct
- [ ] Integrate tui-term for terminal rendering
- [ ] Implement scrollback view
- [ ] Handle pane focus state
- [ ] Support pane titles

### Section 5: Status Bar
- [ ] Create StatusBar widget
- [ ] Display session/pane info
- [ ] Show connection status
- [ ] Add keybinding hints section
- [ ] Implement dynamic updates

### Section 6: Claude State Indicators
- [ ] Define ClaudeState enum
- [ ] Create indicator widget
- [ ] Implement visual styles per state
- [ ] Add animation support for thinking state
- [ ] Integrate with pane display

### Section 7: Border Rendering
- [ ] Implement border widget
- [ ] Support multiple border styles
- [ ] Highlight active pane
- [ ] Render pane titles in borders

### Section 8: Testing
- [ ] Unit tests for layout calculations
- [ ] Widget rendering tests
- [ ] Event handling tests
- [ ] Integration tests with mock terminal

### Section 9: Documentation
- [ ] Document UI architecture
- [ ] Document keybindings
- [ ] Add usage examples
- [ ] Update CHANGELOG

## Acceptance Criteria

- [ ] Ratatui-based UI renders correctly
- [ ] Panes display PTY output via tui-term
- [ ] Status bar shows relevant session info
- [ ] Borders render between panes with active highlight
- [ ] Claude state indicators update in real-time
- [ ] Layout supports multiple pane arrangements
- [ ] Terminal resize handled gracefully
- [ ] No flickering or rendering artifacts
- [ ] Responsive to user input
- [ ] Clean terminal restoration on exit

## Dependencies

- **FEAT-007**: Client-server protocol (needed for receiving pane data)
- **FEAT-011**: Claude state detection (needed for state indicators)

## Notes

- Consider using `crossterm` backend for best cross-platform support
- tui-term provides terminal emulation widget for Ratatui
- May want to support both sync and async rendering
- Consider color theme customization in future iteration
- Performance critical: rendering should be efficient for high-throughput output
