//! Orchestration tools for parallel and sequential command execution
//!
//! Provides tools for:
//! - FEAT-096: `ccmux_expect` - waiting for patterns in pane output
//! - FEAT-094: `ccmux_run_parallel` - parallel command execution

use std::time::{Duration, Instant};
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};
use uuid::Uuid;
use regex::Regex;
use serde_json::json;

use ccmux_protocol::{ClientMessage, ServerMessage, SplitDirection};

use super::connection::ConnectionManager;
use crate::mcp::error::McpError;
use crate::mcp::protocol::ToolResult;

// ============================================================================
// FEAT-096: ccmux_expect
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExpectAction {
    Notify,
    ClosePane,
    ReturnOutput,
}

impl std::str::FromStr for ExpectAction {
    type Err = McpError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "notify" => Ok(ExpectAction::Notify),
            "close_pane" => Ok(ExpectAction::ClosePane),
            "return_output" => Ok(ExpectAction::ReturnOutput),
            _ => Err(McpError::InvalidParams(format!("Invalid action: {}", s))),
        }
    }
}

/// Wait for a regex pattern to appear in a pane's output
pub async fn run_expect(
    connection: &mut ConnectionManager,
    pane_id: Uuid,
    pattern: &str,
    timeout_ms: u64,
    action: ExpectAction,
    poll_interval_ms: u64,
    lines: usize,
) -> Result<ToolResult, McpError> {
    let regex = Regex::new(pattern)
        .map_err(|e| McpError::InvalidParams(format!("Invalid regex pattern: {}", e)))?;

    let start_time = Instant::now();
    let timeout = Duration::from_millis(timeout_ms);
    let poll_interval = Duration::from_millis(poll_interval_ms);

    loop {
        // 1. Check for timeout
        let elapsed = start_time.elapsed();
        if elapsed > timeout {
            return Ok(ToolResult::text(json!({
                "status": "timeout",
                "pattern": pattern,
                "duration_ms": elapsed.as_millis(),
            }).to_string()));
        }

        // 2. Read pane content
        connection.send_to_daemon(ClientMessage::ReadPane { pane_id, lines }).await?;

        let content = match connection.recv_response_from_daemon().await? {
            ServerMessage::PaneContent { content, .. } => content,
            ServerMessage::Error { code, message, .. } => {
                // If pane not found or other error, return immediately
                return Ok(ToolResult::error(format!("{:?}: {}", code, message)));
            }
            msg => return Err(McpError::UnexpectedResponse(format!("{:?}", msg))),
        };

        // 3. Check for match
        if let Some(mat) = regex.find(&content) {
            let match_text = mat.as_str().to_string();
            // Extract the line containing the match for context
            let start_index = mat.start();
            let line_start = content[..start_index].rfind('\n').map(|i| i + 1).unwrap_or(0);
            let line_end = content[start_index..].find('\n').map(|i| start_index + i).unwrap_or(content.len());
            let line_content = content[line_start..line_end].to_string();

            let mut response = json!({
                "status": "matched",
                "pattern": pattern,
                "match": match_text,
                "line": line_content,
                "duration_ms": elapsed.as_millis(),
            });

            // 4. Perform action
            match action {
                ExpectAction::Notify => {
                    // Just return success
                },
                ExpectAction::ClosePane => {
                    connection.send_to_daemon(ClientMessage::ClosePane { pane_id }).await?;
                    match connection.recv_response_from_daemon().await? {
                        ServerMessage::PaneClosed { .. } => {},
                        ServerMessage::Error { code, message, .. } => {
                             return Ok(ToolResult::error(format!("Pattern found but failed to close pane: {:?}: {}", code, message)));
                        }
                         msg => return Err(McpError::UnexpectedResponse(format!("{:?}", msg))),
                    }
                    response["pane_closed"] = json!(true);
                },
                ExpectAction::ReturnOutput => {
                    response["output"] = json!(content);
                }
            }

            return Ok(ToolResult::text(response.to_string()));
        }

        // 5. Wait before next poll
        tokio::time::sleep(poll_interval).await;
    }
}

// ============================================================================
// FEAT-094: ccmux_run_parallel
// ============================================================================

/// Maximum number of parallel commands allowed
pub const MAX_PARALLEL_COMMANDS: usize = 10;

