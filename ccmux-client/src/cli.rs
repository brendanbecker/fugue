//! Command-line argument parsing for ccmux client
//!
//! Uses clap for argument parsing with derive macros.

use clap::Parser;
use std::path::PathBuf;

/// ccmux - Claude Code terminal multiplexer client
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Disable automatic server startup
    ///
    /// By default, ccmux will automatically start the server daemon if it's
    /// not already running. Use this flag to disable that behavior and fail
    /// immediately if the server is not running.
    #[arg(long, default_value_t = false)]
    pub no_auto_start: bool,

    /// Server startup timeout in milliseconds
    ///
    /// How long to wait for the server to start when auto-starting.
    /// Only applies when auto-start is enabled.
    #[arg(long, default_value_t = 2000)]
    pub server_timeout: u64,

    /// Custom socket path
    ///
    /// Override the default Unix socket path for connecting to the server.
    #[arg(long, short = 'S')]
    pub socket: Option<PathBuf>,

    /// Connection address (tcp://host:port or unix://path)
    ///
    /// Specifies the server address to connect to. Supports both TCP and Unix
    /// sockets via URL format. Overrides --socket if provided.
    /// Example: tcp://127.0.0.1:3000 or unix:///tmp/ccmux.sock
    #[arg(long, env = "CCMUX_ADDR")]
    pub addr: Option<String>,

    /// Command to run in new sessions (overrides default_command from config)
    ///
    /// Example: ccmux claude --resume
    /// Example: ccmux bash
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    pub command: Vec<String>,
}

impl Args {
    /// Parse command-line arguments
    pub fn parse_args() -> Self {
        Self::parse()
    }

    /// Check if auto-start is enabled
    pub fn auto_start_enabled(&self) -> bool {
        !self.no_auto_start
    }

    /// Get the command string if provided
    pub fn command_string(&self) -> Option<String> {
        if self.command.is_empty() {
            None
        } else {
            Some(self.command.join(" "))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_args() {
        let args = Args::parse_from(["ccmux"]);
        assert!(!args.no_auto_start);
        assert!(args.auto_start_enabled());
        assert_eq!(args.server_timeout, 2000);
        assert!(args.socket.is_none());
        assert!(args.command.is_empty());
        assert!(args.command_string().is_none());
    }

    #[test]
    fn test_no_auto_start_flag() {
        let args = Args::parse_from(["ccmux", "--no-auto-start"]);
        assert!(args.no_auto_start);
        assert!(!args.auto_start_enabled());
    }

    #[test]
    fn test_server_timeout() {
        let args = Args::parse_from(["ccmux", "--server-timeout", "5000"]);
        assert_eq!(args.server_timeout, 5000);
    }

    #[test]
    fn test_socket_path() {
        let args = Args::parse_from(["ccmux", "-S", "/tmp/custom.sock"]);
        assert_eq!(args.socket, Some(PathBuf::from("/tmp/custom.sock")));

        let args = Args::parse_from(["ccmux", "--socket", "/tmp/other.sock"]);
        assert_eq!(args.socket, Some(PathBuf::from("/tmp/other.sock")));
    }

    #[test]
    fn test_addr_flag() {
        let args = Args::parse_from(["ccmux", "--addr", "tcp://localhost:3000"]);
        assert_eq!(args.addr, Some("tcp://localhost:3000".to_string()));
    }

    #[test]
    fn test_combined_flags() {
        let args = Args::parse_from([
            "ccmux",
            "--no-auto-start",
            "--server-timeout",
            "3000",
            "-S",
            "/run/ccmux.sock",
        ]);
        assert!(args.no_auto_start);
        assert_eq!(args.server_timeout, 3000);
        assert_eq!(args.socket, Some(PathBuf::from("/run/ccmux.sock")));
    }

    #[test]
    fn test_command_simple() {
        let args = Args::parse_from(["ccmux", "bash"]);
        assert_eq!(args.command, vec!["bash"]);
        assert_eq!(args.command_string(), Some("bash".to_string()));
    }

    #[test]
    fn test_command_with_args() {
        let args = Args::parse_from(["ccmux", "claude", "--resume"]);
        assert_eq!(args.command, vec!["claude", "--resume"]);
        assert_eq!(args.command_string(), Some("claude --resume".to_string()));
    }

    #[test]
    fn test_command_with_quoted_args() {
        let args = Args::parse_from(["ccmux", "claude", "do the thing"]);
        assert_eq!(args.command, vec!["claude", "do the thing"]);
        assert_eq!(
            args.command_string(),
            Some("claude do the thing".to_string())
        );
    }

    #[test]
    fn test_flags_before_command() {
        let args = Args::parse_from([
            "ccmux",
            "--no-auto-start",
            "-S",
            "/tmp/sock",
            "claude",
            "--resume",
        ]);
        assert!(args.no_auto_start);
        assert_eq!(args.socket, Some(PathBuf::from("/tmp/sock")));
        assert_eq!(args.command_string(), Some("claude --resume".to_string()));
    }
}
