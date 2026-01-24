# BUG-042: Excessive Result Nesting (Ok(Ok(...))) in MCP Handlers

## Overview
During the FEAT-064 refactor and subsequent merges, we observed pattern matching on `Ok(Ok(ServerMessage::Error { ... }))` in `fugue-server/src/mcp/bridge/handlers.rs`. This indicates that `recv_response_from_daemon` or the handler logic is double-wrapping results, forcing verbose and brittle matching.

## Impact
- **Maintainability**: Harder to read and modify handler logic.
- **Error Handling**: Increases the chance of missing an error case if one layer of `Result` is handled but the inner one isn't.
- **Code Cleanliness**: Typical Rust idiom prefers flattening `Result` chains using `?` or mapping errors early.

## Analysis
The likely cause is the interaction between `tokio::time::timeout` (which returns a `Result`) and the underlying `recv_from_daemon` (which also returns a `Result`).

Current flow (hypothetical):
1. `recv_from_daemon()` -> `Result<ServerMessage, McpError>`
2. `tokio::time::timeout(...)` -> `Result<Result<ServerMessage, McpError>, Elapsed>`

This leads to `Ok(Ok(msg))` matching.

## Proposed Fix
1. **Flatten in Helper**: The helper method `recv_response_from_daemon` should handle the timeout `Result` internally and return a single `Result<ServerMessage, McpError>`.
   - If timeout: map `Elapsed` to `McpError::ResponseTimeout`.
   - If success: return the inner `Result`.
2. **Refactor Match Arms**: Update `handlers.rs` to match on `Ok(ServerMessage::...)` or `Err(McpError::...)`.

## Tasks
- [x] Analyze `recv_response_from_daemon` signature and implementation in `connection.rs`.
- [x] Refactor to flatten the return type.
- [x] Update all call sites in `handlers.rs` to remove the double `Ok`.

## Resolution

The fix was already implemented in `connection.rs`:

1. **`recv_from_daemon_with_timeout()`** (lines 337-347) properly flattens the Result:
   ```rust
   match tokio::time::timeout(timeout, self.recv_from_daemon()).await {
       Ok(result) => result,  // Returns inner Result directly (flattened!)
       Err(_) => Err(McpError::ResponseTimeout { seconds: timeout.as_secs() }),
   }
   ```

2. **`recv_filtered()`** uses `recv_from_daemon_with_timeout()` internally and returns `Result<ServerMessage, McpError>`.

3. **`recv_response_from_daemon()`** calls `recv_filtered()` and returns `Result<ServerMessage, McpError>`.

4. **All handlers** in `handlers.rs` now use the clean pattern:
   ```rust
   match self.connection.recv_response_from_daemon().await? {
       ServerMessage::SessionList { sessions } => { ... }
       ServerMessage::Error { code, message, .. } => { ... }
       msg => Err(McpError::UnexpectedResponse(...)),
   }
   ```

No `Ok(Ok(...))` patterns exist in the codebase. The fix ensures:
- Timeout errors are mapped to `McpError::ResponseTimeout`
- Inner `Result` is returned directly, flattening the double-wrapped Result
- Clean, idiomatic Rust error handling throughout handlers
