# FEAT-018: MCP Server - Model Context Protocol Integration

**Priority**: P2
**Component**: fugue-server
**Type**: new_feature
**Estimated Effort**: large
**Business Value**: medium

## Overview

Model Context Protocol server exposing tools for Claude to interact with fugue (list panes, send input, create panes).

## Requirements

- MCP server implementation using rmcp crate
- Tools exposed to Claude:
  - fugue_list_sessions: List all sessions/windows/panes
  - fugue_create_pane: Create new pane in window
  - fugue_send_input: Send input to pane
  - fugue_get_output: Get recent pane output
  - fugue_close_pane: Close a pane
- JSON-RPC transport over stdio
- Tool schema definitions

## Benefits

- Enables Claude to directly interact with fugue sessions
- Allows AI-driven terminal automation and orchestration
- Provides programmatic access to session/pane management
- Supports advanced multi-pane workflows driven by Claude

## Implementation Tasks

### Section 1: Design
- [ ] Review requirements and acceptance criteria
- [ ] Design MCP server architecture
- [ ] Define tool schemas for all exposed tools
- [ ] Plan integration with existing session manager
- [ ] Document implementation approach in PLAN.md

### Section 2: Implementation
- [ ] Add rmcp crate dependency to fugue-server
- [ ] Implement MCP server module (fugue-server/src/mcp/mod.rs)
- [ ] Implement server initialization and lifecycle (fugue-server/src/mcp/server.rs)
- [ ] Implement tool handlers (fugue-server/src/mcp/tools.rs)
- [ ] Implement fugue_list_sessions tool
- [ ] Implement fugue_create_pane tool
- [ ] Implement fugue_send_input tool
- [ ] Implement fugue_get_output tool
- [ ] Implement fugue_close_pane tool
- [ ] Add JSON-RPC transport over stdio
- [ ] Add error handling for all tool operations
- [ ] Add logging for MCP operations

### Section 3: Testing
- [ ] Add unit tests for tool schema validation
- [ ] Add unit tests for each tool handler
- [ ] Add integration tests for MCP server lifecycle
- [ ] Test JSON-RPC message parsing and serialization
- [ ] Manual testing with Claude

### Section 4: Documentation
- [ ] Document MCP server configuration
- [ ] Document tool schemas and usage
- [ ] Add code comments for tool implementations
- [ ] Update CHANGELOG

### Section 5: Verification
- [ ] All acceptance criteria met
- [ ] Tests passing
- [ ] Code review completed
- [ ] Ready for deployment

## Acceptance Criteria

- [ ] MCP server starts and accepts JSON-RPC connections over stdio
- [ ] fugue_list_sessions returns complete session hierarchy
- [ ] fugue_create_pane creates pane in specified window
- [ ] fugue_send_input delivers input to target pane
- [ ] fugue_get_output returns recent pane output
- [ ] fugue_close_pane properly closes target pane
- [ ] All tools have proper schema definitions
- [ ] Error handling for invalid tool calls
- [ ] All tests passing
- [ ] No regressions in existing functionality

## Dependencies

- FEAT-012: Session Management - Session/Window/Pane Hierarchy (provides session data model)
- FEAT-015: (dependency specified)

## Affected Files

- fugue-server/src/mcp/server.rs
- fugue-server/src/mcp/tools.rs
- fugue-server/src/mcp/mod.rs

## Notes

The MCP (Model Context Protocol) integration enables Claude to interact programmatically with fugue sessions. This is a key feature for AI-driven terminal orchestration, allowing Claude to spawn panes, send commands, and read output without requiring manual user intervention.

Technical considerations:
- rmcp crate provides Rust MCP server implementation
- JSON-RPC 2.0 protocol over stdio for transport
- Tool schemas must follow MCP specification
- Session manager integration requires thread-safe access
