# FEAT-109: MCP Drain Messages Tool

## Summary

Add a `fugue_drain_messages` MCP tool that clears stale broadcast messages from the response channel, providing a workaround for INQ-004 (MCP Response Integrity) while root cause investigation continues.

## Problem

The MCP bridge experiences response mixing when broadcast messages (PaneResized, SessionList, PaneContent) accumulate in the response channel. This causes:
- Unexpected response types returned to MCP calls
- Need for manual retries to clear stale messages
- Unreliable orchestration operations

## Solution

Add an explicit drain tool that:
1. Clears all pending messages from the response channel
2. Returns count of drained messages for diagnostics
3. Can be called before critical operations or after errors

## MCP Tool Schema

```json
{
  "name": "fugue_drain_messages",
  "description": "Drain stale broadcast messages from the MCP response channel. Use this to clear accumulated PaneResized, SessionList, or other broadcast messages that may interfere with subsequent requests. Returns the count of messages drained.",
  "inputSchema": {
    "type": "object",
    "properties": {
      "timeout_ms": {
        "type": "integer",
        "description": "Maximum time to spend draining in milliseconds (default: 100)",
        "default": 100
      }
    }
  }
}
```

## Response Format

```json
{
  "drained_count": 5,
  "message_types": ["PaneResized", "PaneResized", "SessionList", "PaneContent", "PaneResized"],
  "status": "drained"
}
```

## Implementation

### Location
`fugue-server/src/mcp/bridge/connection.rs`

### Approach

1. **Add tool schema** in `fugue-server/src/mcp/tools.rs`

2. **Add handler** in `fugue-server/src/mcp/bridge/handlers.rs`:
   ```rust
   "fugue_drain_messages" => {
       let timeout_ms = params.get("timeout_ms")
           .and_then(|v| v.as_u64())
           .unwrap_or(100) as u64;

       let (count, types) = self.drain_with_diagnostics(timeout_ms).await;

       Ok(json!({
           "drained_count": count,
           "message_types": types,
           "status": "drained"
       }))
   }
   ```

3. **Add drain method** in connection.rs (extend existing `drain_pending_messages`):
   ```rust
   pub async fn drain_with_diagnostics(&self, timeout_ms: u64) -> (usize, Vec<String>) {
       let mut count = 0;
       let mut types = Vec::new();
       let deadline = Instant::now() + Duration::from_millis(timeout_ms);

       while Instant::now() < deadline {
           match self.receiver.try_recv() {
               Ok(msg) => {
                   count += 1;
                   types.push(msg.type_name()); // Add type_name() to ServerMessage
               }
               Err(TryRecvError::Empty) => break,
               Err(TryRecvError::Disconnected) => break,
           }
       }

       (count, types)
   }
   ```

4. **Add type_name to ServerMessage** in `fugue-protocol/src/types/`:
   ```rust
   impl ServerMessage {
       pub fn type_name(&self) -> &'static str {
           match self {
               ServerMessage::PaneResized { .. } => "PaneResized",
               ServerMessage::SessionList { .. } => "SessionList",
               ServerMessage::PaneContent { .. } => "PaneContent",
               // ... etc
           }
       }
   }
   ```

## Testing

1. **Unit test**: Verify drain clears messages and returns correct counts
2. **Integration test**:
   - Generate broadcast messages (resize pane, list sessions)
   - Call drain_messages
   - Verify subsequent MCP calls succeed without unexpected responses

## Acceptance Criteria

- [ ] `fugue_drain_messages` tool is available in MCP
- [ ] Returns count of drained messages
- [ ] Returns list of message types for diagnostics
- [ ] Clears channel without blocking indefinitely
- [ ] Subsequent MCP calls work correctly after drain

## Related

- **INQ-004**: MCP Response Integrity (root cause investigation)
- **BUG-064**: Drain pending messages after timeout
- **BUG-065**: Serialize MCP daemon requests

## Priority

P2 - Workaround for reliability issues, not blocking but improves UX significantly.
