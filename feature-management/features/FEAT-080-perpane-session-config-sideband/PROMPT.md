# FEAT-080: Per-Pane/Session Configuration via Sideband Commands

## Overview
Enable dynamic, agent-driven configuration of individual panes or sessions when spawning or updating them. This allows fine-grained control over isolation, environment, and runtime behavior without relying on global config or manual user intervention.

## Motivation
- **Dynamic isolation needs**: Different sub-agents may require different working directories, environment variables, resource limits, or security postures.
- **Agent autonomy**: Let the controlling LLM decide isolation parameters at runtime.
- **Security & compliance**: Prevent cross-pane leakage by enforcing per-pane constraints.

## Requirements
- Extend sideband commands (e.g., `spawn`, `create-pane`) to accept an optional `config` payload as JSON.
- Support keys for `cwd`, `env`, `isolate_fs`, `readonly`, `timeout_secs`.
- Optionally support named configuration profiles via `config_ref`.
- Ensure secure parsing and validation of configuration.

## Design
### Sideband Syntax
```xml
<ccmux:spawn pane command="llm chat -m gemini-2.0-flash" config='{"cwd":"/tmp/agent-xyz","env":{"API_KEY":"sk-...","LLM_TIMEOUT":"120"},"isolate":true,"readonly":true,"timeout_secs":300}'>
```

### Supported Config Keys (Initial Set)
```json
{
  "cwd": "/path/to/dir",
  "env": { "VAR1": "value" },
  "isolate_fs": true,
  "readonly": true,
  "timeout_secs": 300,
  "memory_limit_mb": 512,
  "log_path": "/tmp/agent.log",
  "secure_env": ["API_KEY"],
  "profile": "local-only"
}
```

## Tasks
### Section 1: Sideband Parsing
- [ ] Extend sideband parser to recognize `config` attribute in `spawn`/`create` directives.
- [ ] Implement safe JSON parsing using `serde_json`.

### Section 2: Server Implementation
- [ ] Apply configuration before `Command::new()` in `ccmux-server`.
  - [ ] `cwd` handling.
  - [ ] `env` merging/overriding.
  - [ ] `isolate_fs` implementation (tempdir).
  - [ ] `timeout_secs` timer implementation.
- [ ] Implement fallback logging for invalid JSON.

### Section 3: MCP Tools (Optional/Phase 2)
- [ ] Add `ccmux_create_pane_with_config` MCP tool.
- [ ] Add `ccmux_set_pane_config` MCP tool.

### Section 4: Testing & Documentation
- [ ] Add integration tests for configured spawns.
- [ ] Document sideband syntax and config keys in README.

## Acceptance Criteria
- [ ] Can spawn a pane with a specific CWD and ENV via sideband.
- [ ] Can set a timeout for a pane that auto-kills it.
- [ ] Invalid configuration is handled gracefully (warning logged, fallback used).
