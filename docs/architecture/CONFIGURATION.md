# Configuration System

> Hot-reload configuration with lock-free access

## Overview

fugue supports hot-reloading of configuration changes without restart. The system uses file watching with debouncing and lock-free atomic config swapping to ensure responsive UI while allowing runtime configuration changes.

## Configuration Files

### File Locations

```
~/.fugue/config/
├── fugue.toml        # User configuration
└── themes/
    ├── default.toml  # Built-in theme
    └── custom.toml   # User themes
```

### Search Path

Configuration is loaded in order (later overrides earlier):

1. Built-in defaults (compiled in)
2. `/etc/fugue/fugue.toml` (system-wide)
3. `~/.fugue/config/fugue.toml` (user)
4. `$FUGUE_CONFIG` (environment override)

## Configuration Schema

### Full Configuration

```toml
# ~/.fugue/config/fugue.toml

[general]
# Default shell to spawn in new panes
default_shell = "/bin/bash"

# Maximum session depth (Claude spawning Claude)
max_depth = 5

# Prefix key (like tmux's Ctrl-b)
prefix_key = "Ctrl-a"

[appearance]
# Theme name or path
theme = "default"

# Status bar position
status_position = "bottom"

# Pane border style: single, double, rounded, none
border_style = "rounded"

# Show pane titles in borders
show_pane_titles = true

[colors]
# Status bar colors
status_bg = "#282c34"
status_fg = "#abb2bf"

# Active pane border
active_border = "#61afef"

# Inactive pane border
inactive_border = "#5c6370"

# Claude state indicators
claude_thinking = "#e5c07b"
claude_idle = "#98c379"
claude_error = "#e06c75"

[keybindings]
# Split panes
split_horizontal = "prefix %"
split_vertical = "prefix \""

# Navigate panes
focus_left = "prefix h"
focus_right = "prefix l"
focus_up = "prefix k"
focus_down = "prefix j"

# Resize panes
resize_left = "prefix H"
resize_right = "prefix L"
resize_up = "prefix K"
resize_down = "prefix J"

# Session management
new_session = "prefix c"
detach = "prefix d"
list_sessions = "prefix s"

# Copy mode
enter_copy_mode = "prefix ["
paste = "prefix ]"

[terminal]
# Scrollback buffer size (lines)
scrollback_lines = 10000

# Render throttle (60 fps = 16ms)
render_interval_ms = 16

# Parser timeout for malformed sequences
parser_timeout_secs = 5

[claude]
# Enable Claude Code detection
detection_enabled = true

# Detection method: pty, stream_json, visual
detection_method = "pty"

# Show Claude state in status bar
show_status = true

# Auto-resume crashed Claude sessions
auto_resume = true

[persistence]
# Checkpoint interval
checkpoint_interval_secs = 30

# Maximum WAL size before rotation
max_wal_size_mb = 128

# Lines to save in screen snapshot
screen_snapshot_lines = 500

[mcp]
# Enable MCP server for Claude integration
enabled = false

# Socket path for MCP server
socket_path = "~/.fugue/mcp.sock"

### Agent Presets (FEAT-105)

Universal presets define reusable configurations for different agent harnesses.

```toml
# Generic Claude worker
[presets.worker]
harness = "claude"
description = "Standard Claude worker"
[presets.worker.config]
model = "claude-3-5-sonnet-20241022"
context_limit = 100000

# Low-cost watchdog
[presets.watchdog]
harness = "claude"
description = "Haiku monitoring agent"
[presets.watchdog.config]
model = "claude-3-haiku-20240307"
dangerously_skip_permissions = true

# Gemini worker
[presets.gemini]
harness = "gemini"
description = "Gemini 2.5 Pro worker"
[presets.gemini.config]
model = "gemini-2.5-pro"

# Shell preset (no agent)
[presets.build-env]
harness = "shell"
description = "Build environment with specific variables"
[presets.build-env.config]
command = "/bin/bash"
[presets.build-env.config.env]
RUST_LOG = "debug"
CARGO_INCREMENTAL = "0"

# Custom harness
[presets.custom-tool]
harness = "custom"
[presets.custom-tool.config]
command = "my-custom-tool"
args = ["--verbose", "--mode=agent"]
```

#### Harness Types

| Harness | Description | Config Options |
|---------|-------------|----------------|
| `claude` | Anthropic's Claude Code | `model`, `system_prompt`, `context_limit`, `allowed_tools`, `dangerously_skip_permissions` |
| `gemini` | Google's Gemini CLI | `model`, `system_prompt` |
| `codex` | OpenAI Codex CLI | `model`, `system_prompt` |
| `shell` | Standard shell | `command`, `args`, `env` |
| `custom` | Custom executable | `command`, `args`, `env` |

*Note: Legacy Claude-only presets (without `harness` field) are still supported for backward compatibility.*

```

