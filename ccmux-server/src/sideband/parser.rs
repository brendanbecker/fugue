//! Sideband command parser for extracting ccmux commands from PTY output
//!
//! Parses OSC (Operating System Command) escape sequences:
//! - Self-closing: `\x1b]ccmux:spawn direction="vertical"\x07`
//! - With content: `\x1b]ccmux:input pane="1"\x07ls -la\x1b]ccmux:/input\x07`
//!
//! The OSC format (ESC ] ... BEL) ensures commands won't accidentally trigger
//! from grep/cat output of source files containing command examples.

use std::collections::HashMap;

use regex::Regex;
use tracing::warn;
use ccmux_protocol::MailPriority;

use super::commands::{ControlAction, NotifyLevel, PaneRef, SidebandCommand, SplitDirection};

/// Parser for extracting sideband commands from terminal output
pub struct SidebandParser {
    /// Regex for matching OSC ccmux commands: ESC ] ccmux:cmd attrs BEL
    /// Format: \x1b]ccmux:CMD ATTRS\x07
    osc_command_regex: Regex,
    /// Regex for matching OSC ccmux closing tags: ESC ] ccmux:/cmd BEL
    /// Format: \x1b]ccmux:/CMD\x07
    osc_close_regex: Regex,
    /// Buffer for incomplete commands across chunks
    buffer: String,
}

impl Default for SidebandParser {
    fn default() -> Self {
        Self::new()
    }
}

impl SidebandParser {
    /// Create a new sideband parser
    pub fn new() -> Self {
        Self {
            // Match OSC commands: ESC ] ccmux:CMD ATTRS BEL (or ESC \)
            // Group 1: command type
            // Group 2: attributes string (may be empty)
            // Terminators: BEL (\x07) or ST (ESC \, i.e., \x1b\\)
            osc_command_regex: Regex::new(
                r"\x1b\]ccmux:(\w+)([^\x07\x1b]*)(?:\x07|\x1b\\)"
            ).expect("Invalid OSC command regex"),
            // Match OSC closing tags: ESC ] ccmux:/CMD BEL (or ESC \)
            // Group 1: command type being closed
            osc_close_regex: Regex::new(
                r"\x1b\]ccmux:/(\w+)(?:\x07|\x1b\\)"
            ).expect("Invalid OSC close regex"),
            buffer: String::new(),
        }
    }

