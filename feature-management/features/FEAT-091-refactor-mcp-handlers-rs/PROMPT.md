# FEAT-091: Refactor fugue-server/src/mcp/handlers.rs

**Priority**: P3
**Component**: fugue-server
**Type**: refactor
**Estimated Effort**: medium
**Current Size**: 17.1k tokens (1836 lines)
**Target Size**: <10k tokens per module

## Overview

The standalone MCP handlers file (not the bridge handlers) has grown to 17.1k tokens. This contains handlers for the integrated MCP server mode. Similar to FEAT-088 but for standalone mode.

## Current Structure Analysis

This file handles MCP tools when running in standalone mode (not through Claude Code's bridge). It likely mirrors the bridge handlers but with different execution context.

## Proposed Module Structure

```
fugue-server/src/mcp/handlers/
├── mod.rs              # Handler dispatch, shared utilities (<3k)
├── session.rs          # Session handlers
├── window.rs           # Window handlers
├── pane.rs             # Pane handlers
├── layout.rs           # Layout handlers
└── io.rs               # Input/output handlers
```

## Refactoring Steps

1. **Compare with bridge handlers** - May be able to share code
2. **Extract by domain** - Same grouping as FEAT-088
3. **Consider shared handler logic** - DRY between bridge and standalone
4. **Update MCP tool dispatch**

## Acceptance Criteria

- [ ] `handlers.rs` replaced with `handlers/` directory
- [ ] Each module <10k tokens
- [ ] Standalone MCP mode works
- [ ] All MCP tests pass

## Testing

- Standalone MCP server tests
- Tool invocation tests
- Compare behavior with bridge mode

## Notes

- Consider whether bridge and standalone handlers can share implementation
- This refactor may reveal duplication that can be eliminated
