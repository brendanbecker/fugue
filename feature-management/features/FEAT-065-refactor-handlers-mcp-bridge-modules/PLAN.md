# Implementation Plan: FEAT-065

**Work Item**: [FEAT-065: Refactor handlers/mcp_bridge.rs into smaller modules](PROMPT.md)
**Component**: fugue-server (handlers module)
**Priority**: P2
**Created**: 2026-01-13

## Overview

The MCP bridge handler implementation (`fugue-server/src/handlers/mcp_bridge.rs`) has grown to 2591 lines (~27k tokens) and exceeds the practical context window limit for AI-assisted development. This is the handler layer that dispatches MCP tool calls to session/pane operations. It needs to be refactored into smaller, more maintainable modules.

## Architecture Decisions

### Module Organization

Create a `handlers/mcp_bridge/` subdirectory with modules organized by tool category:

| Module | Responsibility | Expected Size |
|--------|----------------|---------------|
| `mod.rs` | Module exports, shared types | <100 lines |
| `session_tools.rs` | Session CRUD operations | <500 lines |
| `pane_tools.rs` | Pane operations (read, write, focus) | <500 lines |
| `window_tools.rs` | Window management | <300 lines |
| `layout_tools.rs` | Split and layout operations | <300 lines |
| `orchestration.rs` | Orchestration protocol tools | <300 lines |
| `beads_tools.rs` | Beads integration tools | <300 lines |
| `common.rs` | Shared validation, parsing, response helpers | <200 lines |

### Dispatcher Pattern

The main `handlers/mcp_bridge.rs` will be reduced to:
- Tool name routing (match statement)
- Delegation to appropriate tool module
- No direct business logic

### Backward Compatibility

- No changes to public API
- All existing tool names and parameters preserved
- Response formats unchanged

## Affected Components

| Component | Type of Change | Risk Level |
|-----------|----------------|------------|
| `handlers/mcp_bridge.rs` | Refactor - slim down to dispatcher | Medium |
| `handlers/mcp_bridge/` | New - extracted tool modules | Low |
| MCP tool functionality | None - behavior preserved | Low |

## Dependencies

None - this is a pure refactoring task.

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Regression in tool behavior | Low | High | Comprehensive testing of all tools |
| Breaking API changes | Low | High | Careful re-export of public types |
| Module boundaries unclear | Medium | Low | Analysis phase to identify clean boundaries |
| Shared state complexity | Medium | Medium | Document state ownership clearly |

## Rollback Strategy

If implementation causes issues:
1. Revert commits associated with this work item
2. Verify system returns to previous state
3. Document what went wrong in comments.md

## Implementation Notes

### Analysis Phase Guidance

Before extracting code, analyze:
1. Which tool names exist in the current dispatcher
2. How tools are grouped by functionality
3. What state is shared between tool handlers
4. What validation/utility code is duplicated

### Extraction Order

Recommended order for extraction:
1. Identify and extract shared utilities first (common.rs)
2. Extract largest/clearest category (likely pane_tools or session_tools)
3. Extract remaining categories one at a time
4. Slim down dispatcher last

### Coordination with FEAT-064

FEAT-064 is refactoring `mcp/bridge.rs` (connection/protocol layer). Ensure:
- Consistent naming patterns between the two refactors
- Similar module organization style
- No conflicting changes to shared types

---
*This plan should be updated as implementation progresses.*