/// Default timeout in milliseconds (5 minutes)
pub const DEFAULT_TIMEOUT_MS: u64 = 300_000;

/// Polling interval in milliseconds
const POLL_INTERVAL_MS: u64 = 200;

/// Exit code marker prefix used to detect command completion
const EXIT_MARKER_PREFIX: &str = "___CCMUX_EXIT_";

/// Exit code marker suffix
const EXIT_MARKER_SUFFIX: &str = "___";

/// Name of the hidden orchestration session
const ORCHESTRATION_SESSION_NAME: &str = "__orchestration__";

/// A single command to execute in parallel
#[derive(Debug, Clone, Deserialize)]
pub struct ParallelCommand {
    /// The command to execute
    pub command: String,
    /// Working directory (optional)
    #[serde(default)]
    pub cwd: Option<String>,
    /// Task name for identification (optional)
    #[serde(default)]
    pub name: Option<String>,
}

/// Request parameters for run_parallel
#[derive(Debug, Deserialize)]
pub struct RunParallelRequest {
    /// Commands to execute (max 10)
    pub commands: Vec<ParallelCommand>,
    /// Layout mode: "tiled" for visible splits, "hidden" for orchestration session
    #[serde(default = "default_layout")]
    pub layout: String,
    /// Timeout in milliseconds
    #[serde(default = "default_timeout")]
    pub timeout_ms: u64,
    /// Whether to clean up panes after completion
    #[serde(default = "default_cleanup")]
    pub cleanup: bool,
}

pub fn default_layout() -> String {
    "hidden".to_string()
}

pub fn default_timeout() -> u64 {
    DEFAULT_TIMEOUT_MS
}

pub fn default_cleanup() -> bool {
    true
}

/// Result for a single task
#[derive(Debug, Clone, Serialize)]
pub struct TaskResult {
    /// Task name
    pub name: String,
    /// The command that was executed
    pub command: String,
    /// Exit code (None if timed out or failed to execute)
    pub exit_code: Option<i32>,
    /// Pane ID where the command ran
    pub pane_id: String,
    /// Duration in milliseconds
    pub duration_ms: u64,
    /// Status: "completed", "timeout", "error"
    pub status: String,
}

/// Response from run_parallel
#[derive(Debug, Serialize)]
pub struct RunParallelResponse {
    /// Overall status: "completed", "timeout", "partial"
    pub status: String,
    /// Results for each task
    pub results: Vec<TaskResult>,
    /// Total duration in milliseconds
    pub total_duration_ms: u64,
}

/// State for tracking a running task
struct RunningTask {
    name: String,
    command: String,
    pane_id: Uuid,
    start_time: Instant,
    exit_code: Option<i32>,
    completed: bool,
}

