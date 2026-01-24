# Implementation Plan: FEAT-064

**Work Item**: [FEAT-064: Refactor MCP bridge.rs into modular components](PROMPT.md)
**Component**: fugue-server (mcp module)
**Priority**: P2
**Created**: 2026-01-13

## Overview

The MCP bridge implementation (`fugue-server/src/mcp/bridge.rs`) has grown to over 33,000 tokens and needs to be refactored into smaller, more maintainable modules. This is a pure refactoring task with no behavior changes.

## Architecture Decisions

### Module Boundary Strategy

The refactoring follows a "separation by concern" approach:

1. **Connection Management** (`connection.rs`)
   - Owns the connection lifecycle
   - Handles connect/disconnect/reconnect
   - Manages retry logic with exponential backoff

2. **Health Monitoring** (`health.rs`)
   - Owns `ConnectionState` enum and transitions
   - Implements heartbeat/ping logic
   - Runs health monitoring background task

3. **Core Bridge** (`bridge.rs`)
   - Owns `McpBridge` struct (slimmed down)
   - Orchestrates the main `run()` loop
   - Delegates to specialized modules

4. **Protocol Handling** (`protocol.rs` - existing)
   - JSON-RPC request/response construction
   - May need expansion for message routing

5. **Tool Handlers** (`handlers.rs` - existing)
   - Tool dispatch and execution
   - May need decomposition into `tools/` if large

### Key Design Choices

| Decision | Choice | Rationale |
|----------|--------|-----------|
| State ownership | `McpBridge` owns all state | Simplifies synchronization |
| Module communication | Direct method calls | Modules receive `&self` or `&mut self` references |
| Error handling | Preserve existing error types | No API changes required |
| Async boundaries | Preserve existing boundaries | Minimize refactoring scope |

## Affected Components

| Component | Type of Change | Risk Level |
|-----------|----------------|------------|
| `fugue-server/src/mcp/bridge.rs` | Primary - major reduction | Medium |
| `fugue-server/src/mcp/connection.rs` | New module | Low |
| `fugue-server/src/mcp/health.rs` | New module | Low |
| `fugue-server/src/mcp/mod.rs` | Update exports | Low |
| `fugue-server/src/mcp/protocol.rs` | Possible expansion | Low |
| `fugue-server/src/mcp/handlers.rs` | Possible refactoring | Medium |

## Dependencies

No external dependencies. This is a self-contained refactoring task.

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Breaking existing behavior | Low | High | Comprehensive testing, no API changes |
| Introducing subtle bugs in async code | Medium | Medium | Careful review of async boundaries |
| Missing edge cases in connection handling | Low | Medium | Test reconnection scenarios |
| Compilation errors from ownership changes | Medium | Low | Incremental extraction with tests after each step |

## Rollback Strategy

If implementation causes issues:
1. Revert commits associated with this work item
2. Verify system returns to previous state (single large `bridge.rs`)
3. Document what went wrong in comments.md

## Extraction Order

The recommended extraction order minimizes risk:

1. **Phase 1: Health Module** - Smallest, most isolated
   - `ConnectionState` enum
   - Heartbeat logic
   - Low coupling to other code

2. **Phase 2: Connection Module** - Medium complexity
   - `connect_to_daemon()` and variants
   - Reconnection logic
   - State machine transitions

3. **Phase 3: Slim down bridge.rs** - High-level orchestration only
   - Remove extracted code
   - Update imports and usage
   - Verify tests pass

4. **Phase 4 (optional): Tool decomposition**
   - Only if `handlers.rs` exceeds 500 lines
   - Split by tool category (session, pane, etc.)

## Implementation Notes

<!-- Add notes during implementation -->

### Current Module Structure (Before)

```
fugue-server/src/mcp/
├── mod.rs
├── bridge.rs      # ~33k tokens - TOO LARGE
├── protocol.rs
├── handlers.rs
└── error.rs
```

### Target Module Structure (After)

```
fugue-server/src/mcp/
├── mod.rs         # Updated exports
├── bridge.rs      # <500 lines - orchestration only
├── connection.rs  # NEW - connection management
├── health.rs      # NEW - health monitoring
├── protocol.rs    # Possibly expanded
├── handlers.rs    # Possibly refactored
├── error.rs       # Unchanged
└── tools/         # OPTIONAL if handlers.rs is large
    ├── mod.rs
    ├── session.rs
    └── pane.rs
```

---
*This plan should be updated as implementation progresses.*
