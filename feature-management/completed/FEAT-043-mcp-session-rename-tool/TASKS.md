# Task Breakdown: FEAT-043

**Work Item**: [FEAT-043: MCP Session Rename Tool](PROMPT.md)
**Status**: Completed
**Last Updated**: 2026-01-10

## Prerequisites

- [x] Read and understand PROMPT.md
- [x] Review PLAN.md and update if needed
- [x] Identify Session struct location and verify name field exists
- [x] Understand SessionManager API and session lookup patterns
- [x] Review existing MCP tool patterns (fugue_rename_window, fugue_rename_pane from FEAT-036)

## Phase 1: Session Manager API

### 1.1 Verify Session Structure
- [x] Locate Session struct in fugue-server/src/session/
- [x] Verify `name` field exists on Session
- [x] Session already has name field and set_name() method
- [x] Ensure name is initialized during session creation

### 1.2 Add Rename Method
- [x] Add `rename_session(session_id: Uuid, new_name: String) -> Result<String, Error>` to SessionManager
- [x] Implement session lookup by ID
- [x] Update session's name field
- [x] Return appropriate error if session not found
- [x] Return old name for response

### 1.3 Implement Uniqueness Check
- [x] Before rename, check if new_name is already in use
- [x] Exclude the session being renamed from duplicate check
- [x] Return SessionExists error if name already exists
- [x] Allow rename to same name (no-op case)

### 1.4 Add Error Types
- [x] Used existing `CcmuxError::SessionExists(String)`
- [x] Used existing `CcmuxError::SessionNotFound(String)`
- [x] Added `ErrorCode::SessionNameExists` to protocol

## Phase 2: MCP Tool Definition

### 2.1 Add Tool to tools.rs
- [x] Located tool definitions in fugue-server/src/mcp/tools.rs
- [x] Added `fugue_rename_session` tool definition
- [x] Set name: "fugue_rename_session"
- [x] Set description: "Rename a session for easier identification"

### 2.2 Define Input Schema
- [x] Added "session" property (type: string)
- [x] Added description: "Session to rename (UUID or current name)"
- [x] Added "name" property (type: string)
- [x] Added description: "New display name for the session"
- [x] Added required array: ["session", "name"]

## Phase 3: MCP Handler

### 3.1 Create Handler Function
- [x] Added `tool_rename_session()` function in fugue-server/src/mcp/bridge.rs
- [x] Accept session_filter and new_name parameters
- [x] Return ToolResult

### 3.2 Implement Session Resolution
- [x] Try to parse session argument as UUID
- [x] If UUID, lookup session by ID
- [x] If not UUID, lookup session by name
- [x] Return NotFound error if session doesn't exist

### 3.3 Call SessionManager
- [x] Get previous name returned from rename (for response)
- [x] Call session_manager.rename_session(session_id, new_name)
- [x] Handle and convert any errors

### 3.4 Build Response
- [x] Create JSON response with success: true
- [x] Include session_id in response
- [x] Include previous_name in response
- [x] Include new_name in response

### 3.5 Trigger Persistence
- [x] Verified WAL entry type exists: WalEntry::SessionRenamed
- [x] Verified recovery handles SessionRenamed entries
- [x] SessionSnapshot includes name field for checkpoints

## Phase 4: Routing Integration

### 4.1 Add Tool Routing
- [x] Added dispatch case in dispatch_tool() in bridge.rs
- [x] Added case for "fugue_rename_session"
- [x] Parse arguments (session, name)
- [x] Call tool_rename_session handler

### 4.2 Add Protocol Messages
- [x] Added ClientMessage::RenameSession variant
- [x] Added ServerMessage::SessionRenamed variant
- [x] Added routing in handlers/mod.rs
- [x] Added handle_rename_session() handler in handlers/session.rs

## Phase 5: Persistence Verification

### 5.1 Verify Checkpoint Serialization
- [x] Located session serialization in persistence/types.rs
- [x] Verified name field is included in SessionSnapshot
- [x] Name persisted in checkpoints

### 5.2 Verify WAL Capture
- [x] WalEntry::SessionRenamed exists
- [x] Recovery logic handles SessionRenamed (recovery.rs:167-171)
- [x] Test exists for WAL replay: test_recovery_session_rename

### 5.3 Test Restart Scenario
- [x] Test exists in recovery.rs that verifies rename persists

## Phase 6: Testing

### 6.1 Unit Tests for SessionManager
- [x] test_rename_session_basic - rename with valid session ID
- [x] test_rename_session_not_found - error on invalid session ID
- [x] test_rename_session_duplicate_name - error on duplicate name
- [x] test_rename_session_same_name - success as no-op
- [x] test_rename_session_updates_name_lookup - verify lookup works after rename

### 6.2 Unit Tests for Handler
- [x] test_handle_rename_session_by_uuid - valid UUID
- [x] test_handle_rename_session_by_name - valid current name
- [x] test_handle_rename_session_not_found - invalid session
- [x] test_handle_rename_session_uuid_not_found - non-existent UUID
- [x] test_handle_rename_session_duplicate_name - duplicate name error
- [x] test_handle_rename_session_same_name - same name success

### 6.3 Integration Tests
- [x] MCP bridge tool dispatch tested via unit tests
- [x] Protocol messages tested

### 6.4 Error Case Tests
- [x] Test rename with non-existent session UUID
- [x] Test rename with non-existent session name
- [x] Test rename to name already in use
- [x] Error messages include the conflicting name

### 6.5 Persistence Tests
- [x] test_recovery_session_rename - rename and recovery

### 6.6 Regression Tests
- [x] All existing tests pass (135 tests)

## Phase 7: Documentation

### 7.1 Update Tool Description
- [x] Tool description is clear
- [x] Schema documents UUID vs name matching for session parameter
- [x] Required parameters documented

### 7.2 Code Comments
- [x] Added docstrings to rename_session method in SessionManager
- [x] Added docstring to handle_rename_session handler

### 7.3 Update CHANGELOG
- [x] Updated README.md with fugue_rename_session in MCP tools list

## Completion Checklist

- [x] All Phase 1 tasks complete (Session Manager API)
- [x] All Phase 2 tasks complete (MCP Tool Definition)
- [x] All Phase 3 tasks complete (MCP Handler)
- [x] All Phase 4 tasks complete (Routing Integration)
- [x] All Phase 5 tasks complete (Persistence Verification)
- [x] All Phase 6 tasks complete (Testing)
- [x] All Phase 7 tasks complete (Documentation)
- [x] All tests passing (135 tests)
- [ ] PLAN.md updated with final approach
- [ ] feature_request.json status updated to "completed"
- [x] Ready for review/merge

---
*All implementation tasks completed. Ready for final review.*
