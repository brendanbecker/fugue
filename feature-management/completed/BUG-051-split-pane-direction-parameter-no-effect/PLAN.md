# Implementation Plan: BUG-051

**Work Item**: [BUG-051: Split pane direction parameter has no effect](PROMPT.md)
**Component**: mcp-handlers
**Priority**: P1
**Created**: 2026-01-17

## Overview

The split pane direction parameter is being ignored, resulting in all splits producing horizontally stacked panes regardless of the specified direction.

## Architecture Decisions

- **Approach**: Trace the direction parameter through the call chain from MCP handler to layout engine
- **Trade-offs**: Need to ensure fix doesn't break existing pane layouts

## Investigation Path

1. **MCP Handler** (`crates/fugue-mcp-bridge/src/handlers/`)
   - Check `split_pane` handler for direction parameter handling
   - Verify direction is passed to the daemon

2. **Protocol Messages** (`crates/fugue-protocol/`)
   - Check `SplitPane` request struct includes direction
   - Verify direction enum is properly defined

3. **Daemon Handler** (`crates/fugue-daemon/`)
   - Check how split requests are processed
   - Verify direction is used when creating the split

4. **Layout Engine** (`crates/fugue-tui/` or similar)
   - Check how panes are positioned
   - Verify direction affects the layout calculation

## Affected Components

| Component | Type of Change | Risk Level |
|-----------|----------------|------------|
| mcp-handlers | Investigation/Fix | Medium |
| protocol | May need update | Low |
| daemon | May need update | Medium |
| layout engine | May need update | Medium |

## Dependencies

None - this is a bug fix for existing functionality.

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Fix breaks existing layouts | Medium | High | Careful testing of both directions |
| Direction semantics unclear | Medium | Low | Document the convention used |
| Regression in pane creation | Low | Medium | Test pane creation flows |

## Rollback Strategy

If implementation causes issues:
1. Revert commits associated with this work item
2. Verify system returns to previous state
3. Document what went wrong in comments.md

## Implementation Notes

<!-- Add notes during implementation -->

---
*This plan should be updated as implementation progresses.*
