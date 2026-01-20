# FEAT-105: Universal Agent Presets

**Priority**: P1
**Component**: config/presets
**Effort**: Medium
**Status**: new

## Summary

Extend the preset system (FEAT-071) to support any agent harness, not just Claude. Presets should be able to configure:

1. **Which harness** to use (claude, gemini, codex, shell, custom)
2. **Harness-specific configuration** (model, flags, system prompt, etc.)

This aligns with ccmux's agent-agnostic "dumb pipe" philosophy (ADR-001).

## Current State

FEAT-071 presets only configure Claude:

```toml
[presets.haiku-worker]
model = "claude-3-5-haiku"
context_limit = 50000
```

This is limiting - what if you want a Gemini watchdog or a Codex worker?

## Proposed Schema

```toml
[presets.watchdog]
harness = "claude"
description = "Cheap monitoring agent"

[presets.watchdog.config]
model = "haiku"
# Claude-specific: could include system_prompt, flags, etc.

[presets.gemini-worker]
harness = "gemini"
description = "Gemini worker for parallel tasks"

[presets.gemini-worker.config]
model = "gemini-2.5-pro"
# Gemini-specific options

[presets.codex-reviewer]
harness = "codex"
description = "OpenAI Codex for code review"

[presets.codex-reviewer.config]
model = "o3"
# Codex-specific options

[presets.shell]
harness = "shell"
description = "Plain shell, no agent"

[presets.shell.config]
command = "/bin/zsh"
```

## Harness Types

| Harness | Launch Command | Config Options |
|---------|---------------|----------------|
| `claude` | `claude [flags]` | model, system_prompt, dangerously_skip_permissions, allowed_tools |
| `gemini` | `gemini [flags]` | model, system_prompt |
| `codex` | `codex [flags]` | model, system_prompt |
| `shell` | `$SHELL` or custom | command, args, env |
| `custom` | User-defined | command, args, env |

## Rust Schema

```rust
/// Universal agent preset
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentPreset {
    /// Harness type: "claude", "gemini", "codex", "shell", "custom"
    pub harness: String,

    /// Human-readable description
    pub description: Option<String>,

    /// Harness-specific configuration
    #[serde(default)]
    pub config: HarnessConfig,
}

/// Harness-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum HarnessConfig {
    Claude(ClaudeHarnessConfig),
    Gemini(GeminiHarnessConfig),
    Codex(CodexHarnessConfig),
    Shell(ShellHarnessConfig),
    Custom(CustomHarnessConfig),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeHarnessConfig {
    pub model: Option<String>,
    pub system_prompt: Option<String>,
    pub dangerously_skip_permissions: Option<bool>,
    pub allowed_tools: Option<Vec<String>>,
    pub context_limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeminiHarnessConfig {
    pub model: Option<String>,
    pub system_prompt: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodexHarnessConfig {
    pub model: Option<String>,
    pub system_prompt: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellHarnessConfig {
    pub command: Option<String>,
    pub args: Option<Vec<String>>,
    pub env: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomHarnessConfig {
    pub command: String,
    pub args: Option<Vec<String>>,
    pub env: Option<HashMap<String, String>>,
}
```

## MCP Tool Changes

`ccmux_create_pane` already has `preset` parameter. Enhance it to:

1. Look up preset by name
2. Determine harness type
3. Build launch command from harness config
4. Spawn pane with appropriate command

```json
{
  "tool": "ccmux_create_pane",
  "input": {
    "preset": "watchdog",
    "cwd": "/home/user/project"
  }
}
```

This would:
1. Find `presets.watchdog` in config
2. See `harness = "claude"`
3. Build: `claude --model haiku`
4. Spawn pane running that command

## Migration

Existing FEAT-071 presets should continue to work:

```toml
# Old format (still works, implies harness = "claude")
[presets.haiku-worker]
model = "claude-3-5-haiku"

# New format (explicit)
[presets.haiku-worker]
harness = "claude"
[presets.haiku-worker.config]
model = "claude-3-5-haiku"
```

If `harness` is omitted, default to `"claude"` for backwards compatibility.

## Orchestration Skill Integration

The `/orchestrate` skill should use presets:

```markdown
/orchestrate spawn bug-066 --preset worker
/orchestrate monitor start --preset watchdog
```

Default presets if not specified:
- Workers: `worker` preset (falls back to claude if undefined)
- Watchdog: `watchdog` preset (falls back to claude haiku if undefined)

## Acceptance Criteria

- [ ] `AgentPreset` schema supports multiple harness types
- [ ] Backwards compatible with FEAT-071 Claude-only presets
- [ ] `ccmux_create_pane(preset: "name")` spawns correct harness with config
- [ ] Claude, Gemini, Codex, shell harnesses implemented
- [ ] Custom harness allows arbitrary commands
- [ ] `/orchestrate` skill uses presets for workers and watchdog
- [ ] Documentation updated with preset examples
- [ ] Tests for each harness type
- [ ] `DelegationConfig` schema with strategy and pool
- [ ] `ccmux_select_worker` MCP tool returns preset according to delegation strategy
- [ ] Random and round-robin strategies implemented

## Delegation Strategy

Orchestrators need a way to select which preset to use when spawning workers. Add a `[delegation]` section:

```toml
[delegation]
strategy = "random"  # "random", "round-robin", "weighted"
pool = ["worker", "gemini-worker"]  # preset names to select from

# Future: weighted strategy
# [delegation.weights]
# worker = 0.7
# gemini-worker = 0.3
```

### Rust Schema Addition

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DelegationConfig {
    /// Strategy: "random", "round-robin", "weighted"
    pub strategy: String,

    /// Pool of preset names to select from
    pub pool: Vec<String>,

    /// Optional weights for weighted strategy
    pub weights: Option<HashMap<String, f64>>,
}
```

### MCP Tool Addition

Add `ccmux_select_worker` tool for orchestrators:

```json
{
  "tool": "ccmux_select_worker",
  "input": {}
}
// Returns: { "preset": "gemini-worker", "harness": "gemini" }
```

This reads the delegation config and returns the next preset according to strategy.

## Example Config

```toml
# ~/.ccmux/config.toml

[delegation]
strategy = "random"
pool = ["worker", "gemini-worker"]

[presets.watchdog]
harness = "claude"
description = "Haiku-powered monitoring agent"
[presets.watchdog.config]
model = "haiku"
dangerously_skip_permissions = true

[presets.worker]
harness = "claude"
description = "Standard Claude worker"
[presets.worker.config]
model = "sonnet"

[presets.gemini-fast]
harness = "gemini"
description = "Gemini for quick tasks"
[presets.gemini-fast.config]
model = "gemini-2.5-flash"

[presets.gemini-worker]
harness = "gemini"
description = "Gemini worker for parallel tasks"
[presets.gemini-worker.config]
model = "gemini-2.5-pro"

[presets.reviewer]
harness = "codex"
description = "Codex for code review"
[presets.reviewer.config]
model = "o3"
```

## Related

- FEAT-071: Claude configuration presets (original implementation)
- FEAT-098: Gemini agent detection
- FEAT-101: Codex CLI detection
- FEAT-104: Watchdog orchestration skill
- ADR-001: Dumb pipe strategy (agent-agnostic design)