## Change Categories

Not all configuration changes can be applied at runtime.

### Hot-Reloadable (Immediate)

| Category | Examples | Notes |
|----------|----------|-------|
| Colors | All `[colors]` settings | UI updates immediately |
| Keybindings | All `[keybindings]` settings | Applied to next keypress |
| Status bar | `show_status`, `status_position` | UI updates immediately |
| Claude | `detection_enabled`, `show_status` | Applied immediately |
| Appearance | `theme`, `border_style` | UI updates immediately |

### Restart-Required

| Category | Examples | Notes |
|----------|----------|-------|
| Default shell | `default_shell` | Only affects new panes |
| Terminal settings | `scrollback_lines` | Only affects new panes |
| MCP | `mcp.enabled`, `mcp.socket_path` | Requires server restart |
| Prefix key | `prefix_key` | Applied after reattach |

### Session-Restart-Required

| Category | Examples | Notes |
|----------|----------|-------|
| Parser timeout | `parser_timeout_secs` | Per-pane on creation |

## File Watching

### Implementation

```rust
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use notify_debouncer_full::{new_debouncer, DebouncedEvent};

impl ConfigWatcher {
    pub fn new(config_dir: PathBuf) -> Result<Self> {
        let (tx, rx) = std::sync::mpsc::channel();

        // Create debounced watcher (50ms debounce)
        let mut debouncer = new_debouncer(
            Duration::from_millis(50),
            None,
            move |result: DebounceEventResult| {
                if let Ok(events) = result {
                    let _ = tx.send(events);
                }
            }
        )?;

        // Watch directory, not file (handles atomic renames)
        debouncer.watcher().watch(&config_dir, RecursiveMode::NonRecursive)?;

        Ok(Self {
            debouncer,
            receiver: rx,
            config_dir,
        })
    }

    pub async fn run(&mut self, config: Arc<ArcSwap<AppConfig>>) {
        loop {
            match self.receiver.recv() {
                Ok(events) => {
                    for event in events {
                        if Self::is_config_change(&event) {
                            self.handle_change(&config).await;
                        }
                    }
                }
                Err(_) => break,  // Channel closed
            }
        }
    }

    fn is_config_change(event: &DebouncedEvent) -> bool {
        match event.kind {
            EventKind::Create(_) | EventKind::Modify(_) => {
                event.paths.iter().any(|p| {
                    p.file_name() == Some("fugue.toml".as_ref())
                })
            }
            _ => false,
        }
    }

    async fn handle_change(&self, config: &Arc<ArcSwap<AppConfig>>) {
        match self.load_and_validate() {
            Ok(new_config) => {
                let old_config = config.load();

                // Check for restart-required changes
                let warnings = Self::check_restart_required(&old_config, &new_config);
                for warning in warnings {
                    log::warn!("{}", warning);
                }

                // Swap atomically
                config.store(Arc::new(new_config));
                log::info!("Configuration reloaded");
            }
            Err(e) => {
                log::error!("Configuration error, keeping previous: {}", e);
                // Don't apply invalid config
            }
        }
    }
}
```

### Why Watch Directory?

Editors use atomic saves (write temp file, rename):

```
1. Editor writes: fugue.toml.tmp
2. Editor renames: fugue.toml.tmp -> fugue.toml
```

If we watch the file directly:
- File is deleted during rename (triggers delete event)
- Watcher may be removed
- New file created (may not trigger)

Watching the directory catches the rename event reliably.

## Lock-Free Config Access

### ArcSwap Pattern

```rust
use arc_swap::ArcSwap;
use once_cell::sync::Lazy;

// Global config with lock-free access
static CONFIG: Lazy<ArcSwap<AppConfig>> = Lazy::new(|| {
    ArcSwap::from_pointee(load_default_config())
});

impl Server {
    fn render_frame(&self) {
        // Lock-free read - critical for 60fps
        let cfg = CONFIG.load();

        self.draw_status_bar(&cfg.colors);
        self.draw_borders(&cfg.appearance);
    }
}

impl ConfigWatcher {
    fn reload(&self, new_config: AppConfig) {
        // Atomic swap - readers get consistent view
        CONFIG.store(Arc::new(new_config));
    }
}
```

### Performance Characteristics

| Operation | Cost | Use Case |
|-----------|------|----------|
| `load()` | ~15ns | Every frame, every event |
| `store()` | ~50ns | Config reload (rare) |
| Memory | 2 Arc pointers | Minimal overhead |

## Validation

### Schema Validation

