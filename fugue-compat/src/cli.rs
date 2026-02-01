//! CLI argument parsing for tmux-compatible commands

use clap::{Parser, Subcommand};

/// tmux-compatible CLI for fugue
#[derive(Parser, Debug)]
#[command(name = "fugue-compat")]
#[command(about = "tmux-compatible CLI wrapper for fugue")]
#[command(version)]
pub struct Cli {
    /// Connection address (tcp://host:port or unix://path)
    ///
    /// Specifies the server address to connect to. Supports both TCP and Unix
    /// sockets via URL format. Overrides default Unix socket if provided.
    /// Example: tcp://127.0.0.1:3000 or unix:///tmp/fugue.sock
    #[arg(long, env = "FUGUE_ADDR")]
    pub addr: Option<String>,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Create a new session
    #[command(name = "new-session")]
    NewSession {
        /// Session name
        #[arg(short = 's', long)]
        name: Option<String>,

        /// Start session detached
        #[arg(short = 'd', long)]
        detached: bool,

        /// Working directory
        #[arg(short = 'c', long)]
        cwd: Option<String>,

        /// Tags for the session (comma-separated or repeated)
        #[arg(long, value_delimiter = ',')]
        tags: Vec<String>,

        /// Command to run (positional arguments after options)
        #[arg(trailing_var_arg = true)]
        command: Vec<String>,
    },

    /// Kill/destroy a session
    #[command(name = "kill-session")]
    KillSession {
        /// Target session
        #[arg(short = 't', long = "target")]
        target: String,
    },

    /// List all sessions
    #[command(name = "list-sessions")]
    ListSessions {
        /// Format string (limited support)
        #[arg(short = 'F', long)]
        format: Option<String>,
    },

    /// Check if session exists
    #[command(name = "has-session")]
    HasSession {
        /// Target session (use =NAME for exact match)
        #[arg(short = 't', long = "target")]
        target: String,
    },

    /// Attach to a session
    #[command(name = "attach-session", alias = "attach")]
    AttachSession {
        /// Target session
        #[arg(short = 't', long = "target")]
        target: Option<String>,
    },

    /// Send keys to a pane
    #[command(name = "send-keys")]
    SendKeys {
        /// Target pane
        #[arg(short = 't', long = "target")]
        target: Option<String>,

        /// Disable key lookup (send literal)
        #[arg(short = 'l', long)]
        literal: bool,

        /// Keys to send
        #[arg(trailing_var_arg = true, required = true)]
        keys: Vec<String>,
    },

    /// Capture pane content
    #[command(name = "capture-pane")]
    CapturePane {
        /// Target pane
        #[arg(short = 't', long = "target")]
        target: Option<String>,

        /// Print output to stdout
        #[arg(short = 'p', long)]
        print: bool,

        /// Start line (negative = from start of scrollback)
        #[arg(short = 'S', long)]
        start_line: Option<i32>,

        /// Number of lines to capture
        #[arg(short = 'N', long)]
        line_count: Option<usize>,
    },

    /// Create a new window
    #[command(name = "new-window")]
    NewWindow {
        /// Target session
        #[arg(short = 't', long = "target")]
        target: Option<String>,

        /// Window name
        #[arg(short = 'n', long)]
        name: Option<String>,

        /// Command to run
        #[arg(trailing_var_arg = true)]
        command: Vec<String>,
    },

    /// Kill a window
    #[command(name = "kill-window")]
    KillWindow {
        /// Target window
        #[arg(short = 't', long = "target")]
        target: String,
    },

    /// List windows in a session
    #[command(name = "list-windows")]
    ListWindows {
        /// Target session
        #[arg(short = 't', long = "target")]
        target: Option<String>,
    },

    /// Split a window into panes
    #[command(name = "split-window")]
    SplitWindow {
        /// Target pane
        #[arg(short = 't', long = "target")]
        target: Option<String>,

        /// Horizontal split (top/bottom)
        #[arg(short = 'h', long)]
        horizontal: bool,

        /// Vertical split (left/right) - default
        #[arg(short = 'v', long)]
        vertical: bool,

        /// Working directory
        #[arg(short = 'c', long)]
        cwd: Option<String>,

        /// Command to run
        #[arg(trailing_var_arg = true)]
        command: Vec<String>,
    },

    /// Kill a pane
    #[command(name = "kill-pane")]
    KillPane {
        /// Target pane
        #[arg(short = 't', long = "target")]
        target: Option<String>,
    },

    /// Set an environment variable
    #[command(name = "set-environment")]
    SetEnvironment {
        /// Target session
        #[arg(short = 't', long = "target")]
        target: Option<String>,

        /// Variable name
        name: String,

        /// Variable value
        value: String,
    },

    /// Show environment variables
    #[command(name = "show-environment")]
    ShowEnvironment {
        /// Target session
        #[arg(short = 't', long = "target")]
        target: Option<String>,

        /// Variable name (optional)
        name: Option<String>,
    },

    /// List panes
    #[command(name = "list-panes")]
    ListPanes {
        /// Target session
        #[arg(short = 't', long = "target")]
        target: Option<String>,

        /// Show all panes in all sessions
        #[arg(short = 'a', long)]
        all: bool,
    },
}
