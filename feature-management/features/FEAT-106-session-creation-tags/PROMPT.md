# FEAT-106: Session Creation Tags

**Priority**: P2
**Component**: mcp/tools
**Effort**: Small
**Status**: new

## Summary

Add optional `tags` parameter to `fugue_create_session` to set tags at creation time, eliminating the need for a separate `fugue_set_tags` call.

## Current State

Creating a tagged session requires two calls:

```json
// Step 1: Create session
{"tool": "fugue_create_session", "input": {"name": "feat-105-worker", "cwd": "/path"}}
// Step 2: Set tags
{"tool": "fugue_set_tags", "input": {"session": "feat-105-worker", "add": ["worker"]}}
```

## Proposed Change

Add optional `tags` parameter:

```json
{
  "tool": "fugue_create_session",
  "input": {
    "name": "feat-105-worker",
    "cwd": "/path",
    "tags": ["worker", "feat-105"]
  }
}
```

## Implementation

1. Update `fugue_create_session` tool schema in `fugue-server/src/mcp/tools.rs`
2. Update handler to apply tags after session creation
3. Return tags in response

## Acceptance Criteria

- [ ] `fugue_create_session` accepts optional `tags` array parameter
- [ ] Tags are applied atomically with session creation
- [ ] Response includes applied tags
- [ ] Existing calls without `tags` continue to work

## Related

- FEAT-048: Orchestration MCP tools (original tag implementation)
