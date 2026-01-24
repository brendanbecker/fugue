# Implementation Plan: FEAT-048

**Work Item**: [FEAT-048: Expose orchestration protocol via MCP tools](PROMPT.md)
**Component**: fugue-server
**Priority**: P2
**Created**: 2026-01-10

## Overview

Add MCP tools for the existing orchestration message types (StatusUpdate, TaskAssignment, TaskComplete, HelpRequest, Broadcast, SyncRequest), enabling agents to communicate directly without going through shell commands.

## Architecture Decisions

### Approach: Layered MCP Surface

1. **Core Tool**: `fugue_send_orchestration` - full access to all message types and targets
2. **Convenience Tools**: Simplified wrappers for common patterns with auto-fill
3. **Subscription**: Polling-based initially, with potential for MCP notifications

### Trade-offs

| Decision | Rationale |
|----------|-----------|
| Polling over notifications | MCP notification support varies by client; polling is universally supported |
| Convenience tools | Reduces boilerplate for 80% use case (status updates, help requests) |
| Session context auto-fill | Agents shouldn't need to track their own session ID |

### Session Context Resolution

The MCP bridge needs to know which session/pane the calling agent is in. Options:

1. **Environment variable**: Set `FUGUE_SESSION_ID` when spawning Claude
2. **File-based**: Write session info to `$CLAUDE_CONFIG_DIR/.fugue-session`
3. **Lookup by PID**: Map MCP client PID to pane

Recommend: Environment variable approach (already used for `CLAUDE_CONFIG_DIR`)

## Affected Components

| Component | Type of Change | Risk Level |
|-----------|----------------|------------|
| fugue-server/src/mcp/tools.rs | Add new tools | Low |
| fugue-server/src/mcp/mod.rs | Register tools | Low |
| fugue-server/src/pty/spawn.rs | Set env vars | Low |
| fugue-protocol (reference only) | No changes | None |

## Implementation Phases

### Phase 1: Core Infrastructure

1. Add `FUGUE_SESSION_ID` environment variable to PTY spawn
2. Implement `fugue_send_orchestration` tool
3. Add basic integration test

### Phase 2: Convenience Tools

1. Implement `fugue_report_status`
2. Implement `fugue_request_help`
3. Implement `fugue_broadcast`

### Phase 3: Message Subscription

1. Add `fugue_get_orchestration_messages` polling tool
2. Store received messages in per-session queue
3. Consider notification mechanism for future

## Dependencies

None - this feature builds on existing infrastructure:
- OrchestrationMessage types already defined
- SendOrchestration already handled by daemon
- MCP tool infrastructure already in place

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Session context resolution complexity | Medium | Medium | Start with env var approach |
| MCP client compatibility | Low | Low | Use standard JSON schema |
| Message queue memory usage | Low | Medium | Add queue size limits |

## Rollback Strategy

If implementation causes issues:
1. Remove new MCP tool registrations
2. Revert PTY spawn changes
3. Verify existing orchestration via daemon still works

## Testing Strategy

1. **Unit tests**: Tool schema validation, message construction
2. **Integration tests**: End-to-end message routing
3. **Manual testing**: Claude agent using tools in real orchestration scenario

## Implementation Notes

<!-- Add notes during implementation -->

---
*This plan should be updated as implementation progresses.*
