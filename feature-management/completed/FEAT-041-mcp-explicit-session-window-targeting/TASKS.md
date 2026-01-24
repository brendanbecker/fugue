# Task Breakdown: FEAT-041

**Work Item**: [FEAT-041: MCP Explicit Session and Window Targeting for fugue_create_pane](PROMPT.md)
**Status**: Not Started
**Last Updated**: 2026-01-10

## Prerequisites

- [ ] Read and understand PROMPT.md
- [ ] Review PLAN.md and update if needed
- [ ] Identify current fugue_create_pane implementation in fugue-server/src/mcp/
- [ ] Understand SessionFilter and WindowFilter types in handler
- [ ] Review how handle_create_pane_with_options uses filters

## Phase 1: Schema Update (tools.rs)

### 1.1 Locate Tool Definition
- [ ] Find fugue_create_pane tool definition in fugue-server/src/mcp/tools.rs
- [ ] Understand current input_schema structure
- [ ] Identify where to add new properties

### 1.2 Add Session Property
- [ ] Add "session" property to input_schema
- [ ] Type: string
- [ ] Description: "Target session (UUID or name). Uses active session if omitted."
- [ ] Mark as optional (not in required array)

### 1.3 Add Window Property
- [ ] Add "window" property to input_schema
- [ ] Type: string
- [ ] Description: "Target window (UUID or name). Uses first window in session if omitted."
- [ ] Mark as optional (not in required array)

### 1.4 Update Tool Description
- [ ] Update tool description to mention session/window targeting
- [ ] Clarify default behavior when not specified

## Phase 2: Parsing Update (server.rs)

### 2.1 Locate ToolParams Struct
- [ ] Find ToolParams::CreatePane struct in fugue-server/src/mcp/server.rs
- [ ] Or find equivalent argument parsing structure

### 2.2 Add Session Field
- [ ] Add `session: Option<String>` field to CreatePane args
- [ ] Ensure serde deserialization handles optional field

### 2.3 Add Window Field
- [ ] Add `window: Option<String>` field to CreatePane args
- [ ] Ensure serde deserialization handles optional field

## Phase 3: Bridge Update (bridge.rs)

### 3.1 Add Filter Resolution Functions
- [ ] Implement resolve_session_filter(Option<String>) -> Option<SessionFilter>
- [ ] Parse as UUID first, fall back to name matching
- [ ] Implement resolve_window_filter(Option<String>) -> Option<WindowFilter>
- [ ] Parse as UUID first, fall back to name matching

### 3.2 Update tool_create_pane Function
- [ ] Locate tool_create_pane() in fugue-server/src/mcp/bridge.rs
- [ ] Add session and window parameters to function signature (or args struct)
- [ ] Call resolve_session_filter with session argument
- [ ] Call resolve_window_filter with window argument
- [ ] Pass resolved filters to handle_create_pane_with_options()

### 3.3 Replace Hardcoded None Values
- [ ] Find where session_filter: None is hardcoded
- [ ] Replace with resolved session_filter
- [ ] Find where window_filter: None is hardcoded
- [ ] Replace with resolved window_filter

## Phase 4: Response Enhancement

### 4.1 Update Response Structure
- [ ] Identify current response format in tool_create_pane
- [ ] Add session_id field to response
- [ ] Add window_id field to response
- [ ] Ensure handler returns necessary context

### 4.2 Extract Session/Window Info
- [ ] Get session_id from handler result or lookup
- [ ] Get window_id from handler result or lookup
- [ ] Include in JSON response

## Phase 5: Error Handling

### 5.1 Session Not Found Error
- [ ] Return clear error when session filter matches nothing
- [ ] Error message should include the provided session identifier
- [ ] Error type should be ToolError::NotFound or similar

### 5.2 Window Not Found Error
- [ ] Return clear error when window filter matches nothing
- [ ] Error message should include the provided window identifier
- [ ] Consider: should it create window in session if not found? (Probably no)

## Phase 6: Testing

### 6.1 Unit Tests for Filter Resolution
- [ ] Test resolve_session_filter with valid UUID string
- [ ] Test resolve_session_filter with session name
- [ ] Test resolve_session_filter with None (returns None)
- [ ] Test resolve_window_filter with valid UUID string
- [ ] Test resolve_window_filter with window name
- [ ] Test resolve_window_filter with None (returns None)

### 6.2 Integration Tests
- [ ] Test create pane with explicit session UUID
- [ ] Test create pane with explicit session name
- [ ] Test create pane with explicit window UUID
- [ ] Test create pane with explicit window name
- [ ] Test create pane with session and window together
- [ ] Test create pane with no targeting (default behavior preserved)
- [ ] Test response includes session_id and window_id

### 6.3 Error Case Tests
- [ ] Test create pane with invalid session UUID
- [ ] Test create pane with non-existent session name
- [ ] Test create pane with invalid window UUID
- [ ] Test create pane with non-existent window name

### 6.4 Regression Tests
- [ ] Run existing MCP test suite
- [ ] Verify existing fugue_create_pane calls still work
- [ ] Check that callers not using new params see no change

## Phase 7: Documentation

### 7.1 Update Tool Description
- [ ] Clear description of session parameter
- [ ] Clear description of window parameter
- [ ] Document UUID vs name matching behavior
- [ ] Document default behavior when omitted

### 7.2 Code Comments
- [ ] Comment resolve_session_filter function
- [ ] Comment resolve_window_filter function
- [ ] Document filter resolution order (UUID first, then name)

### 7.3 Update CHANGELOG
- [ ] Add entry for new session/window targeting
- [ ] Note that this enables multi-session orchestration

## Completion Checklist

- [ ] All Phase 1 tasks complete (Schema Update)
- [ ] All Phase 2 tasks complete (Parsing Update)
- [ ] All Phase 3 tasks complete (Bridge Update)
- [ ] All Phase 4 tasks complete (Response Enhancement)
- [ ] All Phase 5 tasks complete (Error Handling)
- [ ] All Phase 6 tasks complete (Testing)
- [ ] All Phase 7 tasks complete (Documentation)
- [ ] All tests passing
- [ ] PLAN.md updated with final approach
- [ ] feature_request.json status updated to "completed"
- [ ] Ready for review/merge

---
*Check off tasks as you complete them. Update status field above.*
