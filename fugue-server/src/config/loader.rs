//! Configuration loader

use std::path::Path;

use ccmux_utils::{config_file, CcmuxError, Result};

use super::AppConfig;

/// Configuration loader
pub struct ConfigLoader;

impl ConfigLoader {
    /// Load configuration from default location
    pub fn load() -> Result<AppConfig> {
        let path = config_file();
        if path.exists() {
            Self::load_from_path(&path)
        } else {
            Ok(AppConfig::default())
        }
    }

    /// Load configuration from a specific path
    pub fn load_from_path(path: &Path) -> Result<AppConfig> {
        let content = std::fs::read_to_string(path).map_err(|e| CcmuxError::FileRead {
            path: path.to_path_buf(),
            source: e,
        })?;

        Self::parse(&content, path)
    }

    /// Parse configuration from string
    pub fn parse(content: &str, path: &Path) -> Result<AppConfig> {
        toml::from_str(content).map_err(|e| CcmuxError::ConfigInvalid {
            path: path.to_path_buf(),
            message: e.to_string(),
        })
    }

    /// Validate configuration
    pub fn validate(config: &AppConfig) -> Result<()> {
        // Validate max_depth
        if config.general.max_depth == 0 || config.general.max_depth > 10 {
            return Err(CcmuxError::config("max_depth must be between 1 and 10"));
        }

        // Validate render interval
        if config.terminal.render_interval_ms < 8 {
            return Err(CcmuxError::config(
                "render_interval_ms must be at least 8 (120fps max)",
            ));
        }

        // Validate checkpoint interval
        if config.persistence.checkpoint_interval_secs < 5 {
            return Err(CcmuxError::config(
                "checkpoint_interval_secs must be at least 5",
            ));
        }

        Ok(())
    }

    /// Load and validate
    pub fn load_and_validate() -> Result<AppConfig> {
        let config = Self::load()?;
        Self::validate(&config)?;
        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_load_missing_file() {
        let config = ConfigLoader::load();
        assert!(config.is_ok());
    }

    #[test]
    fn test_load_from_path() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.toml");

        std::fs::write(
            &path,
            r#"
            [general]
            max_depth = 3
            "#,
        )
        .unwrap();

        let config = ConfigLoader::load_from_path(&path).unwrap();
        assert_eq!(config.general.max_depth, 3);
    }

    #[test]
    fn test_validate_invalid_depth() {
        let mut config = AppConfig::default();
        config.general.max_depth = 0;

        let result = ConfigLoader::validate(&config);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_invalid_toml() {
        let result = ConfigLoader::parse("invalid { toml", Path::new("test.toml"));
        assert!(result.is_err());
    }

    #[test]
    fn test_load_and_validate_success() {
        // Default config should be valid
        let result = ConfigLoader::load_and_validate();
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_valid_config() {
        let config = AppConfig::default();
        let result = ConfigLoader::validate(&config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_max_depth_zero() {
        let mut config = AppConfig::default();
        config.general.max_depth = 0;

        let result = ConfigLoader::validate(&config);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_max_depth_too_high() {
        let mut config = AppConfig::default();
        config.general.max_depth = 11;

        let result = ConfigLoader::validate(&config);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_max_depth_boundary_low() {
        let mut config = AppConfig::default();
        config.general.max_depth = 1;

        let result = ConfigLoader::validate(&config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_max_depth_boundary_high() {
        let mut config = AppConfig::default();
        config.general.max_depth = 10;

        let result = ConfigLoader::validate(&config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_render_interval_too_low() {
        let mut config = AppConfig::default();
        config.terminal.render_interval_ms = 7;

        let result = ConfigLoader::validate(&config);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_render_interval_boundary() {
        let mut config = AppConfig::default();
        config.terminal.render_interval_ms = 8;

        let result = ConfigLoader::validate(&config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_checkpoint_interval_too_low() {
        let mut config = AppConfig::default();
        config.persistence.checkpoint_interval_secs = 4;

        let result = ConfigLoader::validate(&config);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_checkpoint_interval_boundary() {
        let mut config = AppConfig::default();
        config.persistence.checkpoint_interval_secs = 5;

        let result = ConfigLoader::validate(&config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_load_from_path_file_not_found() {
        let result = ConfigLoader::load_from_path(Path::new("/nonexistent/config.toml"));
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_empty_config() {
        let config = ConfigLoader::parse("", Path::new("empty.toml")).unwrap();
        // Should return defaults
        assert_eq!(config.general.max_depth, 5);
    }

    #[test]
    fn test_parse_partial_config() {
        let content = r#"
            [terminal.scrollback]
            default = 5000
        "#;

        let config = ConfigLoader::parse(content, Path::new("partial.toml")).unwrap();
        assert_eq!(config.terminal.scrollback.default, 5000);
        // Other fields should have defaults
        assert_eq!(config.general.max_depth, 5);
    }

    #[test]
    fn test_load_full_config_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.toml");

        std::fs::write(
            &path,
            r#"
            [general]
            max_depth = 3
            prefix_key = "Ctrl-b"

            [terminal]
            render_interval_ms = 16

            [terminal.scrollback]
            default = 20000
            orchestrator = 80000

            [persistence]
            checkpoint_interval_secs = 60
            "#,
        )
        .unwrap();

        let config = ConfigLoader::load_from_path(&path).unwrap();
        assert_eq!(config.general.max_depth, 3);
        assert_eq!(config.general.prefix_key, "Ctrl-b");
        assert_eq!(config.terminal.scrollback.default, 20000);
        assert_eq!(config.terminal.scrollback.orchestrator, 80000);
        assert_eq!(config.persistence.checkpoint_interval_secs, 60);
    }

    #[test]
    fn test_validate_multiple_errors() {
        // Test that validation stops at first error
        let mut config = AppConfig::default();
        config.general.max_depth = 0;
        config.terminal.render_interval_ms = 1;

        let result = ConfigLoader::validate(&config);
        assert!(result.is_err());
        // Error message should be about max_depth (checked first)
        let err_str = format!("{}", result.unwrap_err());
        assert!(err_str.contains("max_depth"));
    }

    #[test]
    fn test_parse_with_unknown_fields() {
        let content = r#"
            [general]
            max_depth = 5
            unknown_field = "ignored"

            [unknown_section]
            foo = "bar"
        "#;

        // Unknown fields should be ignored (serde default behavior)
        let result = ConfigLoader::parse(content, Path::new("test.toml"));
        // This may or may not error depending on serde strict mode
        // For our implementation with #[serde(default)], it should succeed
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_invalid_type() {
        let content = r#"
            [general]
            max_depth = "not a number"
        "#;

        let result = ConfigLoader::parse(content, Path::new("test.toml"));
        assert!(result.is_err());
    }
}
