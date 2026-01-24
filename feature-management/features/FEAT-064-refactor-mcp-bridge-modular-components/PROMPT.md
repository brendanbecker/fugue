# FEAT-064: Refactor MCP bridge.rs into modular components

**Priority**: P2
**Component**: fugue-server (mcp module)
**Type**: improvement
**Estimated Effort**: large
**Business Value**: high

## Overview

The MCP bridge implementation (`fugue-server/src/mcp/bridge.rs`) has grown to over 33,000 tokens and needs to be refactored into smaller, more maintainable modules.

## Problem Statement

- `bridge.rs` is too large to read in a single context window (>33k tokens, >25k limit)
- Mixed concerns: connection management, protocol handling, tool dispatching, health monitoring, reconnection logic
- Difficult to navigate, test, and maintain
- Hard for AI assistants and developers to understand the full picture

## Proposed Module Structure

```
fugue-server/src/mcp/
├── mod.rs              # Module exports
├── bridge.rs           # Main McpBridge struct, high-level orchestration (slimmed down)
├── connection.rs       # Connection state, connect_to_daemon(), reconnection logic
├── health.rs           # Health monitoring, heartbeat, ConnectionState enum
├── protocol.rs         # JSON-RPC request/response handling (already exists, may need expansion)
├── handlers.rs         # Tool dispatch and execution (already exists)
├── tools/              # Individual tool implementations (if handlers.rs is also large)
│   ├── mod.rs
│   ├── session.rs
│   ├── pane.rs
│   └── ...
└── error.rs            # MCP-specific errors (already exists)
```

## Key Refactoring Goals

1. Extract connection management (`connect_with_retry`, reconnection, state tracking) into `connection.rs`
2. Extract health monitoring (heartbeat, `health_monitor` task, `ConnectionState`) into `health.rs`
3. Keep `bridge.rs` focused on high-level orchestration and the main `run()` loop
4. Ensure each module is <500 lines for readability
5. Maintain backward compatibility - no API changes

## Benefits

- Improved code maintainability and testability
- Easier onboarding for new contributors
- Better AI assistant support for code modifications
- Clearer separation of concerns
- Smaller, focused modules that are easier to reason about
- More targeted unit testing per module

## Implementation Tasks

### Section 1: Analysis and Planning
- [ ] Audit current `bridge.rs` to identify logical boundaries
- [ ] Map dependencies between code sections
- [ ] Identify shared state that needs careful handling
- [ ] Document the extraction plan with specific line ranges

### Section 2: Connection Module Extraction
- [ ] Create `connection.rs` with connection state types
- [ ] Extract `connect_to_daemon()` and related functions
- [ ] Extract reconnection logic and retry mechanisms
- [ ] Move connection state tracking code
- [ ] Update `bridge.rs` to use the new module

### Section 3: Health Module Extraction
- [ ] Create `health.rs` with `ConnectionState` enum
- [ ] Extract heartbeat logic
- [ ] Extract `health_monitor` task implementation
- [ ] Move health-related utilities
- [ ] Update `bridge.rs` to use the new module

### Section 4: Tool Handlers (if needed)
- [ ] Assess if `handlers.rs` needs further decomposition
- [ ] If >500 lines, create `tools/` subdirectory
- [ ] Extract session-related tool handlers to `tools/session.rs`
- [ ] Extract pane-related tool handlers to `tools/pane.rs`
- [ ] Extract other tool categories as appropriate

### Section 5: Testing and Verification
- [ ] Ensure all existing tests pass
- [ ] Add unit tests for extracted modules
- [ ] Verify no API changes for external callers
- [ ] Test MCP functionality end-to-end
- [ ] Verify reconnection behavior works correctly

### Section 6: Documentation
- [ ] Update module-level documentation
- [ ] Add doc comments to new public interfaces
- [ ] Update ARCHITECTURE.md if it exists
- [ ] Document the module structure in mod.rs

## Acceptance Criteria

- [ ] `bridge.rs` is under 500 lines (or close to it)
- [ ] All extracted modules are under 500 lines each
- [ ] All existing tests pass without modification
- [ ] No changes to public API
- [ ] MCP functionality works identically to before
- [ ] Code compiles without warnings
- [ ] New modules have appropriate doc comments

## Dependencies

None - this is a pure refactoring task with no external dependencies.

## Notes

- This is a pure refactoring task - behavior should remain identical
- Consider using Rust's `#[cfg(test)]` module pattern for per-module tests
- The `protocol.rs` and `handlers.rs` files already exist; assess whether they need expansion or modification
- If `handlers.rs` is also large, consider the `tools/` subdirectory structure
