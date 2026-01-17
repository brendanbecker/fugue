# FEAT-071: Per-pane Claude configuration on spawn

**Priority**: P2
**Component**: ccmux-server, ccmux-protocol
**Type**: enhancement
**Estimated Effort**: medium
**Business Value**: high

## Overview

Allow panes to specify custom Claude Code configuration on spawn, including model selection, context limits, and other Claude settings. This enables fine-grained control over Claude instances per-pane, supporting use cases like mixed Haiku/Sonnet/Opus agents, custom context limits, and specialized configurations.

## Problem Statement

Currently, all panes in a session inherit the same CLAUDE_CONFIG_DIR and thus share the same Claude configuration. Users want to:
- Use different models per pane (Haiku for fast tasks, Sonnet/Opus for complex ones)
- Set different context limits per agent type
- Customize other Claude settings per pane
- Create pane presets with specific Claude configurations

This requires extending pane spawn to accept configuration parameters and creating isolated config directories per pane with custom settings.

## Requested Feature

- Add `--claude-model` flag to pane spawn (via MCP and CLI)
- Add `--claude-config` flag for full config JSON/TOML
- Create per-pane config directory with merged settings
- Support configuration presets
- Maintain backward compatibility (default to session config)

## Benefits

**Cost Optimization**: Use Haiku for simple tasks, Sonnet for standard work, Opus only when needed. This can significantly reduce API costs for multi-agent workflows.

**Specialized Agents**: Create agents with specific configurations:
- Fast workers with Haiku and small context
- Architecture planners with Opus and large context
- Standard workers with Sonnet and medium context

**Flexible Workflows**: Build heterogeneous agent teams within a single session, each optimized for their role.

**Preset System**: Define reusable configurations for common patterns (haiku-worker, opus-heavy, etc.) reducing setup time.

## Implementation Tasks

### Section 1: Protocol Extensions
- [ ] Add `claude_model` field to CreatePane message
- [ ] Add `claude_config` field for structured configuration
- [ ] Add config preset names/references
- [ ] Update protocol version if needed
- [ ] Document protocol changes

### Section 2: Configuration Management
- [ ] Extend per-pane config directory creation
- [ ] Merge user config with pane-specific settings
- [ ] Support model override in config
- [ ] Support context_limit and other Claude settings
- [ ] Validate configuration values
- [ ] Write merged config to pane's CLAUDE_CONFIG_DIR

### Section 3: MCP Tool Updates
- [ ] Add `model` parameter to ccmux_create_pane
- [ ] Add `config` parameter for full configuration
- [ ] Add `preset` parameter for named configurations
- [ ] Update tool schema and documentation
- [ ] Maintain backward compatibility (optional parameters)

### Section 4: Configuration Presets
- [ ] Design preset storage format (in ccmux config.toml)
- [ ] Load presets from ccmux config
- [ ] Support built-in presets (haiku-worker, sonnet-default, opus-heavy)
- [ ] Allow custom user presets
- [ ] Preset inheritance and overrides
- [ ] Validate preset configurations

### Section 5: Testing
- [ ] Test model override per pane
- [ ] Test custom config per pane
- [ ] Test configuration presets
- [ ] Test backward compatibility (no config specified)
- [ ] Test config validation and errors
- [ ] Integration tests with Claude Code
- [ ] Test preset loading and application

### Section 6: Documentation
- [ ] Document `--claude-model` usage in CLI
- [ ] Document MCP tool parameter usage
- [ ] Document configuration options available
- [ ] Document preset system and built-in presets
- [ ] Provide example configurations
- [ ] Document use cases (mixed models, specialized agents)
- [ ] Update architecture documentation

## Acceptance Criteria

- [ ] Panes can be created with custom model specification
- [ ] Panes can be created with custom Claude config
- [ ] Configuration presets work correctly
- [ ] Per-pane config directories contain merged settings
- [ ] Backward compatibility maintained (no config = session default)
- [ ] Claude Code respects per-pane configuration
- [ ] Documentation covers all options
- [ ] Tests verify all configuration paths

## Dependencies

**Depends on:**
- FEAT-020 (Session isolation) - **completed**, provides per-pane config dirs

**Blocks:**
- FEAT-072 (Per-pane MCP mode control) - builds on this configuration system

## Related Files

- `ccmux-protocol/src/messages.rs` - CreatePane message structure
- `ccmux-server/src/mcp/tools.rs` - MCP tool implementations
- `ccmux-server/src/session/pane.rs` - Pane config creation logic
- `ccmux-server/config.toml` - Configuration and preset definitions
- `ccmux-server/src/config.rs` - Configuration loading and validation

## Example Usage

### MCP Tool Call
```json
{
  "name": "ccmux_create_pane",
  "arguments": {
    "session": "my-session",
    "command": "claude --resume",
    "claude_model": "claude-haiku-3-5-20241022",
    "name": "fast-worker-1"
  }
}
```

### With Preset
```json
{
  "name": "ccmux_create_pane",
  "arguments": {
    "session": "my-session",
    "command": "claude --resume",
    "preset": "haiku-worker",
    "name": "worker-1"
  }
}
```

### With Full Config
```json
{
  "name": "ccmux_create_pane",
  "arguments": {
    "session": "my-session",
    "command": "claude --resume",
    "config": {
      "model": "claude-opus-4-5-20251101",
      "context_limit": 200000,
      "temperature": 0.7
    },
    "name": "architect"
  }
}
```

## Example Preset Configuration

In `ccmux-server/config.toml`:

```toml
[presets.haiku-worker]
model = "claude-haiku-3-5-20241022"
context_limit = 100000
description = "Fast worker for simple tasks"

[presets.sonnet-default]
model = "claude-sonnet-4-5-20250929"
context_limit = 150000
description = "Standard work configuration"

[presets.opus-heavy]
model = "claude-opus-4-5-20251101"
context_limit = 200000
description = "Heavy compute for complex tasks"

[presets.haiku-minimal]
model = "claude-haiku-3-5-20241022"
context_limit = 50000
description = "Minimal context for focused tasks"
```

## Notes

This feature generalizes the "MCP worker mode" concept from ccmux-mcp-worker-mode.md into a flexible per-pane configuration system. Instead of just minimal/none MCP modes, users can configure any Claude setting per-pane.

**Design Considerations:**

1. **Configuration Precedence**: Pane-specific config > Preset config > Session config > Default config
2. **Validation**: Ensure model names are valid, context_limit is reasonable, etc.
3. **Environment Variables**: Per-pane CLAUDE_CONFIG_DIR already exists (FEAT-020), just need to write custom config files there
4. **Backward Compatibility**: All new parameters are optional; existing code continues to work
5. **Future Extensions**: This design allows adding more Claude settings as needed

**Implementation Notes:**

- Leverage existing per-pane config directory structure from FEAT-020
- Consider using serde for config serialization/validation
- May need to support both TOML and JSON config formats
- Preset system should be extensible for user-defined presets