    /// Parse output, returning (display_text, commands)
    ///
    /// - Extracts ccmux OSC commands from the input
    /// - Strips command sequences from display output
    /// - Buffers incomplete sequences for next chunk
    pub fn parse(&mut self, input: &str) -> (String, Vec<SidebandCommand>) {
        let full_input = format!("{}{}", self.buffer, input);
        self.buffer.clear();

        // Collect all opening commands with their positions
        // (start, end, cmd_type, attrs)
        let mut open_commands: Vec<(usize, usize, String, String)> = Vec::new();
        for cap in self.osc_command_regex.captures_iter(&full_input) {
            let m = cap.get(0).unwrap();
            let cmd_type = cap.get(1).unwrap().as_str().to_string();
            let attrs = cap.get(2).unwrap().as_str().trim().to_string();
            open_commands.push((m.start(), m.end(), cmd_type, attrs));
        }

        // Collect all closing tags with their positions
        // (start, end, cmd_type)
        let mut close_tags: Vec<(usize, usize, String)> = Vec::new();
        for cap in self.osc_close_regex.captures_iter(&full_input) {
            let m = cap.get(0).unwrap();
            let cmd_type = cap.get(1).unwrap().as_str().to_string();
            close_tags.push((m.start(), m.end(), cmd_type));
        }

        // Build list of complete commands with their ranges
        // (start, end, cmd_type, attrs, content)
        let mut all_matches: Vec<(usize, usize, String, String, String)> = Vec::new();

        for (open_start, open_end, cmd_type, attrs) in &open_commands {
            // Look for matching close tag after this open tag
            let mut found_close = false;
            for (close_start, close_end, close_type) in &close_tags {
                if close_start > open_end && close_type == cmd_type {
                    // Found matching close - this is a content command
                    let content = full_input[*open_end..*close_start].to_string();
                    all_matches.push((
                        *open_start,
                        *close_end,
                        cmd_type.clone(),
                        attrs.clone(),
                        content,
                    ));
                    found_close = true;
                    break;
                }
            }

            if !found_close {
                // No close tag - this is a self-closing command (no content)
                all_matches.push((
                    *open_start,
                    *open_end,
                    cmd_type.clone(),
                    attrs.clone(),
                    String::new(),
                ));
            }
        }

        // Sort matches by position
        all_matches.sort_by_key(|m| m.0);

        // Process matches and build output
        let mut commands = Vec::new();
        let mut display = String::new();
        let mut last_end = 0;

        for (start, end, cmd_type, attrs_str, content) in all_matches {
            // Skip overlapping matches
            if start < last_end {
                continue;
            }

            // Append text before this match
            display.push_str(&full_input[last_end..start]);
            last_end = end;

            // Parse the command
            match self.parse_command(&cmd_type, &attrs_str, &content) {
                Ok(cmd) => commands.push(cmd),
                Err(e) => {
                    warn!("Invalid sideband command: {}", e);
                    // Don't display malformed commands - just strip them
                }
            }
        }

        // Append remaining text
        display.push_str(&full_input[last_end..]);

        // Check for incomplete OSC sequence at end (buffer for next chunk)
        // Look for ESC ] ccmux: that doesn't have a terminator
        if let Some(incomplete_start) = display.rfind("\x1b]ccmux:") {
            // Check if there's a terminator after it
            let after_tag = &display[incomplete_start..];
            if !after_tag.contains('\x07') && !after_tag.contains("\x1b\\") {
                self.buffer = display[incomplete_start..].to_string();
                display.truncate(incomplete_start);
            }
        }

        (display, commands)
    }

    /// Check if there's buffered incomplete content
    pub fn has_buffered(&self) -> bool {
        !self.buffer.is_empty()
    }

    /// Clear the internal buffer
    pub fn clear_buffer(&mut self) {
        self.buffer.clear();
    }

    /// Parse a single command from its components
    fn parse_command(
        &self,
        cmd_type: &str,
        attrs_str: &str,
        content: &str,
    ) -> Result<SidebandCommand, String> {
        let attrs = Self::parse_attributes(attrs_str);

        match cmd_type {
            "spawn" => Ok(SidebandCommand::Spawn {
                direction: match attrs.get("direction").map(|s| s.as_str()) {
                    Some("horizontal") | Some("h") => SplitDirection::Horizontal,
                    _ => SplitDirection::Vertical, // Default to vertical
                },
                command: attrs.get("command").cloned(),
                cwd: attrs.get("cwd").cloned(),
                config: attrs.get("config").cloned(),
            }),

            "focus" => Ok(SidebandCommand::Focus {
                pane: self.parse_pane_ref(attrs.get("pane"))?,
            }),

            "input" => Ok(SidebandCommand::Input {
                pane: self.parse_pane_ref(attrs.get("pane"))?,
                text: content.to_string(),
            }),

            "scroll" => Ok(SidebandCommand::Scroll {
                pane: attrs
                    .get("pane")
                    .map(|p| self.parse_pane_ref(Some(p)))
                    .transpose()?,
                lines: attrs
                    .get("lines")
                    .and_then(|l| l.parse().ok())
                    .unwrap_or(-10),
            }),

            "notify" => Ok(SidebandCommand::Notify {
                title: attrs.get("title").cloned(),
                message: content.to_string(),
                level: match attrs.get("level").map(|s| s.as_str()) {
                    Some("warning") => NotifyLevel::Warning,
                    Some("error") => NotifyLevel::Error,
                    _ => NotifyLevel::Info,
                },
            }),

            "mail" => Ok(SidebandCommand::Mail {
                summary: attrs.get("summary").cloned().unwrap_or_else(|| content.to_string()),
                priority: match attrs.get("priority").map(|s| s.as_str()) {
                    Some("warning") => MailPriority::Warning,
                    Some("error") => MailPriority::Error,
                    _ => MailPriority::Info,
                },
            }),

            "control" => {
                let action = match attrs.get("action").map(|s| s.as_str()) {
                    Some("close") => ControlAction::Close,
                    Some("pin") => ControlAction::Pin,
                    Some("unpin") => ControlAction::Unpin,
                    Some("resize") => {
                        let cols = attrs
                            .get("cols")
                            .and_then(|c| c.parse().ok())
                            .ok_or("resize requires cols attribute")?;
                        let rows = attrs
                            .get("rows")
                            .and_then(|r| r.parse().ok())
                            .ok_or("resize requires rows attribute")?;
                        ControlAction::Resize { cols, rows }
                    }
                    Some(other) => return Err(format!("Unknown control action: {}", other)),
                    None => return Err("control requires action attribute".to_string()),
                };

                Ok(SidebandCommand::Control {
                    action,
                    pane: self.parse_pane_ref(attrs.get("pane"))?,
                })
            }

            "capabilities" => Ok(SidebandCommand::AdvertiseCapabilities {
                capabilities: content.to_string(),
            }),

            _ => Err(format!("Unknown command type: {}", cmd_type)),
        }
    }

