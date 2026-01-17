# Implementation Plan: FEAT-071

**Work Item**: [FEAT-071: Per-pane Claude configuration on spawn](PROMPT.md)
**Component**: ccmux-server, ccmux-protocol
**Priority**: P2
**Created**: 2026-01-13

## Overview

Allow panes to specify custom Claude Code configuration on spawn, including model selection, context limits, and other Claude settings. This enables fine-grained control over Claude instances per-pane, supporting use cases like mixed Haiku/Sonnet/Opus agents, custom context limits, and specialized configurations.

## Architecture Decisions

### 1. Configuration Layer Model

**Approach**: Four-layer configuration precedence
1. **Default Config**: Built-in Claude defaults
2. **Session Config**: Inherited from session's CLAUDE_CONFIG_DIR
3. **Preset Config**: Named preset from ccmux config.toml
4. **Pane Config**: Explicit overrides for this pane

**Rationale**: Provides flexibility while maintaining sensible defaults. Users can start simple (no config) and progressively add specificity.

**Trade-offs**: More complex configuration merging logic, but much better UX.

### 2. Protocol Design

**Approach**: Add optional fields to CreatePane message:
- `claude_model: Option<String>` - Direct model override
- `claude_config: Option<ClaudeConfig>` - Full config object
- `claude_preset: Option<String>` - Named preset reference

**Rationale**: Three levels of control:
- Simple: Just specify model
- Medium: Use a preset
- Advanced: Full custom config

**Trade-offs**: Multiple ways to do the same thing, but covers all use cases elegantly.

### 3. Configuration Storage

**Approach**: Write merged config to `{pane_config_dir}/claude_config.toml`

**Rationale**:
- Leverages existing per-pane CLAUDE_CONFIG_DIR from FEAT-020
- TOML format matches Claude Code's expected config format
- Easy to inspect and debug

**Trade-offs**: Need to ensure config format matches Claude Code expectations exactly.

### 4. Preset System

**Approach**: Store presets in ccmux's `config.toml` under `[presets.{name}]` sections

```toml
[presets.haiku-worker]
model = "claude-haiku-3-5-20241022"
context_limit = 100000
description = "Fast worker for simple tasks"
```

**Rationale**:
- Centralized preset management
- Easy to version control
- Users can add custom presets

**Trade-offs**: Presets are global to ccmux installation, not per-session. Could add per-session presets later if needed.

### 5. Validation Strategy

**Approach**: Validate at config merge time:
- Model name matches known Claude models (allow custom for future-proofing)
- Context limit is positive and reasonable (warn if > 200k)
- Config keys are recognized Claude settings

**Rationale**: Fail fast with clear error messages rather than silent failures later.

**Trade-offs**: Need to maintain list of valid models. Could use regex pattern instead of hardcoded list.

## Affected Components

| Component | Type of Change | Risk Level |
|-----------|----------------|------------|
| ccmux-protocol/src/messages.rs | Add fields to CreatePane | Low (backward compatible) |
| ccmux-server/src/mcp/tools.rs | Add parameters to MCP tool | Low (optional params) |
| ccmux-server/src/session/pane.rs | Config merging and writing | Medium (core logic) |
| ccmux-server/src/config.rs | Preset loading | Low (new functionality) |
| ccmux-server/config.toml | Add preset definitions | Low (config only) |

## Dependencies

**Required:**
- FEAT-020 (Session isolation) - **completed** ✅
  - Provides per-pane config directory structure
  - Provides CLAUDE_CONFIG_DIR environment variable setup

**Blocks:**
- FEAT-072 (Per-pane MCP mode control)
  - Will use this config system for MCP settings

## Implementation Approach

### Phase 1: Protocol Extensions
1. Add new fields to CreatePane message in protocol
2. Update protocol version number (minor bump)
3. Ensure backward compatibility with old clients

### Phase 2: Configuration Types
1. Define `ClaudeConfig` struct matching Claude Code's config format
2. Implement serde serialization to TOML
3. Add validation methods

### Phase 3: Preset System
1. Add preset definitions to config.toml schema
2. Load presets at server startup
3. Implement preset lookup and merging

### Phase 4: Configuration Merging
1. Implement four-layer merge logic in pane.rs
2. Write merged config to pane's CLAUDE_CONFIG_DIR
3. Add logging for config decisions

### Phase 5: MCP Tool Updates
1. Add new parameters to ccmux_create_pane
2. Update tool schema and help text
3. Handle parameter validation

### Phase 6: Testing & Documentation
1. Unit tests for config merging
2. Integration tests with real Claude instances
3. Update MCP tool documentation
4. Add example presets and use cases

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Config format mismatch with Claude | Medium | High | Carefully study Claude config format; test thoroughly |
| Preset name conflicts | Low | Low | Use namespacing if needed (user.*, built-in.*) |
| Config validation too strict | Medium | Medium | Allow unknown keys, warn instead of error |
| Performance impact (config I/O) | Low | Low | Config is small, one-time per pane spawn |
| Backward compatibility break | Low | High | All new fields are optional |

## Rollback Strategy

If implementation causes issues:
1. Configuration system is additive - can disable by not using new parameters
2. If bugs in config merging, panes still work with session config
3. Revert commits associated with FEAT-071
4. Document issues in comments.md

## Configuration Format Reference

Based on Claude Code's expected format:

```toml
# Main config file (~/.claude/config.toml or per-pane)
model = "claude-sonnet-4-5-20250929"
context_limit = 150000

# Additional settings (as supported by Claude)
temperature = 0.7
max_tokens = 4096

[api]
timeout = 300
```

## Implementation Notes

### Config Merging Algorithm
```rust
fn merge_config(
    session_config: &ClaudeConfig,
    preset: Option<&ClaudeConfig>,
    pane_config: Option<&ClaudeConfig>,
) -> ClaudeConfig {
    let mut merged = session_config.clone();

    if let Some(preset) = preset {
        merged.merge(preset);
    }

    if let Some(pane) = pane_config {
        merged.merge(pane);
    }

    merged
}
```

### Preset Definition Schema
```rust
#[derive(Debug, Clone, Deserialize)]
struct PresetConfig {
    model: Option<String>,
    context_limit: Option<u32>,
    description: String,
    // ... other Claude settings
}
```

### MCP Tool Schema Extension
```json
{
  "name": "ccmux_create_pane",
  "parameters": {
    "claude_model": {
      "type": "string",
      "description": "Claude model to use (e.g., 'claude-haiku-3-5-20241022')"
    },
    "claude_preset": {
      "type": "string",
      "description": "Named configuration preset (e.g., 'haiku-worker')"
    },
    "claude_config": {
      "type": "object",
      "description": "Full Claude configuration object"
    }
  }
}
```

---
*This plan should be updated as implementation progresses.*
