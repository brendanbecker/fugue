# FEAT-025: Pane Output Rendering

**Priority**: P0
**Component**: fugue-client
**Type**: new_feature
**Estimated Effort**: medium (3-4 hours)
**Business Value**: high
**Status**: new

## Overview

Wire ServerMessage::Output data to pane rendering in the client UI. This connects PTY output from the server to the client's terminal emulation and display.

## Requirements

1. Receive `ServerMessage::Output { pane_id, data }`
2. Find pane in `self.panes` map
3. Append data to pane's buffer/scrollback
4. Mark pane as dirty for redraw
5. Render buffer to terminal grid
6. Handle multi-byte UTF-8 sequences

## Affected Files

- `fugue-client/src/ui/app.rs` - stubs at lines 565, 606
- `fugue-client/src/ui/pane.rs` - existing `Pane::process_output()` method

## Current State

The infrastructure is already in place:

- `fugue-client/src/ui/pane.rs` has a `Pane` struct with:
  - `parser: Parser` - tui-term VT100 parser with scrollback
  - `process_output(&mut self, data: &[u8])` - processes bytes through parser
  - `screen()` - returns the underlying Screen for rendering
  - `render_pane()` function that uses `PseudoTerminal` widget

- `fugue-client/src/ui/app.rs` has:
  - `self.panes: HashMap<Uuid, PaneInfo>` from protocol
  - `ServerMessage::Output` handler with TODO stub at line 565
  - Need to bridge between `PaneInfo` (protocol) and `Pane` (ui/pane.rs)

## Technical Notes

- Current stubs: `// TODO: Update pane terminal with output data`
- For MVP: render raw bytes without ANSI parsing (tui-term handles this)
- ANSI parsing (FEAT-014) is not done - tui-term/vt100 handles basic parsing
- Layout computation exists in `ui/layout.rs`
- Pane structure exists in `ui/pane.rs` with full VT100 emulation

## Implementation Tasks

### Section 1: Bridge Pane Types
- [ ] Understand relationship between `PaneInfo` (protocol) and `Pane` (ui)
- [ ] Decide: either add `PaneManager` to `App`, or store `Pane` alongside `PaneInfo`
- [ ] Update `App` struct to maintain UI panes with terminal state

### Section 2: Wire Output to Pane
- [ ] In `handle_server_message()` for `ServerMessage::Output`:
  - [ ] Look up the UI `Pane` by `pane_id`
  - [ ] Call `pane.process_output(&data)`
  - [ ] Mark dirty flag if using dirty tracking
- [ ] Handle case where pane_id doesn't exist (log warning)

### Section 3: Pane Lifecycle Integration
- [ ] On `ServerMessage::PaneCreated`: create corresponding UI `Pane`
- [ ] On `ServerMessage::PaneClosed`: remove UI `Pane`
- [ ] On `ServerMessage::Attached`: create UI panes for all existing panes
- [ ] Sync pane dimensions with layout

### Section 4: Rendering Integration
- [ ] Update `draw()` method to render UI panes using `render_pane()`
- [ ] Use layout system to get pane rectangles
- [ ] Ensure active pane highlighting works

### Section 5: Resize Handling
- [ ] When terminal resizes, resize all UI panes
- [ ] Send resize message to server for PTY resize
- [ ] Handle partial/incomplete UTF-8 at message boundaries (tui-term handles)

### Section 6: Testing
- [ ] Manual test: run shell commands, verify output displays
- [ ] Test multiple panes render independently
- [ ] Test scrollback works (scroll up/down)
- [ ] Test resize doesn't corrupt display
- [ ] Performance test: large output doesn't lag

## Acceptance Criteria

- [ ] Shell output appears in pane
- [ ] Multiple panes render independently
- [ ] Scrollback buffer works
- [ ] No visual corruption on resize
- [ ] Reasonable performance (60fps target)

## Dependencies

- FEAT-023 (PTY Output Polling) - server must send Output messages
- FEAT-022 (Message Routing) - messages must route to client

## Notes

- The `Pane` struct uses `tui_term::vt100::Parser` which handles ANSI escape sequences
- `PseudoTerminal` widget from `tui-term` crate handles rendering the Screen
- Scrollback is configured to 1000 lines in `Pane::new()`
- The `render_pane()` function already handles Claude state indicators
- Focus state affects border color (cyan for focused, dark gray for unfocused)
