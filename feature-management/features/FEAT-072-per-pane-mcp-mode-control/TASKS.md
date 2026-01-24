# Task Breakdown: FEAT-072

**Work Item**: [FEAT-072: Per-pane MCP mode control (full/minimal/none)](PROMPT.md)
**Status**: Not Started
**Last Updated**: 2026-01-13

## Prerequisites

- [ ] Read and understand PROMPT.md
- [ ] Review PLAN.md and update if needed
- [ ] Review FEAT-071 implementation approach and data structures
- [ ] Inspect current Claude MCP config format (`~/.claude/mcp.json`)
- [ ] Locate per-pane config directory creation in fugue-server

## Section 1: Protocol + Tool Surface

- [ ] Add `mcp_mode: Option<String>` to CreatePane message
- [ ] Update protocol documentation/comments for allowed values
- [ ] Add `mcp_mode` parameter to `fugue_create_pane` tool
- [ ] Validate `mcp_mode` values (full/minimal/none)
- [ ] Ensure backward compatibility (default to `full`)
- [ ] Add unit tests for CreatePane serialization with `mcp_mode`

## Section 2: Config Directory Manipulation

- [ ] Add `McpMode` enum and `FromStr` validation helper
- [ ] Implement `apply_mcp_mode()` for per-pane `mcp.json`
- [ ] For `full`: leave `mcp.json` unchanged
- [ ] For `minimal`: filter `mcpServers` to allowlist
- [ ] For `none`: write empty `mcpServers` or omit file
- [ ] Ensure mutations are isolated to per-pane config dir
- [ ] Add logging for selected mode and server count

## Section 3: Minimal Mode Allowlist

- [ ] Define default allowlist in server config
- [ ] Make allowlist configurable (optional per preset)
- [ ] Decide on initial minimal set (document reasoning)
- [ ] Add validation for allowlist entries
- [ ] Add unit test for allowlist filtering behavior

## Section 4: Preset Integration (FEAT-071)

- [ ] Extend preset schema with `mcp_mode`
- [ ] Ensure merge precedence: session -> preset -> pane
- [ ] Add example presets in config comments (or docs)
- [ ] Add tests for preset + explicit mode overrides

## Section 5: Testing

- [ ] Unit test: `mcp_mode` parsing and validation
- [ ] Unit test: `mcp.json` filtering output (full/minimal/none)
- [ ] Integration test: create pane with `mcp_mode=full` (no changes)
- [ ] Integration test: create pane with `mcp_mode=minimal` (allowlist only)
- [ ] Integration test: create pane with `mcp_mode=none` (no servers)
- [ ] Manual test: use `/context` to verify tool list per mode
- [ ] Manual test: measure token usage deltas (baseline vs minimal/none)

## Section 6: Documentation

- [ ] Document `mcp_mode` in protocol docs
- [ ] Update MCP bridge docs/tool usage with `mcp_mode`
- [ ] Document minimal allowlist and how to override
- [ ] Add preset examples: orchestrator/worker/subagent
- [ ] Add troubleshooting notes for missing tools
- [ ] Update token savings examples in docs

## Verification Tasks

- [ ] Verify acceptance criteria from PROMPT.md
- [ ] Verify defaults preserve existing behavior
- [ ] Verify worker panes start with reduced MCP context
- [ ] Verify no MCP tools in `none` mode via `/context`
- [ ] Verify feature_request.json status updated
- [ ] Document completion in comments.md

## Completion Checklist

- [ ] All implementation tasks complete
- [ ] All tests passing
- [ ] Documentation complete
- [ ] PLAN.md reflects final implementation
- [ ] Ready for review/merge

---
*Check off tasks as you complete them. Update status field above.*
