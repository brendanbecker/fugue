# FEAT-119: Auto-propagate CLAUDE_CODE_TASK_LIST_ID on session creation

**Priority**: P2
**Component**: fugue-server/mcp
**Type**: enhancement
**Estimated Effort**: small
**Business Value**: high

## Overview

Add a `task_list_id` parameter to `fugue_create_session` that automatically sets the `CLAUDE_CODE_TASK_LIST_ID` environment variable. This enables all Claude instances spawned in that session to share the same task graph.

## Problem Statement

Claude Code's new task system stores tasks in `~/.claude/tasks/<list-id>/`. By default, each session gets a unique list-id (session UUID), so tasks don't persist across sessions or share between agents.

When orchestrating multiple Claude workers via fugue:
1. Each worker has its own isolated task list
2. Orchestrators can't see worker task progress
3. Task dependencies can't span workers
4. No shared source of truth for complex multi-agent workflows

Setting `CLAUDE_CODE_TASK_LIST_ID` env var causes Claude to use a shared task list, but this must be done manually for each spawned session.

## Solution

Add `task_list_id` parameter to `fugue_create_session`. When provided, automatically set `CLAUDE_CODE_TASK_LIST_ID` in the session environment.

## API Design

### Updated Tool Schema

```json
{
  "name": "fugue_create_session",
  "inputSchema": {
    "type": "object",
    "properties": {
      "name": {
        "type": "string",
        "description": "Optional name for the session"
      },
      "cwd": {
        "type": "string",
        "description": "Working directory for the session"
      },
      "command": {
        "type": "string",
        "description": "Command to run in the default pane"
      },
      "tags": {
        "type": "array",
        "items": { "type": "string" },
        "description": "Optional tags for routing"
      },
      "task_list_id": {
        "type": "string",
        "description": "Claude Code task list ID. Sets CLAUDE_CODE_TASK_LIST_ID env var so all Claude instances in this session share the same task graph."
      }
    }
  }
}
```

### Usage Example

```json
{
  "tool": "fugue_create_session",
  "input": {
    "name": "worker-1",
    "cwd": "/home/user/project",
    "command": "claude --dangerously-skip-permissions",
    "tags": ["worker", "feat-123"],
    "task_list_id": "feat-123-tasks"
  }
}
```

All Claude instances spawned in this session will read/write tasks to `~/.claude/tasks/feat-123-tasks/`.

## Implementation

### Where to Implement

**File**: `fugue-server/src/mcp/bridge/handlers.rs` or the session creation handler

In the `tool_create_session` function:
1. Extract `task_list_id` from arguments (optional)
2. If provided, add to the environment map before creating session
3. Session's panes inherit this env var automatically

### Code Changes

```rust
// In session creation handler
if let Some(task_list_id) = arguments.get("task_list_id").and_then(|v| v.as_str()) {
    // Set env var that Claude Code looks for
    session_env.insert(
        "CLAUDE_CODE_TASK_LIST_ID".to_string(),
        task_list_id.to_string()
    );
}
```

## Implementation Tasks

### Section 1: Update Tool Schema

- [ ] Edit `fugue-server/src/mcp/tools.rs`
- [ ] Add `task_list_id` property to `fugue_create_session` schema
- [ ] Add description explaining its purpose

### Section 2: Update Handler

- [ ] Edit `fugue-server/src/mcp/bridge/handlers.rs`
- [ ] Extract `task_list_id` from request arguments
- [ ] If present, inject into session environment
- [ ] Verify env var propagates to panes

### Section 3: Testing

- [ ] Manual test: create session with `task_list_id`
- [ ] Verify `echo $CLAUDE_CODE_TASK_LIST_ID` shows correct value
- [ ] Verify Claude uses shared task list
- [ ] Test that omitting parameter works (backwards compatible)

## Files to Modify

| File | Changes |
|------|---------|
| `fugue-server/src/mcp/tools.rs` | Add `task_list_id` to schema |
| `fugue-server/src/mcp/bridge/handlers.rs` | Inject env var on session creation |

## Acceptance Criteria

- [ ] `task_list_id` parameter available on `fugue_create_session`
- [ ] Parameter is optional (backwards compatible)
- [ ] When provided, `CLAUDE_CODE_TASK_LIST_ID` env var is set in session
- [ ] New panes in session inherit the env var
- [ ] Claude instances in session share the same task list

## Future Enhancements

- Store `task_list_id` in session metadata for querying
- Add to `fugue_create_pane` for per-pane override
- Integration with FEAT-120 (`fugue_tasks_read`) for direct task file access

## References

- Claude Code Task System: Tasks stored in `~/.claude/tasks/<list-id>/`
- Related: FEAT-120 (fugue_tasks_read tool)
