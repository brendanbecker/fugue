//! Configuration schema structs

use serde::{Deserialize, Serialize};

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
}

/// General settings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GeneralConfig {
    /// Default shell to spawn
    pub default_shell: String,
    /// Maximum Claude session depth
    pub max_depth: u32,
    /// Prefix key for commands
    pub prefix_key: String,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            default_shell: std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".into()),
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
    pub split_horizontal: String,
    pub split_vertical: String,
    pub focus_left: String,
    pub focus_right: String,
    pub focus_up: String,
    pub focus_down: String,
    pub new_session: String,
    pub detach: String,
    pub list_sessions: String,
}

impl Default for KeybindingConfig {
    fn default() -> Self {
        Self {
            split_horizontal: "prefix %".into(),
            split_vertical: "prefix \"".into(),
            focus_left: "prefix h".into(),
            focus_right: "prefix l".into(),
            focus_up: "prefix k".into(),
            focus_down: "prefix j".into(),
            new_session: "prefix c".into(),
            detach: "prefix d".into(),
            list_sessions: "prefix s".into(),
        }
    }
}

/// Terminal settings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct TerminalConfig {
    /// Scrollback buffer lines
    pub scrollback_lines: usize,
    /// Render interval (ms)
    pub render_interval_ms: u64,
    /// Parser timeout (seconds)
    pub parser_timeout_secs: u64,
}

impl Default for TerminalConfig {
    fn default() -> Self {
        Self {
            scrollback_lines: 10000,
            render_interval_ms: 16,
            parser_timeout_secs: 5,
        }
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
    /// Checkpoint interval (seconds)
    pub checkpoint_interval_secs: u64,
    /// Max WAL size (MB)
    pub max_wal_size_mb: u64,
    /// Screen snapshot lines
    pub screen_snapshot_lines: usize,
}

impl Default for PersistenceConfig {
    fn default() -> Self {
        Self {
            checkpoint_interval_secs: 30,
            max_wal_size_mb: 128,
            screen_snapshot_lines: 500,
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
}
