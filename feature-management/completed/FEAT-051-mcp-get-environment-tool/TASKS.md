# Task Breakdown: FEAT-051

**Work Item**: [FEAT-051: Add fugue_get_environment MCP tool](PROMPT.md)
**Status**: Not Started
**Last Updated**: 2026-01-10

## Prerequisites

- [ ] Read and understand PROMPT.md
- [ ] Review PLAN.md and update if needed
- [ ] Review existing fugue_set_environment implementation as reference
- [ ] Identify affected files in fugue-protocol and fugue-server

## Design Tasks

- [ ] Review ClientMessage and ServerMessage enum patterns
- [ ] Confirm session environment storage location
- [ ] Design error response format
- [ ] Update PLAN.md with any design changes

## Implementation Tasks

### Protocol (fugue-protocol)

- [ ] Add `GetEnvironment` variant to `ClientMessage`
- [ ] Add `Environment` variant to `ServerMessage`
- [ ] Ensure serde derives are correct
- [ ] Run `cargo check` in fugue-protocol

### Server Handler (fugue-server)

- [ ] Add handler function for `GetEnvironment` message
- [ ] Implement session lookup by ID or name
- [ ] Implement single-key retrieval
- [ ] Implement all-keys retrieval
- [ ] Wire handler into message dispatch

### MCP Tool (fugue-server)

- [ ] Add tool definition to MCP tool list
- [ ] Define input schema (session required, key optional)
- [ ] Implement tool handler that calls protocol handler
- [ ] Format response appropriately for MCP

## Testing Tasks

- [ ] Add unit test for GetEnvironment with specific key
- [ ] Add unit test for GetEnvironment with no key (all)
- [ ] Add unit test for GetEnvironment with invalid session
- [ ] Add unit test for GetEnvironment with non-existent key
- [ ] Add integration test for MCP tool
- [ ] Run full test suite

## Documentation Tasks

- [ ] Update MCP tool documentation if it exists
- [ ] Add code comments where needed

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
