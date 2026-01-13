# Task Breakdown: FEAT-065

**Work Item**: [FEAT-065: Refactor handlers/mcp_bridge.rs into smaller modules](PROMPT.md)
**Status**: Not Started
**Last Updated**: 2026-01-13

## Prerequisites

- [ ] Read and understand PROMPT.md
- [ ] Review PLAN.md and update if needed
- [ ] Review FEAT-064 for consistent patterns
- [ ] Verify no in-progress changes to handlers/mcp_bridge.rs

## Analysis Tasks

- [ ] Read handlers/mcp_bridge.rs and identify all tool handlers
- [ ] Categorize tools by type (session, pane, window, layout, orchestration, beads)
- [ ] Identify shared validation/utility functions
- [ ] Map dependencies between code sections
- [ ] Document extraction plan with line ranges
- [ ] Update PLAN.md with findings

## Module Structure Tasks

- [ ] Create handlers/mcp_bridge/ directory
- [ ] Create handlers/mcp_bridge/mod.rs with module declarations
- [ ] Define shared types and traits (if any)
- [ ] Create handlers/mcp_bridge/common.rs for shared utilities

## Session Tools Extraction

- [ ] Create handlers/mcp_bridge/session_tools.rs
- [ ] Extract session list handler
- [ ] Extract session create handler
- [ ] Extract session attach/detach handlers
- [ ] Extract session rename handler
- [ ] Extract session delete/kill handlers
- [ ] Move session-related validation logic
- [ ] Add appropriate doc comments
- [ ] Update mod.rs exports

## Pane Tools Extraction

- [ ] Create handlers/mcp_bridge/pane_tools.rs
- [ ] Extract pane create handler
- [ ] Extract pane read handler
- [ ] Extract pane send_input handler
- [ ] Extract pane focus/select handlers
- [ ] Move pane-related validation logic
- [ ] Add appropriate doc comments
- [ ] Update mod.rs exports

## Window Tools Extraction

- [ ] Create handlers/mcp_bridge/window_tools.rs
- [ ] Extract window management handlers
- [ ] Move window-related validation logic
- [ ] Add appropriate doc comments
- [ ] Update mod.rs exports

## Layout Tools Extraction

- [ ] Create handlers/mcp_bridge/layout_tools.rs
- [ ] Extract split operation handlers
- [ ] Extract layout management handlers
- [ ] Move layout-related validation logic
- [ ] Add appropriate doc comments
- [ ] Update mod.rs exports

## Specialty Tools Extraction

- [ ] Assess if orchestration tools exist
- [ ] Create handlers/mcp_bridge/orchestration.rs if needed
- [ ] Assess if beads tools exist
- [ ] Create handlers/mcp_bridge/beads_tools.rs if needed
- [ ] Extract applicable handlers
- [ ] Add appropriate doc comments
- [ ] Update mod.rs exports

## Dispatcher Slimdown

- [ ] Reduce mcp_bridge.rs to routing logic only
- [ ] Implement clean delegation to tool modules
- [ ] Remove extracted code from main file
- [ ] Verify public API unchanged
- [ ] Target <300 lines for dispatcher

## Testing Tasks

- [ ] Verify all existing tests pass
- [ ] Test session tools manually
- [ ] Test pane tools manually
- [ ] Test window/layout tools manually
- [ ] Test orchestration/beads tools if applicable
- [ ] Run full MCP integration test
- [ ] Add unit tests for new modules if applicable

## Documentation Tasks

- [ ] Add module-level doc comments
- [ ] Document public interfaces
- [ ] Update mod.rs with structure overview
- [ ] Update PLAN.md with final implementation notes

## Verification Tasks

- [ ] Confirm mcp_bridge.rs < 300 lines
- [ ] Confirm all modules < 500 lines
- [ ] Confirm no API changes
- [ ] Confirm all tests pass
- [ ] Confirm code compiles without warnings
- [ ] Update feature_request.json status

## Completion Checklist

- [ ] All extraction tasks complete
- [ ] All tests passing
- [ ] Documentation updated
- [ ] PLAN.md reflects final implementation
- [ ] Ready for review/merge

---
*Check off tasks as you complete them. Update status field above.*
