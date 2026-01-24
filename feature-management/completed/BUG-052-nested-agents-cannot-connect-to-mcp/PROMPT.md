# BUG-052: Agents inside fugue panes cannot connect to fugue MCP server

**Priority**: P1
**Component**: mcp-bridge
**Severity**: high
**Status**: new

## Problem Statement

When an AI agent (e.g., Gemini CLI) is launched inside a fugue pane, it cannot connect to the fugue MCP server. This breaks the core orchestration use case where agents inside fugue should be able to control fugue (create panes, split windows, orchestrate other agents, etc.).

## Evidence

**Error Message**:
```
MCP ERROR (fugue)
Error during discovery for MCP server 'fugue': Client is not connected, must connect before interacting with the server. Current state is disconnected
```

**Context**: Error observed when Gemini CLI (with fugue MCP configured) is launched inside a fugue pane.

## Steps to Reproduce

1. Start fugue with MCP bridge running
2. Have an outer agent (Claude Code) connected to fugue MCP successfully
3. Use fugue MCP to create a session and pane
4. Launch Gemini CLI (or another agent with fugue MCP configured) inside the pane
5. Observe Gemini fails to connect to fugue MCP server

## Expected Behavior

Agents running inside fugue panes should be able to connect to the fugue MCP server and use fugue tools to orchestrate (create panes, send input, read output, etc.). This is the core multi-agent orchestration use case.

## Actual Behavior

The nested agent fails to connect with "Client is not connected" error. The agent inside the fugue pane cannot discover or interact with the fugue MCP server.

## Root Cause

To be investigated. Suspected causes:

1. **Socket path inaccessibility**: MCP bridge socket path may not be accessible from the PTY environment inside fugue
2. **Single-client limitation**: Current MCP bridge architecture may only support a single stdio-based client connection
3. **Configuration path issues**: Agent inside pane may not be able to find the MCP configuration (different working directory, missing env vars)
4. **Bridge architecture**: The bridge may not be designed to support multiple concurrent clients

## Implementation Tasks

### Section 1: Investigation
- [ ] Analyze how the outer agent (Claude Code) connects to fugue MCP successfully
- [ ] Trace the connection path for agents spawned inside fugue panes
- [ ] Identify what environment variables and paths are available inside the PTY
- [ ] Determine if the MCP bridge socket/connection is accessible from nested processes
- [ ] Check if the bridge is stdio-based (single client) or socket-based (multi-client)

### Section 2: Architecture Analysis
- [ ] Review mcp-bridge crate architecture for client connection handling
- [ ] Determine if the bridge supports concurrent clients
- [ ] Identify what changes would be needed for multi-client support
- [ ] Evaluate socket-based vs stdio-based transport options

### Section 3: Fix Implementation
- [ ] Implement fix based on root cause findings
- [ ] If socket path issue: Ensure socket path is set/accessible in PTY environment
- [ ] If single-client issue: Refactor bridge to support multiple concurrent clients
- [ ] If config issue: Pass necessary configuration to nested PTY processes
- [ ] Add appropriate error handling and logging

### Section 4: Testing
- [ ] Add integration test for nested agent MCP connectivity
- [ ] Test with multiple agent types (Claude Code, Gemini CLI, etc.)
- [ ] Test concurrent connections from multiple agents
- [ ] Verify existing single-client use cases still work

### Section 5: Verification
- [ ] Confirm nested agents can connect to fugue MCP
- [ ] Verify nested agents can use all fugue MCP tools
- [ ] Confirm multi-agent orchestration workflows function correctly
- [ ] Update documentation if configuration changes required

## Acceptance Criteria

- [ ] Agents spawned inside fugue panes can connect to fugue MCP server
- [ ] Nested agents can discover and use all fugue MCP tools
- [ ] Multiple concurrent agent connections are supported
- [ ] Existing single-agent use cases continue to work
- [ ] Multi-agent orchestration workflows function as expected
- [ ] Tests added to prevent regression

## Notes

This bug blocks the primary orchestration use case for fugue. The expected workflow is:
1. User has an "outer" agent (Claude Code) controlling fugue
2. Outer agent creates panes and launches "inner" agents
3. Inner agents can also use fugue MCP to create their own panes, orchestrate further
4. This enables hierarchical multi-agent workflows

The fact that the outer agent (Claude Code) can connect successfully while nested agents cannot suggests either:
- The connection mechanism is different (outer uses different path)
- The nested PTY environment is missing required configuration
- The bridge rejects additional connections after the first
