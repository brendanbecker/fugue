# Implementation Plan: BUG-052

**Work Item**: [BUG-052: Agents inside ccmux panes cannot connect to ccmux MCP server](PROMPT.md)
**Component**: mcp-bridge
**Priority**: P1
**Created**: 2026-01-17

## Overview

When an AI agent (e.g., Gemini CLI) is launched inside a ccmux pane, it cannot connect to the ccmux MCP server. This breaks the core orchestration use case where agents inside ccmux should be able to control ccmux (create panes, split windows, orchestrate other agents, etc.).

## Architecture Decisions

### Current State Analysis

The MCP bridge currently uses stdio-based communication:
- Single client connects via stdin/stdout
- This works for the outer agent (Claude Code) running ccmux
- Agents spawned INSIDE panes cannot use the same stdio channel

### Potential Solutions

**Option 1: Unix Domain Socket Transport**
- Add socket-based transport to mcp-bridge
- Expose socket path via environment variable in PTY
- Multiple clients can connect concurrently
- Pros: Multi-client support, standard approach
- Cons: More complex, requires bridge refactoring

**Option 2: TCP Transport**
- Add TCP listener on localhost:port
- Pass port via environment variable to nested agents
- Pros: Works across network boundaries (future remote support)
- Cons: Security considerations, port management

**Option 3: Environment Variable + Config Propagation**
- Ensure MCP config path is available in nested PTY environment
- May need to propagate socket/connection info
- Pros: Minimal changes
- Cons: May not solve core single-client limitation

### Recommended Approach

Start with investigation to determine root cause, then likely implement Option 1 (Unix Domain Socket) as it provides:
- Multi-client support
- Local-only security
- Standard MCP transport pattern

## Affected Components

| Component | Type of Change | Risk Level |
|-----------|----------------|------------|
| crates/mcp-bridge | Primary - add socket transport | High |
| crates/daemon | Secondary - environment propagation | Medium |
| crates/protocol | Minor - transport abstraction | Low |

## Dependencies

- Understanding of current MCP bridge architecture
- Analysis of how environment variables are passed to PTY processes
- Review of MCP transport standards (stdio vs socket vs SSE)

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Breaking existing stdio transport | Medium | High | Keep stdio as fallback, add socket as additional |
| Socket permission issues | Low | Medium | Proper socket file permissions |
| Race conditions with multiple clients | Medium | Medium | Proper async handling, connection management |
| Environment not propagated to PTY | Medium | High | Explicit env var handling in pane creation |

## Rollback Strategy

If implementation causes issues:
1. Disable socket transport, fall back to stdio-only
2. Revert changes to mcp-bridge transport layer
3. Document limitations for nested agent use case

## Investigation Checklist

Before implementation, must determine:

- [ ] How does Claude Code currently connect to ccmux MCP?
- [ ] What transport does the MCP bridge use (stdio, socket, etc.)?
- [ ] What environment variables are available inside ccmux PTY?
- [ ] Does Gemini CLI support the same MCP transport types?
- [ ] Is the bridge rejecting connections or is the client unable to find it?

## Implementation Notes

<!-- Add notes during implementation -->

---
*This plan should be updated as implementation progresses.*
