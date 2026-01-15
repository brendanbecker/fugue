//! Configuration schema structs

use ccmux_utils::{SessionLogConfig, SessionLogLevel};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Root configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub general: GeneralConfig,
    pub appearance: AppearanceConfig,
    pub colors: ColorConfig,
    pub keybindings: KeybindingConfig,
    pub terminal: TerminalConfig,
    pub claude: ClaudeConfig,
    pub persistence: PersistenceConfig,
    pub session_logging: SessionLoggingConfig,
    pub beads: BeadsConfig,
    /// Claude configuration presets (FEAT-071)
    pub presets: HashMap<String, ClaudePreset>,
}

/// Claude configuration preset (FEAT-071)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ClaudePreset {
    pub model: Option<String>,
    pub context_limit: Option<usize>,
    pub description: Option<String>,
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// Beads integration settings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct BeadsConfig {
    /// Enable auto-detection of .beads/ directory (default: true)
    pub auto_detect: bool,
    /// Auto-set BEADS_DIR when detected (default: true)
    pub auto_set_beads_dir: bool,
    /// Set BEADS_NO_DAEMON for new panes (useful for worktrees)
    pub no_daemon_default: bool,
    /// Query configuration for daemon integration
    pub query: BeadsQueryConfig,
    /// Workflow integration configuration (FEAT-059)
    pub workflow: BeadsWorkflowConfig,
}

impl Default for BeadsConfig {
    fn default() -> Self {
        Self {
            auto_detect: true,
            auto_set_beads_dir: true,
            no_daemon_default: false,
            query: BeadsQueryConfig::default(),
            workflow: BeadsWorkflowConfig::default(),
        }
    }
}

/// Beads workflow integration settings (FEAT-059)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct BeadsWorkflowConfig {
    /// Enable workflow integration tools (default: true)
    pub enabled: bool,
    /// Include current issue ID in status updates (default: true)
    pub show_issue_in_status: bool,
    /// Maximum history entries to keep per pane (default: 100)
    pub max_history_entries: usize,
}

impl Default for BeadsWorkflowConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            show_issue_in_status: true,
            max_history_entries: 100,
        }
    }
}

/// Beads daemon query settings for TUI integration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct BeadsQueryConfig {
    /// Enable daemon connection for ready task queries (default: true)
    pub enabled: bool,
    /// Show ready count in status bar (default: true)
    pub show_ready_count: bool,
    /// Refresh interval for status bar updates in seconds (default: 30)
    pub refresh_interval: u32,
    /// Socket connection timeout in milliseconds (default: 1000)
    pub socket_timeout: u32,
}

impl Default for BeadsQueryConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            show_ready_count: true,
            refresh_interval: 30,
            socket_timeout: 1000,
        }
    }
}

/// Per-session logging settings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SessionLoggingConfig {
    /// Enable per-session logging
    pub enabled: bool,
    /// Default log level for new sessions
    pub default_level: SessionLogLevel,
    /// Maximum log file size in MB before rotation
    pub max_file_size_mb: u64,
    /// Maximum number of rotated log files to keep
    pub max_rotated_files: u32,
    /// Retention period in days for old logs
    pub retention_days: u32,
    /// Separate user actions into audit trail
    pub separate_audit_trail: bool,
}

impl Default for SessionLoggingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            default_level: SessionLogLevel::Signals,
            max_file_size_mb: 10,
            max_rotated_files: 5,
            retention_days: 7,
            separate_audit_trail: true,
        }
    }
}

impl SessionLoggingConfig {
    /// Convert to SessionLogConfig used by SessionLogger
    pub fn to_session_log_config(&self) -> SessionLogConfig {
        SessionLogConfig {
            default_level: self.default_level,
            max_file_size: self.max_file_size_mb * 1024 * 1024,
            max_rotated_files: self.max_rotated_files,
            retention_secs: u64::from(self.retention_days) * 24 * 60 * 60,
            separate_audit_trail: self.separate_audit_trail,
        }
    }
}

/// General settings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GeneralConfig {
    /// Default shell to spawn (used when default_command is not set)
    pub default_shell: String,
    /// Default command to run in new sessions (overrides default_shell if set)
    /// Example: "claude" to start Claude Code in every new session
    pub default_command: Option<String>,
    /// Maximum Claude session depth
    pub max_depth: u32,
    /// Prefix key for commands
    pub prefix_key: String,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            default_shell: std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".into()),
            default_command: None,
            max_depth: 5,
            prefix_key: "Ctrl-a".into(),
        }
    }
}

