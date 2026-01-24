# Task Breakdown: FEAT-028

**Work Item**: [FEAT-028: Orchestration Flexibility Refactor](PROMPT.md)
**Status**: Not Started
**Last Updated**: 2026-01-09

## Prerequisites

- [ ] Read and understand PROMPT.md
- [ ] Review PLAN.md and update if needed
- [ ] Verify FEAT-007 (Protocol Layer) is complete
- [ ] Identify all code that uses current orchestration types

## Phase 1: Protocol Type Changes (messages.rs)

### 1.1 Add New Types (Keep Old Temporarily)
- [ ] Add new `OrchestrationMessage` struct with `msg_type: String` and `payload: serde_json::Value`
- [ ] Add `OrchestrationTarget::Tagged(String)` variant
- [ ] Add constructor helpers (`OrchestrationMessage::new()`, `sync_request()`)
- [ ] Verify serde_json is already a dependency (should be via serde)

### 1.2 Remove Old Types
- [ ] Delete `WorkerStatus` enum
- [ ] Delete old `OrchestrationMessage` enum (all variants)
- [ ] Delete `OrchestrationTarget::Orchestrator` variant
- [ ] Update `ClientMessage::SendOrchestration` to use new types
- [ ] Update `ServerMessage::OrchestrationReceived` to use new types

### 1.3 Update Tests (messages.rs)
- [ ] Delete `test_worker_status_all_variants`
- [ ] Delete `test_orchestration_message_status_update`
- [ ] Delete `test_orchestration_message_task_assignment`
- [ ] Delete `test_orchestration_message_task_complete`
- [ ] Delete `test_orchestration_message_help_request`
- [ ] Delete `test_orchestration_target_orchestrator`
- [ ] Add `test_orchestration_message_new`
- [ ] Add `test_orchestration_message_serde_roundtrip`
- [ ] Add `test_orchestration_message_complex_payload`
- [ ] Add `test_orchestration_target_tagged`
- [ ] Update `test_client_message_send_orchestration` for new types
- [ ] Update `test_server_message_orchestration_received` for new types

## Phase 2: SessionInfo Changes (types.rs)

### 2.1 Modify SessionInfo
- [ ] Add `use std::collections::HashSet;` if not present
- [ ] Replace `is_orchestrator: bool` with `tags: HashSet<String>` in SessionInfo struct
- [ ] Update serde derives to handle HashSet (should work automatically)

### 2.2 Update Tests (types.rs)
- [ ] Update `test_session_info_creation` to use tags
- [ ] Update `test_session_info_clone` to use tags
- [ ] Update `test_session_info_equality` to test tag equality
- [ ] Update `test_session_info_with_worktree` to use tags
- [ ] Update `test_session_info_without_worktree` to use tags
- [ ] Update `test_session_info_with_worktree_serde` to test tag serialization
- [ ] Add `test_session_info_multiple_tags`
- [ ] Add `test_session_info_empty_tags`

### 2.3 Update Tests (messages.rs that use SessionInfo)
- [ ] Update `test_server_message_session_list` to use tags
- [ ] Update `test_server_message_session_created` to use tags
- [ ] Update `test_server_message_attached` to use tags

## Phase 3: Router Refactor (router.rs)

### 3.1 Update Data Structures
- [ ] Replace `orchestrators: HashMap<String, Uuid>` with `session_tags: HashMap<Uuid, HashSet<String>>`
- [ ] Update `MessageRouter::new()` to initialize `session_tags`

### 3.2 Update register() Method
- [ ] Change parameter from `is_orchestrator: bool` to `tags: HashSet<String>`
- [ ] Store tags in `session_tags` map
- [ ] Remove orchestrator registration logic

### 3.3 Add Tag Management Methods
- [ ] Implement `add_tag(&mut self, session_id: Uuid, tag: String) -> bool`
- [ ] Implement `remove_tag(&mut self, session_id: Uuid, tag: &str) -> bool`
- [ ] Implement `sessions_with_tag(&self, tag: &str) -> Vec<Uuid>`
- [ ] Implement `get_tags(&self, session_id: Uuid) -> Option<&HashSet<String>>`

### 3.4 Update route() Method
- [ ] Handle `OrchestrationTarget::Tagged(tag)` - find sessions with matching tag
- [ ] Remove `OrchestrationTarget::Orchestrator` handling
- [ ] Ensure Tagged routing excludes sender (like Broadcast)

### 3.5 Update unregister() Method
- [ ] Remove session from `session_tags`
- [ ] Remove old orchestrator cleanup logic

### 3.6 Remove Deprecated Methods
- [ ] Delete `get_orchestrator(&self, repo_id: &str) -> Option<Uuid>`

### 3.7 Update Router Tests
- [ ] Update `test_router_register_unregister` for tags
- [ ] Delete `test_router_orchestrator_registration`
- [ ] Delete `test_router_send_to_orchestrator`
- [ ] Delete `test_router_no_orchestrator`
- [ ] Delete `test_router_unregister_orchestrator`
- [ ] Add `test_router_register_with_tags`
- [ ] Add `test_router_add_remove_tag`
- [ ] Add `test_router_sessions_with_tag`
- [ ] Add `test_router_send_to_tagged`
- [ ] Add `test_router_send_to_tagged_multiple_recipients`
- [ ] Add `test_router_send_to_tagged_no_match`
- [ ] Update `test_router_send_to_specific_session` (should still work)
- [ ] Update `test_router_broadcast` for tags
- [ ] Update `test_router_broadcast_different_repos` for tags
- [ ] Update `test_router_worktree_routing` for tags
- [ ] Update `test_router_sessions_in_repo` for tags

## Phase 4: Integration and Cleanup

### 4.1 Verify All Tests Pass
- [ ] Run `cargo test -p fugue-protocol`
- [ ] Run `cargo test -p fugue-server`
- [ ] Fix any compilation errors in dependent crates

### 4.2 Update Dependent Code (if any)
- [ ] Search for uses of `is_orchestrator` in codebase
- [ ] Search for uses of `WorkerStatus` in codebase
- [ ] Search for uses of `OrchestrationTarget::Orchestrator` in codebase
- [ ] Update any found usages

### 4.3 Documentation
- [ ] Update doc comments on OrchestrationMessage
- [ ] Update doc comments on OrchestrationTarget
- [ ] Update doc comments on SessionInfo
- [ ] Update doc comments on MessageRouter methods
- [ ] Add usage examples in doc comments

## Phase 5: Final Review

- [ ] Self-review all changes
- [ ] Verify no methodology-specific types remain
- [ ] Verify tag-based routing works for single and multiple recipients
- [ ] Verify generic payload accepts arbitrary JSON
- [ ] Update feature_request.json status to "in_progress" or "completed"

## Completion Checklist

- [ ] All Phase 1 tasks complete (Protocol Types)
- [ ] All Phase 2 tasks complete (SessionInfo)
- [ ] All Phase 3 tasks complete (Router)
- [ ] All Phase 4 tasks complete (Integration)
- [ ] All tests passing
- [ ] PLAN.md updated with final approach
- [ ] Ready for review/merge

---
*Check off tasks as you complete them. Update status field above.*