/// Execute multiple commands in parallel
pub async fn run_parallel(
    connection: &mut ConnectionManager,
    request: RunParallelRequest,
) -> Result<ToolResult, McpError> {
    // Validate request
    if request.commands.is_empty() {
        return Err(McpError::InvalidParams("commands array cannot be empty".into()));
    }
    if request.commands.len() > MAX_PARALLEL_COMMANDS {
        return Err(McpError::InvalidParams(format!(
            "Maximum {} commands allowed, got {}",
            MAX_PARALLEL_COMMANDS,
            request.commands.len()
        )));
    }

    let total_start = Instant::now();
    let timeout = Duration::from_millis(request.timeout_ms);
    let is_hidden = request.layout == "hidden";

    info!(
        commands_count = request.commands.len(),
        layout = %request.layout,
        timeout_ms = request.timeout_ms,
        cleanup = request.cleanup,
        "Starting parallel command execution"
    );

    // Get or create the session for running commands
    let session_id = if is_hidden {
        get_or_create_orchestration_session(connection).await?
    } else {
        // For tiled layout, use the first available session
        get_first_session(connection).await?
    };

    // Spawn panes for each command
    let mut running_tasks = Vec::with_capacity(request.commands.len());

    for (idx, cmd) in request.commands.iter().enumerate() {
        let task_name = cmd.name.clone().unwrap_or_else(|| format!("task-{}", idx));

        debug!(
            task_name = %task_name,
            command = %cmd.command,
            cwd = ?cmd.cwd,
            "Creating pane for task"
        );

        // Create pane for this task
        let pane_id = create_task_pane(
            connection,
            &session_id.to_string(),
            &task_name,
            cmd.cwd.as_deref(),
            is_hidden,
        ).await?;

        // Wrap command with exit code marker
        let wrapped_command = format!(
            "{{ {} ; }} ; echo \"{}$?{}\"",
            cmd.command,
            EXIT_MARKER_PREFIX,
            EXIT_MARKER_SUFFIX
        );

        // Send command to pane
        send_command_to_pane(connection, pane_id, &wrapped_command).await?;

        running_tasks.push(RunningTask {
            name: task_name,
            command: cmd.command.clone(),
            pane_id,
            start_time: Instant::now(),
            exit_code: None,
            completed: false,
        });
    }

    // Poll for completion
    let deadline = Instant::now() + timeout;
    let poll_interval = Duration::from_millis(POLL_INTERVAL_MS);

    while Instant::now() < deadline {
        let all_completed = running_tasks.iter().all(|t| t.completed);
        if all_completed {
            break;
        }

        // Poll each incomplete task
        for task in running_tasks.iter_mut() {
            if task.completed {
                continue;
            }

            // Read pane output to check for exit marker
            match read_pane_for_exit_code(connection, task.pane_id).await {
                Ok(Some(exit_code)) => {
                    task.exit_code = Some(exit_code);
                    task.completed = true;
                    debug!(
                        task_name = %task.name,
                        exit_code = exit_code,
                        elapsed_ms = task.start_time.elapsed().as_millis() as u64,
                        "Task completed"
                    );
                }
                Ok(None) => {
                    // Not completed yet
                }
                Err(e) => {
                    warn!(
                        task_name = %task.name,
                        error = %e,
                        "Error reading pane output"
                    );
                }
            }
        }

        tokio::time::sleep(poll_interval).await;
    }

    // Build results
    let mut results = Vec::with_capacity(running_tasks.len());
    let mut all_completed = true;

    for task in &running_tasks {
        let duration_ms = task.start_time.elapsed().as_millis() as u64;
        let status = if task.completed {
            "completed"
        } else {
            all_completed = false;
            "timeout"
        };

        results.push(TaskResult {
            name: task.name.clone(),
            command: task.command.clone(),
            exit_code: task.exit_code,
            pane_id: task.pane_id.to_string(),
            duration_ms,
            status: status.to_string(),
        });
    }

    // Cleanup panes if requested
    if request.cleanup {
        for task in &running_tasks {
            if let Err(e) = close_pane(connection, task.pane_id).await {
                warn!(
                    task_name = %task.name,
                    pane_id = %task.pane_id,
                    error = %e,
                    "Failed to close pane during cleanup"
                );
            }
        }
    }

    let total_duration_ms = total_start.elapsed().as_millis() as u64;
    let overall_status = if all_completed {
        "completed"
    } else if results.iter().any(|r| r.status == "completed") {
        "partial"
    } else {
        "timeout"
    };

    let response = RunParallelResponse {
        status: overall_status.to_string(),
        results,
        total_duration_ms,
    };

    info!(
        status = %response.status,
        total_duration_ms = response.total_duration_ms,
        completed_count = response.results.iter().filter(|r| r.status == "completed").count(),
        timeout_count = response.results.iter().filter(|r| r.status == "timeout").count(),
        "Parallel execution completed"
    );

    let json = serde_json::to_string_pretty(&response)
        .map_err(|e| McpError::Internal(e.to_string()))?;
    Ok(ToolResult::text(json))
}

