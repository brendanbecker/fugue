# FEAT-120: ccmux_tasks_read - Read Claude Code task files

**Priority**: P2
**Component**: ccmux-server/mcp
**Type**: feature
**Estimated Effort**: medium
**Business Value**: high

## Overview

Add a new MCP tool `ccmux_tasks_read` that directly reads Claude Code task files from `~/.claude/tasks/<list-id>/`. This enables orchestrators to monitor task progress without asking Claude instances.

## Problem Statement

Claude Code's task system provides excellent dependency-aware task tracking, but visibility is limited:

1. **No external access**: Only the Claude instance can see its tasks via `TaskList`
2. **Polling requires interaction**: Orchestrators must send messages to check progress
3. **No aggregation**: Can't see tasks across multiple workers
4. **Blocked detection**: Hard to know when a task is blocked without asking

Since tasks are stored as simple JSON files, ccmux can read them directly and surface this information to orchestrators.

## Solution

Create an MCP tool that reads the task JSON files directly, providing:
- Full task graph visibility
- Dependency information
- Status filtering
- No need to interrupt Claude instances

## API Design

### Tool Schema

```json
{
  "name": "ccmux_tasks_read",
  "description": "Read Claude Code task files directly. Returns task graph with dependencies and status.",
  "inputSchema": {
    "type": "object",
    "properties": {
      "list_id": {
        "type": "string",
        "description": "Task list ID. Can be a custom ID or 'session:<uuid>' format."
      },
      "status": {
        "type": "string",
        "enum": ["all", "pending", "in_progress", "completed", "blocked"],
        "default": "all",
        "description": "Filter tasks by status. 'blocked' returns pending tasks with unresolved blockedBy."
      },
      "include_description": {
        "type": "boolean",
        "default": false,
        "description": "Include full task descriptions (can be verbose)."
      }
    },
    "required": ["list_id"]
  }
}
```

### Response Format

```json
{
  "list_id": "feat-123-tasks",
  "task_count": 5,
  "summary": {
    "pending": 2,
    "in_progress": 1,
    "completed": 2,
    "blocked": 1
  },
  "tasks": [
    {
      "id": "1",
      "subject": "Set up database connection",
      "status": "completed",
      "owner": "backend-dev",
      "blocks": ["2", "3"],
      "blockedBy": []
    },
    {
      "id": "2",
      "subject": "Create user model",
      "status": "in_progress",
      "owner": "backend-dev",
      "blocks": ["4"],
      "blockedBy": ["1"]
    },
    {
      "id": "3",
      "subject": "Set up auth middleware",
      "status": "blocked",
      "owner": null,
      "blocks": ["4"],
      "blockedBy": ["1"],
      "blocked_reason": "Waiting on task #1"
    }
  ]
}
```

## Implementation

### New Module: `claude_tasks.rs`

Create a new module similar to `beads.rs` for Claude Code task integration:

```rust
// ccmux-server/src/claude_tasks.rs

use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};

/// Task status from Claude Code
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Pending,
    InProgress,
    Completed,
}

/// A Claude Code task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeTask {
    pub id: String,
    pub subject: String,
    pub description: Option<String>,
    #[serde(rename = "activeForm")]
    pub active_form: Option<String>,
    pub owner: Option<String>,
    pub status: TaskStatus,
    #[serde(default)]
    pub blocks: Vec<String>,
    #[serde(default)]
    pub blocked_by: Vec<String>,
    #[serde(default)]
    pub metadata: serde_json::Value,
}

/// Get the tasks directory for a given list ID
pub fn tasks_dir(list_id: &str) -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".claude")
        .join("tasks")
        .join(list_id)
}

/// Read all tasks from a task list
pub fn read_task_list(list_id: &str) -> Result<Vec<ClaudeTask>, TaskError> {
    let dir = tasks_dir(list_id);
    if !dir.exists() {
        return Err(TaskError::ListNotFound(list_id.to_string()));
    }

    let mut tasks = Vec::new();
    for entry in std::fs::read_dir(&dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().map(|e| e == "json").unwrap_or(false) {
            let content = std::fs::read_to_string(&path)?;
            let task: ClaudeTask = serde_json::from_str(&content)?;
            tasks.push(task);
        }
    }

    // Sort by ID (numeric)
    tasks.sort_by(|a, b| {
        a.id.parse::<u32>().unwrap_or(0)
            .cmp(&b.id.parse::<u32>().unwrap_or(0))
    });

    Ok(tasks)
}

/// Check if a task is blocked (has unresolved blockedBy)
pub fn is_blocked(task: &ClaudeTask, all_tasks: &[ClaudeTask]) -> bool {
    if task.blocked_by.is_empty() {
        return false;
    }

    // Check if any blocking task is not completed
    task.blocked_by.iter().any(|blocker_id| {
        all_tasks.iter()
            .find(|t| t.id == *blocker_id)
            .map(|t| t.status != TaskStatus::Completed)
            .unwrap_or(true) // If blocker not found, consider blocked
    })
}
```

