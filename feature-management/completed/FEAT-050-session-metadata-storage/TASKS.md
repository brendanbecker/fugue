# Task Breakdown: FEAT-050

**Work Item**: [FEAT-050: Session Metadata Storage for Agent Identity](PROMPT.md)
**Status**: Not Started
**Last Updated**: 2026-01-10

## Prerequisites

- [ ] Read and understand PROMPT.md
- [ ] Review PLAN.md and update if needed
- [ ] Familiarize with Session struct in fugue-session
- [ ] Review existing MCP tool patterns in fugue-server

## Phase 1: Core Storage (session.rs)

### 1.1 Add Metadata Field
- [ ] Add `use std::collections::HashMap;` if not present
- [ ] Add `metadata: HashMap<String, String>` to Session struct
- [ ] Update Session::new() to initialize empty HashMap
- [ ] Update any other Session constructors

### 1.2 Add Accessor Methods
- [ ] Implement `get_metadata(&self, key: &str) -> Option<&str>`
- [ ] Implement `set_metadata(&mut self, key: String, value: String)`
- [ ] Implement `remove_metadata(&mut self, key: &str) -> Option<String>`
- [ ] Implement `all_metadata(&self) -> &HashMap<String, String>`

### 1.3 Update Session Tests
- [ ] Add test for metadata initialization (empty)
- [ ] Add test for set_metadata
- [ ] Add test for get_metadata
- [ ] Add test for remove_metadata
- [ ] Add test for overwriting metadata key

## Phase 2: Protocol Types (types.rs)

### 2.1 Update SessionInfo
- [ ] Add `metadata: HashMap<String, String>` to SessionInfo struct
- [ ] Update SessionInfo creation in server code
- [ ] Ensure serde derives handle HashMap correctly

### 2.2 Update SessionInfo Tests
- [ ] Update existing SessionInfo tests to include metadata
- [ ] Add test for SessionInfo serialization with metadata
- [ ] Add test for SessionInfo deserialization with metadata

## Phase 3: MCP Tools

### 3.1 Add Tool Definitions (tools.rs)
- [ ] Add `fugue_set_metadata` tool definition
- [ ] Add `fugue_get_metadata` tool definition

### 3.2 Implement Handlers (handlers.rs)
- [ ] Add handler for `fugue_set_metadata`
  - [ ] Parse session parameter (name or ID)
  - [ ] Validate key and value
  - [ ] Call session.set_metadata()
  - [ ] Return success response
- [ ] Add handler for `fugue_get_metadata`
  - [ ] Parse session parameter
  - [ ] If key provided, return single value
  - [ ] If key omitted, return all metadata
  - [ ] Handle key-not-found case

### 3.3 Update list_sessions
- [ ] Include metadata in session list response
- [ ] Update response format in handler
- [ ] Update any documentation/comments

### 3.4 MCP Tool Tests
- [ ] Test set_metadata with valid inputs
- [ ] Test set_metadata with invalid session
- [ ] Test get_metadata with specific key
- [ ] Test get_metadata with all keys
- [ ] Test get_metadata with missing key
- [ ] Test list_sessions includes metadata

## Phase 4: Persistence

### 4.1 Update Checkpoint Format
- [ ] Add metadata field to checkpoint session struct
- [ ] Update checkpoint serialization
- [ ] Update checkpoint deserialization

### 4.2 Handle Migration
- [ ] Ensure old checkpoints (without metadata) load correctly
- [ ] Default to empty HashMap for missing metadata field

### 4.3 Persistence Tests
- [ ] Test checkpoint round-trip with metadata
- [ ] Test checkpoint round-trip with empty metadata
- [ ] Test loading old checkpoint format (no metadata field)

## Phase 5: Integration Testing

- [ ] End-to-end: set metadata via MCP, get via MCP
- [ ] End-to-end: set metadata, list sessions, verify present
- [ ] End-to-end: set metadata, restart server, verify persisted
- [ ] End-to-end: multiple sessions with different metadata

## Phase 6: Documentation

- [ ] Update doc comments on Session struct
- [ ] Update doc comments on SessionInfo struct
- [ ] Add usage examples in MCP tool descriptions
- [ ] Update any relevant README or docs

## Completion Checklist

- [ ] All Phase 1 tasks complete (Core Storage)
- [ ] All Phase 2 tasks complete (Protocol Types)
- [ ] All Phase 3 tasks complete (MCP Tools)
- [ ] All Phase 4 tasks complete (Persistence)
- [ ] All Phase 5 tasks complete (Integration Testing)
- [ ] All tests passing
- [ ] PLAN.md updated with final approach
- [ ] Ready for review/merge

---
*Check off tasks as you complete them. Update status field above.*
