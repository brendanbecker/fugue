# BUG-060: Orchestration MCP tools fail without session attachment

**Priority**: P2
**Component**: mcp, orchestration
**Severity**: medium
**Status**: completed

## Problem

Several orchestration-related MCP tools fail with:
```
InvalidOperation: Must be attached to a session to send orchestration messages
```

Affected tools:
- `ccmux_send_orchestration`
- `ccmux_broadcast`
- `ccmux_report_status`
- `ccmux_request_help`

## Reproduction Steps

1. Connect to ccmux via MCP (e.g., from Claude Code)
2. Call any orchestration tool:
   ```
   ccmux_report_status(status: "working", message: "Test")
   ```
3. Observe: "Must be attached to a session" error

## Expected Behavior

MCP clients should be able to send orchestration messages. The MCP bridge should have session context or use a default/implicit session.

## Actual Behavior

All orchestration message tools fail because the MCP bridge doesn't have session attachment.

## Analysis

The MCP bridge connects to the daemon but doesn't attach to a specific session. Orchestration messages require knowing which session is sending the message (for routing replies, identifying sender, etc.).

### Design Question

How should MCP clients participate in orchestration?

**Option A: Implicit Session**
MCP bridge could auto-attach to the session that spawned it (if detectable).

**Option B: Explicit Attachment**
Add a `ccmux_attach_session` tool that MCP clients must call first.

**Option C: Session Parameter**
Add optional `from_session` parameter to orchestration tools.

**Option D: MCP-Specific Routing**
Allow MCP clients to send messages with a special "mcp-client" identity rather than a session ID.

## Investigation Steps

### Section 1: Understand Current Architecture
- [x] Review how MCP bridge connects to daemon
- [x] Check if session context is available but not used
- [x] Review orchestration message routing requirements

### Section 2: Evaluate Options
- [x] Determine which option best fits the architecture
  - Selected Option B: Explicit Attachment via `ccmux_attach_session`
- [x] Consider security implications (can MCP client impersonate sessions?)
- [x] Check if session info is available from connection context

### Section 3: Implement Fix
- [x] Implement chosen solution
  - Added `ccmux_attach_session` tool
  - Modified `ConnectionManager` to filter broadcasts to prevent channel flooding
- [x] Update MCP tool documentation
- [x] Add tests for orchestration from MCP

## Acceptance Criteria

- [x] `ccmux_report_status` works from MCP client
- [x] `ccmux_send_orchestration` works from MCP client
- [x] `ccmux_broadcast` works from MCP client
- [x] `ccmux_request_help` works from MCP client
- [x] Messages properly identify the sender
- [x] Documentation updated to explain usage

## Related Files

- `ccmux-server/src/mcp/bridge/handlers.rs` - MCP handlers
- `ccmux-server/src/mcp/bridge/connection.rs` - MCP bridge connection
- `ccmux-server/src/orchestration/router.rs` - Message routing (listed as dead code in BUG-047)

## Notes

- This blocks the message-passing demos in DEMO-MULTI-AGENT.md
- The orchestration router itself may also be incomplete (dead code per BUG-047)
- Discovered during multi-agent orchestration demo on 2026-01-18