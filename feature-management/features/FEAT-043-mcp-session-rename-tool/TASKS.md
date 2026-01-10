# Task Breakdown: FEAT-043

**Work Item**: [FEAT-043: MCP Session Rename Tool](PROMPT.md)
**Status**: Not Started
**Last Updated**: 2026-01-10

## Prerequisites

- [ ] Read and understand PROMPT.md
- [ ] Review PLAN.md and update if needed
- [ ] Identify Session struct location and verify name field exists
- [ ] Understand SessionManager API and session lookup patterns
- [ ] Review existing MCP tool patterns (ccmux_rename_window, ccmux_rename_pane from FEAT-036)

## Phase 1: Session Manager API

### 1.1 Verify Session Structure
- [ ] Locate Session struct in ccmux-server/src/session/
- [ ] Verify `name` field exists on Session
- [ ] If not, add `name: String` field to Session struct
- [ ] Ensure name is initialized during session creation

### 1.2 Add Rename Method
- [ ] Add `rename_session(session_id: SessionId, new_name: String) -> Result<(), Error>` to SessionManager
- [ ] Implement session lookup by ID
- [ ] Update session's name field
- [ ] Return appropriate error if session not found

### 1.3 Implement Uniqueness Check
- [ ] Before rename, check if new_name is already in use
- [ ] Exclude the session being renamed from duplicate check
- [ ] Return DuplicateSessionName error if name already exists
- [ ] Allow rename to same name (no-op case)

### 1.4 Add Error Types
- [ ] Add `Error::DuplicateSessionName(String)` if not present
- [ ] Add `Error::SessionNotFound(SessionId)` if not present
- [ ] Ensure errors can be converted to MCP ToolError

## Phase 2: MCP Tool Definition

### 2.1 Add Tool to tools.rs
- [ ] Locate tool definitions in ccmux-server/src/mcp/tools.rs
- [ ] Add `ccmux_rename_session` tool definition
- [ ] Set name: "ccmux_rename_session"
- [ ] Set description: "Rename a session for easier identification. Session names must be unique."

### 2.2 Define Input Schema
- [ ] Add "session" property (type: string)
- [ ] Add description: "Session to rename (UUID or current name)"
- [ ] Add "name" property (type: string)
- [ ] Add description: "New display name for the session"
- [ ] Add required array: ["session", "name"]

## Phase 3: MCP Handler

### 3.1 Create Handler Function
- [ ] Add `tool_rename_session()` function in ccmux-server/src/mcp/bridge.rs
- [ ] Accept session_manager, session (String), and name (String) parameters
- [ ] Return ToolResult

### 3.2 Implement Session Resolution
- [ ] Try to parse session argument as UUID
- [ ] If UUID, lookup session by ID
- [ ] If not UUID, lookup session by name
- [ ] Return NotFound error if session doesn't exist

### 3.3 Call SessionManager
- [ ] Get previous name before rename (for response)
- [ ] Call session_manager.rename_session(session_id, new_name)
- [ ] Handle and convert any errors

### 3.4 Build Response
- [ ] Create JSON response with success: true
- [ ] Include session_id in response
- [ ] Include previous_name in response
- [ ] Include new_name in response

### 3.5 Trigger Persistence
- [ ] Mark session as dirty after rename
- [ ] Ensure WAL captures the change
- [ ] Or trigger checkpoint if using different persistence model

## Phase 4: Routing Integration

### 4.1 Add Tool Routing
- [ ] Locate tool routing in ccmux-server/src/mcp/server.rs
- [ ] Add case for "ccmux_rename_session"
- [ ] Parse arguments (session, name)
- [ ] Call tool_rename_session handler

### 4.2 Add ToolParams Variant
- [ ] Add RenameSession variant to ToolParams enum if needed
- [ ] Include session: String and name: String fields
- [ ] Add serde deserialization support

## Phase 5: Persistence Verification

### 5.1 Verify Checkpoint Serialization
- [ ] Locate session serialization code in persistence/
- [ ] Verify name field is included in serialized session state
- [ ] If not, add name to serialization

### 5.2 Verify WAL Capture
- [ ] Check if session changes trigger WAL entries
- [ ] Verify rename operation creates appropriate WAL entry
- [ ] Test that WAL replay restores correct name

### 5.3 Test Restart Scenario
- [ ] Rename a session
- [ ] Restart the server
- [ ] Verify session has correct name after restart

## Phase 6: Testing

### 6.1 Unit Tests for SessionManager
- [ ] Test rename_session with valid session ID and new name
- [ ] Test rename_session with invalid session ID (should error)
- [ ] Test rename_session with duplicate name (should error)
- [ ] Test rename_session to same name (should succeed)
- [ ] Test session lookup by name after rename

### 6.2 Unit Tests for Handler
- [ ] Test tool_rename_session with valid UUID
- [ ] Test tool_rename_session with valid current name
- [ ] Test tool_rename_session with invalid session
- [ ] Test tool_rename_session with duplicate name
- [ ] Test response format is correct

### 6.3 Integration Tests
- [ ] End-to-end MCP call to rename session by UUID
- [ ] End-to-end MCP call to rename session by name
- [ ] Verify ccmux_list_sessions shows updated name
- [ ] Verify ccmux_create_pane can target by new name
- [ ] Verify ccmux_create_window can target by new name

### 6.4 Error Case Tests
- [ ] Test rename with non-existent session UUID
- [ ] Test rename with non-existent session name
- [ ] Test rename to name already in use
- [ ] Verify error messages are clear and helpful

### 6.5 Persistence Tests
- [ ] Rename session, restart, verify name persists
- [ ] Rename session multiple times, verify final name persists
- [ ] Crash recovery scenario if applicable

### 6.6 Regression Tests
- [ ] Run existing MCP test suite
- [ ] Verify existing session operations still work
- [ ] Verify session lookup by name still works for unrenamed sessions

## Phase 7: Documentation

### 7.1 Update Tool Description
- [ ] Ensure tool description is clear and helpful
- [ ] Document UUID vs name matching for session parameter
- [ ] Document uniqueness constraint

### 7.2 Code Comments
- [ ] Add comments to rename_session method
- [ ] Add comments to tool_rename_session handler
- [ ] Document any non-obvious logic

### 7.3 Update CHANGELOG
- [ ] Add entry for new ccmux_rename_session tool
- [ ] Note that session names must be unique
- [ ] Mention use cases (multi-agent, project organization)

## Completion Checklist

- [ ] All Phase 1 tasks complete (Session Manager API)
- [ ] All Phase 2 tasks complete (MCP Tool Definition)
- [ ] All Phase 3 tasks complete (MCP Handler)
- [ ] All Phase 4 tasks complete (Routing Integration)
- [ ] All Phase 5 tasks complete (Persistence Verification)
- [ ] All Phase 6 tasks complete (Testing)
- [ ] All Phase 7 tasks complete (Documentation)
- [ ] All tests passing
- [ ] PLAN.md updated with final approach
- [ ] feature_request.json status updated to "completed"
- [ ] Ready for review/merge

---
*Check off tasks as you complete them. Update status field above.*
