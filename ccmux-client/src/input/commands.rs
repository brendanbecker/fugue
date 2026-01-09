//! Client command dispatch for ccmux
//!
//! Handles client-side commands triggered by prefix key combinations
//! and command mode input.

/// Client commands that can be triggered by key bindings or command mode
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClientCommand {
    // Pane management
    /// Create a new pane (default split)
    CreatePane,
    /// Close the current pane
    ClosePane,
    /// Split pane vertically (new pane to the right)
    SplitVertical,
    /// Split pane horizontally (new pane below)
    SplitHorizontal,

    // Pane navigation
    /// Focus next pane
    NextPane,
    /// Focus previous pane
    PreviousPane,
    /// Focus pane to the left
    PaneLeft,
    /// Focus pane to the right
    PaneRight,
    /// Focus pane above
    PaneUp,
    /// Focus pane below
    PaneDown,
    /// Focus pane by index
    FocusPane(usize),

    // Window management
    /// Create a new window
    CreateWindow,
    /// Close current window
    CloseWindow,
    /// Switch to next window
    NextWindow,
    /// Switch to previous window
    PreviousWindow,
    /// List all windows
    ListWindows,
    /// Select window by index
    SelectWindow(usize),
    /// Rename current window
    RenameWindow(String),

    // Session management
    /// List all sessions
    ListSessions,
    /// Create a new session
    CreateSession(Option<String>),
    /// Rename current session
    RenameSession(String),

    // Copy/scroll mode
    /// Enter copy/scroll mode
    EnterCopyMode,
    /// Exit copy/scroll mode
    ExitCopyMode,
    /// Clear scrollback buffer
    ClearHistory,

    // Layout
    /// Toggle pane zoom (fullscreen)
    ToggleZoom,
    /// Select next layout preset
    NextLayout,
    /// Resize pane
    ResizePane {
        direction: ResizeDirection,
        amount: u16,
    },

    // Misc
    /// Show help
    ShowHelp,
    /// Reload configuration
    ReloadConfig,
    /// Show clock
    ShowClock,
}

/// Direction for pane resizing
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResizeDirection {
    Up,
    Down,
    Left,
    Right,
}

/// Command handler for parsing and executing commands
pub struct CommandHandler;

impl CommandHandler {
    /// Parse a command string from command mode input
    ///
    /// Supports commands like:
    /// - `split-window -h` / `split-window -v`
    /// - `new-window`
    /// - `select-window <n>`
    /// - `select-pane <n>`
    /// - `kill-pane`
    /// - `kill-window`
    /// - `rename-window <name>`
    /// - `rename-session <name>`
    /// - `list-sessions`
    /// - `list-windows`
    pub fn parse_command(input: &str) -> Option<ClientCommand> {
        let input = input.trim();
        if input.is_empty() {
            return None;
        }

        let mut parts = input.split_whitespace();
        let command = parts.next()?;

        match command {
            // Pane commands
            "split-window" | "split" | "sp" => {
                let flag = parts.next();
                match flag {
                    Some("-h") | Some("-horizontal") => Some(ClientCommand::SplitHorizontal),
                    Some("-v") | Some("-vertical") | None => Some(ClientCommand::SplitVertical),
                    _ => Some(ClientCommand::SplitVertical),
                }
            }
            "kill-pane" | "killp" => Some(ClientCommand::ClosePane),
            "select-pane" | "selectp" => {
                let direction_or_index = parts.next();
                match direction_or_index {
                    Some("-L") => Some(ClientCommand::PaneLeft),
                    Some("-R") => Some(ClientCommand::PaneRight),
                    Some("-U") => Some(ClientCommand::PaneUp),
                    Some("-D") => Some(ClientCommand::PaneDown),
                    Some(n) => n.parse().ok().map(ClientCommand::FocusPane),
                    None => Some(ClientCommand::NextPane),
                }
            }
            "last-pane" | "lastp" => Some(ClientCommand::PreviousPane),
            "resize-pane" | "resizep" => {
                let flag = parts.next();
                let amount: u16 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(1);
                match flag {
                    Some("-U") => Some(ClientCommand::ResizePane {
                        direction: ResizeDirection::Up,
                        amount,
                    }),
                    Some("-D") => Some(ClientCommand::ResizePane {
                        direction: ResizeDirection::Down,
                        amount,
                    }),
                    Some("-L") => Some(ClientCommand::ResizePane {
                        direction: ResizeDirection::Left,
                        amount,
                    }),
                    Some("-R") => Some(ClientCommand::ResizePane {
                        direction: ResizeDirection::Right,
                        amount,
                    }),
                    _ => None,
                }
            }

            // Window commands
            "new-window" | "neww" => Some(ClientCommand::CreateWindow),
            "kill-window" | "killw" => Some(ClientCommand::CloseWindow),
            "next-window" | "next" | "n" => Some(ClientCommand::NextWindow),
            "previous-window" | "prev" | "p" => Some(ClientCommand::PreviousWindow),
            "select-window" | "selectw" => {
                let index = parts.next().and_then(|s| s.parse().ok());
                index.map(ClientCommand::SelectWindow)
            }
            "rename-window" | "renamew" => {
                let name: String = parts.collect::<Vec<_>>().join(" ");
                if name.is_empty() {
                    None
                } else {
                    Some(ClientCommand::RenameWindow(name))
                }
            }
            "list-windows" | "lsw" => Some(ClientCommand::ListWindows),

            // Session commands
            "new-session" | "new" => {
                let name: String = parts.collect::<Vec<_>>().join(" ");
                if name.is_empty() {
                    Some(ClientCommand::CreateSession(None))
                } else {
                    Some(ClientCommand::CreateSession(Some(name)))
                }
            }
            "rename-session" | "rename" => {
                let name: String = parts.collect::<Vec<_>>().join(" ");
                if name.is_empty() {
                    None
                } else {
                    Some(ClientCommand::RenameSession(name))
                }
            }
            "list-sessions" | "ls" => Some(ClientCommand::ListSessions),

            // Copy mode
            "copy-mode" | "copy" => Some(ClientCommand::EnterCopyMode),
            "clear-history" | "clearhist" => Some(ClientCommand::ClearHistory),

            // Layout
            "zoom" | "resize-pane -Z" => Some(ClientCommand::ToggleZoom),
            "next-layout" | "layout" => Some(ClientCommand::NextLayout),

            // Help and misc
            "help" | "?" => Some(ClientCommand::ShowHelp),
            "source" | "reload" => Some(ClientCommand::ReloadConfig),
            "clock" | "clock-mode" => Some(ClientCommand::ShowClock),

            // Quit commands (handled at higher level)
            "quit" | "q" | "exit" => None, // Should be handled as detach/quit at app level

            _ => None,
        }
    }

