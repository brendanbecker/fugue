//! PTY configuration types

use std::collections::HashMap;
use std::path::PathBuf;

/// Configuration for spawning a PTY
#[derive(Debug, Clone)]
pub struct PtyConfig {
    /// Command to execute (shell or program)
    pub command: String,
    /// Arguments to the command
    pub args: Vec<String>,
    /// Working directory
    pub cwd: Option<PathBuf>,
    /// Environment variables to set
    pub env: HashMap<String, String>,
    /// Environment variables to remove
    pub env_remove: Vec<String>,
    /// Initial terminal size (cols, rows)
    pub size: (u16, u16),
}

impl Default for PtyConfig {
    fn default() -> Self {
        Self {
            command: std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".into()),
            args: Vec::new(),
            cwd: None,
            env: HashMap::new(),
            env_remove: Vec::new(),
            size: (80, 24),
        }
    }
}

impl PtyConfig {
    /// Create config for default shell
    pub fn shell() -> Self {
        Self::default()
    }

    /// Create config for a specific command
    pub fn command(cmd: impl Into<String>) -> Self {
        Self {
            command: cmd.into(),
            ..Default::default()
        }
    }

    /// Set working directory
    pub fn with_cwd(mut self, cwd: impl Into<PathBuf>) -> Self {
        self.cwd = Some(cwd.into());
        self
    }

    /// Add environment variable
    pub fn with_env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env.insert(key.into(), value.into());
        self
    }

    /// Remove environment variable
    pub fn without_env(mut self, key: impl Into<String>) -> Self {
        self.env_remove.push(key.into());
        self
    }

    /// Set initial size
    pub fn with_size(mut self, cols: u16, rows: u16) -> Self {
        self.size = (cols, rows);
        self
    }

    /// Add argument
    pub fn with_arg(mut self, arg: impl Into<String>) -> Self {
        self.args.push(arg.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = PtyConfig::default();
        assert_eq!(config.size, (80, 24));
        assert!(config.args.is_empty());
    }

    #[test]
    fn test_config_builder() {
        let config = PtyConfig::command("bash")
            .with_cwd("/home/user")
            .with_env("FOO", "bar")
            .with_size(120, 40)
            .with_arg("-l");

        assert_eq!(config.command, "bash");
        assert_eq!(config.cwd, Some(PathBuf::from("/home/user")));
        assert_eq!(config.env.get("FOO"), Some(&"bar".to_string()));
        assert_eq!(config.size, (120, 40));
        assert_eq!(config.args, vec!["-l"]);
    }
}
