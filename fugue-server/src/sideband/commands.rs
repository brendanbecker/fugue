//! Sideband command types for Claude-fugue communication
//!
//! These commands are embedded in terminal output using XML-like tags:
//! - `<fugue:spawn direction="vertical" command="cargo build" />`
//! - `<fugue:input pane="1">ls -la</fugue:input>`

use uuid::Uuid;
use fugue_protocol::MailPriority;

/// Direction for pane splitting
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SplitDirection {
    Horizontal,
    #[default]
    Vertical,
}

impl From<SplitDirection> for fugue_protocol::SplitDirection {
    fn from(dir: SplitDirection) -> Self {
        match dir {
            SplitDirection::Horizontal => fugue_protocol::SplitDirection::Horizontal,
            SplitDirection::Vertical => fugue_protocol::SplitDirection::Vertical,
        }
    }
}

/// Sideband command from Claude
#[derive(Debug, Clone, PartialEq)]
pub enum SidebandCommand {
    /// Create a new pane
    Spawn {
        direction: SplitDirection,
        command: Option<String>,
        cwd: Option<String>,
        config: Option<String>,
    },
    /// Focus a specific pane
    Focus { pane: PaneRef },
    /// Send input to a pane
    Input { pane: PaneRef, text: String },
    /// Scroll pane content
    Scroll {
        pane: Option<PaneRef>,
        lines: i32,
    },
    /// Show notification
    Notify {
        title: Option<String>,
        message: String,
        level: NotifyLevel,
    },
    /// Send mail/summary to dashboard (FEAT-073)
    Mail {
        summary: String,
        priority: MailPriority,
    },
    /// Pane control action
    Control { action: ControlAction, pane: PaneRef },
    /// Advertise capabilities (identity, resume, etc.)
    AdvertiseCapabilities {
        /// Optional JSON payload containing capability declarations
        capabilities: String,
    },
}

/// Reference to a pane (by index, ID, or active)
#[derive(Debug, Clone, PartialEq, Default)]
pub enum PaneRef {
    /// Reference by numeric index within window
    Index(usize),
    /// Reference by UUID
    Id(Uuid),
    /// Reference the currently active pane
    #[default]
    Active,
}

/// Notification severity level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NotifyLevel {
    #[default]
    Info,
    Warning,
    Error,
}

/// Pane control actions
#[derive(Debug, Clone, PartialEq)]
pub enum ControlAction {
    /// Close the pane
    Close,
    /// Resize the pane to specific dimensions
    Resize { cols: u16, rows: u16 },
    /// Pin viewport (disable auto-scroll)
    Pin,
    /// Unpin viewport (re-enable auto-scroll)
    Unpin,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_direction_default() {
        assert_eq!(SplitDirection::default(), SplitDirection::Vertical);
    }

    #[test]
    fn test_pane_ref_default() {
        assert_eq!(PaneRef::default(), PaneRef::Active);
    }

    #[test]
    fn test_notify_level_default() {
        assert_eq!(NotifyLevel::default(), NotifyLevel::Info);
    }

    #[test]
    fn test_spawn_command() {
        let cmd = SidebandCommand::Spawn {
            direction: SplitDirection::Horizontal,
            command: Some("cargo test".to_string()),
            cwd: Some("/home/user/project".to_string()),
            config: Some("{\"timeout\": 30}".to_string()),
        };

        if let SidebandCommand::Spawn {
            direction,
            command,
            cwd,
            config,
        } = cmd
        {
            assert_eq!(direction, SplitDirection::Horizontal);
            assert_eq!(command, Some("cargo test".to_string()));
            assert_eq!(cwd, Some("/home/user/project".to_string()));
            assert_eq!(config, Some("{\"timeout\": 30}".to_string()));
        } else {
            panic!("Expected Spawn command");
        }
    }

    #[test]
    fn test_input_command() {
        let cmd = SidebandCommand::Input {
            pane: PaneRef::Index(2),
            text: "echo hello".to_string(),
        };

        if let SidebandCommand::Input { pane, text } = cmd {
            assert_eq!(pane, PaneRef::Index(2));
            assert_eq!(text, "echo hello");
        } else {
            panic!("Expected Input command");
        }
    }

