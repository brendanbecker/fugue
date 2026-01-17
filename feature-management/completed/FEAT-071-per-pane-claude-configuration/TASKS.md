# Task Breakdown: FEAT-071

**Work Item**: [FEAT-071: Per-pane Claude configuration on spawn](PROMPT.md)
**Status**: Not Started
**Last Updated**: 2026-01-13

## Prerequisites

- [x] Read and understand PROMPT.md
- [x] Review PLAN.md and update if needed
- [ ] Study Claude Code config format (inspect existing configs)
- [ ] Verify FEAT-020 per-pane config directory structure

## Section 1: Protocol Extensions

- [ ] Add `claude_model: Option<String>` field to CreatePane message
- [ ] Add `claude_config: Option<serde_json::Value>` field to CreatePane
- [ ] Add `claude_preset: Option<String>` field to CreatePane
- [ ] Update protocol version number if breaking changes
- [ ] Update protocol documentation
- [ ] Ensure backward compatibility (all fields optional)
- [ ] Add unit tests for message serialization

## Section 2: Configuration Types

- [ ] Define `ClaudeConfig` struct in ccmux-server/src/config.rs
- [ ] Add model field
- [ ] Add context_limit field
- [ ] Add temperature field
- [ ] Add other Claude settings as needed
- [ ] Implement `Default` trait for sensible defaults
- [ ] Implement serde `Serialize` for TOML output
- [ ] Implement serde `Deserialize` for loading configs
- [ ] Add validation method `validate()` -> Result<(), ConfigError>
- [ ] Add merge method `merge(&self, other: &ClaudeConfig)`
- [ ] Test config serialization to/from TOML

## Section 3: Preset System

- [ ] Add `[presets]` section to config.toml schema
- [ ] Define built-in presets in default config:
  - [ ] haiku-worker (Haiku, 100k context)
  - [ ] sonnet-default (Sonnet, 150k context)
  - [ ] opus-heavy (Opus, 200k context)
  - [ ] haiku-minimal (Haiku, 50k context)
- [ ] Add `presets: HashMap<String, ClaudeConfig>` to ServerConfig
- [ ] Load presets at server startup
- [ ] Implement preset lookup by name
- [ ] Add preset validation on load
- [ ] Test preset loading from config.toml
- [ ] Document preset format in config.toml comments

## Section 4: Configuration Management

- [ ] Implement `create_pane_claude_config()` in session/pane.rs
- [ ] Load session config from session's CLAUDE_CONFIG_DIR
- [ ] Look up preset if specified
- [ ] Merge configs in precedence order:
  - [ ] Session config (base)
  - [ ] Preset config (if specified)
  - [ ] Pane-specific config (if specified)
  - [ ] Model override (if specified)
- [ ] Validate merged config
- [ ] Write merged config to `{pane_config_dir}/config.toml`
- [ ] Log configuration decisions (which preset, overrides, etc.)
- [ ] Handle config errors gracefully
- [ ] Test config merging with various combinations
- [ ] Test backward compatibility (no config specified)

## Section 5: MCP Tool Updates

- [ ] Add `model` parameter to ccmux_create_pane tool
- [ ] Add `config` parameter (JSON object)
- [ ] Add `preset` parameter (string)
- [ ] Update tool schema in mcp/tools.rs
- [ ] Update parameter descriptions
- [ ] Add parameter validation
- [ ] Handle mutual exclusivity (preset vs config vs model)
- [ ] Pass parameters through to CreatePane message
- [ ] Update tool help text
- [ ] Test MCP tool with new parameters

## Section 6: Testing

- [ ] Unit test: ClaudeConfig validation
- [ ] Unit test: Config merging with all precedence levels
- [ ] Unit test: Preset loading and lookup
- [ ] Unit test: TOML serialization/deserialization
- [ ] Integration test: Create pane with model override
- [ ] Integration test: Create pane with preset
- [ ] Integration test: Create pane with full config
- [ ] Integration test: Backward compatibility (no config)
- [ ] Integration test: Invalid model name handling
- [ ] Integration test: Invalid preset name handling
- [ ] Manual test: Verify Claude Code respects config
- [ ] Manual test: Multiple panes with different models

## Section 7: Documentation

- [ ] Document `--claude-model` in CLI help (if CLI support added)
- [ ] Document MCP tool parameters in MCP bridge docs
- [ ] Document configuration options in ccmux docs
- [ ] Document preset system and built-in presets
- [ ] Add example configurations to docs
- [ ] Add example MCP tool calls
- [ ] Document use cases:
  - [ ] Cost optimization with Haiku workers
  - [ ] Specialized agents (Opus for architecture)
  - [ ] Mixed model workflows
- [ ] Update architecture documentation
- [ ] Add troubleshooting section

## Verification Tasks

- [ ] Verify all acceptance criteria from PROMPT.md
- [ ] Verify config files are created correctly
- [ ] Verify Claude Code picks up per-pane config
- [ ] Verify preset system works end-to-end
- [ ] Verify backward compatibility
- [ ] Update feature_request.json status
- [ ] Document completion in comments.md

## Completion Checklist

- [ ] All implementation tasks complete
- [ ] All tests passing
- [ ] Documentation complete
- [ ] PLAN.md reflects final implementation
- [ ] Ready for review/merge

## Notes

**Config Precedence Testing Matrix:**

| Session | Preset | Pane | Model Flag | Expected Result |
|---------|--------|------|------------|-----------------|
| sonnet  | -      | -    | -          | sonnet          |
| sonnet  | haiku  | -    | -          | haiku (preset)  |
| sonnet  | haiku  | opus | -          | opus (pane)     |
| sonnet  | -      | -    | haiku      | haiku (flag)    |

Ensure all combinations are tested.

---
*Check off tasks as you complete them. Update status field above.*
