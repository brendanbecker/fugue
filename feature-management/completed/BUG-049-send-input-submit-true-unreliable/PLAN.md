# Implementation Plan: BUG-049

**Work Item**: [BUG-049: send_input with submit: true doesn't reliably submit input](PROMPT.md)
**Component**: mcp
**Priority**: P2
**Created**: 2026-01-16

## Overview

When using fugue_send_input with submit: true, the text appears in the target pane's input area but doesn't always get submitted. Investigation and fix needed to ensure reliable input submission.

## Architecture Decisions

- **Approach**: Investigate the send_input flow to understand how text and Enter key are sequenced
- **Trade-offs**: May need to combine text + newline into single write vs separate writes with sync

## Affected Components

| Component | Type of Change | Risk Level |
|-----------|----------------|------------|
| mcp | Primary | Low |
| pty | Secondary | Low |

## Key Files to Investigate

1. `fugue-server/src/handlers/mcp.rs` - MCP handler for send_input
2. `fugue-mcp-bridge/src/handlers.rs` - Bridge-side send_input handling
3. PTY write logic in session/pane management

## Dependencies

None

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Regression in send_input | Low | Medium | Test both with and without submit flag |
| Performance impact | Low | Low | Single write should be faster than two |

## Rollback Strategy

If implementation causes issues:
1. Revert commits associated with this work item
2. Verify system returns to previous state
3. Document what went wrong in comments.md

## Implementation Notes

<!-- Add notes during implementation -->

---
*This plan should be updated as implementation progresses.*
