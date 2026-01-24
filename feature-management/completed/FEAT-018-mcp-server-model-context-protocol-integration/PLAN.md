# Implementation Plan: FEAT-018

**Work Item**: [FEAT-018: MCP Server - Model Context Protocol Integration](PROMPT.md)
**Component**: fugue-server
**Priority**: P2
**Created**: 2026-01-08

## Overview

Model Context Protocol server exposing tools for Claude to interact with fugue (list panes, send input, create panes).

## Architecture Decisions

### MCP Server Architecture

- **Approach**: Implement MCP server as a separate module within fugue-server
- **Transport**: JSON-RPC 2.0 over stdio (standard MCP transport)
- **Crate**: Use rmcp crate for MCP protocol handling
- **Integration**: Server will interact with SessionManager via message passing or shared state

### Tool Design

Each tool will follow MCP specification with:
- Name: Descriptive snake_case identifier
- Description: Human-readable explanation
- Input Schema: JSON Schema defining parameters
- Handler: Async function processing the tool call

### Trade-offs

| Decision | Pros | Cons |
|----------|------|------|
| stdio transport | Simple, standard MCP approach | Single client at a time |
| rmcp crate | Reduces implementation effort | External dependency |
| Direct SessionManager access | Low latency | Tight coupling |

## Tool Specifications

### fugue_list_sessions

```json
{
  "name": "fugue_list_sessions",
  "description": "List all fugue sessions with their windows and panes",
  "inputSchema": {
    "type": "object",
    "properties": {},
    "required": []
  }
}
```

Returns: Array of sessions with nested windows and panes

### fugue_create_pane

```json
{
  "name": "fugue_create_pane",
  "description": "Create a new pane in a window",
  "inputSchema": {
    "type": "object",
    "properties": {
      "session_id": { "type": "string", "description": "Target session ID" },
      "window_id": { "type": "string", "description": "Target window ID" },
      "command": { "type": "string", "description": "Command to run in pane" },
      "cwd": { "type": "string", "description": "Working directory" }
    },
    "required": ["session_id", "window_id"]
  }
}
```

### fugue_send_input

```json
{
  "name": "fugue_send_input",
  "description": "Send input to a pane",
  "inputSchema": {
    "type": "object",
    "properties": {
      "pane_id": { "type": "string", "description": "Target pane ID" },
      "input": { "type": "string", "description": "Input to send" }
    },
    "required": ["pane_id", "input"]
  }
}
```

### fugue_get_output

```json
{
  "name": "fugue_get_output",
  "description": "Get recent output from a pane",
  "inputSchema": {
    "type": "object",
    "properties": {
      "pane_id": { "type": "string", "description": "Target pane ID" },
      "lines": { "type": "integer", "description": "Number of lines to retrieve", "default": 100 }
    },
    "required": ["pane_id"]
  }
}
```

### fugue_close_pane

```json
{
  "name": "fugue_close_pane",
  "description": "Close a pane",
  "inputSchema": {
    "type": "object",
    "properties": {
      "pane_id": { "type": "string", "description": "Target pane ID" }
    },
    "required": ["pane_id"]
  }
}
```

## Affected Components

| Component | Type of Change | Risk Level |
|-----------|----------------|------------|
| fugue-server/src/mcp/mod.rs | New module | Low |
| fugue-server/src/mcp/server.rs | New file | Medium |
| fugue-server/src/mcp/tools.rs | New file | Medium |
| fugue-server/src/lib.rs | Module registration | Low |
| Cargo.toml | Add rmcp dependency | Low |

## Dependencies

- FEAT-012: Session Management - Provides SessionManager and data model
- FEAT-015: (Specified dependency)

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| rmcp crate API changes | Low | Medium | Pin version, document API usage |
| Session manager thread safety | Medium | High | Use proper synchronization |
| Tool schema compatibility | Low | Medium | Follow MCP spec strictly |
| Performance with large output | Medium | Medium | Implement output truncation |

## Rollback Strategy

If implementation causes issues:
1. Revert commits associated with this work item
2. Remove mcp module from fugue-server
3. Remove rmcp dependency
4. Verify system returns to previous state
5. Document what went wrong in comments.md

## Implementation Notes

<!-- Add notes during implementation -->

---
*This plan should be updated as implementation progresses.*
