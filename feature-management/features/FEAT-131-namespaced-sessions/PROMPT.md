# FEAT-131: Namespaced Session Identifiers

**Priority**: P1
**Component**: fugue-protocol
**Effort**: Medium
**Status**: new
**Depends**: FEAT-130

## Summary

Introduce `server:session` namespaced identifiers throughout the system to uniquely identify sessions across multiple servers.

## Problem

With multi-server support, session names are no longer unique. Two servers could both have a session named "worker-1". We need a namespacing scheme.

## Proposed Format

```
server_name:session_name

Examples:
  local:nexus
  polecats:worker-bug-073
  workstation:orch-featmgmt
```

## Implementation

### Session Identifier Type

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SessionId {
    pub server: String,
    pub session: String,
}

impl SessionId {
    pub fn new(server: impl Into<String>, session: impl Into<String>) -> Self {
        Self {
            server: server.into(),
            session: session.into(),
        }
    }

    pub fn parse(s: &str) -> Result<Self> {
        if let Some((server, session)) = s.split_once(':') {
            Ok(Self::new(server, session))
        } else {
            // No colon - assume default/local server
            Ok(Self::new("local", s))
        }
    }

    pub fn to_string(&self) -> String {
        format!("{}:{}", self.server, self.session)
    }
}

impl Display for SessionId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.server, self.session)
    }
}
```

### Key Files

| File | Changes |
|------|---------|
| `fugue-protocol/src/session.rs` | Add `SessionId` type |
| `fugue-client/src/ui/*.rs` | Use `SessionId` throughout |
| `fugue-server/src/mcp/bridge/*.rs` | Parse namespaced IDs in MCP |

### CLI Usage

```bash
# Attach to specific session on specific server
fugue attach polecats:worker-1

# Kill session on specific server
fugue kill local:stale-session

# Without server prefix, uses default/current server
fugue attach worker-1  # Assumes current server context
```

### MCP Tool Parameters

```json
{
  "tool": "fugue_send_input",
  "input": {
    "pane_id": "polecats:abc-123-def",
    "input": "hello"
  }
}
```

Or with separate fields:

```json
{
  "tool": "fugue_send_input",
  "input": {
    "server": "polecats",
    "pane_id": "abc-123-def",
    "input": "hello"
  }
}
```

## Display Considerations

- Full format: `polecats:worker-bug-073`
- Abbreviated in context: When viewing polecats server, show just `worker-bug-073`
- Color coding: Different servers get different colors in TUI

## Acceptance Criteria

- [ ] `SessionId` type handles parsing and formatting
- [ ] Works with and without server prefix
- [ ] Default server used when prefix omitted
- [ ] All CLI commands accept namespaced sessions
- [ ] MCP tools accept namespaced identifiers
- [ ] Clear error when server unknown

## Related

- FEAT-130: Multi-connection client (provides server context)
- FEAT-132: TUI server awareness (displays namespaced sessions)
- FEAT-133: MCP server parameter (alternative to namespace prefix)
