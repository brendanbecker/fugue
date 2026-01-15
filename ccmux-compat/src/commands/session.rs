//! Session management commands

use ccmux_protocol::{ClientMessage, ErrorCode, ServerMessage};
use ccmux_utils::Result;

use super::{connect, parse_target, Target};

/// Create a new session
pub async fn new_session(
    name: Option<String>,
    detached: bool,
    cwd: Option<String>,
    command: Vec<String>,
) -> Result<i32> {
    let mut client = connect().await?;

    // Build the command string if provided
    let cmd = if command.is_empty() {
        None
    } else {
        Some(command.join(" "))
    };

    let msg = ClientMessage::CreateSessionWithOptions {
        name,
        command: cmd,
        cwd,
        claude_model: None,
        claude_config: None,
        preset: None,
    };

    match client.request(msg).await? {
        ServerMessage::SessionCreatedWithDetails {
            session_id,
            session_name,
            ..
        } => {
            if !detached {
                // For now, just print the session info
                // Full attach would require the TUI client
                println!("{}: {} (created, run ccmux to attach)", session_name, session_id);
            }
            Ok(0)
        }
        ServerMessage::Error { code, message } => {
            eprintln!("error: {}", message);
            match code {
                ErrorCode::SessionNameExists => Ok(1),
                _ => Ok(1),
            }
        }
        other => {
            eprintln!("unexpected response: {:?}", std::mem::discriminant(&other));
            Ok(1)
        }
    }
}

/// Kill/destroy a session
pub async fn kill_session(target: &str) -> Result<i32> {
    let mut client = connect().await?;

    // First, find the session
    let session_id = match find_session(&mut client, target).await? {
        Some(id) => id,
        None => {
            eprintln!("session not found: {}", target);
            return Ok(1);
        }
    };

    let msg = ClientMessage::DestroySession { session_id };

    match client.request(msg).await? {
        ServerMessage::SessionEnded { .. } => Ok(0),
        ServerMessage::Error { message, .. } => {
            eprintln!("error: {}", message);
            Ok(1)
        }
        _ => Ok(0), // Session may have ended via different message
    }
}

/// List all sessions
pub async fn list_sessions(format: Option<&str>) -> Result<i32> {
    let mut client = connect().await?;

    let msg = ClientMessage::ListSessions;

    match client.request(msg).await? {
        ServerMessage::SessionList { sessions } => {
            if sessions.is_empty() {
                // tmux exits with 1 when no sessions
                return Ok(1);
            }

            for session in sessions {
                // Default format similar to tmux: name: windows (created date) (attached)
                let attached = if session.attached_clients > 0 {
                    "(attached)"
                } else {
                    ""
                };

                if let Some(fmt) = format {
                    // Support basic format strings
                    let output = fmt
                        .replace("#{session_name}", &session.name)
                        .replace("#{session_id}", &session.id.to_string())
                        .replace("#{session_windows}", &session.window_count.to_string())
                        .replace(
                            "#{session_attached}",
                            &session.attached_clients.to_string(),
                        );
                    println!("{}", output);
                } else {
                    println!(
                        "{}: {} windows {} {}",
                        session.name, session.window_count, attached, session.id
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

/// Check if a session exists
pub async fn has_session(target: &str) -> Result<i32> {
    let mut client = connect().await?;

    // Strip '=' prefix for exact match (tmux convention)
    let name = target.strip_prefix('=').unwrap_or(target);

    match find_session(&mut client, name).await? {
        Some(_) => Ok(0), // Session exists
        None => Ok(1),    // Session not found
    }
}

/// Attach to a session (placeholder - full attach requires TUI)
pub async fn attach_session(target: Option<&str>) -> Result<i32> {
    // Attaching requires the full TUI client, not this compat layer
    // For now, print a message directing users to ccmux
    eprintln!(
        "attach-session: use 'ccmux' for interactive session attachment"
    );
    if let Some(t) = target {
        eprintln!("target session: {}", t);
    }
    Ok(1)
}

/// Set an environment variable on a session
pub async fn set_environment(target: Option<&str>, name: String, value: String) -> Result<i32> {
    let mut client = connect().await?;

    // Default to active session if target is None
    let session_filter = target.unwrap_or("").to_string();

    let msg = ClientMessage::SetEnvironment {
        session_filter,
        key: name,
        value,
    };

    match client.request(msg).await? {
        ServerMessage::EnvironmentSet { .. } => Ok(0),
        ServerMessage::Error { message, .. } => {
            eprintln!("error: {}", message);
            Ok(1)
        }
        _ => Ok(0),
    }
}

/// Show environment variables from a session
pub async fn show_environment(target: Option<&str>, name: Option<String>) -> Result<i32> {
    let mut client = connect().await?;

    // Default to active session if target is None
    let session_filter = target.unwrap_or("").to_string();

    let msg = ClientMessage::GetEnvironment {
        session_filter,
        key: name,
    };

    match client.request(msg).await? {
        ServerMessage::EnvironmentList { environment, .. } => {
            let mut keys: Vec<_> = environment.keys().collect();
            keys.sort();
            for key in keys {
                println!("{}={}", key, environment[key]);
            }
            Ok(0)
        }
        ServerMessage::Error { message, .. } => {
            eprintln!("error: {}", message);
            Ok(1)
        }
        _ => Ok(1),
    }
}

/// Helper to find a session by name or UUID
async fn find_session(
    client: &mut super::super::client::Client,
    target: &str,
) -> Result<Option<uuid::Uuid>> {
    let parsed = parse_target(target);

    // If it's a UUID, use it directly
    if let Target::Uuid(uuid) = parsed {
        return Ok(Some(uuid));
    }

    // Otherwise, list sessions and find by name
    let msg = ClientMessage::ListSessions;

    match client.request(msg).await? {
        ServerMessage::SessionList { sessions } => {
            let name = match parsed {
                Target::SessionName(n) => n,
                Target::SessionWindow { session, .. } => session,
                Target::SessionWindowPane { session, .. } => session,
                Target::Uuid(u) => return Ok(Some(u)),
            };

            for session in sessions {
                if session.name == name {
                    return Ok(Some(session.id));
                }
            }
            Ok(None)
        }
        _ => Ok(None),
    }
}
