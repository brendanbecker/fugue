//! Claude command detection and modification utilities
//!
//! This module provides utilities for:
//! - Detecting if a command is launching Claude CLI
//! - Injecting session IDs into Claude commands
//! - Creating resume commands for session restoration

use uuid::Uuid;

/// Check if a command is launching Claude CLI
///
/// Detects variations of the claude command:
/// - `claude` (bare command)
/// - `/usr/bin/claude` or other absolute paths
/// - Commands with arguments like `claude --help`
///
/// # Examples
///
/// ```
/// use ccmux_server::claude::is_claude_command;
///
/// assert!(is_claude_command("claude", &[]));
/// assert!(is_claude_command("claude", &["--help".to_string()]));
/// assert!(is_claude_command("/usr/local/bin/claude", &[]));
/// assert!(!is_claude_command("bash", &[]));
/// assert!(!is_claude_command("vim", &[]));
/// ```
pub fn is_claude_command(command: &str, _args: &[String]) -> bool {
    // Extract the basename from the command path
    let basename = std::path::Path::new(command)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(command);

    // Check if it's the claude command
    basename == "claude"
}

/// Check if a command already has a session ID specified
///
/// Returns true if `--session-id` or `--resume` flags are present.
///
/// # Examples
///
/// ```
/// use ccmux_server::claude::has_session_id;
///
/// assert!(!has_session_id(&[]));
/// assert!(has_session_id(&["--session-id".to_string(), "abc123".to_string()]));
/// assert!(has_session_id(&["--resume".to_string()]));
/// assert!(has_session_id(&["--resume".to_string(), "abc123".to_string()]));
/// ```
pub fn has_session_id(args: &[String]) -> bool {
    args.iter().any(|arg| arg == "--session-id" || arg == "--resume")
}

/// Result of session ID injection
#[derive(Debug, Clone)]
pub struct InjectionResult {
    /// The modified arguments
    pub args: Vec<String>,
    /// The session ID that was injected (if any)
    pub session_id: Option<String>,
    /// Whether injection actually occurred
    pub injected: bool,
}

/// Inject a session ID into Claude command arguments
///
/// If the command is a Claude command and doesn't already have a session ID,
/// generates a new UUID and adds `--session-id <uuid>` to the arguments.
///
/// # Arguments
///
/// * `command` - The command being executed
/// * `args` - The current arguments
///
/// # Returns
///
/// An `InjectionResult` containing the modified arguments and session ID.
///
/// # Examples
///
/// ```
/// use ccmux_server::claude::inject_session_id;
///
/// // Claude command without session ID - injects one
/// let result = inject_session_id("claude", &[]);
/// assert!(result.injected);
/// assert!(result.session_id.is_some());
/// assert!(result.args.contains(&"--session-id".to_string()));
///
/// // Claude command with existing session ID - no change
/// let result = inject_session_id("claude", &["--session-id".to_string(), "abc".to_string()]);
/// assert!(!result.injected);
///
/// // Non-Claude command - no change
/// let result = inject_session_id("bash", &[]);
/// assert!(!result.injected);
/// ```
pub fn inject_session_id(command: &str, args: &[String]) -> InjectionResult {
    if !is_claude_command(command, args) {
        return InjectionResult {
            args: args.to_vec(),
            session_id: None,
            injected: false,
        };
    }

    if has_session_id(args) {
        return InjectionResult {
            args: args.to_vec(),
            session_id: None,
            injected: false,
        };
    }

    // Generate a new session ID and inject it
    let session_id = Uuid::new_v4().to_string();
    let mut new_args = vec!["--session-id".to_string(), session_id.clone()];
    new_args.extend(args.iter().cloned());

    InjectionResult {
        args: new_args,
        session_id: Some(session_id),
        injected: true,
    }
}

/// Create a command to resume a Claude session
///
/// Returns a command and arguments to resume a Claude session with the given ID.
///
/// # Arguments
///
/// * `session_id` - The session ID to resume
///
/// # Returns
///
/// A tuple of (command, args) for spawning the resume process.
///
/// # Examples
///
/// ```
/// use ccmux_server::claude::create_resume_command;
///
/// let (cmd, args) = create_resume_command("abc123");
/// assert_eq!(cmd, "claude");
/// assert_eq!(args, vec!["--resume", "abc123"]);
/// ```
pub fn create_resume_command(session_id: &str) -> (String, Vec<String>) {
    (
        "claude".to_string(),
        vec!["--resume".to_string(), session_id.to_string()],
    )
}

