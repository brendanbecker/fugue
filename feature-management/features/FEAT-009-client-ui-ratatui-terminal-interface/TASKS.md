# Task Breakdown: FEAT-009

**Work Item**: [FEAT-009: Client UI - Ratatui Terminal Interface](PROMPT.md)
**Status**: Not Started
**Last Updated**: 2026-01-08

## Prerequisites

- [ ] Read and understand PROMPT.md
- [ ] Review PLAN.md and update if needed
- [ ] Review current ui.rs stub
- [ ] Review Ratatui documentation and examples
- [ ] Review tui-term crate documentation
- [ ] Verify FEAT-007 (protocol) status
- [ ] Verify FEAT-011 (Claude state detection) status

## Design Tasks

- [ ] Design App struct and state model
- [ ] Design event handling architecture
- [ ] Define widget component hierarchy
- [ ] Design layout constraint system
- [ ] Define ClaudeState enum and transitions
- [ ] Plan integration points with server connection
- [ ] Document keybinding scheme

## Implementation Tasks

### Dependencies Setup
- [ ] Add ratatui to Cargo.toml
- [ ] Add tui-term to Cargo.toml
- [ ] Add crossterm to Cargo.toml
- [ ] Verify dependency compatibility

### Core Framework (ui.rs)
- [ ] Create App struct with state
- [ ] Implement terminal initialization (raw mode, alternate screen)
- [ ] Implement terminal cleanup (restore on exit/panic)
- [ ] Create event loop skeleton
- [ ] Handle keyboard input events
- [ ] Handle mouse events (optional)
- [ ] Handle terminal resize (SIGWINCH)
- [ ] Integrate server message handling into event loop

### Layout System (ui/layout.rs)
- [ ] Define LayoutNode enum (Split, Pane)
- [ ] Implement Direction enum (Horizontal, Vertical)
- [ ] Create layout constraint resolver
- [ ] Implement split ratio calculations
- [ ] Handle minimum pane size constraints
- [ ] Implement layout preset: single pane
- [ ] Implement layout preset: horizontal split
- [ ] Implement layout preset: vertical split
- [ ] Implement layout preset: grid (2x2)
- [ ] Support nested/recursive layouts
- [ ] Handle resize recalculation

### Pane Widget (ui/pane.rs)
- [ ] Create PaneWidget struct
- [ ] Integrate tui-term PseudoTerminal
- [ ] Implement StatefulWidget trait
- [ ] Handle focus state (active/inactive styling)
- [ ] Implement scrollback offset handling
- [ ] Support pane title display
- [ ] Handle "new content" indicator when scrolled
- [ ] Implement pane content updates from server

### Status Bar (ui/status.rs)
- [ ] Create StatusBar widget
- [ ] Display session name/ID
- [ ] Display active pane indicator
- [ ] Show connection status (connected/disconnected)
- [ ] Add keybinding hints section
- [ ] Support dynamic status messages
- [ ] Implement timestamp/uptime display

### Border Rendering (ui/borders.rs)
- [ ] Create BorderWidget
- [ ] Support single-line border style
- [ ] Support double-line border style
- [ ] Support rounded border style
- [ ] Implement active pane border highlight
- [ ] Render pane title in top border
- [ ] Handle corner intersections for splits

### Claude State Indicators (ui/indicators.rs)
- [ ] Define ClaudeState enum
- [ ] Create ClaudeIndicator widget
- [ ] Implement Idle state display
- [ ] Implement Thinking state display (with animation)
- [ ] Implement ToolUse state display
- [ ] Implement Error state display
- [ ] Implement Complete state display
- [ ] Create animation tick handler for Thinking
- [ ] Style indicators with appropriate colors

### Event Handling
- [ ] Define AppEvent enum (Key, Mouse, Resize, Server, Tick)
- [ ] Implement event dispatch to appropriate handlers
- [ ] Handle pane navigation keys
- [ ] Handle scroll keys
- [ ] Handle quit/exit key
- [ ] Handle layout toggle keys
- [ ] Forward input to active pane

### Server Integration
- [ ] Receive pane content updates
- [ ] Receive Claude state updates
- [ ] Handle session/pane creation messages
- [ ] Handle session/pane removal messages
- [ ] Sync local state with server state

## Testing Tasks

- [ ] Unit test: LayoutNode constraint resolution
- [ ] Unit test: Split ratio calculations
- [ ] Unit test: ClaudeState transitions
- [ ] Widget test: PaneWidget rendering
- [ ] Widget test: StatusBar rendering
- [ ] Widget test: BorderWidget rendering
- [ ] Integration test: Event loop with mock events
- [ ] Integration test: Layout resize handling
- [ ] Manual test: Visual verification on xterm
- [ ] Manual test: Visual verification on alacritty
- [ ] Manual test: Visual verification on Windows Terminal

## Documentation Tasks

- [ ] Document UI architecture in PLAN.md
- [ ] Document keybindings
- [ ] Add usage examples
- [ ] Document customization options
- [ ] Update CHANGELOG

## Verification Tasks

- [ ] All acceptance criteria from PROMPT.md met
- [ ] Tests passing
- [ ] No rendering artifacts or flickering
- [ ] Terminal restored cleanly on exit
- [ ] Responsive to user input
- [ ] Update feature_request.json status
- [ ] Document completion in comments.md

## Completion Checklist

- [ ] All implementation tasks complete
- [ ] All tests passing
- [ ] Documentation updated
- [ ] PLAN.md reflects final implementation
- [ ] Dependencies FEAT-007 and FEAT-011 verified
- [ ] Ready for review/merge

---
*Check off tasks as you complete them. Update status field above.*
