# BUG-073: fugue_get_tags returns wrong session's tags

**Priority**: P1
**Component**: mcp, server
**Severity**: medium
**Status**: new

## Problem

When an agent calls `fugue_get_tags({})` without specifying a session ID, fugue returns the tags of the **attached session** (set via `fugue_attach_session`) instead of the calling session's tags. This causes agents to misidentify their role and behave incorrectly.

## Background

The `fugue_get_tags` function accepts an optional `session` parameter. When omitted, it should return the tags of the calling session. However, the implementation appears to fall back to the "attached" session (used for orchestration message routing) rather than inferring the caller's session from MCP request context.

This is particularly problematic in orchestration scenarios where:
1. An orchestrator session attaches via `fugue_attach_session` to coordinate workers
2. Worker sessions call `fugue_get_tags({})` to check their role
3. Workers receive the orchestrator's tags instead of their own

## Reproduction Steps

1. Create an orchestrator session with tags `["orchestrator", "project-x"]`
2. Have orchestrator call `fugue_attach_session` to attach to its session
3. Create a worker session with tags `["worker", "project-x"]`
4. Have worker call `fugue_get_tags({})` (no session parameter)
5. Worker receives `{"session_id": "<orchestrator-id>", "tags": ["orchestrator", "project-x"]}`

## Expected Behavior

`fugue_get_tags({})` should return the tags of the **calling session**:
- If no session parameter provided, infer from MCP request context
- If session context cannot be determined, return an error rather than wrong data

## Actual Behavior

`fugue_get_tags({})` returns tags of the **attached session**:
- Worker in session `codex-f33-index` called `fugue_get_tags({})`
- Received: `{"session_id": "a85418f4...", "session_name": "orch-perf-takehome", "tags": ["perf-takehome", "orchestrator"]}`
- These are the orchestrator's tags, not the worker's

## Impact

- **Role misidentification**: Worker sees "orchestrator" tag and attempts to delegate work instead of implementing
- **Delegation loops**: Worker spawns new workers instead of doing the task
- **Wasted resources**: Unnecessary sessions created, work not completed
- **Confusing debugging**: Agent behavior doesn't match expected role

## Root Cause Hypothesis

The `fugue_get_tags` handler likely has this logic:

```rust
let session = params.session
    .or_else(|| self.attached_session.clone())  // <-- Wrong fallback
    .ok_or("No session specified")?;
```

When `params.session` is None, it falls back to `attached_session` rather than determining the caller's session from the MCP connection context.

## Proposed Solution

Two possible approaches:

### Option A: Require explicit session parameter
- Remove the fallback behavior entirely
- Return error if `session` parameter is not provided
- Agents must always specify which session's tags they want

### Option B: Infer caller's session from context
- Track which session each MCP connection belongs to
- When `session` is omitted, use the connection's associated session
- This requires MCP context to carry session identity

Option A is simpler but requires agents to know their own session ID. Option B is more ergonomic but requires architectural changes to pass session context through MCP calls.

## Workaround

Manually instruct agents to ignore `fugue_get_tags` results and rely on other signals for role detection (e.g., presence of specific environment variables or explicit role assignment in the initial prompt).

## Investigation Steps

- [ ] Review `fugue_get_tags` implementation in `handlers.rs`
- [ ] Identify how "attached session" is used and why it's the fallback
- [ ] Determine if MCP context can carry caller session identity
- [ ] Evaluate Option A vs Option B for fix approach
- [ ] Implement chosen solution

## Acceptance Criteria

- [ ] `fugue_get_tags({})` does NOT return a different session's tags
- [ ] Either: returns caller's session tags OR errors when session not specified
- [ ] Agents can reliably determine their own role via tags
- [ ] No regression in orchestration message routing (attached session still works for that)
- [ ] Add test case for this scenario

## Related Files

- `fugue-server/src/mcp/bridge/handlers.rs` - get_tags handler
- `fugue-server/src/mcp/bridge/mod.rs` - MCP bridge state (attached_session)
- `fugue-protocol/src/messages.rs` - Tag-related message types

## Related Issues

- FEAT-106: Session creation tags (added tag support)
