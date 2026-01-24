# SPIKE-001: Agent Summary MCP Tool - Findings

**Investigation Date:** 2026-01-22
**Investigator:** Worker Agent
**Feasibility:** HIGH
**Recommendation:** PROCEED to FEAT implementation

---

## Executive Summary

The fugue codebase already has robust agent detection infrastructure that makes an "Agent Summary" MCP tool highly feasible. The existing `ClaudeDetector`, `DetectorRegistry`, and PTY processing pipeline provide the foundation needed. Implementation would primarily involve:

1. Aggregating existing data into a summary format
2. Adding spinner text extraction for activity descriptions
3. Exposing token count detection (code exists but unused)

Estimated effort: Small feature (~1-2 sessions)

---

## Investigation Questions & Answers

### Q1: How is agent state currently detected?

**Answer:** Via `DetectorRegistry` in `fugue-server/src/agents/mod.rs` which manages multiple `AgentDetector` implementations:

- **ClaudeAgentDetector** - Detects Claude Code presence and activity
- **GeminiAgentDetector** - Detects Gemini CLI
- **CodexAgentDetector** - Detects OpenAI Codex

Detection flow:
```
PTY Output → Pane::process() → DetectorRegistry::analyze() → AgentState
```

Key file: `fugue-server/src/claude/detector.rs:186-249` - The `analyze()` method:
1. Strips ANSI escape sequences
2. Detects agent presence via string patterns
3. Extracts session ID and model
4. Detects activity state (Thinking, Coding, ToolUse, etc.)
5. Applies debouncing for state transitions

### Q2: What activity states are currently tracked?

**Answer:** From `fugue-protocol/src/types/agent.rs:94-108`:

```rust
pub enum AgentActivity {
    Idle,                   // Waiting for input at prompt
    Processing,             // Thinking/analyzing (generic)
    Generating,             // Writing/coding (generic)
    ToolUse,                // Executing tools (Read, Bash, etc.)
    AwaitingConfirmation,   // Permission prompt displayed
    Custom(String),         // Agent-specific states
}
```

For Claude specifically, `ClaudeActivity` in `fugue-protocol/src/types/agent.rs:143-154`:
```rust
pub enum ClaudeActivity {
    Idle,
    Thinking,
    Coding,
    ToolUse,
    AwaitingConfirmation,
}
```

Detection patterns (from `detector.rs`):
- **Thinking**: "Thinking", "Processing", "Analyzing", "Reading"
- **Coding**: "Writing", "Coding", "Channelling", "Generating", "Editing"
- **ToolUse**: "Running:", "Executing:", tool calls like "Read(", "Bash("
- **AwaitingConfirmation**: "[Y/n]", "Allow?", "Proceed?", "Press Enter"
- **Idle**: Prompt detected ("> " or "❯ ")

### Q3: What data is available in the scrollback buffer?

**Answer:** From `fugue-server/src/pty/buffer.rs`:

The `ScrollbackBuffer` provides:
- Line-by-line terminal history (VT100 escape sequences preserved)
- Configurable max lines (default 1000)
- Search capability: `search(pattern)` returns matching lines with indices
- Range access: `get_last_n(n)`, `get_range(start, end)`
- Memory tracking with global usage monitoring

Access via `Pane`:
```rust
pane.scrollback()           // Get buffer reference
pane.scrollback_lines()     // Get line count
pane.scrollback_bytes()     // Get memory usage
```

### Q4: Can we extract the spinner text for activity description?

**Answer:** YES, with enhancement.

Current spinner detection in `detector.rs:429-436`:
```rust
const SPINNER_CHARS: &[char] = &['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
```

The detector looks for spinner patterns but does NOT currently extract the description text following the spinner. However:

1. The raw text is available in the output buffer
2. Pattern matching infrastructure exists
3. Adding extraction would be straightforward:
   ```rust
   // Example: Extract "Thinking about codebase structure" from "⠋ Thinking about codebase structure"
   if let Some(pos) = line.find(|c| SPINNER_CHARS.contains(&c)) {
       let description = line[pos..].trim_start_matches(SPINNER_CHARS).trim();
   }
   ```

### Q5: Can we detect token count?

**Answer:** YES - code exists but is currently unused.

From `detector.rs:537-557`:
```rust
#[allow(dead_code)]
fn extract_tokens(&mut self, text: &str) {
    for line in text.lines() {
        let line_lower = line.to_lowercase();
        if line_lower.contains("token") {
            // Extract number near "token" keyword
            for word in line.split_whitespace() {
                if let Ok(num) = word.trim_matches(...).parse::<u64>() {
                    self.session_info.set_tokens(num);
                }
            }
        }
    }
}
```

