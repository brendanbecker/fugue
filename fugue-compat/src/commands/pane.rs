//! Pane management commands

use ccmux_protocol::{ClientMessage, ServerMessage, SplitDirection};
use ccmux_utils::Result;
use uuid::Uuid;

use super::{connect, parse_target, Target};

/// Send keys to a pane
pub async fn send_keys(target: Option<&str>, _literal: bool, keys: &[String]) -> Result<i32> {
    let mut client = connect().await?;

    // Find the target pane
    let pane_id = match target {
        Some(t) => match find_pane(&mut client, t).await? {
            Some(id) => id,
            None => {
                eprintln!("pane not found: {}", t);
                return Ok(1);
            }
        },
        None => {
            // Use the first/active pane
            match get_first_pane(&mut client).await? {
                Some(id) => id,
                None => {
                    eprintln!("no panes available");
                    return Ok(1);
                }
            }
        }
    };

    // Convert keys to bytes
    let data = convert_keys_to_bytes(keys);

    let msg = ClientMessage::Input { pane_id, data };
    client.send(msg).await?;

    Ok(0)
}

/// Capture pane content
pub async fn capture_pane(
    target: Option<&str>,
    print: bool,
    _start_line: Option<i32>,
    line_count: Option<usize>,
) -> Result<i32> {
    let mut client = connect().await?;

    // Find the target pane
    let pane_id = match target {
        Some(t) => match find_pane(&mut client, t).await? {
            Some(id) => id,
            None => {
                eprintln!("pane not found: {}", t);
                return Ok(1);
            }
        },
        None => {
            // Use the first/active pane
            match get_first_pane(&mut client).await? {
                Some(id) => id,
                None => {
                    eprintln!("no panes available");
                    return Ok(1);
                }
            }
        }
    };

    let lines = line_count.unwrap_or(100).min(1000);

    let msg = ClientMessage::ReadPane { pane_id, lines };

    match client.request(msg).await? {
        ServerMessage::PaneContent { content, .. } => {
            if print {
                print!("{}", content);
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

/// Split a window/pane
pub async fn split_window(
    target: Option<&str>,
    horizontal: bool,
    _vertical: bool,
    cwd: Option<String>,
    command: Vec<String>,
) -> Result<i32> {
    let mut client = connect().await?;

    // Find the target pane to split
    let pane_id = match target {
        Some(t) => match find_pane(&mut client, t).await? {
            Some(id) => id,
            None => {
                eprintln!("pane not found: {}", t);
                return Ok(1);
            }
        },
        None => {
            match get_first_pane(&mut client).await? {
                Some(id) => id,
                None => {
                    eprintln!("no panes available");
                    return Ok(1);
                }
            }
        }
    };

    // Determine split direction
    // In tmux: -h is horizontal split (left/right), -v is vertical (top/bottom)
    // ccmux uses: Horizontal = stacked (top/bottom), Vertical = side-by-side
    // So tmux -h maps to ccmux Vertical, tmux -v maps to ccmux Horizontal
    let direction = if horizontal {
        SplitDirection::Vertical // tmux -h = side by side
    } else {
        SplitDirection::Horizontal // tmux -v or default = stacked
    };

    let cmd = if command.is_empty() {
        None
    } else {
        Some(command.join(" "))
    };

    let msg = ClientMessage::SplitPane {
        pane_id,
        direction,
        ratio: 0.5,
        command: cmd,
        cwd,
        select: true,
    };

    match client.request(msg).await? {
        ServerMessage::PaneSplit { new_pane_id, .. } => {
            tracing::debug!("Created pane: {}", new_pane_id);
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

/// Kill a pane
pub async fn kill_pane(target: Option<&str>) -> Result<i32> {
    let mut client = connect().await?;

    let pane_id = match target {
        Some(t) => match find_pane(&mut client, t).await? {
            Some(id) => id,
            None => {
                eprintln!("pane not found: {}", t);
                return Ok(1);
            }
        },
        None => {
            match get_first_pane(&mut client).await? {
                Some(id) => id,
                None => {
                    eprintln!("no panes available");
                    return Ok(1);
                }
            }
        }
    };

    let msg = ClientMessage::ClosePane { pane_id };

    match client.request(msg).await? {
        ServerMessage::PaneClosed { .. } => Ok(0),
        ServerMessage::Error { message, .. } => {
            eprintln!("error: {}", message);
            Ok(1)
        }
        _ => Ok(0),
    }
}

/// List all panes
pub async fn list_panes(target: Option<&str>, all: bool) -> Result<i32> {
    let mut client = connect().await?;

    let session_filter = if all { None } else { target.map(|s| s.to_string()) };

    let msg = ClientMessage::ListAllPanes { session_filter };

    match client.request(msg).await? {
        ServerMessage::AllPanesList { panes } => {
            for pane in panes {
                // Format: session:window.pane: [WxH] [title] [cwd]
                println!(
                    "{}:{}.{}: [{}x{}] {}",
                    pane.session_name,
                    pane.window_index,
                    pane.pane_index,
                    pane.cols,
                    pane.rows,
                    pane.cwd.unwrap_or_default()
                );
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

/// Helper to find a pane by target string
async fn find_pane(
    client: &mut super::super::client::Client,
    target: &str,
) -> Result<Option<Uuid>> {
    let parsed = parse_target(target);

    // If it's a UUID, use it directly
    if let Target::Uuid(uuid) = parsed {
        return Ok(Some(uuid));
    }

    // List all panes and find matching one
    let session_filter = match &parsed {
        Target::SessionName(s) => Some(s.clone()),
        Target::SessionWindow { session, .. } => Some(session.clone()),
        Target::SessionWindowPane { session, .. } => Some(session.clone()),
        Target::Uuid(u) => return Ok(Some(*u)),
    };

    let msg = ClientMessage::ListAllPanes { session_filter };

    match client.request(msg).await? {
        ServerMessage::AllPanesList { panes } => {
            // Try to find by indices if specified
            match parsed {
                Target::SessionWindowPane {
                    session,
                    window,
                    pane,
                } => {
                    let window_idx: usize = window.parse().unwrap_or(0);
                    let pane_idx: usize = pane.parse().unwrap_or(0);

                    for p in panes {
                        if p.session_name == session
                            && p.window_index == window_idx
                            && p.pane_index == pane_idx
                        {
                            return Ok(Some(p.id));
                        }
                    }
                }
                Target::SessionWindow { session, window } => {
                    let window_idx: usize = window.parse().unwrap_or(0);

                    for p in panes {
                        if p.session_name == session && p.window_index == window_idx {
                            return Ok(Some(p.id));
                        }
                    }
                }
                Target::SessionName(session) => {
                    for p in panes {
                        if p.session_name == session {
                            return Ok(Some(p.id));
                        }
                    }
                }
                Target::Uuid(u) => return Ok(Some(u)),
            }
            Ok(None)
        }
        _ => Ok(None),
    }
}

/// Get the first available pane
async fn get_first_pane(client: &mut super::super::client::Client) -> Result<Option<Uuid>> {
    let msg = ClientMessage::ListAllPanes {
        session_filter: None,
    };

    match client.request(msg).await? {
        ServerMessage::AllPanesList { panes } => {
            // Return focused pane if any, otherwise first pane
            for p in &panes {
                if p.is_focused {
                    return Ok(Some(p.id));
                }
            }
            Ok(panes.first().map(|p| p.id))
        }
        _ => Ok(None),
    }
}

/// Convert tmux key names to bytes
fn convert_keys_to_bytes(keys: &[String]) -> Vec<u8> {
    let mut data = Vec::new();

    for key in keys {
        match key.as_str() {
            // Special keys
            "Enter" | "C-m" => data.push(b'\r'),
            "Tab" | "C-i" => data.push(b'\t'),
            "Escape" | "C-[" => data.push(0x1b),
            "Space" => data.push(b' '),
            "BSpace" | "C-?" => data.push(0x7f),
            "C-c" => data.push(0x03),
            "C-d" => data.push(0x04),
            "C-z" => data.push(0x1a),
            "C-l" => data.push(0x0c),
            // Arrow keys
            "Up" => data.extend_from_slice(b"\x1b[A"),
            "Down" => data.extend_from_slice(b"\x1b[B"),
            "Right" => data.extend_from_slice(b"\x1b[C"),
            "Left" => data.extend_from_slice(b"\x1b[D"),
            // Function keys
            "F1" => data.extend_from_slice(b"\x1bOP"),
            "F2" => data.extend_from_slice(b"\x1bOQ"),
            "F3" => data.extend_from_slice(b"\x1bOR"),
            "F4" => data.extend_from_slice(b"\x1bOS"),
            // Default: send as literal text
            _ => data.extend_from_slice(key.as_bytes()),
        }
    }

    data
}
