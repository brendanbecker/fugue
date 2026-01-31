# FEAT-133: MCP Server Parameter

**Priority**: P1
**Component**: fugue-server/mcp
**Effort**: Medium
**Status**: new
**Depends**: FEAT-130

## Summary

Add optional `server` parameter to all MCP tools to specify which server should handle the request.

## Problem

With multi-server support, MCP tool calls need to specify which server to target. An orchestrator on `local` should be able to spawn workers on `polecats`.

## Proposed API

### Option A: Explicit server parameter

```json
{
  "tool": "fugue_create_session",
  "input": {
    "server": "polecats",
    "name": "worker-feat-127",
    "command": "claude --dangerously-skip-permissions",
    "tags": ["worker"]
  }
}
```

### Option B: Namespaced identifiers (FEAT-131)

```json
{
  "tool": "fugue_kill_session",
  "input": {
    "session": "polecats:worker-feat-127"
  }
}
```

**Recommendation**: Support both. Explicit `server` parameter for creation, namespaced IDs for referencing existing resources.

## Implementation

### Schema Updates

```rust
// In fugue-server/src/mcp/tools.rs

pub fn create_session_schema() -> Value {
    json!({
        "name": "fugue_create_session",
        "description": "Create a new terminal session",
        "parameters": {
            "type": "object",
            "properties": {
                "server": {
                    "type": "string",
                    "description": "Target server name (default: local/attached)"
                },
                "name": {
                    "type": "string",
                    "description": "Session name"
                },
                // ... other params ...
            }
        }
    })
}
```

### Handler Routing

```rust
// In fugue-client MCP proxy (since client has multi-connection)

async fn handle_mcp_request(&self, tool: &str, params: Value) -> Result<Value> {
    // Extract target server
    let server = params.get("server")
        .and_then(|v| v.as_str())
        .unwrap_or("local");

    // Get connection for that server
    let conn = self.manager.get(server)
        .ok_or_else(|| anyhow!("Not connected to server: {}", server))?;

    // Forward request
    conn.send_mcp_request(tool, params).await
}
```

### Tools Updated

All tools gain optional `server` parameter:

| Tool | Server Parameter Usage |
|------|----------------------|
| `fugue_create_session` | Where to create |
| `fugue_create_pane` | Where to create |
| `fugue_kill_session` | Which server (or parse from namespaced session) |
| `fugue_send_input` | Which server (or parse from pane_id) |
| `fugue_read_pane` | Which server (or parse from pane_id) |
| `fugue_list_sessions` | Which server, or all if omitted |
| `fugue_list_panes` | Which server |
| `fugue_get_tags` | Which server |
| `fugue_send_orchestration` | Source and target servers |

### List Operations

`fugue_list_sessions` without server param returns all:

```json
{
  "tool": "fugue_list_sessions"
}

// Returns:
{
  "servers": {
    "local": [
      {"name": "nexus", "id": "..."},
      {"name": "fugue-orch", "id": "..."}
    ],
    "polecats": [
      {"name": "worker-1", "id": "..."}
    ]
  }
}
```

## MCP Architecture Note

The MCP server runs alongside the fugue-server daemon. For multi-server MCP:

**Option 1**: Client-side MCP proxy
- Client connects to multiple servers
- Client runs MCP server that routes to appropriate backend
- Agents connect to client's MCP

**Option 2**: Server-side federation
- Each server has its own MCP
- Servers can forward requests to each other
- More complex, but works without client

**Recommendation**: Option 1 (client-side proxy) for simplicity.

## Acceptance Criteria

- [ ] All tools accept optional `server` parameter
- [ ] Default to local/attached server when omitted
- [ ] Clear error when server unknown/disconnected
- [ ] `fugue_list_sessions` aggregates across servers
- [ ] Namespaced IDs work as alternative to explicit server
- [ ] Documentation updated with examples

## Related

- FEAT-130: Multi-connection client (routing infrastructure)
- FEAT-131: Namespaced sessions (alternative syntax)
- FEAT-134: Cross-server routing (message routing)
