# FEAT-012: Session Management - Session/Window/Pane Hierarchy

**Priority**: P1
**Component**: fugue-server
**Type**: new_feature
**Estimated Effort**: medium
**Business Value**: high
**Status**: completed

## Overview

Session/Window/Pane hierarchy data model, CRUD operations, active selection tracking, and protocol type conversion. This forms the core data model for the terminal multiplexer, enabling organized management of multiple terminal sessions with windows and panes.

## Requirements

### Session Struct
- Unique session identifier
- Collection of windows
- Active window tracking
- Session metadata (name, creation time, etc.)
- Session state (active, suspended, etc.)

### Window Struct
- Unique window identifier within session
- Collection of panes
- Active pane tracking
- Window metadata (name, index, etc.)
- Layout configuration

### Pane Struct
- Unique pane identifier within window
- Pane state and metadata
- Terminal/PTY association
- Size and position information
- Content type tracking

### SessionManager
- CRUD operations for sessions
- CRUD operations for windows within sessions
- CRUD operations for panes within windows
- Session lookup by ID
- Active session/window/pane tracking

### Active Selection Tracking
- Track currently active session
- Track currently active window within session
- Track currently active pane within window
- Methods to get/set active selections
- Navigation between sessions/windows/panes

### Protocol Type Conversion
- Convert Session to SessionInfo (for protocol messages)
- Convert Window to WindowInfo (for protocol messages)
- Convert Pane to PaneInfo (for protocol messages)
- Support serialization for IPC communication

### Event Emission
- Emit events on session creation/deletion
- Emit events on window creation/deletion
- Emit events on pane creation/deletion
- Emit events on active selection changes
- Support for event subscribers

## Affected Files

- `fugue-server/src/session/manager.rs` - SessionManager implementation
- `fugue-server/src/session/session.rs` - Session struct
- `fugue-server/src/session/window.rs` - Window struct
- `fugue-server/src/session/pane.rs` - Pane struct

## Implementation Tasks

### Section 1: Data Structures
- [x] Define Session struct with ID, name, windows collection
- [x] Define Window struct with ID, name, panes collection
- [x] Define Pane struct with ID, state, metadata
- [x] Define SessionState, WindowState, PaneState enums
- [x] Implement Default and builder patterns

### Section 2: SessionManager Core
- [x] Implement SessionManager struct
- [x] Add session CRUD operations (create, read, update, delete)
- [x] Add window CRUD operations within sessions
- [x] Add pane CRUD operations within windows
- [x] Implement session lookup by ID

### Section 3: Active Selection Tracking
- [x] Track active session ID
- [x] Track active window within sessions
- [x] Track active pane within windows
- [x] Implement get_active_session/window/pane methods
- [x] Implement set_active methods with validation

### Section 4: Protocol Type Conversion
- [x] Implement Into<SessionInfo> for Session
- [x] Implement Into<WindowInfo> for Window
- [x] Implement Into<PaneInfo> for Pane
- [x] Add serialization support for IPC

### Section 5: Event Emission
- [x] Define session management events
- [x] Implement event emission on state changes
- [x] Add event subscriber registration
- [x] Integrate with session lifecycle

### Section 6: Testing
- [x] Unit tests for Session/Window/Pane structs
- [x] Unit tests for SessionManager CRUD operations
- [x] Unit tests for active selection tracking
- [x] Integration tests for hierarchy operations

## Acceptance Criteria

- [x] Session struct manages collection of windows
- [x] Window struct manages collection of panes
- [x] Pane struct holds state and metadata
- [x] SessionManager provides full CRUD for all levels
- [x] Active session/window/pane can be tracked and changed
- [x] Protocol type conversion works correctly
- [x] Events are emitted on state changes
- [x] All unit tests pass

## Dependencies

- FEAT-007 (prerequisite feature)

## Notes

- This is a foundational feature for the session management system
- Protocol types should match fugue-protocol definitions
- Event system should be non-blocking for performance
- Consider thread safety for concurrent access
