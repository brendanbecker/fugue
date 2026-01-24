# Task Breakdown: FEAT-036

**Work Item**: [FEAT-036: Session-aware MCP Commands with Window/Pane Naming](PROMPT.md)
**Status**: Not Started
**Last Updated**: 2026-01-09

## Prerequisites

- [ ] Read and understand PROMPT.md
- [ ] Review PLAN.md and update if needed
- [ ] Verify FEAT-029 (MCP Natural Language Control) is implemented
- [ ] Identify current MCP tool implementations in fugue-server/src/mcp/
- [ ] Understand current session data model in fugue-server/src/session/

## Phase 1: Data Model Updates

### 1.1 Session Attachment Tracking
- [ ] Add `attached_clients: AtomicUsize` field to Session struct
- [ ] Initialize to 0 in Session::new()
- [ ] Add `last_activity: Instant` field if not present
- [ ] Update Session::new() to set last_activity to Instant::now()

### 1.2 Pane Name Field
- [ ] Check if Pane struct has `name` field
- [ ] If not, add `name: Option<String>` field
- [ ] Update Pane::new() to accept optional name parameter
- [ ] Update any existing Pane construction sites

### 1.3 Window Name Field
- [ ] Verify Window struct has `name: String` field
- [ ] If missing, add `name: String` field
- [ ] Ensure Window::new() accepts name parameter

### 1.4 Data Model Unit Tests
- [ ] Test Session creation with attached_clients = 0
- [ ] Test Pane creation with and without name
- [ ] Test Window name field access

## Phase 2: Active Session Selection

### 2.1 Implement get_active_session Method
- [ ] Add `get_active_session(&self) -> Option<SessionId>` to SessionManager
- [ ] Iterate all sessions, find one with most attached clients
- [ ] On tie, prefer most recently active (last_activity)
- [ ] Return None only if no sessions exist

### 2.2 Helper Methods
- [ ] Add `increment_attached_clients(&self, session_id: SessionId)`
- [ ] Add `decrement_attached_clients(&self, session_id: SessionId)`
- [ ] Add `update_last_activity(&self, session_id: SessionId)`

### 2.3 Active Session Unit Tests
- [ ] Test get_active_session with single session, 0 clients
- [ ] Test get_active_session with single session, 1 client
- [ ] Test get_active_session with multiple sessions, one has clients
- [ ] Test get_active_session with multiple sessions, all 0 clients (use last_activity)
- [ ] Test get_active_session with no sessions (returns None)
- [ ] Test get_active_session with tie (uses last_activity)

## Phase 3: Client Attachment Tracking Integration

### 3.1 Track Client Session Attachment
- [ ] Identify where clients attach to sessions in message routing
- [ ] Call increment_attached_clients when client attaches
- [ ] Call decrement_attached_clients when client detaches
- [ ] Handle client disconnect (decrement if was attached)

### 3.2 Activity Tracking
- [ ] Update last_activity on any client interaction with session
- [ ] Consider: pane selection, window creation, input sent

### 3.3 Cleanup Logic
- [ ] Add logic to handle unclean client disconnects
- [ ] Consider heartbeat timeout for stale clients
- [ ] Log warnings when client count seems wrong

### 3.4 Integration Tests
- [ ] Test client attach increments counter
- [ ] Test client detach decrements counter
- [ ] Test client disconnect decrements counter
- [ ] Test multiple clients to same session

## Phase 4: Update Existing MCP Tools

### 4.1 Update fugue_list_windows
- [ ] Find handler in fugue-server/src/mcp/handlers.rs
- [ ] Use get_active_session() when session_id not provided
- [ ] Update tool description: "uses active session if omitted"
- [ ] Add session_context to response

### 4.2 Update fugue_create_window
- [ ] Find handler in handlers.rs
- [ ] Use get_active_session() when session_id not provided
- [ ] Update tool description
- [ ] Add session_context to response

### 4.3 Update fugue_create_pane
- [ ] Find handler in handlers.rs
- [ ] Use get_active_session() when session_id not provided
- [ ] Update tool description
- [ ] Add session_context to response
- [ ] Add name parameter to schema (see Phase 5)

### 4.4 Update fugue_list_panes
- [ ] Find handler in handlers.rs
- [ ] Use get_active_session() when session_id filter not provided
- [ ] Update tool description
- [ ] Include session_context in response

### 4.5 Update Other Session-Scoped Tools
- [ ] Audit all tools for session-scoped operations
- [ ] Apply same pattern to any other affected tools
- [ ] fugue_get_output - if session-scoped
- [ ] fugue_send_input - if session-scoped

### 4.6 Tool Update Tests
- [ ] Test fugue_list_windows uses active session
- [ ] Test fugue_create_window uses active session
- [ ] Test fugue_create_pane uses active session
- [ ] Test fugue_list_panes uses active session
- [ ] Test explicit session_id still works (not overridden)

## Phase 5: Pane Naming on Creation

### 5.1 Update fugue_create_pane Schema
- [ ] Add "name" field to input_schema in tools.rs
- [ ] Make name optional with description
- [ ] Schema: `{"name": {"type": "string", "description": "Optional display name for the pane"}}`

