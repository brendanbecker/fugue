//! Client command parsing and handling
//!
//! Parses user input commands like `/reply worker-3 "message"` and
//! converts them into protocol messages.

use fugue_protocol::{PaneTarget, ReplyMessage};
use uuid::Uuid;

/// Parsed client command
#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    /// Reply to a pane awaiting input
    Reply(ReplyMessage),
    /// Unknown or invalid command
    Unknown(String),
}

/// Error parsing a command
#[derive(Debug, Clone, PartialEq)]
pub enum ParseError {
    /// Empty command
    Empty,
    /// Missing target pane
    MissingTarget,
    /// Missing message content
    MissingMessage,
    /// Invalid syntax
    InvalidSyntax(String),
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::Empty => write!(f, "empty command"),
            ParseError::MissingTarget => write!(f, "missing target pane"),
            ParseError::MissingMessage => write!(f, "missing message content"),
            ParseError::InvalidSyntax(msg) => write!(f, "invalid syntax: {}", msg),
        }
    }
}

impl std::error::Error for ParseError {}

/// Parse a command string into a Command
///
/// # Supported commands
///
/// - `/reply <pane> <message>` - Send a reply to a pane
///   - `<pane>` can be a UUID or a pane name
///   - `<message>` can be quoted or unquoted
///
/// # Examples
///
/// ```
/// use fugue_client::commands::parse_command;
///
/// let cmd = parse_command("/reply worker-3 yes").unwrap();
/// let cmd = parse_command("/reply worker-3 \"use async, we need non-blocking\"").unwrap();
/// let cmd = parse_command("/reply 12345678-1234-1234-1234-123456789012 proceed").unwrap();
/// ```
pub fn parse_command(input: &str) -> Result<Command, ParseError> {
    let input = input.trim();

    if input.is_empty() {
        return Err(ParseError::Empty);
    }

    // Must start with /
    if !input.starts_with('/') {
        return Err(ParseError::InvalidSyntax(
            "command must start with /".to_string(),
        ));
    }

    // Split command and args
    let parts: Vec<&str> = input[1..].splitn(2, char::is_whitespace).collect();
    let command_name = parts.first().map(|s| s.to_lowercase()).unwrap_or_default();

    match command_name.as_str() {
        "reply" => parse_reply_command(parts.get(1).unwrap_or(&"")),
        _ => Ok(Command::Unknown(command_name)),
    }
}

/// Parse the /reply command arguments
///
/// Format: `/reply <target> <message>`
///
/// Target can be:
/// - A UUID: `12345678-1234-1234-1234-123456789012`
/// - A pane name: `worker-3`
///
/// Message can be:
/// - Unquoted: `yes`
/// - Quoted: `"use async, we need non-blocking"`
fn parse_reply_command(args: &str) -> Result<Command, ParseError> {
    let args = args.trim();

    if args.is_empty() {
        return Err(ParseError::MissingTarget);
    }

    // Find the target (first token)
    let (target_str, rest) = split_first_token(args);

    if target_str.is_empty() {
        return Err(ParseError::MissingTarget);
    }

    let rest = rest.trim();
    if rest.is_empty() {
        return Err(ParseError::MissingMessage);
    }

    // Parse the message (may be quoted)
    let message = parse_message(rest)?;

    // Parse the target (UUID or name)
    let target = parse_target(target_str);

    Ok(Command::Reply(ReplyMessage {
        target,
        content: message,
    }))
}

/// Parse a target string as either a UUID or a name
fn parse_target(target: &str) -> PaneTarget {
    // Try to parse as UUID first
    if let Ok(uuid) = Uuid::parse_str(target) {
        PaneTarget::Id(uuid)
    } else {
        PaneTarget::Name(target.to_string())
    }
}

/// Parse a message string (handles quoted and unquoted)
fn parse_message(input: &str) -> Result<String, ParseError> {
    let input = input.trim();

    if let Some(stripped) = input.strip_prefix('"') {
        // Quoted string - find matching end quote
        if let Some(end) = stripped.find('"') {
            Ok(stripped[..end].to_string())
        } else {
            Err(ParseError::InvalidSyntax(
                "unclosed quote in message".to_string(),
            ))
        }
    } else if let Some(stripped) = input.strip_prefix('\'') {
        // Single-quoted string
        if let Some(end) = stripped.find('\'') {
            Ok(stripped[..end].to_string())
        } else {
            Err(ParseError::InvalidSyntax(
                "unclosed quote in message".to_string(),
            ))
        }
    } else {
        // Unquoted - take the rest of the line
        Ok(input.to_string())
    }
}

/// Split the first whitespace-delimited token from a string
fn split_first_token(input: &str) -> (&str, &str) {
    if let Some(pos) = input.find(char::is_whitespace) {
        (&input[..pos], &input[pos..])
    } else {
        (input, "")
    }
}

