# BUG-061: fugue_send_orchestration target parameter parsing fails

**Priority**: P2
**Component**: mcp
**Severity**: medium
**Status**: fixed

## Problem

`fugue_send_orchestration` fails with "Invalid target" error even when valid target objects are provided. The target parameter is not being parsed correctly when passed through the MCP protocol.

```
MCP error -32602: Invalid target: must specify 'tag', 'session', 'broadcast', or 'worktree'
```

## Reproduction Steps

1. Attach to a session: `fugue_attach_session`
2. Call `fugue_send_orchestration` with any valid target:
   - `{"target": {"broadcast": true}, "msg_type": "test", "payload": {}}`
   - `{"target": {"tag": "orchestrator"}, "msg_type": "test", "payload": {}}`
3. Observe: "Invalid target" error returned

## Expected Behavior

Message is sent to the specified target (broadcast to all, or to sessions with tag).

## Actual Behavior

All target formats fail with the same error, indicating the target object is not being parsed correctly.

## Root Cause Analysis

The `fugue_send_orchestration` handler in `mod.rs:502-508` extracts the target as:
```rust
let target = &arguments["target"];
```

But when MCP clients (including Claude Code) pass object parameters, they may arrive as JSON strings rather than parsed objects. The `fugue_create_layout` tool handles this case (lines 447-455):

```rust
let layout = match &raw_layout {
    serde_json::Value::String(s) => {
        serde_json::from_str(s).map_err(|e| {
            McpError::InvalidParams(format!("Invalid layout JSON string: {}", e))
        })?
    }
    other => other.clone(),
};
```

The `fugue_send_orchestration` tool lacks this string-to-object parsing.

## Fix

Add the same string parsing logic to `fugue_send_orchestration` in `fugue-server/src/mcp/bridge/mod.rs`:

```rust
"fugue_send_orchestration" => {
    let raw_target = arguments["target"].clone();
    let target = match &raw_target {
        serde_json::Value::String(s) => {
            serde_json::from_str(s).map_err(|e| {
                McpError::InvalidParams(format!("Invalid target JSON string: {}", e))
            })?
        }
        other => other.clone(),
    };
    let msg_type = arguments["msg_type"]
        .as_str()
        .ok_or_else(|| McpError::InvalidParams("Missing 'msg_type' parameter".into()))?;
    let payload = arguments["payload"].clone();
    handlers.tool_send_orchestration(&target, msg_type, payload).await
}
```

## Acceptance Criteria

- [ ] `fugue_send_orchestration` accepts target as JSON object
- [ ] `fugue_send_orchestration` accepts target as JSON string (for compatibility)
- [ ] All target types work: `tag`, `session`, `broadcast`, `worktree`
- [ ] Add test coverage for both object and string parameter formats

## Related Files

- `fugue-server/src/mcp/bridge/mod.rs:502-508` - dispatch logic (needs fix)
- `fugue-server/src/mcp/bridge/mod.rs:447-455` - reference implementation in create_layout
- `fugue-server/src/mcp/bridge/handlers.rs:1027-1073` - handler (correct, issue is in dispatch)

## Notes

- Discovered during QA of BUG-060 fix
- Other orchestration tools (`fugue_report_status`, `fugue_broadcast`, `fugue_set_tags`) work correctly
- Only `fugue_send_orchestration` is affected due to its complex object parameter
