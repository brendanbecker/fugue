# BUG-052: Agents inside ccmux panes cannot connect to ccmux MCP server

**Priority**: P1
**Component**: mcp-bridge
**Severity**: high
**Status**: new

## Problem Statement

When an AI agent (e.g., Gemini CLI) is launched inside a ccmux pane, it cannot connect to the ccmux MCP server. This breaks the core orchestration use case where agents inside ccmux should be able to control ccmux (create panes, split windows, orchestrate other agents, etc.).

## Evidence

**Error Message**:
```
MCP ERROR (ccmux)
Error during discovery for MCP server 'ccmux': Client is not connected, must connect before interacting with the server. Current state is disconnected
```

**Context**: Error observed when Gemini CLI (with ccmux MCP configured) is launched inside a ccmux pane.

## Steps to Reproduce

1. Start ccmux with MCP bridge running
2. Have an outer agent (Claude Code) connected to ccmux MCP successfully
3. Use ccmux MCP to create a session and pane
4. Launch Gemini CLI (or another agent with ccmux MCP configured) inside the pane
5. Observe Gemini fails to connect to ccmux MCP server

## Expected Behavior

Agents running inside ccmux panes should be able to connect to the ccmux MCP server and use ccmux tools to orchestrate (create panes, send input, read output, etc.). This is the core multi-agent orchestration use case.

## Actual Behavior

The nested agent fails to connect with "Client is not connected" error. The agent inside the ccmux pane cannot discover or interact with the ccmux MCP server.

## Root Cause

To be investigated. Suspected causes:

1. **Socket path inaccessibility**: MCP bridge socket path may not be accessible from the PTY environment inside ccmux
2. **Single-client limitation**: Current MCP bridge architecture may only support a single stdio-based client connection
3. **Configuration path issues**: Agent inside pane may not be able to find the MCP configuration (different working directory, missing env vars)
4. **Bridge architecture**: The bridge may not be designed to support multiple concurrent clients

## Implementation Tasks

### Section 1: Investigation
- [ ] Analyze how the outer agent (Claude Code) connects to ccmux MCP successfully
- [ ] Trace the connection path for agents spawned inside ccmux panes
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
- [ ] Confirm nested agents can connect to ccmux MCP
- [ ] Verify nested agents can use all ccmux MCP tools
- [ ] Confirm multi-agent orchestration workflows function correctly
- [ ] Update documentation if configuration changes required

## Acceptance Criteria

- [ ] Agents spawned inside ccmux panes can connect to ccmux MCP server
- [ ] Nested agents can discover and use all ccmux MCP tools
- [ ] Multiple concurrent agent connections are supported
- [ ] Existing single-agent use cases continue to work
- [ ] Multi-agent orchestration workflows function as expected
- [ ] Tests added to prevent regression

## Notes

This bug blocks the primary orchestration use case for ccmux. The expected workflow is:
1. User has an "outer" agent (Claude Code) controlling ccmux
2. Outer agent creates panes and launches "inner" agents
3. Inner agents can also use ccmux MCP to create their own panes, orchestrate further
4. This enables hierarchical multi-agent workflows

The fact that the outer agent (Claude Code) can connect successfully while nested agents cannot suggests either:
- The connection mechanism is different (outer uses different path)
- The nested PTY environment is missing required configuration
- The bridge rejects additional connections after the first