/// Appearance settings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppearanceConfig {
    /// Theme name or path
    pub theme: String,
    /// Status bar position
    pub status_position: StatusPosition,
    /// Pane border style
    pub border_style: BorderStyle,
    /// Show pane titles
    pub show_pane_titles: bool,
}

impl Default for AppearanceConfig {
    fn default() -> Self {
        Self {
            theme: "default".into(),
            status_position: StatusPosition::Bottom,
            border_style: BorderStyle::Rounded,
            show_pane_titles: true,
        }
    }
}

/// Status bar position
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum StatusPosition {
    Top,
    #[default]
    Bottom,
}

/// Border style
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum BorderStyle {
    Single,
    Double,
    #[default]
    Rounded,
    None,
}

/// Color settings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ColorConfig {
    pub status_bg: String,
    pub status_fg: String,
    pub active_border: String,
    pub inactive_border: String,
    pub claude_thinking: String,
    pub claude_idle: String,
    pub claude_error: String,
}

impl Default for ColorConfig {
    fn default() -> Self {
        Self {
            status_bg: "#282c34".into(),
            status_fg: "#abb2bf".into(),
            active_border: "#61afef".into(),
            inactive_border: "#5c6370".into(),
            claude_thinking: "#e5c07b".into(),
            claude_idle: "#98c379".into(),
            claude_error: "#e06c75".into(),
        }
    }
}

/// Keybinding settings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct KeybindingConfig {
    // Prefix-based bindings (existing)
    pub split_horizontal: String,
    pub split_vertical: String,
    pub focus_left: String,
    pub focus_right: String,
    pub focus_up: String,
    pub focus_down: String,
    pub new_session: String,
    pub detach: String,
    pub list_sessions: String,

    // Quick navigation bindings (no prefix required)
    // Empty string disables the binding
    /// Quick binding for next window (default: Ctrl-PageDown)
    pub next_window_quick: String,
    /// Quick binding for previous window (default: Ctrl-PageUp)
    pub prev_window_quick: String,
    /// Quick binding for next pane in window (default: Ctrl-Shift-PageDown)
    pub next_pane_quick: String,
    /// Quick binding for previous pane (default: Ctrl-Shift-PageUp)
    pub prev_pane_quick: String,
}

impl Default for KeybindingConfig {
    fn default() -> Self {
        Self {
            // Prefix-based bindings
            split_horizontal: "prefix %".into(),
            split_vertical: "prefix \"".into(),
            focus_left: "prefix h".into(),
            focus_right: "prefix l".into(),
            focus_up: "prefix k".into(),
            focus_down: "prefix j".into(),
            new_session: "prefix c".into(),
            detach: "prefix d".into(),
            list_sessions: "prefix s".into(),

            // Quick navigation (no prefix)
            next_window_quick: "Ctrl-PageDown".into(),
            prev_window_quick: "Ctrl-PageUp".into(),
            next_pane_quick: "Ctrl-Shift-PageDown".into(),
            prev_pane_quick: "Ctrl-Shift-PageUp".into(),
        }
    }
}

/// Terminal settings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct TerminalConfig {
    /// Scrollback configuration (per-session-type)
    pub scrollback: ScrollbackConfig,
    /// Render interval (ms)
    pub render_interval_ms: u64,
    /// Parser timeout (seconds)
    pub parser_timeout_secs: u64,
}

impl Default for TerminalConfig {
    fn default() -> Self {
        Self {
            scrollback: ScrollbackConfig::default(),
            render_interval_ms: 16,
            parser_timeout_secs: 5,
        }
    }
}

/// Session type for scrollback configuration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum SessionType {
    /// Default session type
    #[default]
    Default,
    /// Orchestrator sessions (large scrollback)
    Orchestrator,
    /// Worker sessions (minimal scrollback)
    Worker,
}

impl std::fmt::Display for SessionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SessionType::Default => write!(f, "default"),
            SessionType::Orchestrator => write!(f, "orchestrator"),
            SessionType::Worker => write!(f, "worker"),
        }
    }
}

