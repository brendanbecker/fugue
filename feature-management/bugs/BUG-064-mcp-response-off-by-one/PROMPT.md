# BUG-064: MCP Response Off-By-One After Timeout

## Priority: P2

## Component: mcp/bridge

## Summary

MCP tool calls intermittently receive responses meant for previous requests, causing `UnexpectedResponse` errors. This manifests as:
- `fugue_read_pane` receiving `AllPanesList` (meant for `fugue_list_panes`)
- `fugue_list_panes` receiving `PaneContent` (meant for `fugue_read_pane`)
- `fugue_list_sessions` receiving `AllPanesList` (meant for `fugue_list_panes`)

## Root Cause

The MCP bridge uses an unbounded channel (`daemon_rx`) to receive responses from the daemon. When a request times out in `recv_filtered`, the response may still arrive later and remain in the channel. The next request then receives this **stale response** instead of its own.

### Sequence of Events

1. Request A sent to daemon
2. `recv_filtered` waits with 25s timeout
3. Timeout fires before response A arrives → returns `Err(ResponseTimeout)`
4. Response A arrives and sits in `daemon_rx` channel (stale)
5. Request B sent to daemon
6. `recv_filtered` pulls **stale response A** from channel
7. Response A type doesn't match what B expected → `UnexpectedResponse`

### Why This Happens

The protocol has **no request-response correlation by ID**. The code assumes:
- Requests are processed sequentially (they are, via the `while let` loop)
- Responses arrive in the same order as requests (usually true)
- Timeouts are rare (not true under load)

When timeouts occur, responses can arrive out of sync with the request that's currently waiting.

## Affected Files

- `fugue-server/src/mcp/bridge/connection.rs` - `recv_filtered`, `recv_response_from_daemon`
- `fugue-protocol/src/lib.rs` - `ClientMessage`, `ServerMessage` (no request IDs)

## Reproduction

1. Run fugue daemon with multiple active sessions (9+ agents)
2. Make rapid parallel MCP tool calls (`list_panes`, `read_pane`, `list_sessions`)
3. Under load, some requests will timeout
4. Subsequent requests receive wrong response types

## Proposed Fixes

### Option A: Drain Channel After Timeout (Quick Fix)

After a timeout in `recv_filtered`, drain all pending messages from the channel before returning the error. This prevents stale responses from affecting subsequent requests.

```rust
// In recv_filtered, after timeout:
while let Ok(stale) = self.daemon_rx.try_recv() {
    warn!("Draining stale message after timeout: {:?}", std::mem::discriminant(&stale));
}
return Err(McpError::ResponseTimeout { ... });
```

**Pros**: Simple, no protocol changes
**Cons**: Loses potentially valid responses, may cause cascading timeouts

### Option B: Request-Response Correlation (Proper Fix)

Add request IDs to the protocol:

1. Add `request_id: u64` to `ClientMessage` variants that expect responses
2. Add `request_id: u64` to corresponding `ServerMessage` variants
3. In `recv_filtered`, match on request ID instead of just message type

**Pros**: Correct solution, enables pipelining, better debugging
**Cons**: Protocol breaking change, more implementation work

### Option C: Increase Timeout + Retry (Workaround)

Increase `DAEMON_RESPONSE_TIMEOUT_SECS` and add retry logic. Under normal conditions, responses should always arrive before timeout.

**Pros**: No code changes to response handling
**Cons**: Doesn't fix the fundamental issue, longer waits on actual failures

## Recommended Approach

**Short-term**: Option A (drain channel) to stop the bleeding
**Long-term**: Option B (request IDs) for proper correlation

## Acceptance Criteria

- [ ] MCP tool calls no longer receive wrong response types
- [ ] Parallel tool calls work reliably under load
- [ ] Add test for timeout + stale response scenario
- [ ] Document the fix in HANDOFF.md

## Related

- BUG-037: Previous timeout-related fixes
- BUG-043: Sequenced message unwrapping
