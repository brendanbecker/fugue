//! ccmux client - Terminal UI for ccmux
//!
//! This is the main entry point for the ccmux terminal multiplexer client.
//! It provides a Ratatui-based interface for managing Claude Code sessions.

use ccmux_utils::{init_logging_with_config, LogConfig, Result};

mod commands;
mod connection;
mod input;
mod ui;

pub use commands::{is_command, parse_command, Command, ParseError};

use ui::App;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging to file (not stderr, since we're using the terminal)
    init_logging_with_config(LogConfig::client())?;
    tracing::info!("ccmux client starting");

    // Run the application
    match run_app().await {
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

async fn run_app() -> Result<()> {
    let mut app = App::new()?;
    app.run().await
}