### 5.2 Update fugue_create_pane Handler
- [ ] Parse name from arguments
- [ ] Pass name to pane creation
- [ ] Store name in Pane struct

### 5.3 Update List Outputs
- [ ] Update fugue_list_panes to include "name" in pane objects
- [ ] Handle None name (omit or return null)
- [ ] Update fugue_list_windows to include window names

### 5.4 Pane Naming Tests
- [ ] Test create pane with name
- [ ] Test create pane without name
- [ ] Test list panes shows name
- [ ] Test list panes with unnamed pane

## Phase 6: Rename Tools

### 6.1 Implement fugue_rename_pane
- [ ] Add tool definition in tools.rs
- [ ] Schema: pane_id (required), name (required)
- [ ] Implement handler in handlers.rs
- [ ] Find pane by ID
- [ ] Update name field
- [ ] Return success with updated pane info

### 6.2 Implement fugue_rename_window
- [ ] Add tool definition in tools.rs
- [ ] Schema: window_id (required), name (required)
- [ ] Implement handler in handlers.rs
- [ ] Find window by ID
- [ ] Update name field
- [ ] Return success with updated window info

### 6.3 Route New Tools
- [ ] Add routing in server.rs or tools.rs
- [ ] Ensure tools are registered with MCP server

### 6.4 Rename Tool Tests
- [ ] Test rename pane with valid pane_id
- [ ] Test rename pane with invalid pane_id (error)
- [ ] Test rename window with valid window_id
- [ ] Test rename window with invalid window_id (error)
- [ ] Test rename persists (visible in subsequent list)

## Phase 7: Response Format Updates

### 7.1 Define Session Context Structure
- [ ] Create SessionContext struct or use inline JSON
- [ ] Fields: session_id, session_name

### 7.2 Update All Session-Scoped Responses
- [ ] fugue_list_windows - add session_context
- [ ] fugue_create_window - add session_context
- [ ] fugue_create_pane - add session_context
- [ ] fugue_list_panes - add session_context
- [ ] fugue_rename_pane - add session_context
- [ ] fugue_rename_window - add session_context

### 7.3 Response Format Tests
- [ ] Verify all responses include session_context
- [ ] Test session_context has correct session_id
- [ ] Test session_context has correct session_name

## Phase 8: Testing and Verification

### 8.1 Unit Test Summary
- [ ] All Phase 1 unit tests pass
- [ ] All Phase 2 unit tests pass
- [ ] All Phase 3 integration tests pass
- [ ] All Phase 4 tool update tests pass
- [ ] All Phase 5 naming tests pass
- [ ] All Phase 6 rename tests pass
- [ ] All Phase 7 response tests pass

### 8.2 Integration Testing
- [ ] Test full workflow: 2 sessions, create pane via MCP
- [ ] Verify pane created in session with attached client
- [ ] Test naming workflow: create named pane, list, rename
- [ ] Test with Claude via MCP (manual)

### 8.3 Edge Case Testing
- [ ] Test with 0 sessions (appropriate error)
- [ ] Test with 0 attached clients in all sessions (uses recent)
- [ ] Test session_id explicitly provided (overrides active session)
- [ ] Test rapid attach/detach
- [ ] Test very long pane/window names

### 8.4 Regression Testing
- [ ] Run existing MCP test suite
- [ ] Verify no breaking changes to response format
- [ ] Test existing tool consumers (if any)

## Phase 9: Documentation

### 9.1 Update Tool Descriptions
- [ ] fugue_list_windows: "uses active session if omitted"
- [ ] fugue_create_window: "uses active session if omitted"
- [ ] fugue_create_pane: "uses active session if omitted"
- [ ] fugue_list_panes: "uses active session if omitted"

### 9.2 Document Active Session Behavior
- [ ] Add comment in handlers.rs explaining active session logic
- [ ] Document fallback behavior (no attached clients)
- [ ] Document how session_context helps debugging

### 9.3 Code Comments
- [ ] Document get_active_session() method
- [ ] Document attached_clients tracking
- [ ] Document name field in Pane/Window

### 9.4 Update CHANGELOG
- [ ] Add entry for session-aware defaults
- [ ] Add entry for pane/window naming
- [ ] Add entry for new rename tools

## Completion Checklist

- [ ] All Phase 1 tasks complete (Data Model)
- [ ] All Phase 2 tasks complete (Active Session Selection)
- [ ] All Phase 3 tasks complete (Attachment Tracking)
- [ ] All Phase 4 tasks complete (Update Existing Tools)
- [ ] All Phase 5 tasks complete (Pane Naming)
- [ ] All Phase 6 tasks complete (Rename Tools)
- [ ] All Phase 7 tasks complete (Response Format)
- [ ] All Phase 8 tasks complete (Testing)
- [ ] All Phase 9 tasks complete (Documentation)
- [ ] All tests passing
- [ ] PLAN.md updated with final approach
- [ ] feature_request.json status updated to "completed"
- [ ] Ready for review/merge

---
*Check off tasks as you complete them. Update status field above.*
