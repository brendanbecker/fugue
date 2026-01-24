# Implementation Plan: FEAT-051

**Work Item**: [FEAT-051: Add fugue_get_environment MCP tool](PROMPT.md)
**Component**: fugue-server (MCP)
**Priority**: P2
**Created**: 2026-01-10

## Overview

Allow reading environment variables from a session via MCP. This provides parity with the existing fugue_set_environment tool and enables session introspection.

## Architecture Decisions

### Message Design

**Decision**: Add dedicated message types rather than reusing existing ones.

- `GetEnvironment { session_id: SessionId, key: Option<String> }` for requests
- `Environment { session_id: SessionId, vars: HashMap<String, String> }` for responses

**Rationale**: Follows existing pattern for MCP tools. Clear separation of concerns.

### Return Semantics

**Decision**: When key is provided and not found, return empty map (not error).

**Rationale**: Consistent with shell behavior where checking for undefined variable is not an error. Error should only be for invalid session.

### Session Resolution

**Decision**: Use existing session resolution (UUID or name lookup).

**Rationale**: Consistency with other MCP tools like fugue_set_environment.

## Affected Components

| Component | Type of Change | Risk Level |
|-----------|----------------|------------|
| fugue-protocol | Add message variants | Low |
| fugue-server (MCP) | Add tool definition and handler | Low |
| fugue-server (handlers) | Add message handler | Low |

## Implementation Steps

### Step 1: Protocol Changes (fugue-protocol)

```rust
// In ClientMessage enum
GetEnvironment {
    session_id: SessionId,
    key: Option<String>,
}

// In ServerMessage enum
Environment {
    session_id: SessionId,
    vars: HashMap<String, String>,
}
```

### Step 2: Server Handler

Location: `fugue-server/src/handlers.rs` (or equivalent)

```rust
async fn handle_get_environment(
    session_id: SessionId,
    key: Option<String>,
    sessions: &SessionManager,
) -> Result<ServerMessage, Error> {
    let session = sessions.get(&session_id)?;
    let vars = match key {
        Some(k) => session.env.get(&k)
            .map(|v| [(k, v.clone())].into_iter().collect())
            .unwrap_or_default(),
        None => session.env.clone(),
    };
    Ok(ServerMessage::Environment { session_id, vars })
}
```

### Step 3: MCP Tool Definition

```json
{
  "name": "fugue_get_environment",
  "description": "Get environment variables from a session",
  "inputSchema": {
    "type": "object",
    "properties": {
      "session": {
        "type": "string",
        "description": "Session UUID or name"
      },
      "key": {
        "type": "string",
        "description": "Specific key to get, or omit for all"
      }
    },
    "required": ["session"]
  }
}
```

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Regression in existing MCP | Low | Medium | Comprehensive testing |
| Performance impact | Low | Low | HashMap is efficient |

## Rollback Strategy

If implementation causes issues:
1. Revert commits associated with this work item
2. Verify system returns to previous state
3. Document what went wrong in comments.md

## Implementation Notes

- Look at fugue_set_environment implementation as reference
- Ensure consistent error messages with other MCP tools
- Consider adding to MCP tool documentation

---
*This plan should be updated as implementation progresses.*
