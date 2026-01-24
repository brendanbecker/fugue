# Task Breakdown: FEAT-029

**Work Item**: [FEAT-029: MCP Natural Language Terminal Control](PROMPT.md)
**Status**: Not Started
**Last Updated**: 2026-01-09

## Prerequisites

- [ ] Read and understand PROMPT.md
- [ ] Review PLAN.md and update if needed
- [ ] Verify BUG-003 is fixed (session creation with default pane)
- [ ] Review existing MCP handler patterns in handlers.rs
- [ ] Review existing tool definitions in tools.rs
- [ ] Identify where tool routing happens (likely server.rs)

## Phase 1: List Tools (Read-Only, Lowest Risk)

### 1.1 Add fugue_list_sessions

#### Tool Definition (tools.rs)
- [ ] Add `fugue_list_sessions` tool definition
- [ ] Define empty input schema (no required parameters)
- [ ] Write clear description for Claude

#### Handler (handlers.rs)
- [ ] Implement `list_sessions(&self) -> Result<String, McpError>`
- [ ] Iterate over all sessions from session_manager
- [ ] Return JSON array with: id, name, window_count, pane_count, created_at
- [ ] Handle empty sessions case (return empty array)

#### Routing (server.rs)
- [ ] Find handle_call_tool or equivalent routing function
- [ ] Add case for "fugue_list_sessions"
- [ ] Call ctx.list_sessions()

#### Tests
- [ ] Add `test_list_sessions_empty`
- [ ] Add `test_list_sessions_with_sessions`

### 1.2 Add fugue_list_windows

#### Tool Definition (tools.rs)
- [ ] Add `fugue_list_windows` tool definition
- [ ] Define optional "session" parameter
- [ ] Write clear description for Claude

#### Handler (handlers.rs)
- [ ] Implement `list_windows(&self, session: Option<&str>) -> Result<String, McpError>`
- [ ] If session provided, find by name or ID
- [ ] If session not provided, use first session
- [ ] Return error if session not found and name was specified
- [ ] Return JSON array with: id, index, name, pane_count, is_active

#### Routing (server.rs)
- [ ] Add case for "fugue_list_windows"
- [ ] Extract optional session parameter
- [ ] Call ctx.list_windows(session)

#### Tests
- [ ] Add `test_list_windows_empty_session`
- [ ] Add `test_list_windows_with_windows`
- [ ] Add `test_list_windows_session_not_found`

## Phase 2: Create Tools (Builds on BUG-003 Pattern)

### 2.1 Add fugue_create_session

#### Tool Definition (tools.rs)
- [ ] Add `fugue_create_session` tool definition
- [ ] Define optional "name" parameter
- [ ] Write clear description for Claude

#### Handler (handlers.rs)
- [ ] Implement `create_session(&mut self, name: Option<&str>) -> Result<String, McpError>`
- [ ] Generate name if not provided (e.g., "session-{n}")
- [ ] Create session via session_manager
- [ ] Create default window in session
- [ ] Create default pane in window
- [ ] Initialize parser on pane
- [ ] Spawn PTY for pane (BUG-003 pattern)
- [ ] Return JSON with: session_id, session_name, window_id, pane_id, status

#### Routing (server.rs)
- [ ] Add case for "fugue_create_session"
- [ ] Extract optional name parameter
- [ ] Call ctx.create_session(name)

#### Tests
- [ ] Add `test_create_session_default_name`
- [ ] Add `test_create_session_custom_name`
- [ ] Add `test_create_session_has_default_pane_with_pty`

### 2.2 Add fugue_create_window

#### Tool Definition (tools.rs)
- [ ] Add `fugue_create_window` tool definition
- [ ] Define optional parameters: session, name, command
- [ ] Write clear description for Claude

#### Handler (handlers.rs)
- [ ] Implement `create_window(&mut self, session: Option<&str>, name: Option<&str>, command: Option<&str>) -> Result<String, McpError>`
- [ ] Find session by name/ID, or use first, or create new
- [ ] Create window with optional name
- [ ] Create default pane in window
- [ ] Initialize parser on pane
- [ ] Spawn PTY with command or default shell
- [ ] Return JSON with: window_id, pane_id, session (name), status

#### Routing (server.rs)
- [ ] Add case for "fugue_create_window"
- [ ] Extract optional parameters: session, name, command
- [ ] Call ctx.create_window(session, name, command)

#### Tests
- [ ] Add `test_create_window_in_first_session`
- [ ] Add `test_create_window_in_specific_session`
- [ ] Add `test_create_window_session_not_found`
- [ ] Add `test_create_window_with_custom_command`
- [ ] Add `test_create_window_creates_session_if_none`

## Phase 3: Fix Split Direction Bug

### 3.1 Investigate Current Layout Model
- [ ] Read window.rs to understand pane storage
- [ ] Check if Window tracks pane positions/layout
- [ ] Check if there's a layout calculation on pane creation
- [ ] Document findings in PLAN.md

### 3.2 Implement or Document
- [ ] If layout exists: Use `_direction` in layout calculation
- [ ] If layout doesn't exist: Document limitation in PROMPT.md
- [ ] If layout doesn't exist: Create follow-up work item for layout system

### 3.3 Tests (if implemented)
- [ ] Add `test_create_pane_horizontal_split`
- [ ] Add `test_create_pane_vertical_split`
- [ ] Verify splits have correct relative positions

## Phase 4: Update Tests and Finalize

### 4.1 Update tools.rs Tests
- [ ] Update `test_expected_tools_present` to check for all 11 tools
- [ ] Verify test still lists all expected tool names

### 4.2 Run All Tests
- [ ] Run `cargo test -p fugue-server`
- [ ] Fix any failing tests
- [ ] Verify no regressions in existing functionality

### 4.3 Manual Integration Test
- [ ] Start server with MCP enabled
- [ ] Test `fugue_list_sessions` returns correct data
- [ ] Test `fugue_create_session` creates working session
- [ ] Test `fugue_list_windows` shows windows
- [ ] Test `fugue_create_window` creates working window
- [ ] Test natural language scenario end-to-end

### 4.4 Documentation
- [ ] Update any MCP documentation if it exists
- [ ] Ensure all new handlers have doc comments
- [ ] Update PROMPT.md with final status

## Completion Checklist

- [ ] All Phase 1 tasks complete (List Tools)
- [ ] All Phase 2 tasks complete (Create Tools)
- [ ] Phase 3 complete (Split Direction - fix or document)
- [ ] All Phase 4 tasks complete (Tests and Finalize)
- [ ] All tests passing
- [ ] PLAN.md updated with implementation notes
- [ ] feature_request.json status updated to "completed"
- [ ] Ready for review/merge

---
*Check off tasks as you complete them. Update status field above.*