### MCP Handler

Add handler in `ccmux-server/src/mcp/bridge/handlers.rs`:

```rust
pub async fn tool_tasks_read(&mut self, arguments: &serde_json::Value) -> Result<ToolResult, McpError> {
    let list_id = arguments["list_id"]
        .as_str()
        .ok_or_else(|| McpError::InvalidParams("Missing 'list_id'".into()))?;

    let status_filter = arguments.get("status")
        .and_then(|v| v.as_str())
        .unwrap_or("all");

    let include_description = arguments.get("include_description")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let tasks = claude_tasks::read_task_list(list_id)
        .map_err(|e| McpError::Internal(e.to_string()))?;

    // Filter and format response...
}
```

## Implementation Tasks

### Section 1: Create claude_tasks module

- [ ] Create `ccmux-server/src/claude_tasks.rs`
- [ ] Implement `ClaudeTask` struct matching Claude Code's schema
- [ ] Implement `tasks_dir()` function
- [ ] Implement `read_task_list()` function
- [ ] Implement `is_blocked()` helper
- [ ] Add `pub mod claude_tasks;` to `lib.rs`

### Section 2: Add MCP Tool Schema

- [ ] Edit `ccmux-server/src/mcp/tools.rs`
- [ ] Add `ccmux_tasks_read` tool definition
- [ ] Document parameters and response format

### Section 3: Implement Handler

- [ ] Edit `ccmux-server/src/mcp/bridge/handlers.rs`
- [ ] Add `tool_tasks_read` function
- [ ] Wire up in tool dispatch
- [ ] Handle status filtering
- [ ] Handle `include_description` flag

### Section 4: Testing

- [ ] Unit tests for `read_task_list()`
- [ ] Unit tests for `is_blocked()`
- [ ] Integration test with mock task files
- [ ] Manual test with real Claude Code task list

## Files to Modify/Create

| File | Changes |
|------|---------|
| `ccmux-server/src/claude_tasks.rs` | **New** - Claude task reading module |
| `ccmux-server/src/lib.rs` | Add `pub mod claude_tasks` |
| `ccmux-server/src/mcp/tools.rs` | Add tool schema |
| `ccmux-server/src/mcp/bridge/handlers.rs` | Add handler |
| `ccmux-server/src/mcp/bridge/mod.rs` | Wire up handler dispatch |

## Acceptance Criteria

- [ ] `ccmux_tasks_read` tool available via MCP
- [ ] Can read task list by ID
- [ ] Returns task graph with dependencies
- [ ] Status filtering works (pending, in_progress, completed, blocked)
- [ ] "blocked" filter correctly identifies tasks with incomplete dependencies
- [ ] Graceful error when list doesn't exist
- [ ] Description included only when requested

## Session Metadata Integration

Store current task list ID in session metadata for easy discovery:

```rust
pub mod task_keys {
    /// The Claude Code task list ID for this session
    pub const TASK_LIST_ID: &str = "claude.task_list_id";
}
```

When `ccmux_create_session` is called with `task_list_id` (FEAT-119), also store it in session metadata. Then `ccmux_tasks_read` could auto-discover from session context.

## Future Enhancements

- **FEAT-121**: Task file watching with orchestration events
  - Use inotify/fswatch to monitor task directory
  - Emit `task.completed`, `task.started` orchestration messages
  - Enable reactive orchestration without polling

- **Task aggregation**: Read tasks across multiple sessions
- **Task modification**: Write tools for orchestrator-driven task management

## References

- Claude Code Task System: `~/.claude/tasks/<list-id>/<task-id>.json`
- Related: FEAT-119 (task_list_id propagation)
- Similar pattern: `ccmux-server/src/beads.rs`
