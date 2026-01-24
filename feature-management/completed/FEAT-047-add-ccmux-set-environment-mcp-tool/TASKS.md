# Task Breakdown: FEAT-047

**Work Item**: [FEAT-047: Add fugue_set_environment MCP tool](PROMPT.md)
**Status**: Not Started
**Last Updated**: 2026-01-10

## Prerequisites

- [ ] Read and understand PROMPT.md
- [ ] Review PLAN.md and update if needed
- [ ] Review existing Session struct and MCP tool patterns

## Design Tasks

- [ ] Review requirements and acceptance criteria
- [ ] Design solution architecture
- [ ] Identify affected components
- [ ] Update PLAN.md with approach
- [ ] Consider edge cases (duplicate session names, invalid keys)

## Implementation Tasks

- [ ] Add `environment: HashMap<String, String>` field to Session struct
- [ ] Initialize environment as empty HashMap in Session::new()
- [ ] Add `SetEnvironment { session_id, key, value }` variant to ClientMessage
- [ ] Add corresponding response type if needed
- [ ] Add `fugue_set_environment` MCP tool definition to tools.rs
- [ ] Implement MCP tool handler in handlers.rs
- [ ] Add session lookup by UUID or name helper if not exists
- [ ] Implement environment variable storage on session
- [ ] Modify PTY spawn to merge session environment
- [ ] Add error handling for invalid session references
- [ ] Self-review changes

## Testing Tasks

- [ ] Add unit tests for Session environment field
- [ ] Add unit tests for SetEnvironment message serialization
- [ ] Add unit tests for MCP tool schema validation
- [ ] Add integration tests for MCP tool invocation
- [ ] Test environment propagation to spawned panes
- [ ] Test with actual Gas Town environment variables
- [ ] Run full test suite

## Documentation Tasks

- [ ] Update MCP tool documentation in README or docs/
- [ ] Add usage examples for Gas Town integration
- [ ] Add code comments where needed
- [ ] Update CHANGELOG if applicable

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
