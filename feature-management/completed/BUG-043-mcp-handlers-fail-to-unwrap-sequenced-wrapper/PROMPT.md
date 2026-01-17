# BUG-043: MCP tool handlers fail to unwrap Sequenced message wrapper from daemon responses

**Priority**: P1
**Component**: mcp-bridge (ccmux-server/src/mcp/bridge/)
**Severity**: high
**Status**: new

## Problem Statement

MCP tool handlers receive "Unexpected response: Sequenced { ... }" errors because the daemon wraps responses in `ServerMessage::Sequenced` for persistence tracking (FEAT-075), but the MCP bridge's response reception pipeline doesn't unwrap them.

This breaks multiple MCP tools that rely on `recv_response_from_daemon()`.

## Affected Components

| File | Function | Issue |
|------|----------|-------|
| `ccmux-server/src/mcp/bridge/connection.rs` | `recv_response_from_daemon()` | Returns Sequenced wrapper instead of inner message |
| `ccmux-server/src/mcp/bridge/connection.rs` | `is_broadcast_message()` | Missing Sequenced variant in filter |
| `ccmux-server/src/mcp/bridge/handlers.rs` | All tool handlers | Expect unwrapped message types |

## Symptoms

1. Tools return errors like "Unexpected response: Sequenced { seq: N, inner: ActualMessage }"
2. Some tools receive wrong response types (e.g., `broadcast` gets `TagsList`)
3. Operations succeed internally but MCP responses fail to parse

## Affected MCP Tools

| Status | Tools |
|--------|-------|
| **Broken** | kill_session, list_sessions (after changes), set_tags, get_tags, broadcast, report_status, request_help, beads_assign, beads_find_pane, beads_pane_history |
| **Working** | create_pane, close_pane, list_panes, get_status, send_input, read_pane |

**Why working tools work**: They use `recv_filtered()` with specific predicates that bypass the Sequenced wrapper issue.

## Steps to Reproduce

1. Start ccmux daemon with persistence enabled (FEAT-075)
2. Call any affected MCP tool:
   ```
   ccmux_kill_session(session_id: "test-session")
   ```
3. Observe error:
   ```
   MCP error -32603: Unexpected response: Sequenced { seq: 42, inner: Ok }
   ```

## Expected vs Actual Behavior

**Expected**: MCP tool handlers receive unwrapped message types (e.g., `SessionList`, `TagsList`, `Ok`) regardless of daemon's persistence wrapper.

**Actual**: Handlers receive `Sequenced { seq: N, inner: Box<ActualMessage> }` instead of the actual message type.

## Root Cause

The `is_broadcast_message()` function (connection.rs:390-427) doesn't include `ServerMessage::Sequenced` variant, so wrapped responses pass through the filter unchanged.

Tool handlers expect:
```rust
match response {
    ServerMessage::SessionList { sessions } => { /* handle */ }
    _ => Err("Unexpected response")
}
```

But receive:
```rust
ServerMessage::Sequenced {
    seq: 42,
    inner: Box::new(ServerMessage::SessionList { sessions })
}
```

## Proposed Fix

Unwrap `Sequenced` messages in `recv_response_from_daemon()`:

```rust
pub async fn recv_response_from_daemon(&mut self) -> Result<ServerMessage, McpError> {
    let msg = self.recv_filtered(|msg| !Self::is_broadcast_message(msg)).await?;
    // Unwrap Sequenced messages to get the actual response
    match msg {
        ServerMessage::Sequenced { inner, .. } => Ok(*inner),
        other => Ok(other),
    }
}
```

This is the minimal change that preserves the Sequenced wrapper for persistence (WAL replay) while providing unwrapped responses to MCP handlers.

## Implementation Tasks

### Section 1: Investigation
- [ ] Verify ServerMessage::Sequenced enum variant structure
- [ ] Confirm is_broadcast_message() is missing Sequenced handling
- [ ] Trace message flow from daemon to MCP handler
- [ ] Identify all callers of recv_response_from_daemon()

### Section 2: Fix Implementation
- [ ] Modify recv_response_from_daemon() to unwrap Sequenced messages
- [ ] Ensure unwrapping handles nested Sequenced (if possible)
- [ ] Update is_broadcast_message() if needed for consistency
- [ ] Add debug logging for wrapped/unwrapped message tracking

### Section 3: Testing
- [ ] Test all affected MCP tools after fix
- [ ] Verify working tools still work
- [ ] Test with persistence enabled and disabled
- [ ] Test WAL replay still works (Sequenced preserved in persistence layer)

### Section 4: Verification
- [ ] All affected tools return correct response types
- [ ] No regression in working tools
- [ ] Persistence tracking unaffected
- [ ] Add tests for Sequenced message unwrapping

## Acceptance Criteria

- [ ] kill_session returns success/error, not Sequenced wrapper
- [ ] set_tags/get_tags return TagsList, not Sequenced wrapper
- [ ] broadcast returns Ok, not Sequenced wrapper
- [ ] All beads_* tools work correctly
- [ ] Working tools (create_pane, list_panes, etc.) continue to work
- [ ] Persistence/WAL replay unaffected by the fix
- [ ] Tests added to prevent regression

## Related

- **FEAT-075**: Event sequencing for persistence (introduced Sequenced wrapper)
- **BUG-035**: MCP handlers return wrong response types (related pattern)
- **BUG-038**: create_pane returns wrong response type (similar symptoms)

## Notes

The fix should be localized to `recv_response_from_daemon()` to minimize impact. The Sequenced wrapper is still needed for WAL persistence, so the daemon should continue to emit Sequenced messages - we just unwrap them at the MCP bridge layer.