/// Per-session-type scrollback configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ScrollbackConfig {
    /// Default scrollback lines for unspecified session types
    pub default: usize,
    /// Scrollback for orchestrator sessions (large buffer)
    pub orchestrator: usize,
    /// Scrollback for worker sessions (minimal buffer)
    pub worker: usize,
    /// Custom session type overrides
    #[serde(flatten)]
    pub custom: HashMap<String, usize>,
}

impl Default for ScrollbackConfig {
    fn default() -> Self {
        Self {
            default: 1000,
            orchestrator: 50000,
            worker: 500,
            custom: HashMap::new(),
        }
    }
}

impl ScrollbackConfig {
    /// Minimum allowed scrollback lines
    pub const MIN_LINES: usize = 100;
    /// Maximum allowed scrollback lines
    pub const MAX_LINES: usize = 100_000;

    /// Get scrollback lines for a session type
    pub fn lines_for_type(&self, session_type: SessionType) -> usize {
        match session_type {
            SessionType::Default => self.default,
            SessionType::Orchestrator => self.orchestrator,
            SessionType::Worker => self.worker,
        }
    }

    /// Get scrollback lines for a custom session type name
    pub fn lines_for_custom(&self, name: &str) -> usize {
        self.custom.get(name).copied().unwrap_or(self.default)
    }

    /// Validate and clamp a scrollback value to allowed range
    pub fn validate_lines(lines: usize) -> usize {
        lines.clamp(Self::MIN_LINES, Self::MAX_LINES)
    }

    /// Estimate memory usage for a buffer with given line count
    /// Assumes ~100 bytes per line average
    pub fn estimate_memory_bytes(lines: usize) -> usize {
        lines * 100
    }
}

/// Claude integration settings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ClaudeConfig {
    /// Enable detection
    pub detection_enabled: bool,
    /// Detection method
    pub detection_method: DetectionMethod,
    /// Show in status bar
    pub show_status: bool,
    /// Auto-resume crashed sessions
    pub auto_resume: bool,
}

impl Default for ClaudeConfig {
    fn default() -> Self {
        Self {
            detection_enabled: true,
            detection_method: DetectionMethod::Pty,
            show_status: true,
            auto_resume: true,
        }
    }
}

/// Claude detection method
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum DetectionMethod {
    #[default]
    Pty,
    StreamJson,
    Visual,
}

/// Persistence settings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct PersistenceConfig {
    /// Enable persistence (default: true)
    pub enabled: bool,
    /// Custom state directory (default: ~/.ccmux/state)
    pub state_dir: Option<String>,
    /// Checkpoint interval (seconds)
    pub checkpoint_interval_secs: u64,
    /// Max WAL size (MB) before forced checkpoint
    pub max_wal_size_mb: u64,
    /// Screen snapshot lines to persist
    pub screen_snapshot_lines: usize,
    /// Maximum checkpoints to keep
    pub max_checkpoints: usize,
    /// Compression method for scrollback: "none", "lz4", or "zstd"
    pub compression_method: String,
    /// Sync WAL on each write (true = safer, false = faster)
    pub sync_on_write: bool,
}

