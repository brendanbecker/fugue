# Task Breakdown: FEAT-052

**Work Item**: [FEAT-052: Add fugue_kill_session MCP tool](PROMPT.md)
**Status**: Not Started
**Last Updated**: 2026-01-10

## Prerequisites

- [ ] Read and understand PROMPT.md
- [ ] Review PLAN.md and update if needed
- [ ] Review existing MCP tool pattern in fugue-server/src/mcp/tools.rs
- [ ] Review existing handlers in fugue-server/src/mcp/handlers.rs
- [ ] Verify DestroySession message in fugue-protocol/src/messages.rs

## Phase 1: Tool Definition

### 1.1 Add Tool Schema
- [ ] Open fugue-server/src/mcp/tools.rs
- [ ] Add `fugue_kill_session` tool to `get_tool_definitions()` vector
- [ ] Set name: "fugue_kill_session"
- [ ] Set description: "Kill/destroy a fugue session and all its windows and panes"
- [ ] Set input_schema with required "session" string parameter

### 1.2 Verify Tool Registration
- [ ] Build project: `cargo build`
- [ ] Verify no compilation errors
- [ ] Tool should be visible in MCP tool list

## Phase 2: Handler Implementation

### 2.1 Add Handler Function
- [ ] Open fugue-server/src/mcp/handlers.rs
- [ ] Add `handle_kill_session` function
- [ ] Parse "session" parameter from arguments
- [ ] Return error if session parameter missing

### 2.2 Session Resolution
- [ ] Check if session parameter is valid UUID
- [ ] If UUID: look up session by ID
- [ ] If not UUID: search sessions by name
- [ ] Return error if session not found

### 2.3 Send DestroySession Message
- [ ] Construct ClientMessage::DestroySession { session_id }
- [ ] Send message to daemon via existing channel
- [ ] Await response (if applicable)

### 2.4 Format Response
- [ ] On success: return JSON with success=true, session_id, session_name
- [ ] On error: return appropriate MCP error

### 2.5 Wire Up Handler
- [ ] Add routing for "fugue_kill_session" tool calls
- [ ] Call handle_kill_session when tool is invoked

## Phase 3: Testing

### 3.1 Build and Basic Test
- [ ] Build project: `cargo build`
- [ ] Start fugue daemon
- [ ] Create a test session
- [ ] Use MCP client to call fugue_kill_session

### 3.2 Test Kill by UUID
- [ ] Create session
- [ ] Get session UUID from list
- [ ] Call fugue_kill_session with UUID
- [ ] Verify session is removed from list

### 3.3 Test Kill by Name
- [ ] Create session with known name
- [ ] Call fugue_kill_session with name
- [ ] Verify session is removed from list

### 3.4 Test Error Cases
- [ ] Call fugue_kill_session with non-existent UUID
- [ ] Verify appropriate error response
- [ ] Call fugue_kill_session with non-existent name
- [ ] Verify appropriate error response

### 3.5 Unit Tests (Optional)
- [ ] Add unit test for handle_kill_session with valid UUID
- [ ] Add unit test for handle_kill_session with valid name
- [ ] Add unit test for handle_kill_session with invalid session

## Phase 4: Documentation

### 4.1 Tool Description
- [ ] Verify tool description is clear and complete
- [ ] Include example usage in description if helpful

### 4.2 Code Comments
- [ ] Add doc comment to handle_kill_session function
- [ ] Comment any non-obvious logic

### 4.3 Update CHANGELOG
- [ ] Add entry for new fugue_kill_session tool

## Completion Checklist

- [ ] Tool definition added to tools.rs
- [ ] Handler implemented in handlers.rs
- [ ] Handler wired up for routing
- [ ] Project builds without errors
- [ ] Kill by UUID works
- [ ] Kill by name works
- [ ] Error cases handled
- [ ] CHANGELOG updated
- [ ] All tests passing
- [ ] PLAN.md updated with final approach
- [ ] feature_request.json status updated to "completed"
- [ ] Ready for review/merge

---
*Check off tasks as you complete them. Update status field above.*
