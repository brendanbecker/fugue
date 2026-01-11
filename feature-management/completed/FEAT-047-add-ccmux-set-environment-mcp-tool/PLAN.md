# Implementation Plan: FEAT-047

**Work Item**: [FEAT-047: Add ccmux_set_environment MCP tool](PROMPT.md)
**Component**: ccmux-server (MCP)
**Priority**: P1
**Created**: 2026-01-10

## Overview

Allow setting environment variables on a session that will be inherited by panes/processes. This enables Gas Town integration by providing tmux-like `set-environment` functionality.

## Architecture Decisions

<!-- Document key design choices and rationale -->

- **Approach**: Store environment as HashMap on Session struct, propagate to PTY spawn
- **Trade-offs**:
  - Session-level vs pane-level environment storage (choosing session-level for tmux parity)
  - Eager propagation vs lazy resolution (choosing eager for simplicity)

## Affected Components

<!-- List files and modules that will be modified -->

| Component | Type of Change | Risk Level |
|-----------|----------------|------------|
| ccmux-server/src/session/session.rs | Add environment field | Low |
| ccmux-protocol/src/messages.rs | Add SetEnvironment message | Low |
| ccmux-server/src/mcp/tools.rs | Add MCP tool definition | Low |
| ccmux-server/src/mcp/handlers.rs | Add tool handler | Medium |
| ccmux-server/src/pty/spawn.rs | Pass environment to spawn | Medium |
| ccmux-persistence (optional) | Persist environment | Low |

## Implementation Details

### 1. Session Environment Storage

```rust
// In ccmux-server/src/session/session.rs
pub struct Session {
    // existing fields...
    pub environment: HashMap<String, String>,
}
```

### 2. Protocol Message

```rust
// In ccmux-protocol/src/messages.rs
pub enum ClientMessage {
    // existing variants...
    SetEnvironment {
        session_id: SessionId,
        key: String,
        value: String,
    },
}
```

### 3. MCP Tool Schema

```json
{
  "name": "ccmux_set_environment",
  "description": "Set an environment variable on a session that will be inherited by new panes",
  "inputSchema": {
    "type": "object",
    "properties": {
      "session": {
        "type": "string",
        "description": "Session UUID or name"
      },
      "key": {
        "type": "string",
        "description": "Environment variable name"
      },
      "value": {
        "type": "string",
        "description": "Environment variable value"
      }
    },
    "required": ["session", "key", "value"]
  }
}
```

### 4. PTY Spawn Integration

When spawning a new PTY for a pane, merge session environment with system environment:

```rust
// Pseudocode
fn spawn_pty(session: &Session, command: &str) {
    let mut env = std::env::vars().collect::<HashMap<_, _>>();
    env.extend(session.environment.clone());
    // Pass env to portable_pty::CommandBuilder
}
```

## Dependencies

None

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Regression in existing functionality | Low | High | Comprehensive testing |
| Environment not propagating correctly | Medium | Medium | Integration tests with actual spawns |
| Session lookup by name ambiguity | Low | Low | Document behavior for duplicate names |

## Rollback Strategy

If implementation causes issues:
1. Revert commits associated with this work item
2. Verify system returns to previous state
3. Document what went wrong in comments.md

## Implementation Notes

<!-- Add notes during implementation -->

---
*This plan should be updated as implementation progresses.*