/// Get or create the hidden orchestration session
async fn get_or_create_orchestration_session(
    connection: &mut ConnectionManager,
) -> Result<Uuid, McpError> {
    // List sessions to find existing orchestration session
    connection.send_to_daemon(ClientMessage::ListSessions).await?;

    match connection.recv_response_from_daemon().await? {
        ServerMessage::SessionList { sessions } => {
            // Look for existing orchestration session
            for session in &sessions {
                if session.name == ORCHESTRATION_SESSION_NAME {
                    debug!(
                        session_id = %session.id,
                        "Found existing orchestration session"
                    );
                    return Ok(session.id);
                }
            }

            // Create new orchestration session
            debug!("Creating new orchestration session");
            connection.send_to_daemon(ClientMessage::CreateSessionWithOptions {
                name: Some(ORCHESTRATION_SESSION_NAME.to_string()),
                command: None,
                cwd: None,
                claude_model: None,
                claude_config: None,
                preset: None,
            }).await?;

            match connection.recv_response_from_daemon().await? {
                ServerMessage::SessionCreatedWithDetails { session_id, .. } => {
                    info!(
                        session_id = %session_id,
                        "Created orchestration session"
                    );
                    Ok(session_id)
                }
                ServerMessage::Error { code, message, .. } => {
                    Err(McpError::DaemonError(format!("{:?}: {}", code, message)))
                }
                msg => Err(McpError::UnexpectedResponse(format!("{:?}", msg))),
            }
        }
        ServerMessage::Error { code, message, .. } => {
            Err(McpError::DaemonError(format!("{:?}: {}", code, message)))
        }
        msg => Err(McpError::UnexpectedResponse(format!("{:?}", msg))),
    }
}

/// Get the first available session for tiled layout
async fn get_first_session(connection: &mut ConnectionManager) -> Result<Uuid, McpError> {
    connection.send_to_daemon(ClientMessage::ListSessions).await?;

    match connection.recv_response_from_daemon().await? {
        ServerMessage::SessionList { sessions } => {
            // Prefer a non-orchestration session
            for session in &sessions {
                if session.name != ORCHESTRATION_SESSION_NAME {
                    return Ok(session.id);
                }
            }
            // Fall back to any session
            sessions
                .first()
                .map(|s| s.id)
                .ok_or_else(|| McpError::InvalidParams("No sessions available".into()))
        }
        ServerMessage::Error { code, message, .. } => {
            Err(McpError::DaemonError(format!("{:?}: {}", code, message)))
        }
        msg => Err(McpError::UnexpectedResponse(format!("{:?}", msg))),
    }
}

/// Create a pane for running a task
async fn create_task_pane(
    connection: &mut ConnectionManager,
    session_filter: &str,
    name: &str,
    cwd: Option<&str>,
    _is_hidden: bool,
) -> Result<Uuid, McpError> {
    connection.send_to_daemon(ClientMessage::CreatePaneWithOptions {
        session_filter: Some(session_filter.to_string()),
        window_filter: None,
        direction: SplitDirection::Vertical,
        command: None,  // Default shell
        cwd: cwd.map(String::from),
        select: false,
        name: Some(name.to_string()),
        claude_model: None,
        claude_config: None,
        preset: None,
    }).await?;

    match connection.recv_response_from_daemon().await? {
        ServerMessage::PaneCreatedWithDetails { pane_id, .. } => {
            debug!(pane_id = %pane_id, name = %name, "Created task pane");
            Ok(pane_id)
        }
        ServerMessage::Error { code, message, .. } => {
            Err(McpError::DaemonError(format!("{:?}: {}", code, message)))
        }
        msg => Err(McpError::UnexpectedResponse(format!("{:?}", msg))),
    }
}

/// Send a command to a pane
async fn send_command_to_pane(
    connection: &mut ConnectionManager,
    pane_id: Uuid,
    command: &str,
) -> Result<(), McpError> {
    // Send the command with a newline to execute it
    let mut data = command.as_bytes().to_vec();
    data.push(b'\r');

    connection.send_to_daemon(ClientMessage::Input { pane_id, data }).await?;

    debug!(pane_id = %pane_id, "Sent command to pane");
    Ok(())
}

/// Read pane output and check for exit code marker
async fn read_pane_for_exit_code(
    connection: &mut ConnectionManager,
    pane_id: Uuid,
) -> Result<Option<i32>, McpError> {
    connection.send_to_daemon(ClientMessage::ReadPane {
        pane_id,
        lines: 50,  // Read last 50 lines to find exit marker
    }).await?;

    match connection.recv_response_from_daemon().await? {
        ServerMessage::PaneContent { content, .. } => {
            // Look for exit marker pattern: ___CCMUX_EXIT_<code>___
            for line in content.lines().rev() {
                if let Some(exit_code) = parse_exit_marker(line) {
                    return Ok(Some(exit_code));
                }
            }
            Ok(None)
        }
        ServerMessage::Error { code, message, .. } => {
            Err(McpError::DaemonError(format!("{:?}: {}", code, message)))
        }
        msg => Err(McpError::UnexpectedResponse(format!("{:?}", msg))),
    }
}

