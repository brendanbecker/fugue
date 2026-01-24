# FEAT-065: Refactor handlers/mcp_bridge.rs into smaller modules

**Priority**: P2
**Component**: fugue-server (handlers module)
**Type**: improvement
**Estimated Effort**: medium
**Business Value**: high

## Overview

The MCP bridge handler implementation (`fugue-server/src/handlers/mcp_bridge.rs`) has grown to 2591 lines (~27k tokens) and exceeds the practical context window limit for AI-assisted development. It needs to be refactored into smaller, more maintainable modules.

## Problem Statement

- `handlers/mcp_bridge.rs` is 2591 lines, estimated ~27k tokens
- Exceeds the 25k token read limit, making it difficult to review in full
- This is the handler layer that dispatches MCP tool calls to session/pane operations
- Mixed concerns likely include: tool parameter parsing, validation, session lookups, pane operations, response formatting

## Context

- This is separate from `mcp/bridge.rs` (which handles connection/protocol) - that file already has FEAT-064 for refactoring
- This file is in `handlers/` and deals with the business logic of MCP tool execution
- Should follow similar modular patterns being established in FEAT-064

## Proposed Module Structure

```
fugue-server/src/handlers/
├── mcp_bridge.rs          # Main dispatcher, slimmed down to routing only
├── mcp_bridge/
│   ├── mod.rs             # Module exports
│   ├── session_tools.rs   # Session-related tool handlers (list, create, attach, etc.)
│   ├── pane_tools.rs      # Pane-related tool handlers (create, read, send_input, etc.)
│   ├── window_tools.rs    # Window-related tool handlers
│   ├── layout_tools.rs    # Layout and split operations
│   ├── orchestration.rs   # Orchestration message tools (if applicable)
│   └── beads_tools.rs     # Beads integration tools (if applicable)
```

## Key Refactoring Goals

1. Each module should be <500 lines
2. Clear separation of tool categories
3. Shared validation/utility functions extracted
4. Maintain backward compatibility - no API changes
5. Consistent with patterns from FEAT-064

## Benefits

- Improved code maintainability and testability
- Easier onboarding for new contributors
- Better AI assistant support for code modifications
- Clearer separation of concerns
- Smaller, focused modules that are easier to reason about
- More targeted unit testing per module

## Implementation Tasks

### Section 1: Analysis and Planning
- [ ] Audit current `handlers/mcp_bridge.rs` to identify logical boundaries
- [ ] Identify all tool handlers and categorize by type (session, pane, window, layout, etc.)
- [ ] Map dependencies between code sections
- [ ] Identify shared validation/utility functions
- [ ] Document the extraction plan with specific line ranges

### Section 2: Create Module Structure
- [ ] Create `handlers/mcp_bridge/` directory
- [ ] Create `handlers/mcp_bridge/mod.rs` with module declarations
- [ ] Define shared types and traits for tool handlers
- [ ] Set up re-exports for backward compatibility

### Section 3: Extract Session Tools
- [ ] Create `handlers/mcp_bridge/session_tools.rs`
- [ ] Extract session list, create, attach, detach handlers
- [ ] Extract session rename, delete, kill handlers
- [ ] Move session-related validation logic
- [ ] Update main dispatcher to use new module

### Section 4: Extract Pane Tools
- [ ] Create `handlers/mcp_bridge/pane_tools.rs`
- [ ] Extract pane create, read, send_input handlers
- [ ] Extract pane focus, select handlers
- [ ] Move pane-related validation logic
- [ ] Update main dispatcher to use new module

### Section 5: Extract Window and Layout Tools
- [ ] Create `handlers/mcp_bridge/window_tools.rs`
- [ ] Create `handlers/mcp_bridge/layout_tools.rs`
- [ ] Extract window management handlers
- [ ] Extract split and layout operation handlers
- [ ] Update main dispatcher to use new module

### Section 6: Extract Specialty Tools
- [ ] Create `handlers/mcp_bridge/orchestration.rs` (if applicable)
- [ ] Create `handlers/mcp_bridge/beads_tools.rs` (if applicable)
- [ ] Extract orchestration message handlers
- [ ] Extract beads integration handlers
- [ ] Update main dispatcher to use new module

### Section 7: Extract Shared Utilities
- [ ] Identify common validation patterns
- [ ] Create shared utility functions for parameter parsing
- [ ] Create shared response formatting helpers
- [ ] Ensure DRY principles across modules

### Section 8: Slim Down Main Dispatcher
- [ ] Reduce `mcp_bridge.rs` to routing logic only
- [ ] Ensure clean delegation to tool modules
- [ ] Maintain public API compatibility
- [ ] Target <300 lines for main dispatcher

### Section 9: Testing and Verification
- [ ] Ensure all existing tests pass
- [ ] Add unit tests for extracted modules
- [ ] Verify no API changes for external callers
- [ ] Test MCP tool functionality end-to-end
- [ ] Verify all tool handlers work correctly

### Section 10: Documentation
- [ ] Update module-level documentation
- [ ] Add doc comments to new public interfaces
- [ ] Document the module structure in mod.rs
- [ ] Update any relevant architecture docs

## Acceptance Criteria

- [ ] `handlers/mcp_bridge.rs` is under 300 lines (routing only)
- [ ] All extracted modules are under 500 lines each
- [ ] All existing tests pass without modification
- [ ] No changes to public API
- [ ] MCP tool functionality works identically to before
- [ ] Code compiles without warnings
- [ ] New modules have appropriate doc comments
- [ ] Clear separation of tool categories

## Dependencies

None - this is a pure refactoring task with no external dependencies.

## Related

- **FEAT-064**: Refactor mcp/bridge.rs into modular components (connection/protocol layer)

## Notes

- This is a pure refactoring task - behavior should remain identical
- Consider using Rust's `#[cfg(test)]` module pattern for per-module tests
- Coordinate with FEAT-064 to ensure consistent patterns across MCP-related code
- The module structure may need adjustment based on actual code analysis
- If certain tool categories are very small, they can be combined
