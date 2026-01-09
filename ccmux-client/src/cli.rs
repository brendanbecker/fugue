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
}