    /// Get help text for available commands
    pub fn help_text() -> &'static str {
        r#"Available Commands:

Pane Commands:
  split-window [-h|-v]  Split current pane (default: vertical)
  kill-pane            Close current pane
  select-pane [-L|-R|-U|-D|<n>]  Select pane by direction or index
  resize-pane [-U|-D|-L|-R] [n]  Resize pane in direction

Window Commands:
  new-window           Create new window
  kill-window          Close current window
  next-window          Switch to next window
  previous-window      Switch to previous window
  select-window <n>    Select window by index
  rename-window <name> Rename current window
  list-windows         List all windows

Session Commands:
  new-session [name]   Create new session
  rename-session <name> Rename current session
  list-sessions        List all sessions

Other:
  copy-mode            Enter copy/scroll mode
  clear-history        Clear scrollback buffer
  zoom                 Toggle pane zoom
  help                 Show this help
"#
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_split_vertical() {
        assert_eq!(
            CommandHandler::parse_command("split-window -v"),
            Some(ClientCommand::SplitVertical)
        );
        assert_eq!(
            CommandHandler::parse_command("split -vertical"),
            Some(ClientCommand::SplitVertical)
        );
        assert_eq!(
            CommandHandler::parse_command("sp"),
            Some(ClientCommand::SplitVertical)
        );
    }

    #[test]
    fn test_parse_split_horizontal() {
        assert_eq!(
            CommandHandler::parse_command("split-window -h"),
            Some(ClientCommand::SplitHorizontal)
        );
        assert_eq!(
            CommandHandler::parse_command("split -horizontal"),
            Some(ClientCommand::SplitHorizontal)
        );
    }

    #[test]
    fn test_parse_kill_pane() {
        assert_eq!(
            CommandHandler::parse_command("kill-pane"),
            Some(ClientCommand::ClosePane)
        );
        assert_eq!(
            CommandHandler::parse_command("killp"),
            Some(ClientCommand::ClosePane)
        );
    }

    #[test]
    fn test_parse_select_pane_direction() {
        assert_eq!(
            CommandHandler::parse_command("select-pane -L"),
            Some(ClientCommand::PaneLeft)
        );
        assert_eq!(
            CommandHandler::parse_command("selectp -R"),
            Some(ClientCommand::PaneRight)
        );
        assert_eq!(
            CommandHandler::parse_command("select-pane -U"),
            Some(ClientCommand::PaneUp)
        );
        assert_eq!(
            CommandHandler::parse_command("select-pane -D"),
            Some(ClientCommand::PaneDown)
        );
    }

    #[test]
    fn test_parse_select_pane_index() {
        assert_eq!(
            CommandHandler::parse_command("select-pane 2"),
            Some(ClientCommand::FocusPane(2))
        );
    }

    #[test]
    fn test_parse_resize_pane() {
        assert_eq!(
            CommandHandler::parse_command("resize-pane -U 5"),
            Some(ClientCommand::ResizePane {
                direction: ResizeDirection::Up,
                amount: 5
            })
        );
        assert_eq!(
            CommandHandler::parse_command("resizep -D"),
            Some(ClientCommand::ResizePane {
                direction: ResizeDirection::Down,
                amount: 1
            })
        );
    }

