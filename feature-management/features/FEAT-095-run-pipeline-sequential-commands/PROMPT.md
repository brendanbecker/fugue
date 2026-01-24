# FEAT-095: fugue_run_pipeline - Sequential Command Pipeline

**Priority**: P1
**Component**: fugue-server/mcp
**Type**: new_feature
**Estimated Effort**: medium
**Business Value**: high

## Overview

Create a high-level MCP tool `fugue_run_pipeline` that executes commands sequentially within a single pane. Supports `stop_on_error` mode for immediate failure on non-zero exit. Returns structured results per step. Bridge-only implementation.

## Problem Statement

Sequential workflows (build -> test -> deploy) require:
1. Create pane
2. Send command 1, wait for completion
3. Check exit code
4. Send command 2, wait for completion
5. Repeat for each step
6. Cleanup

Each step consumes ~100-200 tokens. A 5-step pipeline: 500-1000 tokens.

With `fugue_run_pipeline`: ~150 tokens total.

## API Design

### Tool Schema

```json
{
  "name": "fugue_run_pipeline",
  "description": "Execute commands sequentially in a single pane",
  "inputSchema": {
    "type": "object",
    "properties": {
      "commands": {
        "type": "array",
        "items": {
          "type": "object",
          "properties": {
            "command": { "type": "string", "description": "Command to execute" },
            "name": { "type": "string", "description": "Step name for identification" }
          },
          "required": ["command"]
        },
        "description": "Commands to execute in sequence"
      },
      "cwd": {
        "type": "string",
        "description": "Working directory for all commands"
      },
      "stop_on_error": {
        "type": "boolean",
        "default": true,
        "description": "Stop pipeline on first non-zero exit"
      },
      "timeout_ms": {
        "type": "integer",
        "default": 600000,
        "description": "Total timeout in milliseconds (default 10 minutes)"
      },
      "cleanup": {
        "type": "boolean",
        "default": false,
        "description": "Close pane after completion"
      }
    },
    "required": ["commands"]
  }
}
```

### Response Format

```json
{
  "status": "completed|failed|timeout",
  "pane_id": "uuid-for-output-inspection",
  "steps": [
    {
      "name": "build",
      "command": "npm run build",
      "exit_code": 0,
      "duration_ms": 12340
    },
    {
      "name": "test",
      "command": "npm test",
      "exit_code": 1,
      "duration_ms": 5678
    }
  ],
  "failed_at": "test",
  "total_duration_ms": 18018
}
```

## Architecture

### Bridge-Only Implementation

Uses existing primitives:
- `create_pane` - create execution pane
- `send_input` - run each command
- `read_pane` - poll for completion + exit code
- `close_pane` - cleanup if requested

### Sequential Execution Loop

```
for each command in commands:
    1. Wrap command: { <cmd> ; } ; echo "___FUGUE_EXIT_$?___"
    2. send_input(wrapped_command)
    3. Poll read_pane for exit marker
    4. Parse exit code
    5. If exit_code != 0 && stop_on_error: break
    6. Record step result
return aggregated results
```

### Exit Code Detection

Same pattern as FEAT-094:
```bash
{ npm run build ; } ; echo "___FUGUE_EXIT_$?___"
```

Ensures exit code is captured even for commands that produce no output.

## Implementation Tasks

### Section 1: Add to Orchestration Module

- [ ] Add `RunPipelineRequest` and `RunPipelineResponse` types
- [ ] Implement in `fugue-server/src/mcp/bridge/orchestration.rs`
- [ ] Share exit code parsing with FEAT-094

### Section 2: Pane Management

- [ ] Create single pane for pipeline
- [ ] Apply `cwd` via initial `cd` command
- [ ] Keep pane open by default for output inspection

### Section 3: Sequential Execution Loop

- [ ] Iterate through commands in order
- [ ] Wrap each command with exit marker
- [ ] Wait for completion before next command
- [ ] Track duration per step

### Section 4: Error Handling

- [ ] Parse exit codes from output
- [ ] Implement `stop_on_error` logic
- [ ] Record `failed_at` step name
- [ ] Continue on error if `stop_on_error: false`

### Section 5: Timeout Handling

- [ ] Track total elapsed time
- [ ] Abort on timeout, return partial results
- [ ] Include `timeout` status in response

### Section 6: Result Aggregation

- [ ] Build structured step results
- [ ] Calculate total duration
- [ ] Determine overall status
- [ ] Return pane_id for follow-up inspection

### Section 7: Tool Registration

- [ ] Add tool schema to `fugue-server/src/mcp/tools.rs`
- [ ] Register handler in `fugue-server/src/mcp/bridge/handlers.rs`

### Section 8: Testing

- [ ] Integration test: successful 3-step pipeline
- [ ] Integration test: `stop_on_error` behavior
- [ ] Integration test: `stop_on_error: false` continues
- [ ] Integration test: timeout handling
- [ ] Integration test: pane preservation for inspection

## Files to Modify

| File | Changes |
|------|---------|
| `fugue-server/src/mcp/bridge/orchestration.rs` | Add pipeline implementation |
| `fugue-server/src/mcp/bridge/handlers.rs` | Register handler |
| `fugue-server/src/mcp/tools.rs` | Add tool schema |

## Acceptance Criteria

- [ ] `fugue_run_pipeline` tool available in MCP
- [ ] Executes commands sequentially in single pane
- [ ] `stop_on_error: true` halts on first failure
- [ ] `stop_on_error: false` continues through failures
- [ ] Returns structured results per step
- [ ] Returns `pane_id` for output inspection
- [ ] `failed_at` indicates which step failed
- [ ] Timeout returns partial results
- [ ] No protocol changes required

## Dependencies

- Existing MCP infrastructure
- Shared exit code parsing with FEAT-094/FEAT-096

## Notes

### Context Savings

Typical 5-step sequential workflow: ~700-1000 tokens
With `fugue_run_pipeline`: ~150 tokens

**Savings: 75-85% context reduction**

### Use Cases

1. **Build/Test/Deploy**:
   ```json
   {
     "commands": [
       {"name": "build", "command": "npm run build"},
       {"name": "test", "command": "npm test"},
       {"name": "deploy", "command": "npm run deploy"}
     ],
     "stop_on_error": true
   }
   ```

2. **Data Processing**:
   ```json
   {
     "commands": [
       {"name": "extract", "command": "python extract.py"},
       {"name": "transform", "command": "python transform.py"},
       {"name": "load", "command": "python load.py"}
     ]
   }
   ```

3. **Git Workflow**:
   ```json
   {
     "commands": [
       {"name": "add", "command": "git add ."},
       {"name": "commit", "command": "git commit -m 'fix'"},
       {"name": "push", "command": "git push"}
     ]
   }
   ```

### Pane Preservation

By default, `cleanup: false` keeps the pane alive. This allows:
- Output inspection via `read_pane`
- Manual intervention if needed
- Follow-up commands in same pane

Set `cleanup: true` for fire-and-forget pipelines.
