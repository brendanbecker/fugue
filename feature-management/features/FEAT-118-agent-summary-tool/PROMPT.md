# FEAT-118: Agent Summary MCP Tool

**Priority**: P2
**Component**: ccmux-server/mcp
**Type**: new_feature
**Estimated Effort**: small
**Business Value**: high
**Based On**: SPIKE-001

## Overview

Create a `ccmux_agent_summary` MCP tool that returns structured agent state data instead of raw pane output, dramatically reducing orchestrator context consumption.

## Problem Statement

Currently, orchestrators must call `ccmux_read_pane` to check worker status, consuming 500-2000 tokens of raw terminal output per check. A structured summary could reduce this to ~50-100 tokens while providing more actionable information.

**Current approach:**
```json
// ~1500 tokens of ANSI-laden terminal output
{"tool": "ccmux_read_pane", "pane_id": "..."}
```

**Proposed approach:**
```json
// ~80 tokens of structured data
{
  "agent_type": "claude",
  "activity": {"state": "Thinking", "description": "Analyzing codebase"},
  "tokens_used": 78500,
  "is_awaiting_input": false
}
```

**Token savings: 90%+**

## API Design

### Input Schema

```json
{
  "name": "ccmux_agent_summary",
  "description": "Get structured summary of agent state in a pane",
  "inputSchema": {
    "type": "object",
    "properties": {
      "pane_id": {
        "type": "string",
        "description": "UUID of the pane to summarize"
      },
      "include_recent_output": {
        "type": "boolean",
        "default": false,
        "description": "Include last N lines of stripped output"
      },
      "output_lines": {
        "type": "integer",
        "default": 10,
        "description": "Number of output lines if include_recent_output=true (max 50)"
      }
    },
    "required": ["pane_id"]
  }
}
```

### Output Schema

```json
{
  "pane_id": "abc-123...",
  "agent_type": "claude",
  "is_agent": true,

  "activity": {
    "state": "Thinking",
    "description": "Analyzing codebase structure",
    "duration_secs": 45
  },

  "session": {
    "id": "session-uuid",
    "model": "claude-opus-4-5",
    "tokens_used": 78500
  },

  "recent_tools": ["Read", "Grep", "Bash"],

  "context": {
    "is_awaiting_input": false,
    "is_awaiting_confirmation": false,
    "tags": ["worker", "feat-123"],
    "cwd": "/home/user/project"
  },

  "recent_output": [
    "Reading file: src/main.rs",
    "Thinking about implementation..."
  ]
}
```

## Existing Infrastructure (from SPIKE-001)

The following infrastructure already exists and can be leveraged:

### Agent Detection
- `DetectorRegistry` in `ccmux-server/src/agents/mod.rs`
- `ClaudeDetector` in `ccmux-server/src/claude/detector.rs`
- Activity states: Idle, Processing, Generating, ToolUse, AwaitingConfirmation

### Token Extraction (exists but unused)
- `extract_tokens()` in `detector.rs:537-557` - currently `#[allow(dead_code)]`
- Needs to be enabled and called during `analyze()`

### Spinner Text
- Spinner chars defined: `['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏']`
- Need to extract description text following spinner

## Implementation Tasks

### Section 1: Enable Token Extraction

- [ ] Remove `#[allow(dead_code)]` from `extract_tokens()` in `detector.rs`
- [ ] Call `extract_tokens()` in `analyze()` when Claude is detected
- [ ] Verify tokens appear in `AgentState`

### Section 2: Add Spinner Text Extraction

- [ ] Add `activity_description: Option<String>` to `AgentState` or metadata
- [ ] Implement extraction in `detector.rs`:
  ```rust
  fn extract_spinner_description(line: &str) -> Option<String> {
      // Find spinner char, extract text after it
  }
  ```
- [ ] Call during activity detection

### Section 3: Track Recent Tools

- [ ] Add `recent_tools: Vec<String>` to `AgentState` (keep last 10)
- [ ] Update when ToolUse activity detected
- [ ] Parse tool name from output patterns like "Read(", "Bash("

### Section 4: Add Duration Tracking

- [ ] Verify `state_changed_at` exists in `Pane`
- [ ] Calculate `duration_secs` in summary response

### Section 5: Create MCP Tool

- [ ] Add tool schema to `ccmux-server/src/mcp/tools.rs`
- [ ] Add handler in `ccmux-server/src/mcp/bridge/handlers.rs`:
  - Extract pane_id
  - Get pane from daemon
  - Aggregate state data from pane, detector, scrollback
  - Format and return response

### Section 6: Testing

- [ ] Unit test: spinner text extraction
- [ ] Unit test: token parsing
- [ ] Integration test: get summary for Claude pane
- [ ] Integration test: handle non-agent pane gracefully
- [ ] Integration test: include_recent_output option

## Files to Modify

| File | Changes |
|------|---------|
| `ccmux-server/src/claude/detector.rs` | Enable token extraction, add spinner text extraction |
| `ccmux-protocol/src/types/agent.rs` | Add activity_description, recent_tools to AgentState |
| `ccmux-server/src/mcp/tools.rs` | Add ccmux_agent_summary schema |
| `ccmux-server/src/mcp/bridge/handlers.rs` | Add handler implementation |

## Acceptance Criteria

- [ ] `ccmux_agent_summary` tool available in MCP
- [ ] Returns agent type and activity state
- [ ] Includes spinner description text when available
- [ ] Includes token count when available
- [ ] Includes recent tools list
- [ ] Includes activity duration
- [ ] `include_recent_output` option works with stripped escapes
- [ ] Gracefully handles non-agent panes (is_agent: false)
- [ ] All existing tests pass

## Dependencies

- FEAT-117 (strip_escapes) - COMPLETED - provides escape stripping for recent_output

## Notes

### Graceful Degradation

If any field cannot be determined:
- `tokens_used`: null
- `activity.description`: null or empty string
- `recent_tools`: empty array

### Performance

This should be fast since:
- Agent state is already tracked continuously
- No new parsing required on-demand
- Just aggregation of existing data
