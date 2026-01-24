# FEAT-096: fugue_expect - Pattern-Based Wait

**Priority**: P1
**Component**: fugue-server/mcp
**Type**: new_feature
**Estimated Effort**: small
**Business Value**: high

## Overview

Create a high-level MCP tool `fugue_expect` that blocks until a regex pattern appears in pane output or timeout occurs. Supports configurable actions on match: `notify`, `close_pane`, or `return_output`. This is the foundational primitive for FEAT-094 and FEAT-095.

## Problem Statement

Currently, waiting for specific output requires:
1. Call `read_pane`
2. Check for pattern
3. If not found, wait and repeat
4. Eventually timeout or find pattern

Each poll cycle: ~100 tokens. 10 polls = 1000 tokens.

With `fugue_expect`: ~100 tokens (single call).

## API Design

### Tool Schema

```json
{
  "name": "fugue_expect",
  "description": "Wait for regex pattern in pane output",
  "inputSchema": {
    "type": "object",
    "properties": {
      "pane_id": {
        "type": "string",
        "description": "UUID of the pane to monitor"
      },
      "pattern": {
        "type": "string",
        "description": "Regex pattern to match"
      },
      "timeout_ms": {
        "type": "integer",
        "default": 60000,
        "description": "Timeout in milliseconds (default 1 minute)"
      },
      "action": {
        "type": "string",
        "enum": ["notify", "close_pane", "return_output"],
        "default": "notify",
        "description": "Action to take on pattern match"
      },
      "poll_interval_ms": {
        "type": "integer",
        "default": 200,
        "description": "Polling interval (default 200ms)"
      },
      "lines": {
        "type": "integer",
        "default": 100,
        "description": "Number of lines to check from end of output"
      }
    },
    "required": ["pane_id", "pattern"]
  }
}
```

### Response Format

```json
{
  "status": "matched|timeout",
  "pattern": "___FUGUE_EXIT_0___",
  "match": "___FUGUE_EXIT_0___",
  "line": "{ npm test ; } ; echo \"___FUGUE_EXIT_0___\"",
  "duration_ms": 5234,
  "output": "... (if action=return_output)"
}
```

## Architecture

### Bridge-Only Implementation

Uses existing primitives:
- `read_pane` - fetch recent output
- `close_pane` - close pane (if action=close_pane)

### Polling Loop

```rust
let start = Instant::now();
loop {
    let output = read_pane(pane_id, lines)?;
    if let Some(m) = regex.find(&output) {
        // Pattern found
        match action {
            Notify => return success,
            ClosPane => { close_pane(pane_id); return success },
            ReturnOutput => return success with output,
        }
    }
    if start.elapsed() > timeout {
        return timeout_error;
    }
    tokio::time::sleep(poll_interval).await;
}
```

### Regex Handling

- Use `regex` crate for pattern matching
- Compile regex once at start
- Search entire output buffer (last N lines)
- Return first match found

## Implementation Tasks

### Section 1: Create Expect Handler

- [ ] Add `ExpectRequest` and `ExpectResponse` types to orchestration module
- [ ] Implement in `fugue-server/src/mcp/bridge/orchestration.rs`
- [ ] Add regex dependency if not present

### Section 2: Polling Loop

- [ ] Implement async polling loop
- [ ] Use `tokio::time::sleep` for intervals
- [ ] Track elapsed time for timeout
- [ ] Handle cancellation gracefully

### Section 3: Pattern Matching

- [ ] Compile regex from input pattern
- [ ] Handle invalid regex (return error)
- [ ] Search output buffer
- [ ] Extract match and surrounding line

### Section 4: Action Handling

- [ ] Implement `notify` action (just return)
- [ ] Implement `close_pane` action (close then return)
- [ ] Implement `return_output` action (include full output in response)

### Section 5: Tool Registration

- [ ] Add tool schema to `fugue-server/src/mcp/tools.rs`
- [ ] Register handler in `fugue-server/src/mcp/bridge/handlers.rs`

### Section 6: Testing

- [ ] Unit test: regex compilation and matching
- [ ] Integration test: wait for echo output
- [ ] Integration test: timeout behavior
- [ ] Integration test: close_pane action
- [ ] Integration test: return_output action
- [ ] Integration test: invalid regex handling

## Files to Modify

| File | Changes |
|------|---------|
| `fugue-server/src/mcp/bridge/orchestration.rs` | Add expect implementation |
| `fugue-server/src/mcp/bridge/handlers.rs` | Register handler |
| `fugue-server/src/mcp/tools.rs` | Add tool schema |
| `fugue-server/Cargo.toml` | Add `regex` dependency (if needed) |

## Acceptance Criteria

- [ ] `fugue_expect` tool available in MCP
- [ ] Blocks until pattern matches or timeout
- [ ] `notify` action returns on match
- [ ] `close_pane` action closes pane then returns
- [ ] `return_output` action includes full output
- [ ] Invalid regex returns clear error
- [ ] Configurable poll interval
- [ ] Returns match details (line, match text)
- [ ] No protocol changes required

## Dependencies

- Existing MCP infrastructure
- `regex` crate

## Notes

### Context Savings

Manual polling loop (10 iterations): ~1000 tokens
With `fugue_expect`: ~100 tokens

**Savings: 90% context reduction**

### Foundation for Other Tools

This tool provides the completion detection primitive for:
- FEAT-094 (`fugue_run_parallel`) - detect command completion
- FEAT-095 (`fugue_run_pipeline`) - wait for each step

Internal implementation may reuse this logic directly.

### Common Patterns

1. **Wait for command completion**:
   ```json
   {
     "pane_id": "...",
     "pattern": "___FUGUE_EXIT_\\d+___",
     "timeout_ms": 300000
   }
   ```

2. **Wait for server startup**:
   ```json
   {
     "pane_id": "...",
     "pattern": "Server listening on port \\d+",
     "action": "notify"
   }
   ```

3. **Wait and capture output**:
   ```json
   {
     "pane_id": "...",
     "pattern": "Build completed",
     "action": "return_output"
   }
   ```

4. **Wait and cleanup**:
   ```json
   {
     "pane_id": "...",
     "pattern": "Done\\.",
     "action": "close_pane"
   }
   ```

### Error Handling

- Invalid pane_id: Return error immediately
- Invalid regex: Return error with regex compilation message
- Timeout: Return `status: "timeout"` with elapsed time
- Pane closed externally: Return error

### Performance Considerations

- Default 200ms poll interval balances responsiveness vs overhead
- `lines` parameter limits search scope (default 100 lines)
- Consider increasing poll interval for long-running commands
- Regex is compiled once, not per poll
