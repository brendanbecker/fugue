# FEAT-072: Per-pane MCP mode control (full/minimal/none)

**Priority**: P2
**Component**: fugue-server
**Type**: enhancement
**Estimated Effort**: small
**Business Value**: high

## Overview

Add per-pane MCP mode control to reduce token overhead in worker panes. This builds on FEAT-071 (per-pane Claude config) to specifically address MCP server loading, providing dramatic token savings (5k-50k+ per worker) by controlling which MCP servers/tools are available per pane.

## Problem Statement

Claude Code loads all active MCP servers + tools into context at session start, even if unused. In multi-agent swarms (gastown polecats), this bloats every worker's context window with unnecessary tools. Workers rarely need advanced tools like GitHub, Playwright, etc.

**Current State:**
- All panes load full MCP tool set (~5k-50k+ tokens)
- Workers waste tokens on tools they never use
- Slower initialization due to MCP loading
- More hallucinated tool use in workers

**Goal:**
- Workers load minimal/no MCP tools
- Token savings: 5k-50k+ per worker pane
- Faster init, less hallucinated tool use
- For 10 worker panes: 50k-500k tokens saved per session

## Evidence

From `docs/architecture/MCP_WORKER_MODE.md`:

- Huge token savings (5k–50k+ per worker)
- Faster startup speed for subagents
- Sandboxed workers (no dangerous APIs)
- Simple config/env change implementation

## Requested Feature

Add `mcp_mode` parameter to pane spawn with three modes:

1. **`full`** (default): All MCP servers/tools (current behavior)
   - Use case: Orchestrator/Mayor pane
   - Tools: GitHub, Playwright, filesystem, etc.

2. **`minimal`**: Core safe tools only
   - Use case: Worker panes needing basic I/O
   - Tools: fs read/write, basic bash (if allowed)
   - Token savings: ~80-90% reduction

3. **`none`**: No MCP servers loaded
   - Use case: Subagents, polecats
   - Tools: None
   - Token savings: ~100% reduction (5k-50k+ tokens)

**Integration with FEAT-071:**
- Combine `mcp_mode` with `claude_model` and `claude_config`
- Support in configuration presets
- Example: `worker` preset = Haiku + minimal MCP

**Future Enhancement (stretch goal):**
- Dynamic tool granting via `<fugue:enable-tool name="git">` sideband protocol
- Workers can request tools from orchestrator as needed

## Implementation Tasks

### Section 1: MCP Mode Configuration
- [ ] Add `mcp_mode` field to `CreatePane` message in fugue-protocol
- [ ] Support "full", "minimal", "none" values
- [ ] Add `mcp_mode` to configuration presets
- [ ] Default to "full" (preserve current behavior)

### Section 2: Config Directory Manipulation
- [ ] For "none": Create config dir with `mcp_servers = []`
- [ ] For "minimal": Include only safe core tools (fs, basic bash if allowed)
- [ ] For "full": Use normal config (current behavior)
- [ ] Ensure isolation (don't modify source config)

### Section 3: Minimal Mode Tool Selection
- [ ] Define "minimal" tool set (read/write files, basic commands)
- [ ] Filter MCP servers to minimal set
- [ ] Document minimal tool list
- [ ] Make minimal set configurable (optional enhancement)

### Section 4: Integration with FEAT-071
- [ ] Combine `mcp_mode` with `claude_model` and `claude_config`
- [ ] Support `mcp_mode` in presets
- [ ] Ensure precedence order is correct
- [ ] Test interaction with custom configs

### Section 5: Preset Examples
- [ ] Create "orchestrator" preset (full MCP, Sonnet)
- [ ] Create "worker" preset (minimal MCP, Haiku)
- [ ] Create "subagent" preset (no MCP, Haiku)
- [ ] Document use cases for each preset

### Section 6: Testing
- [ ] Test "none" mode produces no MCP tools
- [ ] Test "minimal" mode produces core tools only
- [ ] Test "full" mode matches current behavior
- [ ] Test token usage reduction (context size via `/context` in Claude)
- [ ] Verify no MCP tool hallucinations in workers

### Section 7: Documentation
- [ ] Document `mcp_mode` parameter in fugue-protocol
- [ ] Document tool selection for minimal mode
- [ ] Document token savings with examples
- [ ] Provide examples for multi-agent workflows
- [ ] Document future dynamic tool granting

## Acceptance Criteria

- [ ] `mcp_mode` parameter controls MCP tool loading
- [ ] "none" mode eliminates MCP tools entirely (verified via `/context`)
- [ ] "minimal" mode provides core tools only
- [ ] "full" mode preserves current behavior
- [ ] Token savings measurable (5k-50k+ per worker)
- [ ] Workers start faster with reduced MCP
- [ ] Presets include `mcp_mode` configurations
- [ ] Documentation covers all modes and use cases
- [ ] No regressions in existing MCP functionality

## Dependencies

- **FEAT-071** (Per-pane Claude configuration) - provides config infrastructure
- **FEAT-020** (Session isolation) - completed, provides per-pane config dirs

## Related Files

- `fugue-server/src/session/pane.rs` - config creation logic
- `fugue-protocol/src/messages.rs` - `mcp_mode` field definition
- `fugue-server/config.toml` - presets with `mcp_mode`
- `docs/architecture/MCP_WORKER_MODE.md` - design document

## Notes

### Token Savings Example

From design doc:
- **Full MCP**: 5k-50k+ tokens per pane
- **Minimal MCP**: ~500-1k tokens per pane
- **No MCP**: ~0 tokens per pane

**For 10 worker panes:** 50k-500k tokens saved per session

### Implementation Approach

This is derived from `fugue-mcp-worker-mode.md` but generalized into FEAT-071's configuration system. The `mcp_mode` parameter is a specific application of per-pane config for MCP tool control.

**Implementation levers:**
1. Isolated `CLAUDE_CONFIG_DIR` per pane (already in fugue via FEAT-020)
2. On spawn: create stripped config dir for workers
3. Copy original config, modify MCP sections based on mode
4. Set `CLAUDE_CONFIG_DIR` to isolated directory

### Future Enhancement: Dynamic Tool Granting

**Stretch goal** (not in current scope):
- Workers can request tools from orchestrator
- Sideband protocol: `<fugue:enable-tool name="git">`
- Worker reloads config with added tool
- Enables zero-overhead start with on-demand capability

### Testing Verification

Use `/context` command in Claude Code to verify:
- Full mode: All MCP tools present in context
- Minimal mode: Only core tools present (~80-90% reduction)
- None mode: No MCP tools present (~100% reduction)

Measure token counts in context window to confirm savings.