    /// Parse XML-style attributes from a string
    fn parse_attributes(attrs_str: &str) -> HashMap<String, String> {
        // Match: key="value" or key='value'
        // Supports nested quotes if different from delimiter (e.g. key='{"a":1}')
        let attr_regex = Regex::new(r#"(\w+)=(?:"([^"]*)"|'([^']*)')"#).expect("Invalid attr regex");

        attr_regex
            .captures_iter(attrs_str)
            .map(|c| {
                let key = c.get(1).unwrap().as_str().to_string();
                // Value is in group 2 (double quotes) or group 3 (single quotes)
                let value = c.get(2).or_else(|| c.get(3)).map(|m| m.as_str()).unwrap_or("").to_string();
                (key, value)
            })
            .collect()
    }

    /// Parse a pane reference from an attribute value
    fn parse_pane_ref(&self, value: Option<&String>) -> Result<PaneRef, String> {
        match value {
            None => Ok(PaneRef::Active),
            Some(v) if v == "active" => Ok(PaneRef::Active),
            Some(v) => {
                // Try parsing as index first
                if let Ok(idx) = v.parse::<usize>() {
                    Ok(PaneRef::Index(idx))
                } else if let Ok(id) = uuid::Uuid::parse_str(v) {
                    Ok(PaneRef::Id(id))
                } else {
                    Err(format!("Invalid pane reference: {}", v))
                }
            }
        }
    }
}

impl std::fmt::Debug for SidebandParser {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SidebandParser")
            .field("buffer_len", &self.buffer.len())
            .field("has_buffered", &self.has_buffered())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn test_parse_spawn_command() {
        let mut parser = SidebandParser::new();
        let input = format!(
            "Hello {} World",
            osc(r#"spawn direction="vertical" command="npm test""#)
        );

        let (display, commands) = parser.parse(&input);

        assert_eq!(display, "Hello  World");
        assert_eq!(commands.len(), 1);
        assert!(matches!(
            commands[0],
            SidebandCommand::Spawn {
                direction: SplitDirection::Vertical,
                ..
            }
        ));
    }

    #[test]
    fn test_parse_spawn_horizontal() {
        let mut parser = SidebandParser::new();
        let input = osc(r#"spawn direction="horizontal""#);

        let (display, commands) = parser.parse(&input);

        assert_eq!(display, "");
        assert_eq!(commands.len(), 1);
        if let SidebandCommand::Spawn { direction, .. } = &commands[0] {
            assert_eq!(*direction, SplitDirection::Horizontal);
        } else {
            panic!("Expected Spawn command");
        }
    }

    #[test]
    fn test_parse_spawn_with_config() {
        let mut parser = SidebandParser::new();
        let input = osc(r#"spawn config='{"timeout":30,"env":{"FOO":"bar"}}'"#);

        let (_, commands) = parser.parse(&input);

        if let SidebandCommand::Spawn { config, .. } = &commands[0] {
            assert_eq!(config.as_deref(), Some(r#"{"timeout":30,"env":{"FOO":"bar"}}"#));
        } else {
            panic!("Expected Spawn command");
        }
    }

    #[test]
    fn test_parse_spawn_shorthand_direction() {
        let mut parser = SidebandParser::new();
        let input = osc(r#"spawn direction="h""#);

        let (_, commands) = parser.parse(&input);

        if let SidebandCommand::Spawn { direction, .. } = &commands[0] {
            assert_eq!(*direction, SplitDirection::Horizontal);
        } else {
            panic!("Expected Spawn command");
        }
    }

    #[test]
    fn test_parse_spawn_with_command_and_cwd() {
        let mut parser = SidebandParser::new();
        let input = osc(r#"spawn command="cargo build" cwd="/home/user""#);

        let (_, commands) = parser.parse(&input);

        if let SidebandCommand::Spawn { command, cwd, .. } = &commands[0] {
            assert_eq!(command.as_deref(), Some("cargo build"));
            assert_eq!(cwd.as_deref(), Some("/home/user"));
        } else {
            panic!("Expected Spawn command");
        }
    }

    #[test]
    fn test_parse_input_with_content() {
        let mut parser = SidebandParser::new();
        let input = osc_content("input", r#"pane="1""#, "ls -la");

        let (display, commands) = parser.parse(&input);

        assert_eq!(display, "");
        assert_eq!(commands.len(), 1);
        if let SidebandCommand::Input { pane, text } = &commands[0] {
            assert_eq!(*pane, PaneRef::Index(1));
            assert_eq!(text, "ls -la");
        } else {
            panic!("Expected Input command");
        }
    }

    #[test]
    fn test_parse_input_active_pane() {
        let mut parser = SidebandParser::new();
        let input = osc_content("input", "", "echo hello");

        let (_, commands) = parser.parse(&input);

        if let SidebandCommand::Input { pane, text } = &commands[0] {
            assert_eq!(*pane, PaneRef::Active);
            assert_eq!(text, "echo hello");
        } else {
            panic!("Expected Input command");
        }
    }

    #[test]
    fn test_parse_focus_command() {
        let mut parser = SidebandParser::new();
        let input = osc(r#"focus pane="2""#);

        let (_, commands) = parser.parse(&input);

        if let SidebandCommand::Focus { pane } = &commands[0] {
            assert_eq!(*pane, PaneRef::Index(2));
        } else {
            panic!("Expected Focus command");
        }
    }

    #[test]
    fn test_parse_focus_by_uuid() {
        let mut parser = SidebandParser::new();
        let uuid = "550e8400-e29b-41d4-a716-446655440000";
        let input = osc(&format!(r#"focus pane="{}""#, uuid));

        let (_, commands) = parser.parse(&input);

        if let SidebandCommand::Focus { pane } = &commands[0] {
            assert!(matches!(pane, PaneRef::Id(_)));
        } else {
            panic!("Expected Focus command");
        }
    }

    #[test]
    fn test_parse_scroll_command() {
        let mut parser = SidebandParser::new();
        let input = osc(r#"scroll lines="-20" pane="0""#);

        let (_, commands) = parser.parse(&input);

        if let SidebandCommand::Scroll { pane, lines } = &commands[0] {
            assert_eq!(*pane, Some(PaneRef::Index(0)));
            assert_eq!(*lines, -20);
        } else {
            panic!("Expected Scroll command");
        }
    }

    #[test]
    fn test_parse_scroll_default_lines() {
        let mut parser = SidebandParser::new();
        let input = osc("scroll");

        let (_, commands) = parser.parse(&input);

        if let SidebandCommand::Scroll { pane, lines } = &commands[0] {
            assert_eq!(*pane, None);
            assert_eq!(*lines, -10); // Default
        } else {
            panic!("Expected Scroll command");
        }
    }

    #[test]
    fn test_parse_notify_command() {
        let mut parser = SidebandParser::new();
        let input = osc_content(
            "notify",
            r#"title="Build Complete" level="info""#,
            "Build succeeded with 0 errors",
        );

        let (_, commands) = parser.parse(&input);

        if let SidebandCommand::Notify {
            title,
            message,
            level,
        } = &commands[0]
        {
            assert_eq!(title.as_deref(), Some("Build Complete"));
            assert_eq!(message, "Build succeeded with 0 errors");
            assert_eq!(*level, NotifyLevel::Info);
        } else {
            panic!("Expected Notify command");
        }
    }

    #[test]
    fn test_parse_notify_warning() {
        let mut parser = SidebandParser::new();
        let input = osc_content("notify", r#"level="warning""#, "Warning message");

        let (_, commands) = parser.parse(&input);

        if let SidebandCommand::Notify { level, .. } = &commands[0] {
            assert_eq!(*level, NotifyLevel::Warning);
        } else {
            panic!("Expected Notify command");
        }
    }

    #[test]
    fn test_parse_notify_error() {
        let mut parser = SidebandParser::new();
        let input = osc_content("notify", r#"level="error""#, "Error!");

        let (_, commands) = parser.parse(&input);

        if let SidebandCommand::Notify { level, .. } = &commands[0] {
            assert_eq!(*level, NotifyLevel::Error);
        } else {
            panic!("Expected Notify command");
        }
    }

    #[test]
    fn test_parse_mail_command() {
        let mut parser = SidebandParser::new();
        let input = osc(r#"mail summary="Task complete" priority="info""#);

        let (_, commands) = parser.parse(&input);

        if let SidebandCommand::Mail { summary, priority } = &commands[0] {
            assert_eq!(summary, "Task complete");
            assert_eq!(*priority, MailPriority::Info);
        } else {
            panic!("Expected Mail command");
        }
    }

    #[test]
    fn test_parse_control_close() {
        let mut parser = SidebandParser::new();
        let input = osc(r#"control action="close" pane="1""#);

        let (_, commands) = parser.parse(&input);

        if let SidebandCommand::Control { action, pane } = &commands[0] {
            assert_eq!(*action, ControlAction::Close);
            assert_eq!(*pane, PaneRef::Index(1));
        } else {
            panic!("Expected Control command");
        }
    }

    #[test]
    fn test_parse_control_pin() {
        let mut parser = SidebandParser::new();
        let input = osc(r#"control action="pin""#);

        let (_, commands) = parser.parse(&input);

        if let SidebandCommand::Control { action, .. } = &commands[0] {
            assert_eq!(*action, ControlAction::Pin);
        } else {
            panic!("Expected Control command");
        }
    }

    #[test]
    fn test_parse_control_resize() {
        let mut parser = SidebandParser::new();
        let input = osc(r#"control action="resize" cols="120" rows="40""#);

        let (_, commands) = parser.parse(&input);

        if let SidebandCommand::Control { action, .. } = &commands[0] {
            assert_eq!(*action, ControlAction::Resize { cols: 120, rows: 40 });
        } else {
            panic!("Expected Control command");
        }
    }

    #[test]
    fn test_incomplete_osc_buffering() {
        let mut parser = SidebandParser::new();

        // First chunk with incomplete OSC sequence (no terminator)
        let (display1, commands1) = parser.parse("Hello \x1b]ccmux:spa");
        assert_eq!(display1, "Hello ");
        assert!(commands1.is_empty());
        assert!(parser.has_buffered());

        // Second chunk completes the OSC
        let (display2, commands2) = parser.parse("wn direction=\"h\"\x07");
        assert_eq!(display2, "");
        assert_eq!(commands2.len(), 1);
        assert!(!parser.has_buffered());
    }

    #[test]
    fn test_incomplete_content_command() {
        let mut parser = SidebandParser::new();

        // When there's an open tag without a close tag, parser treats it as
        // self-closing (no content). The "content" becomes display text.
        let (display, commands) = parser.parse("\x1b]ccmux:input pane=\"1\"\x07ls -la");

        // The open tag is parsed as a self-closing command (empty content)
        // "ls -la" becomes display text since there's no close tag
        assert_eq!(display, "ls -la");
        assert_eq!(commands.len(), 1);
        if let SidebandCommand::Input { text, .. } = &commands[0] {
            assert_eq!(text, ""); // No content captured without close tag
        } else {
            panic!("Expected Input command");
        }
    }

    #[test]
    fn test_malformed_command_stripped() {
        let mut parser = SidebandParser::new();
        let input = format!("{} visible", osc(r#"unknown foo="bar""#));

        let (display, commands) = parser.parse(&input);

        assert_eq!(display, " visible"); // Command stripped
        assert!(commands.is_empty()); // Unknown command ignored
    }

    #[test]
    fn test_multiple_commands() {
        let mut parser = SidebandParser::new();
        let input = format!(
            "{}{}",
            osc(r#"focus pane="0""#),
            osc_content("input", r#"pane="0""#, "test")
        );

        let (display, commands) = parser.parse(&input);

        assert_eq!(display, "");
        assert_eq!(commands.len(), 2);
    }

    #[test]
    fn test_mixed_content_and_commands() {
        let mut parser = SidebandParser::new();
        let input = format!(
            "Start {} Middle {} End",
            osc(r#"focus pane="1""#),
            osc(r#"scroll lines="5""#)
        );

        let (display, commands) = parser.parse(&input);

        assert_eq!(display, "Start  Middle  End");
        assert_eq!(commands.len(), 2);
    }

    #[test]
    fn test_no_commands() {
        let mut parser = SidebandParser::new();
        let input = "Just regular terminal output\nNo commands here";

        let (display, commands) = parser.parse(input);

        assert_eq!(display, input);
        assert!(commands.is_empty());
    }

    #[test]
    fn test_parser_default() {
        let parser = SidebandParser::default();
        assert!(!parser.has_buffered());
    }

    #[test]
    fn test_clear_buffer() {
        let mut parser = SidebandParser::new();

        // Create incomplete OSC sequence
        let _ = parser.parse("\x1b]ccmux:spawn");
        assert!(parser.has_buffered());

        parser.clear_buffer();
        assert!(!parser.has_buffered());
    }

    #[test]
    fn test_parser_debug() {
        let parser = SidebandParser::new();
        let debug_str = format!("{:?}", parser);
        assert!(debug_str.contains("SidebandParser"));
        assert!(debug_str.contains("buffer_len"));
    }

    #[test]
    fn test_single_quote_attributes() {
        let mut parser = SidebandParser::new();
        let input = osc("spawn direction='vertical' command='cargo test'");

        let (_, commands) = parser.parse(&input);

        if let SidebandCommand::Spawn {
            direction, command, ..
        } = &commands[0]
        {
            assert_eq!(*direction, SplitDirection::Vertical);
            assert_eq!(command.as_deref(), Some("cargo test"));
        } else {
            panic!("Expected Spawn command");
        }
    }

    #[test]
    fn test_pane_ref_active_explicit() {
        let mut parser = SidebandParser::new();
        let input = osc(r#"focus pane="active""#);

        let (_, commands) = parser.parse(&input);

        if let SidebandCommand::Focus { pane } = &commands[0] {
            assert_eq!(*pane, PaneRef::Active);
        } else {
            panic!("Expected Focus command");
        }
    }

    #[test]
    fn test_invalid_pane_ref() {
        let mut parser = SidebandParser::new();
        let input = osc(r#"focus pane="invalid-ref""#);

        let (_, commands) = parser.parse(&input);

        // Invalid pane ref should result in command being dropped
        assert!(commands.is_empty());
    }

    #[test]
    fn test_control_missing_action() {
        let mut parser = SidebandParser::new();
        let input = osc(r#"control pane="1""#);

        let (_, commands) = parser.parse(&input);

        // Missing action should result in command being dropped
        assert!(commands.is_empty());
    }

    #[test]
    fn test_control_unknown_action() {
        let mut parser = SidebandParser::new();
        let input = osc(r#"control action="unknown""#);

        let (_, commands) = parser.parse(&input);

        // Unknown action should result in command being dropped
        assert!(commands.is_empty());
    }

    #[test]
    fn test_empty_input() {
        let mut parser = SidebandParser::new();
        let (display, commands) = parser.parse("");

        assert_eq!(display, "");
        assert!(commands.is_empty());
    }

    #[test]
    fn test_whitespace_only() {
        let mut parser = SidebandParser::new();
        let (display, commands) = parser.parse("   \n\t  ");

        assert_eq!(display, "   \n\t  ");
        assert!(commands.is_empty());
    }

    #[test]
    fn test_command_at_start() {
        let mut parser = SidebandParser::new();
        let input = format!("{}After", osc(r#"focus pane="0""#));

        let (display, commands) = parser.parse(&input);

        assert_eq!(display, "After");
        assert_eq!(commands.len(), 1);
    }

    #[test]
    fn test_command_at_end() {
        let mut parser = SidebandParser::new();
        let input = format!("Before{}", osc(r#"focus pane="0""#));

        let (display, commands) = parser.parse(&input);

        assert_eq!(display, "Before");
        assert_eq!(commands.len(), 1);
    }

    #[test]
    fn test_consecutive_commands() {
        let mut parser = SidebandParser::new();
        let input = format!(
            "{}{}{}",
            osc(r#"focus pane="0""#),
            osc(r#"focus pane="1""#),
            osc(r#"focus pane="2""#)
        );

        let (display, commands) = parser.parse(&input);

        assert_eq!(display, "");
        assert_eq!(commands.len(), 3);
    }

    #[test]
    fn test_preserve_ansi_escapes() {
        let mut parser = SidebandParser::new();
        // ANSI color codes around a command - these should NOT be parsed as ccmux commands
        let input = format!(
            "\x1b[31mRed\x1b[0m {} \x1b[32mGreen\x1b[0m",
            osc(r#"focus pane="0""#)
        );

        let (display, commands) = parser.parse(&input);

        assert!(display.contains("\x1b[31m"));
        assert!(display.contains("\x1b[32m"));
        assert_eq!(commands.len(), 1);
    }

    #[test]
    fn test_control_resize_missing_dimensions() {
        let mut parser = SidebandParser::new();
        let input = osc(r#"control action="resize""#);

        let (_, commands) = parser.parse(&input);

        // Missing cols/rows should result in command being dropped
        assert!(commands.is_empty());
    }

    #[test]
    fn test_control_resize_partial_dimensions() {
        let mut parser = SidebandParser::new();
        let input = osc(r#"control action="resize" cols="80""#);

        let (_, commands) = parser.parse(&input);

        // Missing rows should result in command being dropped
        assert!(commands.is_empty());
    }

    #[test]
    fn test_old_xml_format_not_parsed() {
        // CRITICAL: Old XML format should NOT trigger commands anymore
        // This is the fix for the runaway spawning bug
        let mut parser = SidebandParser::new();
        let input = r#"<ccmux:spawn direction="vertical" /> some text"#;

        let (display, commands) = parser.parse(input);

        // Old format should pass through as plain text, not be parsed
        assert_eq!(display, input);
        assert!(commands.is_empty());
    }

    #[test]
    fn test_grep_output_not_parsed() {
        // Simulating grep output that shows source code containing old format
        let mut parser = SidebandParser::new();
        let input = r#"parser.rs:123: <ccmux:spawn direction="vertical" />"#;

        let (display, commands) = parser.parse(input);

        // Should pass through unchanged - old XML format not parsed
        assert_eq!(display, input);
        assert!(commands.is_empty());
    }

    #[test]
    fn test_st_terminator() {
        // Test ESC followed by backslash (ST) as terminator instead of BEL
        let mut parser = SidebandParser::new();
        // ST terminator is ESC followed by backslash: \x1b\x5c
        let input = "\x1b]ccmux:spawn direction=\"vertical\"\x1b\x5c";

        let (display, commands) = parser.parse(input);

        assert_eq!(display, "");
        assert_eq!(commands.len(), 1);
        if let SidebandCommand::Spawn { direction, .. } = &commands[0] {
            assert_eq!(*direction, SplitDirection::Vertical);
        } else {
            panic!("Expected Spawn command");
        }
    }
}
