# BUG-021: ccmux_rename_session Not Handled in Standalone MCP Server

## Priority: P2
## Status: New
## Created: 2026-01-10

## Problem Summary

The `ccmux_rename_session` tool is defined in `tools.rs` and handled correctly in MCP bridge mode (`bridge.rs`), but is completely missing from the standalone MCP server mode (`server.rs`). Calling this tool in standalone mode returns an "unknown tool" error.

## Symptoms Observed

1. **Tool defined**: `ccmux_rename_session` exists in the tools list (`tools.rs:225`)
2. **Bridge mode works**: Tool is correctly handled in `bridge.rs:372`
3. **Standalone mode fails**: Tool returns "unknown tool" error
4. **Inconsistent behavior**: Same MCP tool works in one mode but not the other

## Steps to Reproduce

1. Start ccmux-server in standalone MCP server mode (not bridge mode)
2. Send a `tools/call` request for `ccmux_rename_session` with valid `session` and `name` parameters
3. Observe that the server returns an "unknown tool" error

## Expected Behavior

- The `ccmux_rename_session` tool should work identically in both MCP modes
- Session should be renamed successfully
- Response should confirm the rename operation

## Actual Behavior

- In standalone MCP server mode, the tool is not recognized
- Returns "unknown tool" error
- Session is not renamed

## Root Cause

Incomplete implementation during FEAT-043 (MCP Session Rename Tool). Only the bridge mode implementation was added. The standalone MCP server mode in `server.rs` is missing:

1. `ccmux_rename_session` in `is_known_tool()` function
2. `RenameSession` variant in `ToolParams` enum
3. Parsing case in `dispatch_tool()` match statement
4. Execution case in the result match statement
5. `rename_session()` method in `ToolContext` (handlers.rs)

## Files Affected

| File | Line(s) | Issue |
|------|---------|-------|
| `ccmux-server/src/mcp/server.rs` | 274-290 | `is_known_tool()` missing entry |
| `ccmux-server/src/mcp/server.rs` | 294-315 | `ToolParams` enum missing variant |
| `ccmux-server/src/mcp/server.rs` | 181-232 | `dispatch_tool()` missing parsing case |
| `ccmux-server/src/mcp/server.rs` | 238+ | Execution match missing case |
| `ccmux-server/src/mcp/handlers.rs` | - | Missing `rename_session()` method |

## Fix Required

### 1. Add to `is_known_tool()` (server.rs ~line 290)

```rust
fn is_known_tool(name: &str) -> bool {
    matches!(
        name,
        "ccmux_list_panes"
            | "ccmux_read_pane"
            // ... existing tools ...
            | "ccmux_create_window"
            | "ccmux_rename_session"  // ADD THIS
    )
}
```

### 2. Add `ToolParams` variant (server.rs ~line 315)

```rust
enum ToolParams {
    // ... existing variants ...
    CreateWindow { session: Option<String>, name: Option<String>, command: Option<String> },
    RenameSession { session: String, name: String },  // ADD THIS
}
```

### 3. Add parsing case in `dispatch_tool()` (server.rs ~line 231)

```rust
"ccmux_rename_session" => ToolParams::RenameSession {
    session: arguments["session"]
        .as_str()
        .ok_or_else(|| McpError::InvalidParams("Missing 'session' parameter".into()))?
        .to_string(),
    name: arguments["name"]
        .as_str()
        .ok_or_else(|| McpError::InvalidParams("Missing 'name' parameter".into()))?
        .to_string(),
},
```

### 4. Add execution case (server.rs ~line 260+)

```rust
ToolParams::RenameSession { session, name } => ctx.rename_session(&session, &name),
```

### 5. Add `rename_session()` method to `ToolContext` (handlers.rs)

```rust
pub fn rename_session(&mut self, session: &str, name: &str) -> ToolResult {
    // Find session by UUID or name
    let session_id = self.resolve_session(session)?;

    // Rename the session
    if let Some(session) = self.session_manager.get_session_mut(&session_id) {
        session.name = name.to_string();
        ToolResult::success(serde_json::json!({
            "renamed": true,
            "session_id": session_id.to_string(),
            "new_name": name
        }))
    } else {
        ToolResult::error(format!("Session not found: {}", session))
    }
}
```

## Related Issues

- **FEAT-043**: MCP Session Rename Tool - was marked complete but only implemented bridge mode

## Acceptance Criteria

- [ ] `ccmux_rename_session` added to `is_known_tool()` in server.rs
- [ ] `RenameSession` variant added to `ToolParams` enum
- [ ] Parsing case added to `dispatch_tool()` match
- [ ] Execution case added to result match
- [ ] `rename_session()` method added to `ToolContext`
- [ ] Tool works identically in both MCP modes
- [ ] Existing bridge mode functionality unaffected
- [ ] Unit tests added/updated for standalone mode

## Resolution

_To be determined after implementation_
