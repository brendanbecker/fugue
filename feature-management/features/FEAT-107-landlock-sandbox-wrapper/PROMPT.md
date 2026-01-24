# FEAT-107: Landlock Sandbox Wrapper Integration

## Overview

Integrate `fugue-sandbox` as a transparent wrapper for pane commands, enabling per-pane Landlock filesystem sandboxing. When enabled, panes are restricted to their working directory plus explicitly allowed paths.

## Motivation

- **Worker agent safety**: Agents with `--dangerously-skip-permissions` can be safely isolated to their worktree
- **Defense in depth**: Even if an agent misbehaves, it cannot access files outside allowed paths
- **Zero-trust orchestration**: Orchestrators can spawn untrusted workers without risk to the broader filesystem

## Current State

- `fugue-sandbox` binary exists with hardcoded paths
- FEAT-081 spec exists but was not fully implemented
- Landlock works on the system (Linux 5.15, tested)

## Requirements

### Phase 1: Enhance fugue-sandbox CLI

Add configurable path arguments to `fugue-sandbox`:

```bash
fugue-sandbox \
  --allow-ro ~/.local \
  --allow-ro ~/.cargo \
  --allow-ro ~/.rustup \
  --allow-rw /tmp \
  -- claude --dangerously-skip-permissions
```

**CLI Arguments:**
- `--allow-ro <path>` - Add read-only path (repeatable)
- `--allow-rw <path>` - Add read-write path (repeatable)
- `--cwd-rw` - Allow read-write to current working directory (default: true)
- `--no-cwd-rw` - Disable CWD write access (read-only CWD)
- `--` - Separator before command

**Default allowed paths (always RO):**
- `/bin`, `/usr`, `/lib`, `/lib64`, `/etc`, `/proc`, `/sys`

**Default allowed paths (always RW):**
- `/tmp`, `/dev`, `/run`

### Phase 2: Add Sandbox Config to PtyConfig

```rust
// In fugue-server/src/pty/config.rs

#[derive(Debug, Clone, Default)]
pub struct SandboxConfig {
    /// Enable Landlock sandboxing via fugue-sandbox wrapper
    pub enabled: bool,
    /// Additional read-only paths beyond defaults
    pub allow_ro: Vec<PathBuf>,
    /// Additional read-write paths beyond CWD
    pub allow_rw: Vec<PathBuf>,
    /// Allow write access to CWD (default: true)
    pub cwd_write: bool,
}

impl PtyConfig {
    pub fn with_sandbox(mut self, config: SandboxConfig) -> Self {
        self.sandbox = Some(config);
        self
    }
}
```

### Phase 3: Integrate Wrapper in PtyManager

In `fugue-server/src/pty/manager.rs`, when sandbox is enabled:

```rust
pub fn spawn(&mut self, pane_id: Uuid, config: PtyConfig) -> Result<&PtyHandle> {
    let (command, args) = if let Some(ref sandbox) = config.sandbox {
        if sandbox.enabled {
            self.wrap_with_sandbox(&config, sandbox)
        } else {
            (config.command.clone(), config.args.clone())
        }
    } else {
        (config.command.clone(), config.args.clone())
    };

    // ... rest of spawn logic using wrapped command
}

fn wrap_with_sandbox(&self, config: &PtyConfig, sandbox: &SandboxConfig) -> (String, Vec<String>) {
    let mut args = Vec::new();

    for path in &sandbox.allow_ro {
        args.push("--allow-ro".to_string());
        args.push(path.to_string_lossy().to_string());
    }

    for path in &sandbox.allow_rw {
        args.push("--allow-rw".to_string());
        args.push(path.to_string_lossy().to_string());
    }

    if !sandbox.cwd_write {
        args.push("--no-cwd-rw".to_string());
    }

    args.push("--".to_string());
    args.push(config.command.clone());
    args.extend(config.args.clone());

    ("fugue-sandbox".to_string(), args)
}
```

### Phase 4: Preset Integration

Add sandbox config to presets in `~/.fugue/config.toml`:

```toml
[presets.sandboxed-worker]
command = "claude --dangerously-skip-permissions"
tags = ["worker"]

[presets.sandboxed-worker.sandbox]
enabled = true
allow_ro = ["~/.local", "~/.cargo", "~/.rustup", "~/.nvm"]
allow_rw = []
cwd_write = true
```

### Phase 5: MCP Tool Support

Add `sandbox` parameter to `fugue_create_pane` and `fugue_create_session`:

```json
{
  "tool": "fugue_create_pane",
  "input": {
    "command": "claude --dangerously-skip-permissions",
    "cwd": "/path/to/worktree",
    "sandbox": {
      "enabled": true,
      "allow_ro": ["~/.local", "~/.cargo"],
      "allow_rw": []
    }
  }
}
```

Or via preset:
```json
{
  "tool": "fugue_create_pane",
  "preset": "sandboxed-worker",
  "cwd": "/path/to/worktree"
}
```

## Tasks

### Section 1: Enhance fugue-sandbox CLI
- [ ] Add `clap` arguments for `--allow-ro`, `--allow-rw`, `--cwd-rw`, `--no-cwd-rw`
- [ ] Parse paths and expand `~` to home directory
- [ ] Update `apply_landlock()` to use configurable paths
- [ ] Add `--help` documentation
- [ ] Test with various path configurations

### Section 2: PtyConfig Sandbox Support
- [ ] Add `SandboxConfig` struct to `fugue-server/src/pty/config.rs`
- [ ] Add `with_sandbox()` builder method
- [ ] Add sandbox field to `PtyConfig`

### Section 3: PtyManager Integration
- [ ] Add `wrap_with_sandbox()` helper method
- [ ] Modify `spawn()` to detect and apply sandbox wrapper
- [ ] Ensure `fugue-sandbox` binary is found (PATH or absolute)
- [ ] Add tests for sandbox wrapping logic

### Section 4: Preset Integration
- [ ] Extend preset parsing to include sandbox config
- [ ] Add `SandboxConfig` deserialization from TOML
- [ ] Test preset-based sandbox spawning

### Section 5: MCP Tool Integration
- [ ] Add `sandbox` parameter to `fugue_create_pane` schema
- [ ] Add `sandbox` parameter to `fugue_create_session` schema
- [ ] Parse and apply sandbox config in handlers
- [ ] Update tool documentation

### Section 6: Testing & Documentation
- [ ] Integration test: sandboxed pane cannot write outside CWD
- [ ] Integration test: sandboxed pane can read allowed RO paths
- [ ] Integration test: Claude runs successfully in sandbox
- [ ] Update AGENTS.md with sandbox usage
- [ ] Add example preset configurations

## Acceptance Criteria

- [ ] `fugue-sandbox --allow-ro ~/.local -- cat ~/.bashrc` fails (not in allowed paths)
- [ ] `fugue-sandbox --allow-ro ~/.local -- ls ~/.local/bin` succeeds
- [ ] Pane created with `sandbox.enabled = true` cannot write to `~`
- [ ] Pane created with `preset: "sandboxed-worker"` is sandboxed
- [ ] Claude with `--dangerously-skip-permissions` runs in sandbox
- [ ] Graceful fallback message if Landlock not supported

## Dependencies

- Landlock-enabled Linux kernel (5.13+)
- `fugue-sandbox` binary in PATH or configured location

## Security Considerations

- Sandbox is **opt-in** - existing behavior unchanged
- Default RO paths include system directories needed for execution
- CWD write is enabled by default for practical use
- Agent binaries (claude, gemini) often live in ~/.local - must be allowed RO
- Network access is NOT restricted (Landlock v1 doesn't support network)
