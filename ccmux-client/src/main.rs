//! ccmux client - Terminal UI for ccmux
//!
//! This is the main entry point for the ccmux terminal multiplexer client.
//! It provides a Ratatui-based interface for managing Claude Code sessions.

use ccmux_utils::{init_logging_with_config, LogConfig, Result};

mod auto_start;
mod cli;
mod commands;
mod config;
mod connection;
mod input;
mod ui;

pub use commands::{is_command, parse_command, Command, ParseError};

use auto_start::{ensure_server_running, AutoStartConfig, ServerStartResult};
use cli::Args;
use config::load_quick_bindings;
use ui::App;

#[tokio::main]
async fn main() -> Result<()> {
    // Parse command-line arguments first (before terminal setup)
    let args = Args::parse_args();

    // Initialize logging to file (not stderr, since we're using the terminal)
    init_logging_with_config(LogConfig::client())?;
    tracing::info!("ccmux client starting");
    tracing::debug!("CLI args: {:?}", args);

    // Run the application
    match run_app(args).await {
        Ok(()) => {
            tracing::info!("ccmux client exiting normally");
            Ok(())
        }
        Err(e) => {
            tracing::error!("ccmux client error: {}", e);
            // Print error to stderr after terminal restoration
            eprintln!("Error: {}", e);
            Err(e)
        }
    }
}

async fn run_app(args: Args) -> Result<()> {
    // Configure auto-start behavior based on CLI args
    let auto_start_config = AutoStartConfig {
        enabled: args.auto_start_enabled(),
        timeout_ms: args.server_timeout,
        ..Default::default()
    };

    // Ensure server is running (auto-start if enabled)
    match ensure_server_running(&auto_start_config).await {
        Ok(ServerStartResult::AlreadyRunning) => {
            tracing::info!("Server already running");
        }
        Ok(ServerStartResult::Started) => {
            tracing::info!("Server started automatically");
        }
        Ok(ServerStartResult::NotRunning) => {
            // Auto-start disabled and server not running
            eprintln!("Error: Server not running. Start it with 'ccmux-server' or run without --no-auto-start");
            return Err(ccmux_utils::CcmuxError::ServerNotRunning {
                path: ccmux_utils::socket_path(),
            });
        }
        Err(e) => {
            eprintln!("Error: Failed to start server: {}", e);
            return Err(e);
        }
    }

    // Load keybindings from config
    let quick_bindings = load_quick_bindings();

    // Create and run the app with optional custom socket path
    let mut app = if let Some(socket) = args.socket {
        App::with_socket_path(socket)?
    } else {
        App::new()?
    };

    // Apply loaded keybindings
    app.set_quick_bindings(quick_bindings);

    app.run().await
}