/// Extract session ID from command arguments if present
///
/// Looks for `--session-id <value>` or `--resume <value>` in the arguments.
///
/// # Examples
///
/// ```
/// use ccmux_server::claude::extract_session_id_from_args;
///
/// let args = vec!["--session-id".to_string(), "abc123".to_string()];
/// assert_eq!(extract_session_id_from_args(&args), Some("abc123".to_string()));
///
/// let args = vec!["--resume".to_string(), "def456".to_string()];
/// assert_eq!(extract_session_id_from_args(&args), Some("def456".to_string()));
///
/// let args = vec!["--help".to_string()];
/// assert_eq!(extract_session_id_from_args(&args), None);
/// ```
#[allow(dead_code)] // Reserved for future reactive detection
pub fn extract_session_id_from_args(args: &[String]) -> Option<String> {
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        if arg == "--session-id" || arg == "--resume" {
            // Get the next argument as the value
            if let Some(value) = iter.next() {
                // Skip if it looks like another flag
                if !value.starts_with('-') {
                    return Some(value.clone());
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== is_claude_command Tests ====================

    #[test]
    fn test_is_claude_command_bare() {
        assert!(is_claude_command("claude", &[]));
    }

    #[test]
    fn test_is_claude_command_absolute_path() {
        assert!(is_claude_command("/usr/bin/claude", &[]));
        assert!(is_claude_command("/usr/local/bin/claude", &[]));
        assert!(is_claude_command("/home/user/.local/bin/claude", &[]));
    }

    #[test]
    fn test_is_claude_command_with_args() {
        assert!(is_claude_command("claude", &["--help".to_string()]));
        assert!(is_claude_command("claude", &["--version".to_string()]));
        assert!(is_claude_command("claude", &["--resume".to_string(), "abc".to_string()]));
    }

    #[test]
    fn test_is_not_claude_command() {
        assert!(!is_claude_command("bash", &[]));
        assert!(!is_claude_command("vim", &[]));
        assert!(!is_claude_command("/bin/sh", &[]));
        assert!(!is_claude_command("python", &[]));
    }

    #[test]
    fn test_is_not_claude_command_similar_names() {
        // These should NOT match - they're different commands
        assert!(!is_claude_command("claudecode", &[]));
        assert!(!is_claude_command("claude-code", &[]));
        assert!(!is_claude_command("claude2", &[]));
    }

    // ==================== has_session_id Tests ====================

    #[test]
    fn test_has_session_id_none() {
        assert!(!has_session_id(&[]));
        assert!(!has_session_id(&["--help".to_string()]));
    }

    #[test]
    fn test_has_session_id_with_session_id() {
        assert!(has_session_id(&["--session-id".to_string(), "abc".to_string()]));
    }

    #[test]
    fn test_has_session_id_with_resume() {
        assert!(has_session_id(&["--resume".to_string()]));
        assert!(has_session_id(&["--resume".to_string(), "abc".to_string()]));
    }

    #[test]
    fn test_has_session_id_mixed_args() {
        assert!(has_session_id(&[
            "--help".to_string(),
            "--session-id".to_string(),
            "abc".to_string(),
        ]));
    }

    // ==================== inject_session_id Tests ====================

    #[test]
    fn test_inject_session_id_claude_no_args() {
        let result = inject_session_id("claude", &[]);
        assert!(result.injected);
        assert!(result.session_id.is_some());
        assert_eq!(result.args.len(), 2);
        assert_eq!(result.args[0], "--session-id");
    }

    #[test]
    fn test_inject_session_id_claude_with_args() {
        let result = inject_session_id("claude", &["--help".to_string()]);
        assert!(result.injected);
        assert!(result.session_id.is_some());
        assert_eq!(result.args.len(), 3);
        assert_eq!(result.args[0], "--session-id");
        assert_eq!(result.args[2], "--help");
    }

    #[test]
    fn test_inject_session_id_already_has_session() {
        let result = inject_session_id("claude", &["--session-id".to_string(), "abc".to_string()]);
        assert!(!result.injected);
        assert!(result.session_id.is_none());
        assert_eq!(result.args, vec!["--session-id", "abc"]);
    }

    #[test]
    fn test_inject_session_id_already_has_resume() {
        let result = inject_session_id("claude", &["--resume".to_string(), "abc".to_string()]);
        assert!(!result.injected);
        assert!(result.session_id.is_none());
    }

    #[test]
    fn test_inject_session_id_not_claude() {
        let result = inject_session_id("bash", &[]);
        assert!(!result.injected);
        assert!(result.session_id.is_none());
        assert!(result.args.is_empty());
    }

    #[test]
    fn test_inject_session_id_generates_valid_uuid() {
        let result = inject_session_id("claude", &[]);
        assert!(result.injected);
        let session_id = result.session_id.unwrap();
        // Verify it's a valid UUID
        assert!(Uuid::parse_str(&session_id).is_ok());
    }

    // ==================== create_resume_command Tests ====================

    #[test]
    fn test_create_resume_command() {
        let (cmd, args) = create_resume_command("abc123");
        assert_eq!(cmd, "claude");
        assert_eq!(args, vec!["--resume", "abc123"]);
    }

    #[test]
    fn test_create_resume_command_with_uuid() {
        let session_id = Uuid::new_v4().to_string();
        let (cmd, args) = create_resume_command(&session_id);
        assert_eq!(cmd, "claude");
        assert_eq!(args[0], "--resume");
        assert_eq!(args[1], session_id);
    }

    // ==================== extract_session_id_from_args Tests ====================

    #[test]
    fn test_extract_session_id_none() {
        assert_eq!(extract_session_id_from_args(&[]), None);
        assert_eq!(extract_session_id_from_args(&["--help".to_string()]), None);
    }

    #[test]
    fn test_extract_session_id_with_session_id() {
        let args = vec!["--session-id".to_string(), "abc123".to_string()];
        assert_eq!(extract_session_id_from_args(&args), Some("abc123".to_string()));
    }

    #[test]
    fn test_extract_session_id_with_resume() {
        let args = vec!["--resume".to_string(), "def456".to_string()];
        assert_eq!(extract_session_id_from_args(&args), Some("def456".to_string()));
    }

    #[test]
    fn test_extract_session_id_mixed_args() {
        let args = vec![
            "--help".to_string(),
            "--session-id".to_string(),
            "abc".to_string(),
            "--verbose".to_string(),
        ];
        assert_eq!(extract_session_id_from_args(&args), Some("abc".to_string()));
    }

    #[test]
    fn test_extract_session_id_flag_without_value() {
        // --session-id at end without value
        let args = vec!["--session-id".to_string()];
        assert_eq!(extract_session_id_from_args(&args), None);
    }

    #[test]
    fn test_extract_session_id_flag_followed_by_another_flag() {
        // --session-id followed by another flag (no value)
        let args = vec!["--session-id".to_string(), "--verbose".to_string()];
        assert_eq!(extract_session_id_from_args(&args), None);
    }
}
