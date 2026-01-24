//! Sideband protocol for Claude-ccmux communication
//!
//! This module implements the OSC (Operating System Command) sideband protocol
//! that allows Claude to send structured commands to ccmux embedded in its
//! terminal output. The protocol supports commands for pane management, input
//! routing, notifications, and viewport control.
//!
//! ## Protocol Format
//!
//! Commands use OSC escape sequences (ESC ] ... BEL) with the `ccmux:` prefix:
//!
//! ```text
//! # Self-closing commands (terminated by BEL \x07 or ST \x1b\x5c)
//! \x1b]ccmux:spawn direction="vertical" command="cargo build"\x07
//! \x1b]ccmux:focus pane="1"\x07
//! \x1b]ccmux:scroll lines="-10"\x07
//!
//! # Commands with content
//! \x1b]ccmux:input pane="1"\x07ls -la\x1b]ccmux:/input\x07
//! \x1b]ccmux:notify title="Build Complete"\x07Build succeeded\x1b]ccmux:/notify\x07
//! ```
//!
//! The OSC format prevents accidental command triggering from grep/cat output
//! of source files containing command examples.
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
//! The sideband protocol is integrated into the PTY output flow via
//! `PtyOutputPoller::spawn_with_sideband()`. The pipeline:
//!
//! ```text
//! PTY Output → PtyOutputPoller.handle_output()
//!                    │
//!                    ├─→ SidebandParser.parse() → (display_text, commands)
//!                    │
//!                    ├─→ Commands → AsyncCommandExecutor → SessionManager
//!                    │
//!                    └─→ Display Text → Client Broadcast
//! ```
//!
//! Commands are stripped from the output before reaching the terminal display.
//!
//! ## Components
//!
//! - [`SidebandParser`]: Extracts XML commands from PTY output text
//! - [`AsyncCommandExecutor`]: Executes commands (async, uses tokio RwLock)
//! - [`CommandExecutor`]: Executes commands (sync, uses parking_lot Mutex)
//! - [`SidebandCommand`]: Enum of all command types
//! - [`SpawnResult`]: Result of spawn commands with PTY reader for new pollers

mod async_executor;
mod commands;
mod executor;
mod parser;

pub use async_executor::{AsyncCommandExecutor, SpawnLimits};
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

    // Helper to create OSC command string
    fn osc(cmd: &str) -> String {
        format!("\x1b]ccmux:{}\x07", cmd)
    }

    // Helper to create OSC command with content
    fn osc_content(cmd: &str, attrs: &str, content: &str) -> String {
        format!(
            "\x1b]ccmux:{} {}\x07{}\x1b]ccmux:/{}\x07",
            cmd, attrs, content, cmd
        )
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

        // Parse input with embedded OSC command
        let input = format!(
            "Building project...{}\nOutput continues...",
            osc_content("notify", r#"title="Build Status""#, "Starting build")
        );

        let (display, commands) = parser.parse(&input);

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

        // Parse multiple OSC commands
        let input = format!(
            "{}{}{}",
            osc(r#"focus pane="0""#),
            osc(r#"control action="resize" cols="100" rows="50""#),
            osc_content("notify", r#"level="info""#, "Ready")
        );

        let (display, commands) = parser.parse(&input);

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

        // Simulate OSC data arriving in chunks
        let chunk1 = "Hello \x1b]ccmux:noti";
        let chunk2 = "fy title=\"Test\"\x07Message\x1b]ccmux:/notify\x07 World";

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

        // Mix of unknown command and valid command
        let input = format!(
            "Start {} Middle {} End",
            osc(r#"unknown attr="value""#),
            osc(r#"focus pane="0""#)
        );

        let (display, commands) = parser.parse(&input);

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
            (osc(r#"spawn direction="h""#), "Spawn"),
            (osc(r#"focus pane="0""#), "Focus"),
            (osc_content("input", r#"pane="0""#, "test"), "Input"),
            (osc(r#"scroll lines="-5""#), "Scroll"),
            (osc_content("notify", "", "msg"), "Notify"),
            (osc(r#"control action="close""#), "Control"),
        ];

        for (input, expected_type) in inputs {
            let (_, commands) = parser.parse(&input);
            assert_eq!(commands.len(), 1, "Failed for: {:?}", input);
            let debug_str = format!("{:?}", commands[0]);
            assert!(debug_str.contains(expected_type), "Expected {} in {:?}", expected_type, debug_str);
        }
    }

    /// Test that old XML format is NOT parsed (security fix for BUG-024)
    #[test]
    fn test_old_xml_format_ignored() {
        let mut parser = SidebandParser::new();

        // Old XML format should pass through as plain text
        let input = r#"<ccmux:spawn direction="vertical" /> some grep output"#;
        let (display, commands) = parser.parse(input);

        assert_eq!(display, input);
        assert!(commands.is_empty(), "Old XML format should NOT be parsed");
    }
}
