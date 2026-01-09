//! Sideband protocol for Claude-ccmux communication
//!
//! This module implements the XML sideband protocol that allows Claude to send
//! structured commands to ccmux embedded in its terminal output. The protocol
//! supports commands for pane management, input routing, notifications, and
//! viewport control.
//!
//! ## Protocol Format
//!
//! Commands are embedded using XML-like tags with the `ccmux:` namespace:
//!
//! ```xml
//! <!-- Self-closing commands -->
//! <ccmux:spawn direction="vertical" command="cargo build" />
//! <ccmux:focus pane="1" />
//! <ccmux:scroll lines="-10" />
//!
//! <!-- Commands with content -->
//! <ccmux:input pane="1">ls -la</ccmux:input>
//! <ccmux:notify title="Build Complete">Build succeeded</ccmux:notify>
//! ```
//!
//! ## Supported Commands
//!
//! | Command | Description |
//! |---------|-------------|
//! | `spawn` | Create a new pane with optional command |
//! | `focus` | Focus a specific pane |
//! | `input` | Send input text to a pane |
//! | `scroll` | Scroll pane viewport |
//! | `notify` | Display a notification |
//! | `control` | Pane control (close, resize, pin/unpin) |
//!
//! ## Processing Pipeline
//!
//! ```text
//! PTY Output → SidebandParser → (display_text, commands)
//!                    │
//!                    ├─→ Commands → CommandExecutor → SessionManager
//!                    │
//!                    └─→ Display Text → vt100 Parser → Client
//! ```
//!
//! Commands are stripped from the output before reaching the terminal display.

mod commands;
mod executor;
mod parser;

pub use commands::{ControlAction, NotifyLevel, PaneRef, SidebandCommand, SplitDirection};
pub use executor::{CommandExecutor, ExecuteError, ExecuteResult, SpawnResult};
pub use parser::SidebandParser;

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use parking_lot::Mutex;
    use crate::pty::PtyManager;
    use crate::registry::ClientRegistry;
    use crate::session::SessionManager;

    fn create_test_executor() -> (CommandExecutor, Arc<Mutex<SessionManager>>) {
        let manager = Arc::new(Mutex::new(SessionManager::new()));
        let pty_manager = Arc::new(Mutex::new(PtyManager::new()));
        let registry = Arc::new(ClientRegistry::new());
        let executor = CommandExecutor::new(
            Arc::clone(&manager),
            pty_manager,
            registry,
        );
        (executor, manager)
    }

    /// Integration test: parse and execute commands
    #[test]
    fn test_parse_and_execute_integration() {
        // Setup
        let (executor, manager) = create_test_executor();
        let mut parser = SidebandParser::new();

        // Create a test session with a pane
        let pane_id = {
            let mut mgr = manager.lock();
            let session = mgr.create_session("test").unwrap();
            let session_id = session.id();

            let session = mgr.get_session_mut(session_id).unwrap();
            let window = session.create_window(None);
            let window_id = window.id();

            let window = session.get_window_mut(window_id).unwrap();
            let pane = window.create_pane();
            pane.id()
        };

        // Parse input with embedded command
        let input = r#"Building project...<ccmux:notify title="Build Status">Starting build</ccmux:notify>
Output continues..."#;

        let (display, commands) = parser.parse(input);

        // Verify parsing
        assert_eq!(display, "Building project...\nOutput continues...");
        assert_eq!(commands.len(), 1);

        // Execute commands
        for cmd in commands {
            let result = executor.execute(cmd, pane_id);
            assert!(result.is_ok());
        }
    }

    /// Integration test: multiple commands in sequence
    #[test]
    fn test_multiple_commands_integration() {
        let (executor, manager) = create_test_executor();
        let mut parser = SidebandParser::new();

        // Create test pane
        let pane_id = {
            let mut mgr = manager.lock();
            let session = mgr.create_session("test").unwrap();
            let session_id = session.id();

            let session = mgr.get_session_mut(session_id).unwrap();
            let window = session.create_window(None);
            let window_id = window.id();

            let window = session.get_window_mut(window_id).unwrap();
            window.create_pane().id()
        };

        // Parse multiple commands
        let input = r#"<ccmux:focus pane="0" /><ccmux:control action="resize" cols="100" rows="50" /><ccmux:notify level="info">Ready</ccmux:notify>"#;

        let (display, commands) = parser.parse(input);

        assert_eq!(display, "");
        assert_eq!(commands.len(), 3);

        // Execute all
        let results = executor.execute_batch(commands, pane_id);
        assert!(results.iter().all(|r| r.is_ok()));

        // Verify resize was applied
        let mgr = manager.lock();
        let (_, _, pane) = mgr.find_pane(pane_id).unwrap();
        assert_eq!(pane.dimensions(), (100, 50));
    }

    /// Integration test: chunked parsing
    #[test]
    fn test_chunked_parsing_integration() {
        let mut parser = SidebandParser::new();

        // Simulate data arriving in chunks
        let chunk1 = "Hello <ccmux:noti";
        let chunk2 = r#"fy title="Test">Message</ccmux:notify> World"#;

        let (display1, commands1) = parser.parse(chunk1);
        assert_eq!(display1, "Hello ");
        assert!(commands1.is_empty());
        assert!(parser.has_buffered());

        let (display2, commands2) = parser.parse(chunk2);
        assert_eq!(display2, " World");
        assert_eq!(commands2.len(), 1);
        assert!(!parser.has_buffered());
    }

    /// Integration test: malformed commands are stripped but don't break flow
    #[test]
    fn test_malformed_commands_stripped() {
        let mut parser = SidebandParser::new();

        let input = r#"Start <ccmux:unknown attr="value" /> Middle <ccmux:focus pane="0" /> End"#;

        let (display, commands) = parser.parse(input);

        // Both tags stripped, only valid command parsed
        assert_eq!(display, "Start  Middle  End");
        assert_eq!(commands.len(), 1);
        assert!(matches!(commands[0], SidebandCommand::Focus { .. }));
    }

    /// Test command types match expected variants
    #[test]
    fn test_all_command_types() {
        let mut parser = SidebandParser::new();

        let inputs = vec![
            (r#"<ccmux:spawn direction="h" />"#, "Spawn"),
            (r#"<ccmux:focus pane="0" />"#, "Focus"),
            (r#"<ccmux:input pane="0">test</ccmux:input>"#, "Input"),
            (r#"<ccmux:scroll lines="-5" />"#, "Scroll"),
            (r#"<ccmux:notify>msg</ccmux:notify>"#, "Notify"),
            (r#"<ccmux:control action="close" />"#, "Control"),
        ];

        for (input, expected_type) in inputs {
            let (_, commands) = parser.parse(input);
            assert_eq!(commands.len(), 1, "Failed for: {}", input);
            let debug_str = format!("{:?}", commands[0]);
            assert!(debug_str.contains(expected_type), "Expected {} in {:?}", expected_type, debug_str);
        }
    }
}