impl Default for PersistenceConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            state_dir: None,
            checkpoint_interval_secs: 30,
            max_wal_size_mb: 128,
            screen_snapshot_lines: 500,
            max_checkpoints: 5,
            compression_method: "lz4".to_string(),
            sync_on_write: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = AppConfig::default();
        assert_eq!(config.general.max_depth, 5);
        assert_eq!(config.terminal.render_interval_ms, 16);
    }

    #[test]
    fn test_serialize_roundtrip() {
        let config = AppConfig::default();
        let toml_str = toml::to_string(&config).unwrap();
        let parsed: AppConfig = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed.general.max_depth, config.general.max_depth);
    }

    #[test]
    fn test_partial_config() {
        let toml_str = r#"
            [general]
            max_depth = 10
        "#;
        let config: AppConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.general.max_depth, 10);
        // Other fields should have defaults
        assert_eq!(config.terminal.render_interval_ms, 16);
    }

    #[test]
    fn test_general_config_defaults() {
        let config = GeneralConfig::default();
        assert_eq!(config.max_depth, 5);
        assert_eq!(config.prefix_key, "Ctrl-a");
        assert!(!config.default_shell.is_empty());
    }

    #[test]
    fn test_appearance_config_defaults() {
        let config = AppearanceConfig::default();
        assert_eq!(config.theme, "default");
        assert_eq!(config.status_position, StatusPosition::Bottom);
        assert_eq!(config.border_style, BorderStyle::Rounded);
        assert!(config.show_pane_titles);
    }

    #[test]
    fn test_color_config_defaults() {
        let config = ColorConfig::default();
        assert_eq!(config.status_bg, "#282c34");
        assert_eq!(config.claude_thinking, "#e5c07b");
    }

    #[test]
    fn test_keybinding_config_defaults() {
        let config = KeybindingConfig::default();
        assert_eq!(config.split_horizontal, "prefix %");
        assert_eq!(config.detach, "prefix d");
        // Quick bindings
        assert_eq!(config.next_window_quick, "Ctrl-PageDown");
        assert_eq!(config.prev_window_quick, "Ctrl-PageUp");
        assert_eq!(config.next_pane_quick, "Ctrl-Shift-PageDown");
        assert_eq!(config.prev_pane_quick, "Ctrl-Shift-PageUp");
    }

    #[test]
    fn test_keybinding_config_quick_bindings_parse() {
        let toml_str = r#"
            [keybindings]
            next_window_quick = "F7"
            prev_window_quick = "Shift-F7"
            next_pane_quick = ""
            prev_pane_quick = "Alt-p"
        "#;
        let config: AppConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.keybindings.next_window_quick, "F7");
        assert_eq!(config.keybindings.prev_window_quick, "Shift-F7");
        assert_eq!(config.keybindings.next_pane_quick, ""); // Disabled
        assert_eq!(config.keybindings.prev_pane_quick, "Alt-p");
    }

    #[test]
    fn test_terminal_config_defaults() {
        let config = TerminalConfig::default();
        assert_eq!(config.scrollback.default, 1000);
        assert_eq!(config.scrollback.orchestrator, 50000);
        assert_eq!(config.scrollback.worker, 500);
        assert_eq!(config.render_interval_ms, 16);
        assert_eq!(config.parser_timeout_secs, 5);
    }

    #[test]
    fn test_claude_config_defaults() {
        let config = ClaudeConfig::default();
        assert!(config.detection_enabled);
        assert_eq!(config.detection_method, DetectionMethod::Pty);
        assert!(config.show_status);
        assert!(config.auto_resume);
    }

    #[test]
    fn test_persistence_config_defaults() {
        let config = PersistenceConfig::default();
        assert_eq!(config.checkpoint_interval_secs, 30);
        assert_eq!(config.max_wal_size_mb, 128);
        assert_eq!(config.screen_snapshot_lines, 500);
    }

    #[test]
    fn test_status_position_variants() {
        assert_eq!(StatusPosition::default(), StatusPosition::Bottom);

        // Test parsing from TOML config
        let toml_str = "[appearance]\nstatus_position = \"top\"";
        let config: AppConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.appearance.status_position, StatusPosition::Top);

        let toml_str = "[appearance]\nstatus_position = \"bottom\"";
        let config: AppConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.appearance.status_position, StatusPosition::Bottom);
    }

    #[test]
    fn test_border_style_variants() {
        assert_eq!(BorderStyle::default(), BorderStyle::Rounded);

        let toml_str = "[appearance]\nborder_style = \"single\"";
        let config: AppConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.appearance.border_style, BorderStyle::Single);

        let toml_str = "[appearance]\nborder_style = \"double\"";
        let config: AppConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.appearance.border_style, BorderStyle::Double);

        let toml_str = "[appearance]\nborder_style = \"none\"";
        let config: AppConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.appearance.border_style, BorderStyle::None);

        let toml_str = "[appearance]\nborder_style = \"rounded\"";
        let config: AppConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.appearance.border_style, BorderStyle::Rounded);
    }

    #[test]
    fn test_detection_method_variants() {
        assert_eq!(DetectionMethod::default(), DetectionMethod::Pty);

        let toml_str = "[claude]\ndetection_method = \"pty\"";
        let config: AppConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.claude.detection_method, DetectionMethod::Pty);

        let toml_str = "[claude]\ndetection_method = \"streamjson\"";
        let config: AppConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.claude.detection_method, DetectionMethod::StreamJson);

        let toml_str = "[claude]\ndetection_method = \"visual\"";
        let config: AppConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.claude.detection_method, DetectionMethod::Visual);
    }

    #[test]
    fn test_app_config_clone() {
        let config = AppConfig::default();
        let cloned = config.clone();
        assert_eq!(cloned.general.max_depth, config.general.max_depth);
    }

    #[test]
    fn test_app_config_debug() {
        let config = AppConfig::default();
        let debug_str = format!("{:?}", config);
        assert!(debug_str.contains("AppConfig"));
    }

    #[test]
    fn test_full_config_parse() {
        // Build the TOML string without using raw strings for hex colors
        let toml_str = "[general]
default_shell = \"/bin/zsh\"
max_depth = 3
prefix_key = \"Ctrl-b\"

[appearance]
theme = \"dark\"
status_position = \"top\"
border_style = \"double\"
show_pane_titles = false

[colors]
status_bg = \"#000000\"
status_fg = \"#ffffff\"
active_border = \"#ff0000\"
inactive_border = \"#333333\"
claude_thinking = \"#ffff00\"
claude_idle = \"#00ff00\"
claude_error = \"#ff0000\"

[keybindings]
split_horizontal = \"prefix |\"
split_vertical = \"prefix -\"
focus_left = \"prefix Left\"
focus_right = \"prefix Right\"
focus_up = \"prefix Up\"
focus_down = \"prefix Down\"
new_session = \"prefix n\"
detach = \"prefix d\"
list_sessions = \"prefix w\"

[terminal]
render_interval_ms = 8
parser_timeout_secs = 10

[terminal.scrollback]
default = 2000
orchestrator = 60000
worker = 300

[claude]
detection_enabled = false
detection_method = \"visual\"
show_status = false
auto_resume = false

[persistence]
checkpoint_interval_secs = 60
max_wal_size_mb = 256
screen_snapshot_lines = 1000
";

        let config: AppConfig = toml::from_str(toml_str).unwrap();

        // General
        assert_eq!(config.general.default_shell, "/bin/zsh");
        assert_eq!(config.general.max_depth, 3);
        assert_eq!(config.general.prefix_key, "Ctrl-b");

        // Appearance
        assert_eq!(config.appearance.theme, "dark");
        assert_eq!(config.appearance.status_position, StatusPosition::Top);
        assert_eq!(config.appearance.border_style, BorderStyle::Double);
        assert!(!config.appearance.show_pane_titles);

        // Colors
        assert_eq!(config.colors.status_bg, "#000000");

        // Keybindings
        assert_eq!(config.keybindings.split_horizontal, "prefix |");

        // Terminal
        assert_eq!(config.terminal.scrollback.default, 2000);
        assert_eq!(config.terminal.scrollback.orchestrator, 60000);
        assert_eq!(config.terminal.scrollback.worker, 300);
        assert_eq!(config.terminal.render_interval_ms, 8);

        // Claude
        assert!(!config.claude.detection_enabled);
        assert_eq!(config.claude.detection_method, DetectionMethod::Visual);

        // Persistence
        assert_eq!(config.persistence.checkpoint_interval_secs, 60);
    }

    #[test]
    fn test_status_position_clone_copy() {
        let pos = StatusPosition::Top;
        let cloned = pos.clone();
        let copied = pos;
        assert_eq!(pos, cloned);
        assert_eq!(pos, copied);
    }

    #[test]
    fn test_border_style_clone_copy() {
        let style = BorderStyle::Double;
        let cloned = style.clone();
        let copied = style;
        assert_eq!(style, cloned);
        assert_eq!(style, copied);
    }

    #[test]
    fn test_detection_method_clone_copy() {
        let method = DetectionMethod::StreamJson;
        let cloned = method.clone();
        let copied = method;
        assert_eq!(method, cloned);
        assert_eq!(method, copied);
    }

    #[test]
    fn test_config_sections_debug() {
        assert!(format!("{:?}", GeneralConfig::default()).contains("GeneralConfig"));
        assert!(format!("{:?}", AppearanceConfig::default()).contains("AppearanceConfig"));
        assert!(format!("{:?}", ColorConfig::default()).contains("ColorConfig"));
        assert!(format!("{:?}", KeybindingConfig::default()).contains("KeybindingConfig"));
        assert!(format!("{:?}", TerminalConfig::default()).contains("TerminalConfig"));
        assert!(format!("{:?}", ClaudeConfig::default()).contains("ClaudeConfig"));
        assert!(format!("{:?}", PersistenceConfig::default()).contains("PersistenceConfig"));
    }

    #[test]
    fn test_enum_debug() {
        assert!(format!("{:?}", StatusPosition::Top).contains("Top"));
        assert!(format!("{:?}", BorderStyle::Single).contains("Single"));
        assert!(format!("{:?}", DetectionMethod::Pty).contains("Pty"));
    }

    #[test]
    fn test_scrollback_config_defaults() {
        let config = ScrollbackConfig::default();
        assert_eq!(config.default, 1000);
        assert_eq!(config.orchestrator, 50000);
        assert_eq!(config.worker, 500);
        assert!(config.custom.is_empty());
    }

    #[test]
    fn test_scrollback_config_lines_for_type() {
        let config = ScrollbackConfig::default();
        assert_eq!(config.lines_for_type(SessionType::Default), 1000);
        assert_eq!(config.lines_for_type(SessionType::Orchestrator), 50000);
        assert_eq!(config.lines_for_type(SessionType::Worker), 500);
    }

    #[test]
    fn test_scrollback_config_lines_for_custom() {
        let mut config = ScrollbackConfig::default();
        config.custom.insert("special".to_string(), 25000);

        assert_eq!(config.lines_for_custom("special"), 25000);
        assert_eq!(config.lines_for_custom("unknown"), 1000); // Falls back to default
    }

    #[test]
    fn test_scrollback_config_validate_lines() {
        assert_eq!(ScrollbackConfig::validate_lines(50), ScrollbackConfig::MIN_LINES);
        assert_eq!(ScrollbackConfig::validate_lines(5000), 5000);
        assert_eq!(ScrollbackConfig::validate_lines(200_000), ScrollbackConfig::MAX_LINES);
    }

    #[test]
    fn test_scrollback_config_estimate_memory() {
        assert_eq!(ScrollbackConfig::estimate_memory_bytes(1000), 100_000);
        assert_eq!(ScrollbackConfig::estimate_memory_bytes(50000), 5_000_000);
    }

    #[test]
    fn test_session_type_default() {
        assert_eq!(SessionType::default(), SessionType::Default);
    }

    #[test]
    fn test_session_type_display() {
        assert_eq!(SessionType::Default.to_string(), "default");
        assert_eq!(SessionType::Orchestrator.to_string(), "orchestrator");
        assert_eq!(SessionType::Worker.to_string(), "worker");
    }

    #[test]
    fn test_session_type_clone_copy() {
        let st = SessionType::Orchestrator;
        let cloned = st.clone();
        let copied = st;
        assert_eq!(st, cloned);
        assert_eq!(st, copied);
    }

    #[test]
    fn test_session_type_debug() {
        assert!(format!("{:?}", SessionType::Default).contains("Default"));
        assert!(format!("{:?}", SessionType::Orchestrator).contains("Orchestrator"));
        assert!(format!("{:?}", SessionType::Worker).contains("Worker"));
    }

    #[test]
    fn test_scrollback_config_clone() {
        let mut config = ScrollbackConfig::default();
        config.custom.insert("test".to_string(), 5000);

        let cloned = config.clone();
        assert_eq!(cloned.default, config.default);
        assert_eq!(cloned.custom.get("test"), Some(&5000));
    }

    #[test]
    fn test_scrollback_config_debug() {
        let config = ScrollbackConfig::default();
        let debug_str = format!("{:?}", config);
        assert!(debug_str.contains("ScrollbackConfig"));
    }

    #[test]
    fn test_scrollback_config_parse_nested() {
        let toml_str = r#"
            [terminal.scrollback]
            default = 1500
            orchestrator = 40000
            worker = 250
        "#;
        let config: AppConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.terminal.scrollback.default, 1500);
        assert_eq!(config.terminal.scrollback.orchestrator, 40000);
        assert_eq!(config.terminal.scrollback.worker, 250);
    }

    #[test]
    fn test_scrollback_config_parse_partial() {
        let toml_str = r#"
            [terminal.scrollback]
            orchestrator = 75000
        "#;
        let config: AppConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.terminal.scrollback.default, 1000); // Default
        assert_eq!(config.terminal.scrollback.orchestrator, 75000);
        assert_eq!(config.terminal.scrollback.worker, 500); // Default
    }

    // ==================== SessionLoggingConfig Tests ====================

    #[test]
    fn test_session_logging_config_defaults() {
        let config = SessionLoggingConfig::default();
        assert!(config.enabled);
        assert_eq!(config.default_level, SessionLogLevel::Signals);
        assert_eq!(config.max_file_size_mb, 10);
        assert_eq!(config.max_rotated_files, 5);
        assert_eq!(config.retention_days, 7);
        assert!(config.separate_audit_trail);
    }

    #[test]
    fn test_session_logging_config_parse() {
        let toml_str = r#"
            [session_logging]
            enabled = true
            default_level = "full"
            max_file_size_mb = 20
            max_rotated_files = 10
            retention_days = 14
            separate_audit_trail = false
        "#;
        let config: AppConfig = toml::from_str(toml_str).unwrap();
        assert!(config.session_logging.enabled);
        assert_eq!(config.session_logging.default_level, SessionLogLevel::Full);
        assert_eq!(config.session_logging.max_file_size_mb, 20);
        assert_eq!(config.session_logging.max_rotated_files, 10);
        assert_eq!(config.session_logging.retention_days, 14);
        assert!(!config.session_logging.separate_audit_trail);
    }

    #[test]
    fn test_session_logging_config_to_session_log_config() {
        let config = SessionLoggingConfig {
            enabled: true,
            default_level: SessionLogLevel::Prompts,
            max_file_size_mb: 5,
            max_rotated_files: 3,
            retention_days: 3,
            separate_audit_trail: true,
        };

        let log_config = config.to_session_log_config();

        assert_eq!(log_config.default_level, SessionLogLevel::Prompts);
        assert_eq!(log_config.max_file_size, 5 * 1024 * 1024);
        assert_eq!(log_config.max_rotated_files, 3);
        assert_eq!(log_config.retention_secs, 3 * 24 * 60 * 60);
        assert!(log_config.separate_audit_trail);
    }

    #[test]
    fn test_session_logging_config_debug() {
        let config = SessionLoggingConfig::default();
        let debug = format!("{:?}", config);
        assert!(debug.contains("SessionLoggingConfig"));
        assert!(debug.contains("enabled"));
    }

    #[test]
    fn test_session_logging_config_clone() {
        let config = SessionLoggingConfig::default();
        let cloned = config.clone();
        assert_eq!(config.enabled, cloned.enabled);
        assert_eq!(config.default_level, cloned.default_level);
    }

    #[test]
    fn test_session_logging_all_levels() {
        for level in ["spawns", "signals", "prompts", "full"] {
            let toml_str = format!(
                r#"
                [session_logging]
                default_level = "{}"
            "#,
                level
            );
            let config: AppConfig = toml::from_str(&toml_str).unwrap();
            assert!(format!("{:?}", config.session_logging.default_level)
                .to_lowercase()
                .contains(level));
        }
    }

    // ==================== BeadsConfig Tests ====================

    #[test]
    fn test_beads_config_defaults() {
        let config = BeadsConfig::default();
        assert!(config.auto_detect);
        assert!(config.auto_set_beads_dir);
        assert!(!config.no_daemon_default);
    }

    #[test]
    fn test_beads_config_parse() {
        let toml_str = r#"
            [beads]
            auto_detect = false
            auto_set_beads_dir = false
            no_daemon_default = true
        "#;
        let config: AppConfig = toml::from_str(toml_str).unwrap();
        assert!(!config.beads.auto_detect);
        assert!(!config.beads.auto_set_beads_dir);
        assert!(config.beads.no_daemon_default);
    }

    #[test]
    fn test_beads_config_partial_parse() {
        let toml_str = r#"
            [beads]
            no_daemon_default = true
        "#;
        let config: AppConfig = toml::from_str(toml_str).unwrap();
        assert!(config.beads.auto_detect); // Default
        assert!(config.beads.auto_set_beads_dir); // Default
        assert!(config.beads.no_daemon_default); // Explicitly set
    }

    #[test]
    fn test_beads_config_debug() {
        let config = BeadsConfig::default();
        let debug = format!("{:?}", config);
        assert!(debug.contains("BeadsConfig"));
        assert!(debug.contains("auto_detect"));
    }

    #[test]
    fn test_beads_config_clone() {
        let config = BeadsConfig {
            auto_detect: true,
            auto_set_beads_dir: false,
            no_daemon_default: true,
            query: BeadsQueryConfig::default(),
            workflow: BeadsWorkflowConfig::default(),
        };
        let cloned = config.clone();
        assert_eq!(config.auto_detect, cloned.auto_detect);
        assert_eq!(config.auto_set_beads_dir, cloned.auto_set_beads_dir);
        assert_eq!(config.no_daemon_default, cloned.no_daemon_default);
        assert_eq!(config.query.enabled, cloned.query.enabled);
        assert_eq!(config.workflow.enabled, cloned.workflow.enabled);
    }

    #[test]
    fn test_beads_config_in_full_config() {
        let config = AppConfig::default();
        assert!(config.beads.auto_detect);
        assert!(config.beads.auto_set_beads_dir);
        assert!(!config.beads.no_daemon_default);
    }

    // ==================== BeadsQueryConfig Tests ====================

    #[test]
    fn test_beads_query_config_defaults() {
        let config = BeadsQueryConfig::default();
        assert!(config.enabled);
        assert!(config.show_ready_count);
        assert_eq!(config.refresh_interval, 30);
        assert_eq!(config.socket_timeout, 1000);
    }

    #[test]
    fn test_beads_query_config_parse() {
        let toml_str = r#"
            [beads.query]
            enabled = false
            show_ready_count = false
            refresh_interval = 60
            socket_timeout = 2000
        "#;
        let config: AppConfig = toml::from_str(toml_str).unwrap();
        assert!(!config.beads.query.enabled);
        assert!(!config.beads.query.show_ready_count);
        assert_eq!(config.beads.query.refresh_interval, 60);
        assert_eq!(config.beads.query.socket_timeout, 2000);
    }

    #[test]
    fn test_beads_query_config_partial_parse() {
        let toml_str = r#"
            [beads.query]
            refresh_interval = 15
        "#;
        let config: AppConfig = toml::from_str(toml_str).unwrap();
        assert!(config.beads.query.enabled); // Default
        assert!(config.beads.query.show_ready_count); // Default
        assert_eq!(config.beads.query.refresh_interval, 15); // Explicitly set
        assert_eq!(config.beads.query.socket_timeout, 1000); // Default
    }

    #[test]
    fn test_beads_query_config_debug() {
        let config = BeadsQueryConfig::default();
        let debug = format!("{:?}", config);
        assert!(debug.contains("BeadsQueryConfig"));
        assert!(debug.contains("enabled"));
        assert!(debug.contains("refresh_interval"));
    }

    #[test]
    fn test_beads_query_config_clone() {
        let config = BeadsQueryConfig {
            enabled: false,
            show_ready_count: true,
            refresh_interval: 45,
            socket_timeout: 500,
        };
        let cloned = config.clone();
        assert_eq!(config.enabled, cloned.enabled);
        assert_eq!(config.show_ready_count, cloned.show_ready_count);
        assert_eq!(config.refresh_interval, cloned.refresh_interval);
        assert_eq!(config.socket_timeout, cloned.socket_timeout);
    }

    #[test]
    fn test_beads_config_with_query_nested() {
        let toml_str = r#"
            [beads]
            auto_detect = false
            no_daemon_default = true

            [beads.query]
            enabled = true
            refresh_interval = 10
        "#;
        let config: AppConfig = toml::from_str(toml_str).unwrap();
        // Parent beads config
        assert!(!config.beads.auto_detect);
        assert!(config.beads.no_daemon_default);
        // Nested query config
        assert!(config.beads.query.enabled);
        assert_eq!(config.beads.query.refresh_interval, 10);
    }

    #[test]
    fn test_beads_query_config_in_full_config() {
        let config = AppConfig::default();
        assert!(config.beads.query.enabled);
        assert!(config.beads.query.show_ready_count);
        assert_eq!(config.beads.query.refresh_interval, 30);
        assert_eq!(config.beads.query.socket_timeout, 1000);
    }
}
