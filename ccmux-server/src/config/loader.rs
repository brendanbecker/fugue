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
}