    #[test]
    fn test_parse_window_commands() {
        assert_eq!(
            CommandHandler::parse_command("new-window"),
            Some(ClientCommand::CreateWindow)
        );
        assert_eq!(
            CommandHandler::parse_command("kill-window"),
            Some(ClientCommand::CloseWindow)
        );
        assert_eq!(
            CommandHandler::parse_command("next-window"),
            Some(ClientCommand::NextWindow)
        );
        assert_eq!(
            CommandHandler::parse_command("n"),
            Some(ClientCommand::NextWindow)
        );
        assert_eq!(
            CommandHandler::parse_command("previous-window"),
            Some(ClientCommand::PreviousWindow)
        );
        assert_eq!(
            CommandHandler::parse_command("p"),
            Some(ClientCommand::PreviousWindow)
        );
    }

    #[test]
    fn test_parse_select_window() {
        assert_eq!(
            CommandHandler::parse_command("select-window 3"),
            Some(ClientCommand::SelectWindow(3))
        );
        assert_eq!(CommandHandler::parse_command("selectw invalid"), None);
    }

    #[test]
    fn test_parse_rename_window() {
        assert_eq!(
            CommandHandler::parse_command("rename-window my window"),
            Some(ClientCommand::RenameWindow("my window".to_string()))
        );
        assert_eq!(CommandHandler::parse_command("rename-window"), None);
    }

    #[test]
    fn test_parse_session_commands() {
        assert_eq!(
            CommandHandler::parse_command("list-sessions"),
            Some(ClientCommand::ListSessions)
        );
        assert_eq!(
            CommandHandler::parse_command("ls"),
            Some(ClientCommand::ListSessions)
        );
        assert_eq!(
            CommandHandler::parse_command("new-session"),
            Some(ClientCommand::CreateSession(None))
        );
        assert_eq!(
            CommandHandler::parse_command("new my-session"),
            Some(ClientCommand::CreateSession(Some("my-session".to_string())))
        );
    }

    #[test]
    fn test_parse_rename_session() {
        assert_eq!(
            CommandHandler::parse_command("rename-session new-name"),
            Some(ClientCommand::RenameSession("new-name".to_string()))
        );
        assert_eq!(CommandHandler::parse_command("rename-session"), None);
    }

    #[test]
    fn test_parse_copy_mode() {
        assert_eq!(
            CommandHandler::parse_command("copy-mode"),
            Some(ClientCommand::EnterCopyMode)
        );
        assert_eq!(
            CommandHandler::parse_command("copy"),
            Some(ClientCommand::EnterCopyMode)
        );
    }

    #[test]
    fn test_parse_misc_commands() {
        assert_eq!(
            CommandHandler::parse_command("help"),
            Some(ClientCommand::ShowHelp)
        );
        assert_eq!(
            CommandHandler::parse_command("?"),
            Some(ClientCommand::ShowHelp)
        );
        assert_eq!(
            CommandHandler::parse_command("zoom"),
            Some(ClientCommand::ToggleZoom)
        );
        assert_eq!(
            CommandHandler::parse_command("clock"),
            Some(ClientCommand::ShowClock)
        );
    }

    #[test]
    fn test_parse_empty_and_invalid() {
        assert_eq!(CommandHandler::parse_command(""), None);
        assert_eq!(CommandHandler::parse_command("   "), None);
        assert_eq!(CommandHandler::parse_command("unknown-command"), None);
    }

    #[test]
    fn test_parse_quit_commands_return_none() {
        // Quit commands are handled at a higher level
        assert_eq!(CommandHandler::parse_command("quit"), None);
        assert_eq!(CommandHandler::parse_command("q"), None);
        assert_eq!(CommandHandler::parse_command("exit"), None);
    }

    #[test]
    fn test_help_text_not_empty() {
        let help = CommandHandler::help_text();
        assert!(!help.is_empty());
        assert!(help.contains("Pane Commands"));
        assert!(help.contains("Window Commands"));
        assert!(help.contains("Session Commands"));
    }

    #[test]
    fn test_client_command_equality() {
        assert_eq!(ClientCommand::CreatePane, ClientCommand::CreatePane);
        assert_ne!(ClientCommand::CreatePane, ClientCommand::ClosePane);

        assert_eq!(
            ClientCommand::FocusPane(1),
            ClientCommand::FocusPane(1)
        );
        assert_ne!(
            ClientCommand::FocusPane(1),
            ClientCommand::FocusPane(2)
        );
    }

    #[test]
    fn test_resize_direction_equality() {
        assert_eq!(ResizeDirection::Up, ResizeDirection::Up);
        assert_ne!(ResizeDirection::Up, ResizeDirection::Down);
    }

    #[test]
    fn test_client_command_debug() {
        let cmd = ClientCommand::CreatePane;
        let debug = format!("{:?}", cmd);
        assert_eq!(debug, "CreatePane");
    }

    #[test]
    fn test_client_command_clone() {
        let cmd = ClientCommand::RenameWindow("test".to_string());
        let cloned = cmd.clone();
        assert_eq!(cmd, cloned);
    }
}
