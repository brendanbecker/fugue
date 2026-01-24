# FEAT-123: Add fugue_whoami MCP tool for agent self-identification

**Priority**: P1
**Component**: fugue-server
**Type**: enhancement
**Estimated Effort**: small
**Business Value**: high

## Problem Statement

Agents running inside fugue panes cannot reliably determine their own identity. When `fugue_get_tags` is called without a session filter, it returns an arbitrary session (e.g., "obsidian") instead of the session the calling agent is actually in (e.g., "fugue-orch").

This breaks:
- Lineage tagging (`child:<parent>`) - agents don't know their own session name to communicate to children
- Status reporting - agents can't verify their own tags
- Orchestration routing - agents can't construct proper parent references

## Root Cause

The MCP bridge runs as a subprocess of Claude Code, which runs inside a fugue pane. The pane has `FUGUE_PANE_ID` set in its environment (FEAT-053), but the MCP bridge doesn't use this to establish context. Instead, `fugue_get_tags` without a filter just returns the first session it finds.

## Solution

Add a `fugue_whoami` tool that:
1. Reads `FUGUE_PANE_ID` from the MCP server process's environment
2. Looks up the pane to find its session
3. Returns complete identity information

## Proposed Tool

### Tool Name: `fugue_whoami`

### Parameters
None required (reads from environment).

Optional:
- `include_tags`: boolean (default true) - whether to include session tags

### Response

```json
{
  "pane_id": "16e4c9c7-d0ad-48f7-b265-5f6557eb497c",
  "session_id": "78b0f4a6-91cf-4e9c-936e-e400c4fd6eb7",
  "session_name": "fugue-orch",
  "window_id": "4de6333e-3c84-4d0f-97d3-0b8b4d3bb99c",
  "tags": ["orchestrator", "fugue", "child:main-orch"],
  "cwd": "/home/user/projects/fugue"
}
```

### Error Cases

If `FUGUE_PANE_ID` is not set (running outside fugue):
```json
{
  "error": "Not running inside fugue",
  "detail": "FUGUE_PANE_ID environment variable not set"
}
```

If pane ID doesn't resolve (stale/orphaned):
```json
{
  "error": "Pane not found",
  "pane_id": "16e4c9c7-d0ad-48f7-b265-5f6557eb497c"
}
```

## Implementation

### Section 1: MCP Tool Schema

**File**: `fugue-server/src/mcp/tools.rs`

Add tool definition:
```rust
{
    "name": "fugue_whoami",
    "description": "Get the identity of the current pane/session. Returns pane ID, session ID, session name, tags, and cwd. Uses FUGUE_PANE_ID environment variable.",
    "inputSchema": {
        "type": "object",
        "properties": {
            "include_tags": {
                "type": "boolean",
                "default": true,
                "description": "Whether to include session tags in response"
            }
        }
    }
}
```

### Section 2: Handler Implementation

**File**: `fugue-server/src/mcp/bridge/handlers.rs`

```rust
pub async fn tool_whoami(&mut self, include_tags: bool) -> Result<ToolResult, McpError> {
    // Read pane ID from environment
    let pane_id_str = std::env::var("FUGUE_PANE_ID")
        .map_err(|_| McpError::InvalidParams(
            "Not running inside fugue: FUGUE_PANE_ID not set".into()
        ))?;

    let pane_id = Uuid::parse_str(&pane_id_str)
        .map_err(|e| McpError::InvalidParams(
            format!("Invalid FUGUE_PANE_ID: {}", e)
        ))?;

    // Query daemon for pane info
    match self.connection.send_and_recv(ClientMessage::GetPaneInfo { pane_id }).await? {
        ServerMessage::PaneInfo { pane_id, session_id, session_name, window_id, cwd, tags } => {
            let mut result = serde_json::json!({
                "pane_id": pane_id.to_string(),
                "session_id": session_id.to_string(),
                "session_name": session_name,
                "window_id": window_id.to_string(),
                "cwd": cwd,
            });

            if include_tags {
                result["tags"] = serde_json::json!(tags);
            }

            Ok(ToolResult::text(serde_json::to_string_pretty(&result)?))
        }
        ServerMessage::Error { message, .. } => {
            Ok(ToolResult::error(format!("Pane not found: {}", message)))
        }
        msg => Err(McpError::UnexpectedResponse(format!("{:?}", msg)))
    }
}
```

### Section 3: Protocol Message (if needed)

**File**: `fugue-protocol/src/messages.rs`

May need to add `GetPaneInfo` request and `PaneInfo` response if not already present:

```rust
// In ClientMessage
GetPaneInfo { pane_id: Uuid },

// In ServerMessage
PaneInfo {
    pane_id: Uuid,
    session_id: Uuid,
    session_name: String,
    window_id: Uuid,
    cwd: Option<String>,
    tags: HashSet<String>,
},
```

### Section 4: Daemon Handler

**File**: `fugue-server/src/session/manager.rs`

Handle `GetPaneInfo` by looking up the pane across all sessions.

## Files to Modify

| File | Changes |
|------|---------|
| `fugue-server/src/mcp/tools.rs` | Add tool schema |
| `fugue-server/src/mcp/bridge/mod.rs` | Route tool call |
| `fugue-server/src/mcp/bridge/handlers.rs` | Implement handler |
| `fugue-protocol/src/messages.rs` | Add GetPaneInfo/PaneInfo messages |
| `fugue-server/src/session/manager.rs` | Handle GetPaneInfo |

## Implementation Tasks

### Section 1: Protocol
- [ ] Add `GetPaneInfo` client message
- [ ] Add `PaneInfo` server message
- [ ] Implement handler in session manager

### Section 2: MCP Tool
- [ ] Add tool schema to tools.rs
- [ ] Add tool routing in mod.rs
- [ ] Implement `tool_whoami` handler

### Section 3: Testing
- [ ] Test inside fugue pane - returns correct identity
- [ ] Test outside fugue - returns appropriate error
- [ ] Test with include_tags=false

## Acceptance Criteria

- [ ] `fugue_whoami` returns correct pane_id, session_id, session_name
- [ ] Tags included by default, optional to exclude
- [ ] Graceful error when not running inside fugue
- [ ] Works correctly in nested orchestrator/worker scenarios

## Usage Example

Agent startup:
```
Agent calls fugue_whoami
Gets: session_name = "worker-feat-108"
Agent knows its identity for logging, status reports, and lineage
```

Spawning child with correct lineage:
```
Orchestrator calls fugue_whoami -> session_name = "orch-main"
Orchestrator spawns worker with tag "child:orch-main"
Worker can later use this to route messages back
```

## Related

- FEAT-053: Auto-inject fugue context env vars (provides FUGUE_PANE_ID)
- FEAT-122: Parent target for orchestration (uses lineage tags)
- BUG: fugue_get_tags returns wrong session without filter
