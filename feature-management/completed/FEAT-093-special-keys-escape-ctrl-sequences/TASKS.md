# Task Breakdown: FEAT-093

**Work Item**: [FEAT-093: Add support for sending special keys](PROMPT.md)
**Status**: Not Started
**Last Updated**: 2026-01-16

## Prerequisites

- [ ] Read and understand PROMPT.md
- [ ] Review PLAN.md and update if needed
- [ ] Review existing send_input implementation in handlers/mcp.rs
- [ ] Review crossterm key handling for reference

## Design Tasks

- [ ] Finalize decision: new tool vs extend existing
- [ ] Define complete key name vocabulary
- [ ] Document key-to-escape-sequence mapping
- [ ] Update PLAN.md with final approach

## Protocol Tasks

- [ ] Add `PressKey` message to fugue-protocol/src/types.rs
- [ ] Define key parameters (key name, count, modifiers)
- [ ] Add response type if different from SendInput

## Implementation Tasks

- [ ] Create fugue-server/src/keys.rs module
- [ ] Implement key name parsing (case-insensitive)
- [ ] Implement modifier parsing (Ctrl+, Alt+, Shift+)
- [ ] Implement escape sequence generation for:
  - [ ] Control characters (Ctrl+A through Ctrl+Z)
  - [ ] Escape key
  - [ ] Arrow keys (Up, Down, Left, Right)
  - [ ] Navigation keys (Home, End, PageUp, PageDown)
  - [ ] Editing keys (Backspace, Delete, Insert)
  - [ ] Function keys (F1-F12)
- [ ] Add `fugue_press_key` tool handler in handlers/mcp.rs
- [ ] Register tool in MCP tool list
- [ ] Self-review changes

## Testing Tasks

- [ ] Unit tests for key name parsing
- [ ] Unit tests for modifier parsing
- [ ] Unit tests for escape sequence generation
- [ ] Integration test: send Escape to exit vim insert mode
- [ ] Integration test: send Ctrl+C to interrupt process
- [ ] Integration test: send arrow keys to navigate
- [ ] Run full test suite

## Documentation Tasks

- [ ] Add docstrings to keys.rs functions
- [ ] Document tool in MCP tool descriptions
- [ ] List supported key names in tool description
- [ ] Add examples to tool description
- [ ] Update CHANGELOG

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
