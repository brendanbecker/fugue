//! Command implementations

mod pane;
mod session;
mod window;

use crate::cli::Command;
use crate::client::Client;
use ccmux_utils::Result;
use std::sync::OnceLock;

/// Global storage for server address
static SERVER_ADDR: OnceLock<Option<String>> = OnceLock::new();

/// Execute a CLI command
pub async fn execute(command: Command, addr: Option<String>) -> Result<i32> {
    // Store address for use by connect()
    SERVER_ADDR.set(addr).ok();

    match command {
        // Session commands
        Command::NewSession {
            name,
            detached,
            cwd,
            command,
        } => session::new_session(name, detached, cwd, command).await,

        Command::KillSession { target } => session::kill_session(&target).await,

        Command::ListSessions { format } => session::list_sessions(format.as_deref()).await,

        Command::HasSession { target } => session::has_session(&target).await,

        Command::AttachSession { target } => session::attach_session(target.as_deref()).await,

        // Pane commands
        Command::SendKeys {
            target,
            literal,
            keys,
        } => pane::send_keys(target.as_deref(), literal, &keys).await,

        Command::CapturePane {
            target,
            print,
            start_line,
            line_count,
        } => pane::capture_pane(target.as_deref(), print, start_line, line_count).await,

        Command::SplitWindow {
            target,
            horizontal,
            vertical,
            cwd,
            command,
        } => pane::split_window(target.as_deref(), horizontal, vertical, cwd, command).await,

        Command::KillPane { target } => pane::kill_pane(target.as_deref()).await,

        Command::ListPanes { target, all } => pane::list_panes(target.as_deref(), all).await,

        // Window commands
        Command::NewWindow {
            target,
            name,
            command,
        } => window::new_window(target.as_deref(), name, command).await,

        Command::KillWindow { target } => window::kill_window(&target).await,

        Command::ListWindows { target } => window::list_windows(target.as_deref()).await,

        // Environment commands
        Command::SetEnvironment {
            target,
            name,
            value,
        } => session::set_environment(target.as_deref(), name, value).await,

        Command::ShowEnvironment { target, name } => {
            session::show_environment(target.as_deref(), name).await
        }
    }
}

/// Helper to connect to the server
async fn connect() -> Result<Client> {
    let addr = SERVER_ADDR.get().cloned().flatten();
    Client::connect(addr).await
}

/// Parse a target string which may be a session name, UUID, or session:window:pane
fn parse_target(target: &str) -> Target {
    // Handle exact match prefix (=name)
    let target = target.strip_prefix('=').unwrap_or(target);

    // Try parsing as UUID first
    if let Ok(uuid) = uuid::Uuid::parse_str(target) {
        return Target::Uuid(uuid);
    }

    // Check for session:window:pane format
    let parts: Vec<&str> = target.split(':').collect();
    match parts.len() {
        1 => Target::SessionName(parts[0].to_string()),
        2 => Target::SessionWindow {
            session: parts[0].to_string(),
            window: parts[1].to_string(),
        },
        3 => Target::SessionWindowPane {
            session: parts[0].to_string(),
            window: parts[1].to_string(),
            pane: parts[2].to_string(),
        },
        _ => Target::SessionName(target.to_string()),
    }
}

/// Parsed target specification
#[derive(Debug)]
enum Target {
    /// UUID directly
    Uuid(uuid::Uuid),
    /// Session name only
    SessionName(String),
    /// Session and window
    SessionWindow { session: String, window: String },
    /// Session, window, and pane
    SessionWindowPane {
        session: String,
        window: String,
        pane: String,
    },
}
