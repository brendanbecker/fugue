# Task Breakdown: BUG-052

**Work Item**: [BUG-052: Agents inside fugue panes cannot connect to fugue MCP server](PROMPT.md)
**Status**: Not Started
**Last Updated**: 2026-01-17

## Prerequisites

- [ ] Read and understand PROMPT.md
- [ ] Review PLAN.md and update if needed
- [ ] Identify current MCP bridge transport mechanism

## Investigation Tasks

- [ ] Analyze how Claude Code connects to fugue MCP (stdio? socket?)
- [ ] Check MCP configuration for Claude Code vs Gemini CLI
- [ ] List environment variables inside a fugue PTY pane
- [ ] Determine if MCP bridge socket/path is accessible from PTY
- [ ] Trace Gemini CLI's MCP connection attempt to identify failure point
- [ ] Review mcp-bridge crate for transport implementation
- [ ] Check if bridge supports multiple concurrent clients
- [ ] Document root cause in PLAN.md

## Design Tasks

- [ ] Evaluate transport options (stdio, socket, TCP)
- [ ] Design multi-client connection architecture
- [ ] Plan environment variable propagation strategy
- [ ] Update PLAN.md with chosen approach
- [ ] Consider backward compatibility with existing clients

## Implementation Tasks

### If Socket Transport Needed:
- [ ] Add Unix Domain Socket listener to mcp-bridge
- [ ] Implement socket path configuration
- [ ] Add concurrent client connection handling
- [ ] Implement connection lifecycle management (connect, disconnect)
- [ ] Add socket cleanup on shutdown

### Environment Propagation:
- [ ] Identify required environment variables for MCP connection
- [ ] Pass MCP socket path/config to PTY spawn
- [ ] Verify variables are accessible inside pane

### Error Handling:
- [ ] Add meaningful error messages for connection failures
- [ ] Log connection attempts from nested agents
- [ ] Handle socket permission errors gracefully

## Testing Tasks

- [ ] Test outer agent (Claude Code) can still connect
- [ ] Test nested agent (Gemini CLI) can connect via socket
- [ ] Test multiple concurrent agent connections
- [ ] Test connection cleanup when agent exits
- [ ] Test socket cleanup on daemon shutdown
- [ ] Add integration test for nested MCP connectivity

## Documentation Tasks

- [ ] Update configuration documentation
- [ ] Document environment variables for nested agents
- [ ] Add troubleshooting guide for connection issues

## Verification Tasks

- [ ] Confirm nested agents can connect to fugue MCP
- [ ] Verify all MCP tools work from nested agents
- [ ] Test multi-agent orchestration workflow end-to-end
- [ ] Update bug_report.json status
- [ ] Document resolution in comments.md

## Completion Checklist

- [ ] All implementation tasks complete
- [ ] All tests passing
- [ ] PLAN.md updated with final approach
- [ ] Root cause documented
- [ ] Ready for review/merge

---
*Check off tasks as you complete them. Update status field above.*
