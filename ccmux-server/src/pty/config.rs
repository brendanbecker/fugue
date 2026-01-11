//! PTY configuration types

use std::collections::HashMap;
use std::path::PathBuf;

use crate::config::SessionType;
use uuid::Uuid;

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
    /// Session type for scrollback configuration
    pub session_type: SessionType,
    /// Override scrollback lines (if None, uses session_type default)
    pub scrollback_lines: Option<usize>,
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
            session_type: SessionType::Default,
            scrollback_lines: None,
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

    /// Create config from a command string with arguments (e.g., "claude --resume")
    ///
    /// Parses the string by splitting on whitespace. For more complex argument
    /// parsing (quoted strings, etc.), use `command()` with `with_arg()` instead.
    pub fn from_command_string(cmd_str: &str) -> Self {
        let parts: Vec<&str> = cmd_str.split_whitespace().collect();
        if parts.is_empty() {
            return Self::shell();
        }
        let mut config = Self::command(parts[0]);
        for arg in parts.iter().skip(1) {
            config = config.with_arg(*arg);
        }
        config
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

    /// Set session type
    pub fn with_session_type(mut self, session_type: SessionType) -> Self {
        self.session_type = session_type;
        self
    }

    /// Set scrollback lines override
    pub fn with_scrollback(mut self, lines: usize) -> Self {
        self.scrollback_lines = Some(lines);
        self
    }

    /// Inject CCMUX context environment variables
    ///
    /// Sets the following environment variables:
    /// - `CCMUX_SESSION_ID`: UUID of the session
    /// - `CCMUX_SESSION_NAME`: Human-readable session name
    /// - `CCMUX_WINDOW_ID`: UUID of the window
    /// - `CCMUX_PANE_ID`: UUID of the pane
    ///
    /// These enable processes to be self-aware of their ccmux context.
    pub fn with_ccmux_context(
        self,
        session_id: Uuid,
        session_name: &str,
        window_id: Uuid,
        pane_id: Uuid,
    ) -> Self {
        self.with_env("CCMUX_SESSION_ID", session_id.to_string())
            .with_env("CCMUX_SESSION_NAME", session_name)
            .with_env("CCMUX_WINDOW_ID", window_id.to_string())
            .with_env("CCMUX_PANE_ID", pane_id.to_string())
    }

    /// Get effective scrollback lines based on session type and override
    pub fn effective_scrollback(&self, config: &crate::config::ScrollbackConfig) -> usize {
        self.scrollback_lines
            .unwrap_or_else(|| config.lines_for_type(self.session_type))
    }

    /// Merge environment variables from a HashMap
    ///
    /// Used to propagate session-level environment variables to PTY spawn.
    pub fn with_env_map(mut self, env: &HashMap<String, String>) -> Self {
        self.env.extend(env.iter().map(|(k, v)| (k.clone(), v.clone())));
        self
    }

    /// Configure beads environment variables (FEAT-057)
    ///
    /// Sets the following environment variables based on beads config:
    /// - `BEADS_DIR`: Path to the .beads/ directory (if auto_set_beads_dir)
    /// - `BEADS_NO_DAEMON`: Set to "true" if no_daemon_default is enabled
    ///
    /// # Arguments
    /// * `beads_dir` - Path to the .beads/ directory
    /// * `config` - Beads configuration settings
    pub fn with_beads_config(
        mut self,
        beads_dir: &std::path::Path,
        config: &crate::config::BeadsConfig,
    ) -> Self {
        if config.auto_set_beads_dir {
            self.env.insert(
                "BEADS_DIR".to_string(),
                beads_dir.to_string_lossy().into_owned(),
            );
        }
        if config.no_daemon_default {
            self.env.insert("BEADS_NO_DAEMON".to_string(), "true".to_string());
        }
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

    #[test]
    fn test_shell_creates_default() {
        let config = PtyConfig::shell();
        assert_eq!(config.size, (80, 24));
        assert!(config.args.is_empty());
        assert!(config.env.is_empty());
    }

    #[test]
    fn test_command_sets_command() {
        let config = PtyConfig::command("vim");
        assert_eq!(config.command, "vim");
        assert_eq!(config.size, (80, 24)); // Default size preserved
    }

    #[test]
    fn test_from_command_string_simple() {
        let config = PtyConfig::from_command_string("bash");
        assert_eq!(config.command, "bash");
        assert!(config.args.is_empty());
    }

    #[test]
    fn test_from_command_string_with_args() {
        let config = PtyConfig::from_command_string("claude --resume");
        assert_eq!(config.command, "claude");
        assert_eq!(config.args, vec!["--resume"]);
    }

    #[test]
    fn test_from_command_string_multiple_args() {
        let config = PtyConfig::from_command_string("python -m pytest -v");
        assert_eq!(config.command, "python");
        assert_eq!(config.args, vec!["-m", "pytest", "-v"]);
    }

    #[test]
    fn test_from_command_string_empty() {
        let config = PtyConfig::from_command_string("");
        // Should fall back to shell
        assert!(!config.command.is_empty());
    }

    #[test]
    fn test_from_command_string_whitespace_only() {
        let config = PtyConfig::from_command_string("   ");
        // Should fall back to shell
        assert!(!config.command.is_empty());
    }

    #[test]
    fn test_with_cwd() {
        let config = PtyConfig::default().with_cwd("/tmp");
        assert_eq!(config.cwd, Some(PathBuf::from("/tmp")));
    }

    #[test]
    fn test_with_env_multiple() {
        let config = PtyConfig::default()
            .with_env("KEY1", "value1")
            .with_env("KEY2", "value2");

        assert_eq!(config.env.get("KEY1"), Some(&"value1".to_string()));
        assert_eq!(config.env.get("KEY2"), Some(&"value2".to_string()));
    }

    #[test]
    fn test_without_env() {
        let config = PtyConfig::default().without_env("PATH");
        assert!(config.env_remove.contains(&"PATH".to_string()));
    }

    #[test]
    fn test_without_env_multiple() {
        let config = PtyConfig::default()
            .without_env("PATH")
            .without_env("HOME");

        assert!(config.env_remove.contains(&"PATH".to_string()));
        assert!(config.env_remove.contains(&"HOME".to_string()));
        assert_eq!(config.env_remove.len(), 2);
    }

    #[test]
    fn test_with_size() {
        let config = PtyConfig::default().with_size(200, 100);
        assert_eq!(config.size, (200, 100));
    }

    #[test]
    fn test_with_size_zero() {
        let config = PtyConfig::default().with_size(0, 0);
        assert_eq!(config.size, (0, 0));
    }

    #[test]
    fn test_with_size_max() {
        let config = PtyConfig::default().with_size(u16::MAX, u16::MAX);
        assert_eq!(config.size, (u16::MAX, u16::MAX));
    }

    #[test]
    fn test_with_arg_multiple() {
        let config = PtyConfig::command("ls")
            .with_arg("-l")
            .with_arg("-a")
            .with_arg("-h");

        assert_eq!(config.args, vec!["-l", "-a", "-h"]);
    }

    #[test]
    fn test_config_clone() {
        let config = PtyConfig::command("bash")
            .with_cwd("/home")
            .with_env("FOO", "bar")
            .with_size(100, 50);

        let cloned = config.clone();
        assert_eq!(cloned.command, "bash");
        assert_eq!(cloned.cwd, Some(PathBuf::from("/home")));
        assert_eq!(cloned.env.get("FOO"), Some(&"bar".to_string()));
        assert_eq!(cloned.size, (100, 50));
    }

    #[test]
    fn test_config_debug() {
        let config = PtyConfig::command("test");
        let debug_str = format!("{:?}", config);
        assert!(debug_str.contains("PtyConfig"));
        assert!(debug_str.contains("test"));
    }

    #[test]
    fn test_builder_chain_order_independent() {
        let config1 = PtyConfig::command("cmd")
            .with_cwd("/tmp")
            .with_size(100, 50);

        let config2 = PtyConfig::command("cmd")
            .with_size(100, 50)
            .with_cwd("/tmp");

        assert_eq!(config1.command, config2.command);
        assert_eq!(config1.cwd, config2.cwd);
        assert_eq!(config1.size, config2.size);
    }

    #[test]
    fn test_env_overwrite() {
        let config = PtyConfig::default()
            .with_env("KEY", "first")
            .with_env("KEY", "second");

        assert_eq!(config.env.get("KEY"), Some(&"second".to_string()));
    }

    #[test]
    fn test_complex_command() {
        let config = PtyConfig::command("/usr/bin/python3")
            .with_arg("-c")
            .with_arg("print('hello')")
            .with_cwd("/tmp")
            .with_env("PYTHONPATH", "/opt/lib")
            .without_env("PYTHONDONTWRITEBYTECODE")
            .with_size(120, 40);

        assert_eq!(config.command, "/usr/bin/python3");
        assert_eq!(config.args.len(), 2);
        assert_eq!(config.cwd, Some(PathBuf::from("/tmp")));
        assert!(config.env.contains_key("PYTHONPATH"));
        assert!(config.env_remove.contains(&"PYTHONDONTWRITEBYTECODE".to_string()));
        assert_eq!(config.size, (120, 40));
    }

    #[test]
    fn test_default_command_is_shell() {
        let config = PtyConfig::default();
        // Command should be from SHELL env var or /bin/sh
        assert!(!config.command.is_empty());
    }

    #[test]
    fn test_default_session_type() {
        let config = PtyConfig::default();
        assert_eq!(config.session_type, SessionType::Default);
        assert!(config.scrollback_lines.is_none());
    }

    #[test]
    fn test_with_session_type() {
        let config = PtyConfig::default().with_session_type(SessionType::Orchestrator);
        assert_eq!(config.session_type, SessionType::Orchestrator);
    }

    #[test]
    fn test_with_scrollback() {
        let config = PtyConfig::default().with_scrollback(25000);
        assert_eq!(config.scrollback_lines, Some(25000));
    }

    #[test]
    fn test_effective_scrollback_with_override() {
        let pty_config = PtyConfig::default()
            .with_session_type(SessionType::Worker)
            .with_scrollback(10000);

        let scrollback_config = crate::config::ScrollbackConfig::default();
        assert_eq!(pty_config.effective_scrollback(&scrollback_config), 10000);
    }

    #[test]
    fn test_effective_scrollback_without_override() {
        let pty_config = PtyConfig::default().with_session_type(SessionType::Orchestrator);

        let scrollback_config = crate::config::ScrollbackConfig::default();
        assert_eq!(pty_config.effective_scrollback(&scrollback_config), 50000);
    }

    #[test]
    fn test_effective_scrollback_worker() {
        let pty_config = PtyConfig::default().with_session_type(SessionType::Worker);

        let scrollback_config = crate::config::ScrollbackConfig::default();
        assert_eq!(pty_config.effective_scrollback(&scrollback_config), 500);
    }

    #[test]
    fn test_builder_chain_with_scrollback() {
        let config = PtyConfig::command("bash")
            .with_session_type(SessionType::Orchestrator)
            .with_scrollback(75000)
            .with_size(120, 40);

        assert_eq!(config.command, "bash");
        assert_eq!(config.session_type, SessionType::Orchestrator);
        assert_eq!(config.scrollback_lines, Some(75000));
        assert_eq!(config.size, (120, 40));
    }

    #[test]
    fn test_with_ccmux_context() {
        let session_id = Uuid::new_v4();
        let session_name = "test-session";
        let window_id = Uuid::new_v4();
        let pane_id = Uuid::new_v4();

        let config = PtyConfig::default().with_ccmux_context(
            session_id,
            session_name,
            window_id,
            pane_id,
        );

        assert_eq!(
            config.env.get("CCMUX_SESSION_ID"),
            Some(&session_id.to_string())
        );
        assert_eq!(
            config.env.get("CCMUX_SESSION_NAME"),
            Some(&session_name.to_string())
        );
        assert_eq!(
            config.env.get("CCMUX_WINDOW_ID"),
            Some(&window_id.to_string())
        );
        assert_eq!(
            config.env.get("CCMUX_PANE_ID"),
            Some(&pane_id.to_string())
        );
    }

    #[test]
    fn test_with_ccmux_context_builder_chain() {
        let session_id = Uuid::new_v4();
        let window_id = Uuid::new_v4();
        let pane_id = Uuid::new_v4();

        let config = PtyConfig::command("bash")
            .with_cwd("/home/user")
            .with_ccmux_context(session_id, "my-session", window_id, pane_id)
            .with_size(120, 40);

        assert_eq!(config.command, "bash");
        assert_eq!(config.cwd, Some(PathBuf::from("/home/user")));
        assert_eq!(config.size, (120, 40));
        assert_eq!(config.env.len(), 4);
        assert!(config.env.contains_key("CCMUX_SESSION_ID"));
        assert!(config.env.contains_key("CCMUX_SESSION_NAME"));
        assert!(config.env.contains_key("CCMUX_WINDOW_ID"));
        assert!(config.env.contains_key("CCMUX_PANE_ID"));
    }

    // ==================== Beads Config Tests (FEAT-057) ====================

    #[test]
    fn test_with_beads_config_all_enabled() {
        let beads_config = crate::config::BeadsConfig {
            auto_detect: true,
            auto_set_beads_dir: true,
            no_daemon_default: true,
        };
        let beads_dir = PathBuf::from("/path/to/repo/.beads");

        let config = PtyConfig::default().with_beads_config(&beads_dir, &beads_config);

        assert_eq!(
            config.env.get("BEADS_DIR"),
            Some(&"/path/to/repo/.beads".to_string())
        );
        assert_eq!(config.env.get("BEADS_NO_DAEMON"), Some(&"true".to_string()));
    }

    #[test]
    fn test_with_beads_config_only_beads_dir() {
        let beads_config = crate::config::BeadsConfig {
            auto_detect: true,
            auto_set_beads_dir: true,
            no_daemon_default: false,
        };
        let beads_dir = PathBuf::from("/path/to/repo/.beads");

        let config = PtyConfig::default().with_beads_config(&beads_dir, &beads_config);

        assert_eq!(
            config.env.get("BEADS_DIR"),
            Some(&"/path/to/repo/.beads".to_string())
        );
        assert!(config.env.get("BEADS_NO_DAEMON").is_none());
    }

    #[test]
    fn test_with_beads_config_only_no_daemon() {
        let beads_config = crate::config::BeadsConfig {
            auto_detect: true,
            auto_set_beads_dir: false,
            no_daemon_default: true,
        };
        let beads_dir = PathBuf::from("/path/to/repo/.beads");

        let config = PtyConfig::default().with_beads_config(&beads_dir, &beads_config);

        assert!(config.env.get("BEADS_DIR").is_none());
        assert_eq!(config.env.get("BEADS_NO_DAEMON"), Some(&"true".to_string()));
    }

    #[test]
    fn test_with_beads_config_all_disabled() {
        let beads_config = crate::config::BeadsConfig {
            auto_detect: false,
            auto_set_beads_dir: false,
            no_daemon_default: false,
        };
        let beads_dir = PathBuf::from("/path/to/repo/.beads");

        let config = PtyConfig::default().with_beads_config(&beads_dir, &beads_config);

        assert!(config.env.get("BEADS_DIR").is_none());
        assert!(config.env.get("BEADS_NO_DAEMON").is_none());
    }

    #[test]
    fn test_with_beads_config_builder_chain() {
        let beads_config = crate::config::BeadsConfig::default();
        let beads_dir = PathBuf::from("/project/.beads");

        let config = PtyConfig::command("bash")
            .with_cwd("/project/src")
            .with_beads_config(&beads_dir, &beads_config)
            .with_size(120, 40);

        assert_eq!(config.command, "bash");
        assert_eq!(config.cwd, Some(PathBuf::from("/project/src")));
        assert_eq!(config.size, (120, 40));
        assert!(config.env.contains_key("BEADS_DIR"));
    }
}
