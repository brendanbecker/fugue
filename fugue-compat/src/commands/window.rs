//! Window management commands

use fugue_protocol::{ClientMessage, ServerMessage};
use fugue_utils::Result;

use super::{connect, parse_target, Target};

/// Create a new window
pub async fn new_window(
    target: Option<&str>,
    name: Option<String>,
    command: Vec<String>,
) -> Result<i32> {
    let mut client = connect().await?;

    // Parse session from target if provided
    let session_filter = target.map(|t| {
        match parse_target(t) {
            Target::SessionName(s) => s,
            Target::SessionWindow { session, .. } => session,
            Target::SessionWindowPane { session, .. } => session,
            Target::Uuid(u) => u.to_string(),
        }
    });

    let cmd = if command.is_empty() {
        None
    } else {
        Some(command.join(" "))
    };

    let msg = ClientMessage::CreateWindowWithOptions {
        session_filter,
        name,
        command: cmd,
        cwd: None, // BUG-050: Default to inheriting from session
    };

    match client.request(msg).await? {
        ServerMessage::WindowCreatedWithDetails {
            window_id,
            session_name,
            ..
        } => {
            tracing::debug!("Created window {} in session {}", window_id, session_name);
            Ok(0)
        }
        ServerMessage::Error { message, .. } => {
            eprintln!("error: {}", message);
            Ok(1)
        }
        other => {
            eprintln!("unexpected response: {:?}", std::mem::discriminant(&other));
            Ok(1)
        }
    }
}

/// Kill a window
pub async fn kill_window(target: &str) -> Result<i32> {
    // fugue doesn't have a direct kill-window command yet
    // We would need to kill all panes in the window
    eprintln!("kill-window: not yet implemented (kill panes individually with kill-pane)");
    eprintln!("target: {}", target);
    Ok(1)
}

/// List windows in a session
pub async fn list_windows(target: Option<&str>) -> Result<i32> {
    let mut client = connect().await?;

    let session_filter = target.map(|t| {
        match parse_target(t) {
            Target::SessionName(s) => s,
            Target::SessionWindow { session, .. } => session,
            Target::SessionWindowPane { session, .. } => session,
            Target::Uuid(u) => u.to_string(),
        }
    });

    let msg = ClientMessage::ListWindows { session_filter };

    match client.request(msg).await? {
        ServerMessage::WindowList {
            session_name,
            windows,
        } => {
            if windows.is_empty() {
                eprintln!("no windows in session {}", session_name);
            } else {
                for window in windows {
                    // Format: index: name (pane_count panes)
                    println!(
                        "{}: {} ({} panes)",
                        window.index, window.name, window.pane_count
                    );
                }
            }
            Ok(0)
        }
        ServerMessage::Error { message, .. } => {
            eprintln!("error: {}", message);
            Ok(1)
        }
        other => {
            eprintln!("unexpected response: {:?}", std::mem::discriminant(&other));
            Ok(1)
        }
    }
}
