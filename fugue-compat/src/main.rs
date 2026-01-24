//! ccmux-compat: tmux-compatible CLI wrapper for ccmux
//!
//! This binary provides a tmux-compatible CLI interface that translates
//! tmux commands to ccmux protocol messages, enabling drop-in replacement
//! for existing tmux workflows.

mod cli;
mod client;
mod commands;

use clap::Parser;
use cli::Cli;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    // Initialize logging from RUST_LOG env var, default to warn
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env()
                .add_directive("ccmux_compat=info".parse().unwrap()),
        )
        .with_writer(std::io::stderr)
        .init();

    let cli = Cli::parse();

    let exit_code = match commands::execute(cli.command, cli.addr).await {
        Ok(code) => code,
        Err(e) => {
            eprintln!("{}", e);
            1
        }
    };

    std::process::exit(exit_code);
}
