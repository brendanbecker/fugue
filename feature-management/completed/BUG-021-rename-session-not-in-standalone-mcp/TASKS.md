# Task Breakdown: BUG-021

**Work Item**: [BUG-021: fugue_rename_session Not Handled in Standalone MCP Server](PROMPT.md)
**Status**: Not Started
**Last Updated**: 2026-01-10

## Prerequisites

- [ ] Read and understand PROMPT.md
- [ ] Review PLAN.md and update if needed
- [ ] Review bridge.rs implementation for reference (line 372-380)
- [ ] Review existing ToolContext methods in handlers.rs

## Investigation Tasks

- [ ] Confirm tool definition in tools.rs (line 225)
- [ ] Confirm bridge.rs handling (line 372)
- [ ] Confirm server.rs is missing handling
- [ ] Check if ToolContext has session rename capability

## Implementation Tasks

### server.rs Changes

- [ ] Add `"fugue_rename_session"` to `is_known_tool()` match (around line 290)
- [ ] Add `RenameSession { session: String, name: String }` variant to `ToolParams` enum (around line 315)
- [ ] Add parsing case in `dispatch_tool()` match (around line 231)
  - Parse `session` as required string
  - Parse `name` as required string
  - Return `McpError::InvalidParams` for missing parameters
- [ ] Add execution case in result match (around line 260+)
  - Call `ctx.rename_session(&session, &name)`

### handlers.rs Changes

- [ ] Add `rename_session(&mut self, session: &str, name: &str) -> ToolResult` method to `ToolContext`
  - Resolve session by UUID or name
  - Update session name
  - Return success JSON with `renamed`, `session_id`, `new_name`
  - Return error if session not found

## Testing Tasks

- [ ] Test `fugue_rename_session` in standalone MCP server mode
- [ ] Verify same behavior as bridge mode
- [ ] Test error cases (missing session, missing name, invalid session)
- [ ] Run existing MCP test suite to check for regressions
- [ ] Run `cargo test` for full test suite

## Verification Tasks

- [ ] Confirm tool recognized in standalone mode (no "unknown tool" error)
- [ ] Confirm session rename works end-to-end
- [ ] Verify all acceptance criteria from PROMPT.md
- [ ] Update bug_report.json status to "fixed"

## Completion Checklist

- [ ] All implementation tasks complete
- [ ] All tests passing
- [ ] PLAN.md updated with final approach
- [ ] Ready for review/merge

---
*Check off tasks as you complete them. Update status field above.*
