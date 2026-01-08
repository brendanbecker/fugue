# ADR-003: Concurrent Claude Session Isolation

## Status

**Accepted** - 2026-01-07

## Context

Claude Code writes state to `~/.claude.json` at approximately 1.5 writes per second during active use. When running multiple Claude instances (e.g., in different ccmux panes), this causes race conditions:

```
Time    Instance A              Instance B              ~/.claude.json
────────────────────────────────────────────────────────────────────────
T1      write session:abc       -                       {session: "abc"}
T2      -                       write session:xyz       {session: "xyz"}
T3      read → gets "xyz"       -                       {session: "xyz"}
        ↑ WRONG!
```

This leads to:
- Session confusion (wrong session resumed)
- State corruption
- Lost context between interactions

Three isolation strategies were identified:

### Option A: HOME Directory Isolation

Set a unique `HOME` for each Claude instance:

```bash
HOME=/tmp/claude-instance-1 claude
```

**Pros:**
- Complete isolation
- Simple to implement

**Cons:**
- Breaks shell integrations (~/.bashrc, ~/.zshrc)
- Loses access to user tools and configs
- Poor user experience
- May break other tools expecting real HOME

### Option B: CLAUDE_CONFIG_DIR Environment Variable

Set `CLAUDE_CONFIG_DIR` to a per-instance directory:

```bash
CLAUDE_CONFIG_DIR=~/.ccmux/claude-configs/pane-abc123 claude
```

**Pros:**
- Only isolates Claude's config, not shell
- Shell works normally
- User tools accessible
- Clean separation

**Cons:**
- Relies on undocumented environment variable
- May change in future Claude versions

### Option C: Session ID Flags

Use Claude's `--session-id` flag:

```bash
claude --session-id unique-id-per-pane
```

**Pros:**
- Official CLI interface
- No environment manipulation

**Cons:**
- May not prevent all ~/.claude.json writes
- Doesn't isolate all state
- Limited control

## Decision

**Use CLAUDE_CONFIG_DIR** as the primary isolation mechanism, with HOME isolation as a fallback option (configurable).

## Rationale

### Why CLAUDE_CONFIG_DIR?

1. **Minimal disruption**: Shell environment preserved
   - ~/.bashrc, ~/.zshrc loaded normally
   - User's PATH, aliases, functions available
   - Git config, SSH keys accessible

2. **Clean isolation**: Only Claude's state isolated
   - Each pane gets: `~/.ccmux/claude-configs/pane-<uuid>/`
   - Contains: `.claude.json`, session files
   - No interference between instances

3. **Easy cleanup**: Config dirs scoped to panes
   - Can delete old dirs on pane close
   - Clear ownership model

4. **Testable**: Environment variable well-defined
   - Easy to verify in tests
   - Clear failure mode if unsupported

### Implementation

```rust
impl Pane {
    /// Get isolated config directory for this pane's Claude instance
    pub fn claude_config_dir(&self) -> PathBuf {
        ccmux_utils::state_dir()
            .join("claude-configs")
            .join(format!("pane-{}", self.id))
    }

    /// Spawn Claude with isolation
    pub fn spawn_claude(&mut self, resume_id: Option<&str>) -> Result<()> {
        let config_dir = self.claude_config_dir();
        std::fs::create_dir_all(&config_dir)?;

        let mut cmd = std::process::Command::new("claude");

        // Primary isolation via config dir
        cmd.env("CLAUDE_CONFIG_DIR", &config_dir);

        // Optional: session ID for additional clarity
        let session_id = resume_id.unwrap_or(&self.id.to_string());
        cmd.arg("--session-id").arg(session_id);

        if let Some(id) = resume_id {
            cmd.arg("--resume").arg(id);
        }

        self.pty.spawn(cmd)?;
        Ok(())
    }
}
```

### Directory Structure

```
~/.ccmux/
└── claude-configs/
    ├── pane-550e8400-e29b-41d4-a716-446655440000/
    │   ├── .claude.json          # This pane's session state
    │   └── projects/
    │       └── <encoded-path>/
    │           └── <session>.jsonl
    ├── pane-6ba7b810-9dad-11d1-80b4-00c04fd430c8/
    │   └── ...
    └── ...
```

### Fallback: HOME Isolation

For cases where CLAUDE_CONFIG_DIR doesn't work:

```rust
impl Pane {
    pub fn spawn_claude_with_home_isolation(&mut self) -> Result<()> {
        let isolated_home = ccmux_utils::state_dir()
            .join("claude-homes")
            .join(format!("pane-{}", self.id));

        std::fs::create_dir_all(&isolated_home)?;

        // Copy minimal shell config
        self.setup_isolated_home(&isolated_home)?;

        let mut cmd = std::process::Command::new("claude");
        cmd.env("HOME", &isolated_home);

        self.pty.spawn(cmd)?;
        Ok(())
    }

    fn setup_isolated_home(&self, home: &Path) -> Result<()> {
        let real_home = dirs::home_dir().unwrap();

        // Symlink essential configs
        for item in &[".bashrc", ".zshrc", ".gitconfig", ".ssh"] {
            let src = real_home.join(item);
            let dst = home.join(item);
            if src.exists() && !dst.exists() {
                std::os::unix::fs::symlink(&src, &dst)?;
            }
        }

        Ok(())
    }
}
```

## Consequences

### Positive

- Multiple Claude instances work correctly
- Shell environment preserved
- Clean resource management
- Configurable per-user preference

### Negative

- Relies on undocumented environment variable
- May need updates if Claude changes
- Extra disk space for config copies
- Cleanup needed on pane close

### Configuration

```toml
# ~/.ccmux/config/ccmux.toml

[claude.isolation]
# Isolation method: config_dir, home, none
method = "config_dir"

# Cleanup config dirs when pane closes
cleanup_on_close = true

# For home isolation: items to symlink
home_symlinks = [".bashrc", ".zshrc", ".gitconfig", ".ssh"]
```

## Monitoring

Track isolation effectiveness:

```rust
impl ClaudeMonitor {
    pub fn check_isolation_health(&self) -> IsolationHealth {
        let mut health = IsolationHealth::default();

        for pane in self.claude_panes() {
            let config_dir = pane.claude_config_dir();
            let claude_json = config_dir.join(".claude.json");

            if claude_json.exists() {
                // Read and verify session belongs to this pane
                if let Ok(state) = self.read_claude_state(&claude_json) {
                    if state.session_id != pane.expected_session_id() {
                        health.mismatches.push(pane.id);
                    }
                }
            }
        }

        health
    }
}
```

## Future Considerations

1. **Official API**: If Anthropic provides official isolation, migrate
2. **Container isolation**: For maximum isolation, run in containers
3. **Session namespacing**: If CLAUDE_CONFIG_DIR removed, use session ID prefix

## References

- Research: `docs/research/SYNTHESIS.md` Section 3.3
- Research: `docs/research/parsed/chatgpt_metadata.json` (isolation strategies)
- Claude Code documentation on session management
