# Implementation Plan: FEAT-072

**Work Item**: [FEAT-072: Per-pane MCP mode control (full/minimal/none)](PROMPT.md)
**Component**: fugue-server, fugue-protocol
**Priority**: P2
**Created**: 2026-01-13

## Overview

Add per-pane MCP mode control so each pane can choose how much MCP tooling is available at startup: full (current behavior), minimal (core safe tools), or none (no MCP servers). This builds on FEAT-071's per-pane configuration pipeline by altering the per-pane Claude config directory contents, primarily `mcp.json`, while keeping defaults fully backward compatible.

## Architecture Decisions

### 1. Protocol Field for MCP Mode

**Approach**: Add `mcp_mode` to `CreatePane` as an optional enum-like string (`"full" | "minimal" | "none"`).

**Rationale**: Keeps wire format backward compatible and mirrors Claude config concepts.

**Trade-offs**: String-based validation at the boundary; use a server-side enum for safety.

### 2. Per-Pane Config Directory Mutation

**Approach**: On pane spawn, copy or synthesize a per-pane config directory (already in FEAT-020/071), then adjust `mcp.json` based on `mcp_mode`.

- **full**: leave `mcp.json` unchanged
- **minimal**: filter to an allowlist of core MCP servers
- **none**: write `mcpServers = {}` or remove `mcp.json`

**Rationale**: Claude Code loads MCP servers from the config dir; manipulating `mcp.json` is the lowest-friction control point.

**Trade-offs**: Requires careful handling of missing/invalid config files and ensures the modified file is valid JSON.

### 3. Minimal MCP Allowlist

**Approach**: Define a small default allowlist in fugue config (and/or presets) and make it overrideable.

**Rationale**: Minimal needs to be predictable yet configurable to user needs.

**Trade-offs**: Users may need to expand allowlist if they rely on additional tools.

### 4. Preset Integration

**Approach**: Extend FEAT-071 presets with `mcp_mode` and ensure the precedence order is:
`session defaults -> preset -> pane override`.

**Rationale**: Presets are the most user-friendly way to standardize worker modes.

**Trade-offs**: Need to validate interactions with `claude_config` overrides.

## Affected Components

| Component | Type of Change | Risk Level |
|-----------|----------------|------------|
| fugue-protocol/src/messages.rs | Add `mcp_mode` to CreatePane | Low |
| fugue-server/src/mcp/tools.rs | Add MCP tool parameter | Low |
| fugue-server/src/session/pane.rs | Per-pane config creation + MCP filtering | Medium |
| fugue-server/src/config.rs | Preset + allowlist config | Low |
| docs/architecture/CLAUDE_INTEGRATION.md | MCP config guidance | Low |

## Dependencies

**Required:**
- FEAT-071 (Per-pane Claude configuration) - **in backlog**
- FEAT-020 (Session isolation) - **completed** ✅

## Implementation Approach

### Phase 1: Protocol + Tool Surface
1. Add `mcp_mode` to CreatePane and MCP tool input schema
2. Validate values and maintain default `full` behavior

### Phase 2: Config Mutation
1. Read or create per-pane `mcp.json`
2. Implement `apply_mcp_mode(mcp_mode, mcp_json)` transformation
3. Ensure isolation (never modify the source config)

### Phase 3: Presets + Allowlist
1. Extend preset schema with `mcp_mode`
2. Add configurable minimal allowlist (default set)
3. Document preset examples

### Phase 4: Testing + Documentation
1. Unit tests for `mcp_mode` parsing and config filtering
2. Integration tests for each mode (full/minimal/none)
3. Docs updates with examples and token savings notes

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Invalid `mcp.json` after filtering | Medium | High | Strict JSON serialization + tests |
| Missing critical tool in minimal mode | Medium | Medium | Allowlist configurable, document clearly |
| Regressions in default behavior | Low | High | Default `full`, regression tests |

## Rollback Strategy

1. Keep `mcp_mode` optional and default to `full`
2. If issues arise, disable filtering logic behind a config flag
3. Revert FEAT-072 changes; FEAT-071 remains intact

## Implementation Notes

- Prefer a small `McpMode` enum in server code with `FromStr` validation.
- When `mcp.json` is missing, synthesize a minimal valid structure instead of failing.
- Keep logs concise: log chosen mode and number of servers enabled.

