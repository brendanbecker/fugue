# FEAT-094: fugue_run_parallel - Parallel Command Execution

**Priority**: P1
**Component**: fugue-server/mcp
**Type**: new_feature
**Estimated Effort**: medium
**Business Value**: high

## Overview

Create a high-level MCP tool `fugue_run_parallel` that executes multiple commands in parallel across separate panes and returns aggregated results. This is a bridge-only implementation requiring no protocol changes.

## Problem Statement

Currently, orchestrators must manually:
1. Create multiple panes
2. Send commands to each pane
3. Poll each pane for completion
4. Aggregate results manually
5. Clean up panes

This consumes significant context (~500-1000 tokens per parallel task managed). A single high-level tool call can replace this entire workflow.

## API Design

### Tool Schema

```json
{
  "name": "fugue_run_parallel",
  "description": "Execute commands in parallel across separate panes",
  "inputSchema": {
    "type": "object",
    "properties": {
      "commands": {
        "type": "array",
        "items": {
          "type": "object",
          "properties": {
            "command": { "type": "string", "description": "Command to execute" },
            "cwd": { "type": "string", "description": "Working directory (optional)" },
            "name": { "type": "string", "description": "Task name for identification" }
          },
          "required": ["command"]
        },
        "maxItems": 10,
        "description": "Commands to execute (max 10)"
      },
      "layout": {
        "type": "string",
        "enum": ["tiled", "hidden"],
        "default": "hidden",
        "description": "tiled=visible splits, hidden=__orchestration__ session"
      },
      "timeout_ms": {
        "type": "integer",
        "default": 300000,
        "description": "Timeout in milliseconds (default 5 minutes)"
      },
      "cleanup": {
        "type": "boolean",
        "default": true,
        "description": "Close panes after completion"
      }
    },
    "required": ["commands"]
  }
}
```

### Response Format

```json
{
  "status": "completed|timeout|partial",
  "results": [
    {
      "name": "task-1",
      "command": "npm test",
      "exit_code": 0,
      "pane_id": "uuid",
      "duration_ms": 5234
    }
  ],
  "total_duration_ms": 8432
}
```

## Architecture

### Bridge-Only Implementation

All logic resides in MCP bridge handlers using existing daemon primitives:
- `create_pane` - spawn panes for each command
- `send_input` - execute commands
- `read_pane` - poll for output (completion detection)
- `get_status` - check pane state
- `close_pane` - cleanup

### Completion Detection

Use shell prompt pattern matching or command wrapper:
```bash
# Wrapper approach
{ <command> ; } ; echo "___FUGUE_EXIT_$?___"
```

Poll `read_pane` output for `___FUGUE_EXIT_<code>___` pattern.

### Layout Modes

1. **hidden** (default): Create panes in dedicated `__orchestration__` session
   - Not visible to user
   - Automatic cleanup
   - Preferred for background work

2. **tiled**: Create visible splits in current session
   - User can observe progress
   - Useful for debugging
   - May require resize handling

## Implementation Tasks

### Section 1: Create Orchestration Module

- [ ] Create `fugue-server/src/mcp/bridge/orchestration.rs`
- [ ] Add module to `fugue-server/src/mcp/bridge/mod.rs`
- [ ] Define `RunParallelRequest` and `RunParallelResponse` types
- [ ] Implement command validation (max 10, required fields)

### Section 2: Pane Spawning Logic

- [ ] Implement layout mode handling (hidden vs tiled)
- [ ] Create or find `__orchestration__` session for hidden mode
- [ ] Spawn panes concurrently using `tokio::spawn`
- [ ] Track pane IDs and task names

### Section 3: Command Execution

- [ ] Wrap commands with exit code marker
- [ ] Send commands via `send_input`
- [ ] Handle `cwd` by prepending `cd` command

### Section 4: Completion Polling

- [ ] Implement polling loop with 200ms interval
- [ ] Parse exit codes from output markers
- [ ] Track completion status per task
- [ ] Handle timeout (aggregate partial results)

### Section 5: Result Aggregation

- [ ] Collect exit codes and durations
- [ ] Determine overall status (completed/timeout/partial)
- [ ] Format response JSON
- [ ] Cleanup panes if `cleanup: true`

### Section 6: Tool Registration

- [ ] Add tool schema to `fugue-server/src/mcp/tools.rs`
- [ ] Register handler in `fugue-server/src/mcp/bridge/handlers.rs`
- [ ] Add to `available_tools` list

### Section 7: Testing

- [ ] Unit tests for command validation
- [ ] Integration test: parallel echo commands
- [ ] Integration test: timeout handling
- [ ] Integration test: mixed success/failure
- [ ] Integration test: hidden vs tiled layout

## Files to Modify

| File | Changes |
|------|---------|
| `fugue-server/src/mcp/bridge/orchestration.rs` | **NEW** - Core implementation |
| `fugue-server/src/mcp/bridge/mod.rs` | Add orchestration module |
| `fugue-server/src/mcp/bridge/handlers.rs` | Register handler |
| `fugue-server/src/mcp/tools.rs` | Add tool schema |

## Acceptance Criteria

- [ ] `fugue_run_parallel` tool available in MCP
- [ ] Executes up to 10 commands in parallel
- [ ] Returns aggregated results with exit codes
- [ ] `hidden` layout uses `__orchestration__` session
- [ ] `tiled` layout creates visible splits
- [ ] Timeout handling returns partial results
- [ ] Cleanup removes panes when enabled
- [ ] No protocol changes required

## Dependencies

- Existing MCP infrastructure
- Existing pane management primitives

## Notes

### Context Savings

Typical parallel workflow without this tool: ~800-1200 tokens
With `fugue_run_parallel`: ~200 tokens (single tool call + response)

**Savings: 70-80% context reduction**

### Error Handling

- Invalid commands: Return error before spawning
- Pane spawn failure: Include in results with error status
- Partial timeout: Return completed results + timed-out tasks

### Future Enhancements

- Streaming progress updates
- Dependency ordering between tasks
- Resource limits per task