The token display in Claude's TUI appears in the bottom-right status line. We would need to:
1. Analyze the vt100 screen (available via `pane.screen()`)
2. Look at the last row for token count pattern
3. Claude displays tokens like: `79286 tokens` or `108750 tokens`

### Q6: What's the pattern for adding new MCP tools?

**Answer:** Well-established with ~50+ existing tools.

1. **Define tool schema** in `fugue-server/src/mcp/tools.rs`:
```rust
Tool {
    name: "fugue_agent_summary".into(),
    description: "Get summary of agent activity in a pane".into(),
    input_schema: serde_json::json!({...}),
}
```

2. **Implement handler** in `fugue-server/src/mcp/bridge/handlers.rs`:
```rust
"fugue_agent_summary" => {
    let pane_id = get_pane_id(arguments)?;
    // Gather data from pane state, scrollback, detector
    Ok(serde_json::to_value(summary)?)
}
```

3. **Add tests** in handler tests file

### Q7: What does `fugue_get_status` currently return?

**Answer:** From `handlers.rs`, it returns:

```json
{
  "pane_id": "uuid",
  "state": "Agent",
  "cols": 120,
  "rows": 40,
  "title": "optional-title",
  "cwd": "/path/to/cwd",
  "name": "optional-name",
  "is_mirror": false,
  "claude_state": {           // Only if agent is Claude
    "session_id": "uuid",
    "activity": "Thinking",
    "model": "claude-opus-4-5",
    "tokens_used": 45000
  }
}
```

This is a good starting point but doesn't include:
- Activity description (spinner text)
- Recent tools used
- Time in current state
- Session tags

---

## Proposed API Schema

### Tool: `fugue_agent_summary`

**Input:**
```json
{
  "type": "object",
  "properties": {
    "pane_id": {
      "type": "string",
      "description": "UUID of the pane (required)"
    },
    "include_recent_output": {
      "type": "boolean",
      "default": false,
      "description": "Include last N lines of output"
    },
    "output_lines": {
      "type": "integer",
      "default": 10,
      "description": "Number of output lines to include (max 50)"
    }
  },
  "required": ["pane_id"]
}
```

**Output:**
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
    "⠋ Thinking about implementation..."
  ]
}
```

---

## Implementation Approach

### Option A: Extend `fugue_get_status` (Minimal Change)

Add optional parameters to existing tool:
```json
{
  "pane_id": "...",
  "include_summary": true,
  "include_recent_tools": true
}
```

**Pros:** No new tool, backward compatible
**Cons:** Overloads existing tool, mixed concerns

### Option B: New `fugue_agent_summary` Tool (Recommended)

Dedicated tool with focused purpose.

**Pros:**
- Clear separation of concerns
- Easier to evolve independently
- Self-documenting API

**Cons:**
- One more tool to maintain

### Implementation Steps

1. **Add spinner text extraction** to `ClaudeDetector`
   - Parse output for spinner + description pattern
   - Store in `AgentState.metadata["activity_description"]`

2. **Enable token extraction**
   - Remove `#[allow(dead_code)]` from `extract_tokens()`
   - Call during `analyze()` when Claude is detected
   - Parse vt100 screen for status line

3. **Track recent tools**
   - Add `recent_tools: Vec<String>` to `AgentState`
   - Update when ToolUse state detected
   - Keep last 10 tools

4. **Add state duration tracking**
   - Already have `state_changed_at` in `Pane`
   - Calculate duration in summary response

5. **Create MCP tool**
   - Add schema to `tools.rs`
   - Add handler in `handlers.rs`
   - Aggregate data from pane state

---

## Risks and Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Token parsing breaks with Claude updates | Medium | Low | Regex fallback, graceful degradation |
| Spinner text extraction unreliable | Low | Low | Return empty string if not found |
| Performance overhead | Low | Low | Data already tracked, just aggregation |

---

## Recommendation

**PROCEED to FEAT implementation.**

Rationale:
1. Infrastructure exists - this is primarily aggregation work
2. Clear use case for orchestrator visibility
3. Low risk, low complexity
4. Builds on proven patterns

Suggested FEAT ID: `FEAT-XXX-agent-summary-tool`

Priority: Medium (valuable for orchestration workflows but not blocking)

---

## Appendix: Key File References

| File | Purpose |
|------|---------|
| `fugue-server/src/claude/detector.rs` | Claude state detection logic |
| `fugue-server/src/agents/mod.rs` | Generic agent detection registry |
| `fugue-server/src/session/pane.rs` | Pane state and process() method |
| `fugue-server/src/pty/buffer.rs` | Scrollback buffer implementation |
| `fugue-server/src/mcp/tools.rs` | MCP tool definitions |
| `fugue-server/src/mcp/bridge/handlers.rs` | MCP tool handlers |
| `fugue-protocol/src/types/agent.rs` | AgentState, ClaudeActivity types |