/// Parse exit code from marker line
pub fn parse_exit_marker(line: &str) -> Option<i32> {
    let trimmed = line.trim();
    if trimmed.starts_with(EXIT_MARKER_PREFIX) && trimmed.ends_with(EXIT_MARKER_SUFFIX) {
        let code_str = &trimmed[EXIT_MARKER_PREFIX.len()..trimmed.len() - EXIT_MARKER_SUFFIX.len()];
        code_str.parse().ok()
    } else {
        None
    }
}

/// Close a pane
async fn close_pane(connection: &mut ConnectionManager, pane_id: Uuid) -> Result<(), McpError> {
    connection.send_to_daemon(ClientMessage::ClosePane { pane_id }).await?;

    // Wait for close confirmation with timeout
    let timeout = Duration::from_secs(5);
    match connection.recv_from_daemon_with_timeout(timeout).await {
        Ok(ServerMessage::PaneClosed { pane_id: closed_id, .. }) if closed_id == pane_id => {
            debug!(pane_id = %pane_id, "Pane closed");
            Ok(())
        }
        Ok(ServerMessage::Error { code, message, .. }) => {
            Err(McpError::DaemonError(format!("{:?}: {}", code, message)))
        }
        Ok(_) => {
            // Got some other message, pane might still be closing
            debug!(pane_id = %pane_id, "Close confirmation not received, assuming closed");
            Ok(())
        }
        Err(McpError::ResponseTimeout { .. }) => {
            debug!(pane_id = %pane_id, "Timeout waiting for close confirmation");
            Ok(())
        }
        Err(e) => Err(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    // FEAT-096 tests
    #[test]
    fn test_expect_action_parsing() {
        assert_eq!(ExpectAction::from_str("notify").unwrap(), ExpectAction::Notify);
        assert_eq!(ExpectAction::from_str("close_pane").unwrap(), ExpectAction::ClosePane);
        assert_eq!(ExpectAction::from_str("return_output").unwrap(), ExpectAction::ReturnOutput);
        assert!(ExpectAction::from_str("invalid").is_err());
    }

    #[test]
    fn test_regex_compilation() {
        let regex = Regex::new(r"___CCMUX_EXIT_\d+___");
        assert!(regex.is_ok());

        let regex = Regex::new(r"[invalid");
        assert!(regex.is_err());
    }

    #[test]
    fn test_regex_matching() {
        let regex = Regex::new(r"___CCMUX_EXIT_0___").unwrap();
        let content = "some output\n___CCMUX_EXIT_0___\nmore output";
        assert!(regex.find(content).is_some());
    }

    // FEAT-094 tests
    #[test]
    fn test_parse_exit_marker_success() {
        assert_eq!(parse_exit_marker("___CCMUX_EXIT_0___"), Some(0));
        assert_eq!(parse_exit_marker("___CCMUX_EXIT_1___"), Some(1));
        assert_eq!(parse_exit_marker("___CCMUX_EXIT_127___"), Some(127));
        assert_eq!(parse_exit_marker("___CCMUX_EXIT_-1___"), Some(-1));
    }

    #[test]
    fn test_parse_exit_marker_with_whitespace() {
        assert_eq!(parse_exit_marker("  ___CCMUX_EXIT_0___  "), Some(0));
        assert_eq!(parse_exit_marker("\t___CCMUX_EXIT_42___\n"), Some(42));
    }

    #[test]
    fn test_parse_exit_marker_failure() {
        assert_eq!(parse_exit_marker("___CCMUX_EXIT_abc___"), None);
        assert_eq!(parse_exit_marker("___CCMUX_EXIT_0"), None);
        assert_eq!(parse_exit_marker("CCMUX_EXIT_0___"), None);
        assert_eq!(parse_exit_marker("some random text"), None);
        assert_eq!(parse_exit_marker(""), None);
    }

    #[test]
    fn test_default_values() {
        assert_eq!(default_layout(), "hidden");
        assert_eq!(default_timeout(), DEFAULT_TIMEOUT_MS);
        assert!(default_cleanup());
    }
}
