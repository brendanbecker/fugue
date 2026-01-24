//! Client-side configuration loading
//!
//! Loads keybinding configuration from the shared config file.

use crate::input::QuickBindings;
use std::collections::HashMap;
use std::path::PathBuf;

/// Get the config file path (~/.config/fugue/config.toml)
fn config_file() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("fugue")
        .join("config.toml")
}

/// Client keybinding configuration (subset of full server config)
#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(default)]
struct ClientConfig {
    keybindings: KeybindingConfig,
    #[serde(default)]
    remotes: HashMap<String, String>,
}

/// Keybinding configuration for quick navigation
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(default)]
struct KeybindingConfig {
    /// Quick binding for next window (default: Ctrl-PageDown)
    next_window_quick: String,
    /// Quick binding for previous window (default: Ctrl-PageUp)
    prev_window_quick: String,
    /// Quick binding for next pane in window (default: Ctrl-Shift-PageDown)
    next_pane_quick: String,
    /// Quick binding for previous pane (default: Ctrl-Shift-PageUp)
    prev_pane_quick: String,
}

impl Default for KeybindingConfig {
    fn default() -> Self {
        Self {
            next_window_quick: "Ctrl-PageDown".into(),
            prev_window_quick: "Ctrl-PageUp".into(),
            next_pane_quick: "Ctrl-Shift-PageDown".into(),
            prev_pane_quick: "Ctrl-Shift-PageUp".into(),
        }
    }
}

/// Resolve a remote alias to an address
pub fn resolve_remote(name: &str) -> Option<String> {
    let path = config_file();

    if !path.exists() {
        return None;
    }

    match std::fs::read_to_string(&path) {
        Ok(content) => match toml::from_str::<ClientConfig>(&content) {
            Ok(config) => config.remotes.get(name).cloned(),
            Err(e) => {
                tracing::warn!("Failed to parse config file for remotes: {}", e);
                None
            }
        },
        Err(e) => {
            tracing::warn!("Failed to read config file for remotes: {}", e);
            None
        }
    }
}

/// Load quick bindings from config file
///
/// Returns default bindings if config file doesn't exist or can't be parsed.
pub fn load_quick_bindings() -> QuickBindings {
    let path = config_file();

    if !path.exists() {
        tracing::debug!("Config file not found, using default keybindings");
        return QuickBindings::default();
    }

    match std::fs::read_to_string(&path) {
        Ok(content) => match toml::from_str::<ClientConfig>(&content) {
            Ok(config) => {
                let kb = &config.keybindings;
                tracing::debug!(
                    "Loaded keybindings from config: next_window={}, prev_window={}, next_pane={}, prev_pane={}",
                    kb.next_window_quick,
                    kb.prev_window_quick,
                    kb.next_pane_quick,
                    kb.prev_pane_quick
                );
                QuickBindings::from_config(
                    &kb.next_window_quick,
                    &kb.prev_window_quick,
                    &kb.next_pane_quick,
                    &kb.prev_pane_quick,
                )
            }
            Err(e) => {
                tracing::warn!("Failed to parse config file: {}, using defaults", e);
                QuickBindings::default()
            }
        },
        Err(e) => {
            tracing::warn!("Failed to read config file: {}, using defaults", e);
            QuickBindings::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_keybindings() {
        let config = KeybindingConfig::default();
        assert_eq!(config.next_window_quick, "Ctrl-PageDown");
        assert_eq!(config.prev_window_quick, "Ctrl-PageUp");
        assert_eq!(config.next_pane_quick, "Ctrl-Shift-PageDown");
        assert_eq!(config.prev_pane_quick, "Ctrl-Shift-PageUp");
    }

    #[test]
    fn test_parse_empty_config() {
        let config: ClientConfig = toml::from_str("").unwrap();
        assert_eq!(config.keybindings.next_window_quick, "Ctrl-PageDown");
    }

    #[test]
    fn test_parse_partial_config() {
        let toml = r#"
            [keybindings]
            next_window_quick = "F7"
        "#;
        let config: ClientConfig = toml::from_str(toml).unwrap();
        assert_eq!(config.keybindings.next_window_quick, "F7");
        // Defaults for unspecified
        assert_eq!(config.keybindings.prev_window_quick, "Ctrl-PageUp");
    }

    #[test]
    fn test_parse_disabled_binding() {
        let toml = r#"
            [keybindings]
            next_window_quick = ""
            prev_window_quick = "Ctrl-PageUp"
        "#;
        let config: ClientConfig = toml::from_str(toml).unwrap();
        assert_eq!(config.keybindings.next_window_quick, "");
    }

    #[test]
    fn test_parse_remotes() {
        let toml = r#"
            [remotes]
            gaming-pc = "tcp://192.168.1.5:9999"
            cloud-gpu = "tcp://203.0.113.10:9999"
        "#;
        let config: ClientConfig = toml::from_str(toml).unwrap();
        assert_eq!(config.remotes.get("gaming-pc"), Some(&"tcp://192.168.1.5:9999".to_string()));
        assert_eq!(config.remotes.get("cloud-gpu"), Some(&"tcp://203.0.113.10:9999".to_string()));
        assert_eq!(config.remotes.get("missing"), None);
    }
}
