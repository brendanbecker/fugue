# BUG-065: Parallel MCP requests cause response mismatches

**Priority**: P1
**Component**: mcp/bridge
**Severity**: high
**Status**: fixed

## Problem

When an MCP client sends multiple requests in parallel (e.g., Claude Code making 4 tool calls in one message), responses are delivered to the wrong callers. Each request receives a response intended for a different request.

This is distinct from BUG-064 (post-timeout stale responses), which was fixed by draining the channel after timeout. This bug occurs even without timeouts, during normal parallel request handling.

## Reproduction Steps

1. Connect to fugue daemon via MCP
2. Send 4 parallel tool calls in one message:
   - `fugue_list_sessions`
   - `fugue_list_panes`
   - `fugue_read_pane`
   - `fugue_get_status`
3. Observe: All 4 calls fail with "Unexpected response" errors

```
list_sessions received AllPanesList (meant for list_panes)
list_panes received SessionList (meant for list_sessions)
read_pane received AllPanesList (duplicate?)
get_status received PaneContent (meant for read_pane)
```

## Expected Behavior

Each MCP request receives its corresponding response.

## Actual Behavior

Responses are delivered to wrong callers. The response types are correct (all expected responses arrive), but they're mismatched with their requests.

## Root Cause

The MCP JSON-RPC protocol uses request IDs for correlation, but the internal daemon protocol (`ClientMessage`/`ServerMessage`) has **no request IDs**. When multiple requests are processed concurrently:

1. MCP bridge receives requests with IDs 1, 2, 3, 4
2. Bridge forwards to daemon as separate messages (no correlation)
3. Daemon processes and responds
4. Responses arrive in channel (potentially out of order)
5. Bridge matches response to waiting caller by **type only**
6. If responses arrive in different order than requests, mismatches occur

## Architecture Impact

This fundamentally affects how Claude Code uses fugue. Claude Code frequently makes parallel tool calls when gathering information (e.g., calling `list_sessions`, `list_panes`, and `read_pane` simultaneously).

Current workaround: Make all MCP calls sequentially. This works but is slower and doesn't match natural Claude Code behavior.

## Proposed Fixes

### Option A: Add Request IDs to Protocol (Proper Fix)

Add `request_id: u64` to `ClientMessage` variants and corresponding `ServerMessage` variants:

```rust
// In ClientMessage
ListSessions { request_id: u64 },
ListAllPanes { request_id: u64 },
// etc.

// In ServerMessage
SessionList { request_id: u64, sessions: Vec<SessionInfo> },
AllPanesList { request_id: u64, panes: Vec<PaneListEntry> },
// etc.
```

The MCP bridge would:
1. Generate a unique request_id for each daemon request
2. Include request_id in the ClientMessage
3. Match response by request_id instead of type

**Pros**: Correct solution, enables true pipelining, better debugging
**Cons**: Protocol breaking change, requires updating all message variants

### Option B: Serialize MCP Requests (Quick Fix)

Add a mutex/semaphore to the MCP bridge that ensures only one daemon request is in-flight at a time:

```rust
let _guard = self.request_lock.lock().await;
// Now safe to send request and wait for response
```

**Pros**: Simple, no protocol changes
**Cons**: Serializes all requests (slower), doesn't scale with multiple MCP clients

### Option C: Per-Request Channel (Medium Fix)

Create a per-request oneshot channel for each daemon request:

```rust
let (tx, rx) = oneshot::channel();
self.pending_requests.insert(discriminant(&message), tx);
self.daemon_tx.send(message)?;
let response = rx.await?;
```

**Pros**: No protocol changes, maintains parallelism
**Cons**: Complex matching logic, may have issues with broadcast messages

## Recommended Approach

**Short-term**: Option B (serialize requests) to immediately fix the issue
**Long-term**: Option A (request IDs) for proper correlation

## Acceptance Criteria

- [ ] Parallel MCP tool calls return correct responses
- [ ] No "Unexpected response" errors during normal operation
- [ ] Performance impact documented (if serializing)
- [ ] Test case for parallel requests

## Related

- BUG-064: Post-timeout response correlation (drain fix)
- BUG-037: Previous timeout-related fixes
- BUG-043: Sequenced message unwrapping

## Discovery Context

Found during QA of BUG-061/062/063/064 fixes in Session 14. Triggered by making 4 parallel MCP calls to test response handling.
