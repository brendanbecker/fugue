# Task Breakdown: FEAT-048

**Work Item**: [FEAT-048: Expose orchestration protocol via MCP tools](PROMPT.md)
**Status**: Not Started
**Last Updated**: 2026-01-10

## Prerequisites

- [ ] Read and understand PROMPT.md
- [ ] Review PLAN.md and update if needed
- [ ] Review existing orchestration types in fugue-protocol/src/messages.rs
- [ ] Review existing MCP tools in fugue-server/src/mcp/tools.rs

## Phase 1: Core Infrastructure

- [ ] Add FUGUE_SESSION_ID environment variable to PTY spawn in fugue-server/src/pty/spawn.rs
- [ ] Create OrchestrationTarget JSON schema
- [ ] Create OrchestrationMessage JSON schema
- [ ] Implement `fugue_send_orchestration` tool handler
- [ ] Register tool in MCP tool listing
- [ ] Add unit tests for schema validation
- [ ] Add integration test for basic message send

## Phase 2: Convenience Tools

- [ ] Implement session context resolution from FUGUE_SESSION_ID
- [ ] Implement `fugue_report_status` tool
  - [ ] Tool schema
  - [ ] Handler with auto-fill session_id
  - [ ] Unit tests
- [ ] Implement `fugue_request_help` tool
  - [ ] Tool schema
  - [ ] Handler with auto-target to Orchestrator
  - [ ] Unit tests
- [ ] Implement `fugue_broadcast` tool
  - [ ] Tool schema
  - [ ] Handler with auto-fill from_session_id
  - [ ] Unit tests

## Phase 3: Message Subscription

- [ ] Design message queue structure for received orchestration messages
- [ ] Implement `fugue_get_orchestration_messages` polling tool
- [ ] Add queue size limits and cleanup
- [ ] Integration test for message receive flow

## Testing Tasks

- [ ] Test all OrchestrationMessage variants via fugue_send_orchestration
- [ ] Test all OrchestrationTarget variants
- [ ] Test error cases: NoRepository, NoRecipients
- [ ] Test convenience tools with auto-fill
- [ ] Run full test suite

## Documentation Tasks

- [ ] Document tool schemas in fugue-server/src/mcp/README.md (or equivalent)
- [ ] Add usage examples for common orchestration patterns
- [ ] Update project CLAUDE.md with orchestration guidance

## Verification Tasks

- [ ] All acceptance criteria from PROMPT.md met
- [ ] Tests passing
- [ ] Update feature_request.json status
- [ ] Document completion in comments.md

## Completion Checklist

- [ ] All implementation tasks complete
- [ ] All tests passing
- [ ] Documentation updated
- [ ] PLAN.md reflects final implementation
- [ ] Ready for review/merge

---
*Check off tasks as you complete them. Update status field above.*