```rust
use serde_valid::Validate;

#[derive(Deserialize, Validate)]
pub struct AppConfig {
    #[validate(min_length = 1)]
    pub default_shell: String,

    #[validate(minimum = 1, maximum = 10)]
    pub max_depth: u32,

    #[validate]
    pub colors: ColorConfig,

    #[validate]
    pub persistence: PersistenceConfig,
}

#[derive(Deserialize, Validate)]
pub struct PersistenceConfig {
    #[validate(minimum = 5, maximum = 300)]
    pub checkpoint_interval_secs: u32,

    #[validate(minimum = 1, maximum = 1024)]
    pub max_wal_size_mb: u32,
}

impl ConfigLoader {
    pub fn load_and_validate(path: &Path) -> Result<AppConfig> {
        let content = std::fs::read_to_string(path)?;
        let config: AppConfig = toml::from_str(&content)?;

        // Validate constraints
        config.validate()?;

        // Additional semantic validation
        Self::validate_paths(&config)?;
        Self::validate_keybindings(&config)?;

        Ok(config)
    }
}
```

### Semantic Validation

```rust
impl ConfigLoader {
    fn validate_paths(config: &AppConfig) -> Result<()> {
        // Check shell exists
        let shell = Path::new(&config.default_shell);
        if !shell.exists() {
            return Err(ConfigError::InvalidShell(config.default_shell.clone()));
        }

        // Check theme exists
        if !config.appearance.theme.starts_with('/') {
            let theme_path = config_dir()
                .join("themes")
                .join(&config.appearance.theme)
                .with_extension("toml");

            if !theme_path.exists() {
                return Err(ConfigError::ThemeNotFound(config.appearance.theme.clone()));
            }
        }

        Ok(())
    }

    fn validate_keybindings(config: &AppConfig) -> Result<()> {
        // Check for conflicts
        let mut bindings: HashMap<String, String> = HashMap::new();

        for (action, binding) in &config.keybindings {
            if let Some(existing) = bindings.get(binding) {
                return Err(ConfigError::KeybindingConflict {
                    binding: binding.clone(),
                    action1: existing.clone(),
                    action2: action.clone(),
                });
            }
            bindings.insert(binding.clone(), action.clone());
        }

        Ok(())
    }
}
```

## Error Handling

### Invalid Configuration

When configuration is invalid, the system:
1. Logs detailed error message
2. Keeps previous (working) configuration
3. Shows user notification in status bar

```rust
impl ConfigWatcher {
    async fn handle_change(&self, config: &Arc<ArcSwap<AppConfig>>) {
        match self.load_and_validate() {
            Ok(new_config) => {
                config.store(Arc::new(new_config));
                self.notify_clients(ConfigNotification::Reloaded).await;
            }
            Err(e) => {
                log::error!("Config error: {}", e);
                self.notify_clients(ConfigNotification::Error {
                    message: e.to_string(),
                }).await;
            }
        }
    }
}
```

### Client Notification

```rust
// Server → Client message
ServerMessage::ConfigNotification {
    kind: ConfigNotificationKind::Error,
    message: "Invalid config: max_depth must be between 1 and 10",
}

// Client displays in status bar for 5 seconds
// "⚠ Config error: max_depth must be between 1 and 10"
```

## Default Configuration

Built-in defaults ensure fugue works without any config file:

```rust
impl Default for AppConfig {
    fn default() -> Self {
        Self {
            general: GeneralConfig {
                default_shell: std::env::var("SHELL")
                    .unwrap_or_else(|_| "/bin/sh".into()),
                max_depth: 5,
                prefix_key: "Ctrl-a".into(),
            },
            appearance: AppearanceConfig {
                theme: "default".into(),
                status_position: StatusPosition::Bottom,
                border_style: BorderStyle::Rounded,
                show_pane_titles: true,
            },
            colors: ColorConfig::default(),
            keybindings: KeybindingConfig::default(),
            terminal: TerminalConfig {
                scrollback_lines: 10000,
                render_interval_ms: 16,
                parser_timeout_secs: 5,
            },
            claude: ClaudeConfig {
                detection_enabled: true,
                detection_method: DetectionMethod::Pty,
                show_status: true,
                auto_resume: true,
            },
            persistence: PersistenceConfig {
                checkpoint_interval_secs: 30,
                max_wal_size_mb: 128,
                screen_snapshot_lines: 500,
            },
            mcp: McpConfig {
                enabled: false,
                socket_path: "~/.fugue/mcp.sock".into(),
            },
        }
    }
}
```

## Configuration Commands

### Runtime Commands

```
:config reload         # Force reload config
:config show           # Show current config
:config set <k> <v>    # Set value (session only)
:config reset          # Reset to defaults
```

### CLI Commands

```bash
# Generate default config
fugue config init

# Validate config
fugue config check

# Show effective config
fugue config show

# Edit config in $EDITOR
fugue config edit
```

## Related Documents

- [ARCHITECTURE.md](./ARCHITECTURE.md) - System overview
- [PERSISTENCE.md](./PERSISTENCE.md) - State persistence settings
- [CRATE_STRUCTURE.md](./CRATE_STRUCTURE.md) - Where config code lives
