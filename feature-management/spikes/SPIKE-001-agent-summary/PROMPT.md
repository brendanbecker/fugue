# SPIKE-001: Agent Summary MCP Tool Feasibility

**Type**: spike/investigation
**Priority**: P2
**Component**: fugue-server/mcp
**Timeboxed**: 2-4 hours
**Business Value**: high (if feasible)

## Overview

Investigate the feasibility of a `fugue_get_agent_summary` MCP tool that returns structured data about agent state instead of raw pane output. This would dramatically reduce context consumption for orchestration.

## Motivation

Currently, orchestrators must:
1. Call `fugue_read_pane` to get raw terminal output (~500-2000 tokens)
2. Parse that output to extract meaningful information
3. Much of the content is formatting noise, not semantics

A structured summary could return:
```json
{
  "tokens": 79286,
  "activity": "working",
  "last_tool": "Read",
  "idle": false,
  "spinner_text": "Reading file...",
  "context_pressure": "healthy"
}
```

**Potential savings**: 90%+ token reduction vs raw output

## Investigation Questions

### Q1: Token Count Detection

**Question**: Where do Claude TUI token counts come from?

Investigate:
- [ ] Is token count displayed in the TUI output itself?
- [ ] Does Claude emit structured status via some protocol?
- [ ] Can we parse token counts from the visible terminal buffer?
- [ ] What patterns indicate token count location?

Look for patterns like:
- `79286 tokens` in bottom-right corner
- Status bar rendering
- Any structured output from Claude process

### Q2: Activity Detection

**Question**: How can we reliably detect agent activity state?

Investigate:
- [ ] Spinner text patterns (`Thinking...`, `Reading...`, etc.)
- [ ] Tool use indicators in output
- [ ] Prompt idle state (autosuggest pattern)
- [ ] PTY activity (recent I/O timestamp)

Document patterns found:
- Spinner regex: `?`
- Autosuggest regex: `?`
- Tool indicator regex: `?`

### Q3: Last Tool Detection

**Question**: Can we detect what tool the agent last used?

Investigate:
- [ ] Tool use output format in Claude TUI
- [ ] Structured tool results
- [ ] Patterns that indicate tool completion

### Q4: Idle Detection

**Question**: How to reliably detect prompt idle state?

Current hypothesis from CLAUDE.md:
```
❯ [7mc[27m[2mheck on them in a few minutes    ↵ send[22m
```

Investigate:
- [ ] Verify autosuggest escape sequence pattern
- [ ] Test across different shells (bash, zsh, fish)
- [ ] Check if pattern is consistent across Claude versions

### Q5: Existing Infrastructure

**Question**: What existing fugue infrastructure can we leverage?

Review:
- [ ] `claude_state` detection in handlers.rs
- [ ] PTY output buffers
- [ ] Terminal parser state
- [ ] Existing pattern matching code

## Deliverables

At the end of this spike, produce:

### 1. Feasibility Assessment (required)

Write to `FINDINGS.md`:
- Overall feasibility score: `high|medium|low|not_feasible`
- Summary of what's achievable vs not
- Recommended approach if feasible

### 2. Technical Findings (required)

Document:
- Token count detection: `possible|partial|not_possible` + method
- Activity detection: `possible|partial|not_possible` + method
- Idle detection: `possible|partial|not_possible` + method
- Last tool detection: `possible|partial|not_possible` + method

### 3. Code Exploration Notes (required)

Note key files and their relevance:
- Which handlers are relevant?
- What patterns exist for state detection?
- What would need to be added?

### 4. Proposed API (if feasible)

If feasibility is medium or higher, draft the tool schema.

## Where to Look

### Primary Files

- `fugue-server/src/mcp/bridge/handlers.rs` - MCP handlers, claude_state detection
- `fugue-server/src/pty/` - PTY handling, output buffers
- `fugue-server/src/mcp/tools.rs` - Existing tool schemas

### Secondary Files

- Terminal parser code if it exists
- Any pattern matching utilities
- Existing status/state detection code

## Scope Boundaries

### In Scope

- Reading existing code to understand state detection
- Testing pattern matching against actual Claude output
- Documenting findings
- Proposing API design if feasible

### Out of Scope

- Actually implementing the feature
- Protocol changes
- Claude-side modifications
- Upstream patches

## Success Criteria

Spike is successful if:
- [ ] All investigation questions have documented answers
- [ ] FINDINGS.md written with feasibility assessment
- [ ] Clear recommendation: proceed to FEAT or abandon
- [ ] If proceeding, rough implementation approach documented

## Notes

### Context

This spike was triggered by orchestrator context pressure. Reading pane output consumes 500-2000 tokens per check. With structured summaries, that could drop to 50-100 tokens.

### Related Work

- FEAT-117 (strip_escapes) is a simpler approach that saves 70-80% tokens
- This spike explores whether we can go further with structured data
- The two approaches are complementary, not exclusive
