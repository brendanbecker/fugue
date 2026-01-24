# FEAT-024: Session Selection UI

**Priority**: P1
**Component**: fugue-client
**Type**: enhancement
**Estimated Effort**: small (2 hours)
**Business Value**: high

## Overview

Implement the session selection UI that appears when the client connects. This UI allows users to select from available sessions, view session metadata, and create new sessions.

## Requirements

### 1. Render Session List Using Ratatui Widgets
- Display available sessions in a scrollable list
- Use ratatui List or Table widget for session display
- Handle empty state gracefully (no sessions available)

### 2. Highlight Currently Selected Session
- Visual indication of the currently selected session
- Use distinct styling (color, prefix marker, or background)
- Ensure selection is visible even when scrolled

### 3. Handle Up/Down Arrow Key Navigation
- Up arrow / 'k' moves selection up
- Down arrow / 'j' moves selection down
- Wrap-around behavior optional (currently not implemented)
- Page up/down for faster navigation (optional enhancement)

### 4. Enter Key Sends AttachSession Message
- Pressing Enter attaches to the selected session
- Send `ClientMessage::AttachSession { session_id }` to server
- Transition to Attached state on successful attachment

### 5. Option to Create New Session
- 'n' key creates a new session
- Optionally prompt for session name
- Auto-generate unique name if not provided
- Auto-attach to newly created session

### 6. Show Session Metadata
- Session name
- Window count
- Pane count (if available)
- Number of attached clients
- Creation time (optional)

## Current State

The session selection UI has a basic implementation in `fugue-client/src/ui/app.rs`:
- `handle_session_select_input()` at line ~478 handles keyboard input
- `draw_session_select()` at line ~667 renders the session list
- Basic navigation and selection works
- Session list populated from `ServerMessage::SessionList`

## Location

- **Primary file**: `fugue-client/src/ui/app.rs`
- **Handler**: `handle_session_select_input()` at line ~478
- **Renderer**: `draw_session_select()` at line ~667

## Affected Files

- `fugue-client/src/ui/app.rs` - Main session selection logic

## Implementation Tasks

### Section 1: Review and Assessment
- [ ] Review current `handle_session_select_input()` implementation
- [ ] Review current `draw_session_select()` implementation
- [ ] Identify gaps vs requirements
- [ ] Determine if ratatui List widget should replace Paragraph

### Section 2: UI Enhancement
- [ ] Improve session list rendering with better styling
- [ ] Add session metadata display (window count, pane count)
- [ ] Improve selection highlight visibility
- [ ] Add border and title styling

### Section 3: Navigation Enhancement
- [ ] Verify up/down navigation works correctly
- [ ] Add 'r' key for refresh (already implemented)
- [ ] Consider adding Page Up/Down for long lists
- [ ] Ensure selection bounds checking is correct

### Section 4: Session Creation
- [ ] Verify 'n' key creates new session
- [ ] Consider adding session name prompt (optional)
- [ ] Auto-attach behavior after creation

### Section 5: Testing
- [ ] Manual test: navigation with arrow keys and j/k
- [ ] Manual test: session creation with 'n'
- [ ] Manual test: attach with Enter
- [ ] Manual test: empty session list display
- [ ] Manual test: refresh with 'r'

### Section 6: Documentation
- [ ] Document keybindings in help text
- [ ] Update any relevant documentation

## Acceptance Criteria

- [ ] Sessions displayed in a list with clear visual formatting
- [ ] Keyboard navigation works (up/down, j/k)
- [ ] Can attach to existing session with Enter
- [ ] Can create new session with 'n'
- [ ] Visual feedback on current selection
- [ ] Session metadata visible (name, window count, client count)

## Dependencies

- **FEAT-021**: Server Socket - required to receive session list from server

## Technical Notes

- State machine already exists (`AppState::SessionSelect`)
- Handler stub exists and is functional
- Session list populated from `ServerMessage::SessionList`
- FEAT-009 UI framework is complete and provides ratatui infrastructure
- Current implementation uses Paragraph widget; could upgrade to List widget for better semantics