    #[test]
    fn test_focus_command() {
        let uuid = Uuid::new_v4();
        let cmd = SidebandCommand::Focus {
            pane: PaneRef::Id(uuid),
        };

        if let SidebandCommand::Focus { pane } = cmd {
            assert_eq!(pane, PaneRef::Id(uuid));
        } else {
            panic!("Expected Focus command");
        }
    }

    #[test]
    fn test_scroll_command() {
        let cmd = SidebandCommand::Scroll {
            pane: Some(PaneRef::Index(0)),
            lines: -20,
        };

        if let SidebandCommand::Scroll { pane, lines } = cmd {
            assert_eq!(pane, Some(PaneRef::Index(0)));
            assert_eq!(lines, -20);
        } else {
            panic!("Expected Scroll command");
        }
    }

    #[test]
    fn test_notify_command() {
        let cmd = SidebandCommand::Notify {
            title: Some("Build Complete".to_string()),
            message: "Build succeeded with 0 errors".to_string(),
            level: NotifyLevel::Info,
        };

        if let SidebandCommand::Notify {
            title,
            message,
            level,
        } = cmd
        {
            assert_eq!(title, Some("Build Complete".to_string()));
            assert_eq!(message, "Build succeeded with 0 errors");
            assert_eq!(level, NotifyLevel::Info);
        } else {
            panic!("Expected Notify command");
        }
    }

    #[test]
    fn test_mail_command() {
        let cmd = SidebandCommand::Mail {
            summary: "Task completed".to_string(),
            priority: MailPriority::Info,
        };

        if let SidebandCommand::Mail { summary, priority } = cmd {
            assert_eq!(summary, "Task completed");
            assert_eq!(priority, MailPriority::Info);
        } else {
            panic!("Expected Mail command");
        }
    }

    #[test]
    fn test_control_close() {
        let cmd = SidebandCommand::Control {
            action: ControlAction::Close,
            pane: PaneRef::Active,
        };

        if let SidebandCommand::Control { action, pane } = cmd {
            assert_eq!(action, ControlAction::Close);
            assert_eq!(pane, PaneRef::Active);
        } else {
            panic!("Expected Control command");
        }
    }

    #[test]
    fn test_control_resize() {
        let cmd = SidebandCommand::Control {
            action: ControlAction::Resize { cols: 120, rows: 40 },
            pane: PaneRef::Index(1),
        };

        if let SidebandCommand::Control { action, pane } = cmd {
            assert_eq!(action, ControlAction::Resize { cols: 120, rows: 40 });
            assert_eq!(pane, PaneRef::Index(1));
        } else {
            panic!("Expected Control command");
        }
    }

    #[test]
    fn test_control_pin_unpin() {
        let pin_cmd = SidebandCommand::Control {
            action: ControlAction::Pin,
            pane: PaneRef::Active,
        };
        let unpin_cmd = SidebandCommand::Control {
            action: ControlAction::Unpin,
            pane: PaneRef::Active,
        };

        assert!(matches!(
            pin_cmd,
            SidebandCommand::Control {
                action: ControlAction::Pin,
                ..
            }
        ));
        assert!(matches!(
            unpin_cmd,
            SidebandCommand::Control {
                action: ControlAction::Unpin,
                ..
            }
        ));
    }

    #[test]
    fn test_pane_ref_variants() {
        let by_index = PaneRef::Index(5);
        let by_id = PaneRef::Id(Uuid::new_v4());
        let active = PaneRef::Active;

        assert!(matches!(by_index, PaneRef::Index(5)));
        assert!(matches!(by_id, PaneRef::Id(_)));
        assert!(matches!(active, PaneRef::Active));
    }

    #[test]
    fn test_command_clone() {
        let cmd = SidebandCommand::Input {
            pane: PaneRef::Active,
            text: "test".to_string(),
        };
        let cloned = cmd.clone();
        assert_eq!(cmd, cloned);
    }

    #[test]
    fn test_command_debug() {
        let cmd = SidebandCommand::Notify {
            title: None,
            message: "test".to_string(),
            level: NotifyLevel::Warning,
        };
        let debug_str = format!("{:?}", cmd);
        assert!(debug_str.contains("Notify"));
        assert!(debug_str.contains("Warning"));
    }
}