/// Check if input looks like a command (starts with /)
pub fn is_command(input: &str) -> bool {
    input.trim().starts_with('/')
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== is_command Tests ====================

    #[test]
    fn test_is_command_with_slash() {
        assert!(is_command("/reply"));
        assert!(is_command("/help"));
        assert!(is_command("  /reply"));
    }

    #[test]
    fn test_is_command_without_slash() {
        assert!(!is_command("reply"));
        assert!(!is_command("hello"));
        assert!(!is_command(""));
    }

    // ==================== parse_command Tests ====================

    #[test]
    fn test_parse_command_empty() {
        assert_eq!(parse_command(""), Err(ParseError::Empty));
        assert_eq!(parse_command("  "), Err(ParseError::Empty));
    }

    #[test]
    fn test_parse_command_no_slash() {
        let result = parse_command("reply worker yes");
        assert!(matches!(result, Err(ParseError::InvalidSyntax(_))));
    }

    #[test]
    fn test_parse_command_unknown() {
        let result = parse_command("/unknown");
        assert_eq!(result, Ok(Command::Unknown("unknown".to_string())));
    }

    // ==================== /reply Command Tests ====================

    #[test]
    fn test_reply_by_name_unquoted() {
        let result = parse_command("/reply worker-3 yes").unwrap();

        if let Command::Reply(msg) = result {
            assert_eq!(msg.target, PaneTarget::Name("worker-3".to_string()));
            assert_eq!(msg.content, "yes");
        } else {
            panic!("Expected Reply command");
        }
    }

    #[test]
    fn test_reply_by_name_quoted() {
        let result = parse_command("/reply worker-3 \"use async, we need non-blocking\"").unwrap();

        if let Command::Reply(msg) = result {
            assert_eq!(msg.target, PaneTarget::Name("worker-3".to_string()));
            assert_eq!(msg.content, "use async, we need non-blocking");
        } else {
            panic!("Expected Reply command");
        }
    }

    #[test]
    fn test_reply_by_uuid() {
        let uuid_str = "12345678-1234-1234-1234-123456789012";
        let result = parse_command(&format!("/reply {} proceed", uuid_str)).unwrap();

        if let Command::Reply(msg) = result {
            let expected_uuid = Uuid::parse_str(uuid_str).unwrap();
            assert_eq!(msg.target, PaneTarget::Id(expected_uuid));
            assert_eq!(msg.content, "proceed");
        } else {
            panic!("Expected Reply command");
        }
    }

    #[test]
    fn test_reply_missing_target() {
        let result = parse_command("/reply");
        assert_eq!(result, Err(ParseError::MissingTarget));
    }

    #[test]
    fn test_reply_missing_message() {
        let result = parse_command("/reply worker-3");
        assert_eq!(result, Err(ParseError::MissingMessage));
    }

    #[test]
    fn test_reply_single_quoted() {
        let result = parse_command("/reply worker 'single quoted'").unwrap();

        if let Command::Reply(msg) = result {
            assert_eq!(msg.content, "single quoted");
        } else {
            panic!("Expected Reply command");
        }
    }

    #[test]
    fn test_reply_unclosed_quote() {
        let result = parse_command("/reply worker \"unclosed");
        assert!(matches!(result, Err(ParseError::InvalidSyntax(_))));
    }

    #[test]
    fn test_reply_case_insensitive() {
        let result = parse_command("/REPLY worker yes").unwrap();
        assert!(matches!(result, Command::Reply(_)));

        let result = parse_command("/Reply worker yes").unwrap();
        assert!(matches!(result, Command::Reply(_)));
    }

    #[test]
    fn test_reply_with_extra_whitespace() {
        let result = parse_command("  /reply   worker-3   hello  ").unwrap();

        if let Command::Reply(msg) = result {
            assert_eq!(msg.target, PaneTarget::Name("worker-3".to_string()));
            assert_eq!(msg.content, "hello");
        } else {
            panic!("Expected Reply command");
        }
    }

    #[test]
    fn test_reply_multiword_unquoted() {
        let result = parse_command("/reply worker hello world how are you").unwrap();

        if let Command::Reply(msg) = result {
            assert_eq!(msg.content, "hello world how are you");
        } else {
            panic!("Expected Reply command");
        }
    }

    // ==================== parse_target Tests ====================

    #[test]
    fn test_parse_target_uuid() {
        let uuid_str = "12345678-1234-1234-1234-123456789012";
        let target = parse_target(uuid_str);
        let expected_uuid = Uuid::parse_str(uuid_str).unwrap();
        assert_eq!(target, PaneTarget::Id(expected_uuid));
    }

    #[test]
    fn test_parse_target_name() {
        let target = parse_target("worker-3");
        assert_eq!(target, PaneTarget::Name("worker-3".to_string()));
    }

    #[test]
    fn test_parse_target_invalid_uuid() {
        // Invalid UUID should be treated as a name
        let target = parse_target("12345678-invalid");
        assert_eq!(target, PaneTarget::Name("12345678-invalid".to_string()));
    }

    // ==================== parse_message Tests ====================

    #[test]
    fn test_parse_message_unquoted() {
        let result = parse_message("hello world").unwrap();
        assert_eq!(result, "hello world");
    }

    #[test]
    fn test_parse_message_double_quoted() {
        let result = parse_message("\"hello world\"").unwrap();
        assert_eq!(result, "hello world");
    }

    #[test]
    fn test_parse_message_single_quoted() {
        let result = parse_message("'hello world'").unwrap();
        assert_eq!(result, "hello world");
    }

    #[test]
    fn test_parse_message_with_leading_whitespace() {
        let result = parse_message("  hello").unwrap();
        assert_eq!(result, "hello");
    }

    // ==================== ParseError Tests ====================

    #[test]
    fn test_parse_error_display() {
        assert_eq!(ParseError::Empty.to_string(), "empty command");
        assert_eq!(ParseError::MissingTarget.to_string(), "missing target pane");
        assert_eq!(
            ParseError::MissingMessage.to_string(),
            "missing message content"
        );
        assert_eq!(
            ParseError::InvalidSyntax("test".to_string()).to_string(),
            "invalid syntax: test"
        );
    }

    #[test]
    fn test_parse_error_clone() {
        let err = ParseError::MissingTarget;
        let cloned = err.clone();
        assert_eq!(err, cloned);
    }

    #[test]
    fn test_parse_error_debug() {
        let err = ParseError::Empty;
        let debug = format!("{:?}", err);
        assert!(debug.contains("Empty"));
    }
}
