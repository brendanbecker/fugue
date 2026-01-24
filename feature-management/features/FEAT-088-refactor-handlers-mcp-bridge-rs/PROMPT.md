# FEAT-088: Refactor fugue-server/src/handlers/mcp_bridge.rs

**Priority**: P2
**Component**: fugue-server
**Type**: refactor
**Estimated Effort**: medium
**Current Size**: 27.2k tokens (2880 lines)
**Target Size**: <10k tokens per module

## Overview

The MCP bridge handlers file has grown to 27.2k tokens. This file handles MCP tool requests routed through the bridge. It should be split into logical handler groups.

## Current Structure Analysis

The file likely contains handlers for:
- Session management (create, list, rename, kill)
- Window management (create, select, list)
- Pane management (create, close, split, resize)
- Layout operations (create, apply)
- Focus/selection operations
- Metadata operations (get/set)
- Environment operations
- Output reading (read_pane)
- Input sending (send_input)
- Widget operations (new generic widget system)

## Proposed Module Structure

```
fugue-server/src/handlers/mcp_bridge/
├── mod.rs              # Re-exports, handler dispatch (<3k)
├── session.rs          # Session CRUD handlers
├── window.rs           # Window CRUD handlers
├── pane.rs             # Pane CRUD, split, resize handlers
├── layout.rs           # Layout create/apply handlers
├── focus.rs            # Focus, select handlers
├── metadata.rs         # Metadata get/set handlers
├── environment.rs      # Environment get/set handlers
├── io.rs               # read_pane, send_input handlers
└── widgets.rs          # Widget get/set handlers
```

## Refactoring Steps

1. **Audit current handlers** - List all handler functions
2. **Group by domain** - Session, window, pane, etc.
3. **Extract one group at a time** - Start with smallest (environment?)
4. **Update dispatch** - Ensure handler routing still works
5. **Run tests after each extraction**

## Acceptance Criteria

- [ ] `mcp_bridge.rs` replaced with `mcp_bridge/` module directory
- [ ] Each module <10k tokens
- [ ] All MCP tool tests pass
- [ ] Handler dispatch unchanged
- [ ] No functionality changes

## Testing

- Run MCP bridge tests
- Test each tool category manually
- Verify error handling preserved

## Notes

- Keep handler signatures unchanged
- This is a pure refactor
- The `mcp/bridge/` directory is different - that's the bridge connection itself
