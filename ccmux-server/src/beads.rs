//! Beads integration for ccmux
//!
//! Provides passive beads awareness with automatic detection
//! of .beads/ directories and environment configuration.
//!
//! Also provides an RPC client for querying the beads daemon (FEAT-058).

use std::path::{Path, PathBuf};
use std::time::Duration;

use ccmux_protocol::types::{BeadsStatus, BeadsTask};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use tokio::time::timeout;
use tracing::{debug, warn};

/// Result of beads detection
#[derive(Debug, Clone)]
pub struct BeadsDetection {
    /// Path to the .beads/ directory
    pub beads_dir: PathBuf,
    /// Root of the repository (parent of .beads/)
    #[allow(dead_code)] // Included for completeness; may be used in future features
    pub repo_root: PathBuf,
}

/// Detect the beads root directory by searching up from the given path
///
/// Walks up the directory tree from `cwd` looking for a `.beads/` directory.
/// Returns the path to the `.beads/` directory if found, along with the repo root.
///
/// # Arguments
/// * `cwd` - The starting directory to search from
///
/// # Returns
/// * `Some(BeadsDetection)` - If a .beads/ directory is found
/// * `None` - If no .beads/ directory is found
///
/// # Example
/// ```ignore
/// use std::path::Path;
/// use ccmux_server::beads::detect_beads_root;
///
/// if let Some(detection) = detect_beads_root(Path::new("/home/user/project/src")) {
///     println!("Found beads at: {:?}", detection.beads_dir);
/// }
/// ```
pub fn detect_beads_root(cwd: &Path) -> Option<BeadsDetection> {
    let mut current = cwd.to_path_buf();
    loop {
        let beads_dir = current.join(".beads");
        if beads_dir.is_dir() {
            return Some(BeadsDetection {
                beads_dir,
                repo_root: current,
            });
        }
        if !current.pop() {
            return None;
        }
    }
}

/// Check if a path is within a beads-tracked repository
///
/// Convenience function that returns true if the path or any parent
/// contains a .beads/ directory.
#[allow(dead_code)] // Utility function available for future use
pub fn is_beads_tracked(cwd: &Path) -> bool {
    detect_beads_root(cwd).is_some()
}

/// Beads metadata keys for session storage
#[allow(dead_code)] // Metadata keys for future beads integration
pub mod metadata_keys {
    /// The root path of the beads directory
    pub const BEADS_ROOT: &str = "beads.root";
    /// Whether beads was detected
    pub const BEADS_DETECTED: &str = "beads.detected";
    /// Repository root path
    pub const BEADS_REPO_ROOT: &str = "beads.repo_root";
    /// Ready task count (for status bar caching)
    pub const BEADS_READY_COUNT: &str = "beads.ready_count";
    /// Daemon availability status
    pub const BEADS_DAEMON_AVAILABLE: &str = "beads.daemon_available";
    /// Last refresh timestamp
    pub const BEADS_LAST_REFRESH: &str = "beads.last_refresh";

    // ==================== Workflow Integration Keys (FEAT-059) ====================
    /// Current assigned issue ID (e.g., "bd-456", "BUG-042")
    pub const CURRENT_ISSUE: &str = "beads.current_issue";
    /// ISO 8601 timestamp when current issue was assigned
    pub const ASSIGNED_AT: &str = "beads.assigned_at";
    /// JSON array of issue history entries
    pub const ISSUE_HISTORY: &str = "beads.issue_history";
}

// ==================== Beads Daemon RPC Client (FEAT-058) ====================

/// Error type for beads daemon communication
#[derive(Debug, Error)]
#[allow(dead_code)] // Error variants for beads daemon communication
pub enum BeadsError {
    #[error("Daemon socket not found")]
    SocketNotFound,

    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    #[error("Connection timeout")]
    Timeout,

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Protocol error: {0}")]
    Protocol(String),

    #[error("Daemon returned error: {0}")]
    DaemonError(String),
}

/// RPC request to the beads daemon
#[derive(Debug, Serialize)]
struct RpcRequest {
    operation: String,
    args: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    cwd: Option<String>,
}

/// RPC response from the beads daemon
#[derive(Debug, Deserialize)]
struct RpcResponse {
    success: bool,
    #[serde(default)]
    data: serde_json::Value,
    #[serde(default)]
    error: Option<String>,
}

/// Task data from daemon (matches beads RPC format)
#[derive(Debug, Deserialize)]
struct DaemonTask {
    id: String,
    title: String,
    #[serde(default)]
    priority: i32,
    #[serde(default)]
    status: String,
    #[serde(default, rename = "issue_type")]
    issue_type: String,
    #[serde(default)]
    assignee: Option<String>,
    #[serde(default)]
    labels: Vec<String>,
}

impl From<DaemonTask> for BeadsTask {
    fn from(task: DaemonTask) -> Self {
        BeadsTask {
            id: task.id,
            title: task.title,
            priority: task.priority,
            status: task.status,
            issue_type: task.issue_type,
            assignee: task.assignee,
            labels: task.labels,
        }
    }
}

/// Client for communicating with the beads daemon
pub struct BeadsClient {
    socket_path: PathBuf,
    timeout: Duration,
    cwd: Option<String>,
}

impl BeadsClient {
    /// Create a new client for a beads-tracked directory
    ///
    /// Returns None if no .beads/bd.sock is found.
    pub fn new(working_dir: &Path, timeout_ms: u32) -> Option<Self> {
        let socket_path = Self::discover_socket(working_dir)?;
        Some(Self {
            socket_path,
            timeout: Duration::from_millis(timeout_ms as u64),
            cwd: working_dir.to_str().map(|s| s.to_string()),
        })
    }

    /// Discover the daemon socket by walking up from the working directory
    fn discover_socket(working_dir: &Path) -> Option<PathBuf> {
        let detection = detect_beads_root(working_dir)?;
        let socket_path = detection.beads_dir.join("bd.sock");
        if socket_path.exists() {
            debug!("Found beads daemon socket at {:?}", socket_path);
            Some(socket_path)
        } else {
            debug!("No beads daemon socket at {:?}", socket_path);
            None
        }
    }

    /// Check if the daemon socket exists (quick availability check)
    pub fn is_available(&self) -> bool {
        self.socket_path.exists()
    }

    /// Query ready tasks from the daemon
    pub async fn query_ready(&self, limit: Option<usize>) -> Result<Vec<BeadsTask>, BeadsError> {
        let mut args = serde_json::json!({});
        if let Some(limit) = limit {
            args["limit"] = serde_json::json!(limit);
        }

        let request = RpcRequest {
            operation: "ready".to_string(),
            args,
            cwd: self.cwd.clone(),
        };

        let response = self.send_request(&request).await?;

        if !response.success {
            return Err(BeadsError::DaemonError(
                response.error.unwrap_or_else(|| "Unknown error".to_string()),
            ));
        }

        // Parse the response data as a list of tasks
        let tasks: Vec<DaemonTask> = serde_json::from_value(response.data)
            .map_err(|e| BeadsError::Protocol(format!("Failed to parse tasks: {}", e)))?;

        Ok(tasks.into_iter().map(BeadsTask::from).collect())
    }

    /// Get full beads status (convenience method)
    pub async fn get_status(&self, limit: Option<usize>) -> BeadsStatus {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        if !self.is_available() {
            return BeadsStatus::unavailable();
        }

        match self.query_ready(limit).await {
            Ok(tasks) => BeadsStatus::with_tasks(tasks, now),
            Err(e) => {
                warn!("Beads daemon query failed: {}", e);
                BeadsStatus::with_error(e.to_string())
            }
        }
    }

    /// Send an RPC request to the daemon
    async fn send_request(&self, request: &RpcRequest) -> Result<RpcResponse, BeadsError> {
        // Connect to the socket with timeout
        let stream = timeout(self.timeout, UnixStream::connect(&self.socket_path))
            .await
            .map_err(|_| BeadsError::Timeout)?
            .map_err(|e| BeadsError::ConnectionFailed(e.to_string()))?;

        let (reader, mut writer) = stream.into_split();
        let mut reader = BufReader::new(reader);

        // Serialize and send the request (line-delimited JSON)
        let request_json = serde_json::to_string(request)
            .map_err(|e| BeadsError::Protocol(format!("Failed to serialize request: {}", e)))?;

        debug!("Sending beads RPC request: {}", request_json);

        timeout(
            self.timeout,
            async {
                writer.write_all(request_json.as_bytes()).await?;
                writer.write_all(b"\n").await?;
                writer.flush().await?;
                Ok::<_, std::io::Error>(())
            },
        )
        .await
        .map_err(|_| BeadsError::Timeout)?
        .map_err(BeadsError::Io)?;

        // Read the response (line-delimited JSON)
        let mut response_line = String::new();
        timeout(self.timeout, reader.read_line(&mut response_line))
            .await
            .map_err(|_| BeadsError::Timeout)?
            .map_err(BeadsError::Io)?;

        debug!("Received beads RPC response: {}", response_line.trim());

        // Parse the response
        let response: RpcResponse = serde_json::from_str(&response_line)
            .map_err(|e| BeadsError::Protocol(format!("Failed to parse response: {}", e)))?;

        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_detect_beads_root_found() {
        let temp = TempDir::new().unwrap();
        let beads_dir = temp.path().join(".beads");
        fs::create_dir(&beads_dir).unwrap();

        let detection = detect_beads_root(temp.path()).unwrap();
        assert_eq!(detection.beads_dir, beads_dir);
        assert_eq!(detection.repo_root, temp.path());
    }

    #[test]
    fn test_detect_beads_root_nested() {
        let temp = TempDir::new().unwrap();
        let beads_dir = temp.path().join(".beads");
        fs::create_dir(&beads_dir).unwrap();

        // Create nested directories
        let nested = temp.path().join("src").join("lib").join("deep");
        fs::create_dir_all(&nested).unwrap();

        let detection = detect_beads_root(&nested).unwrap();
        assert_eq!(detection.beads_dir, beads_dir);
        assert_eq!(detection.repo_root, temp.path());
    }

    #[test]
    fn test_detect_beads_root_not_found() {
        let temp = TempDir::new().unwrap();
        let nested = temp.path().join("no_beads_here");
        fs::create_dir_all(&nested).unwrap();

        assert!(detect_beads_root(&nested).is_none());
    }

    #[test]
    fn test_detect_beads_root_file_not_dir() {
        let temp = TempDir::new().unwrap();
        // Create .beads as a file, not a directory
        let beads_file = temp.path().join(".beads");
        fs::write(&beads_file, "not a directory").unwrap();

        assert!(detect_beads_root(temp.path()).is_none());
    }

    #[test]
    fn test_is_beads_tracked_true() {
        let temp = TempDir::new().unwrap();
        let beads_dir = temp.path().join(".beads");
        fs::create_dir(&beads_dir).unwrap();

        assert!(is_beads_tracked(temp.path()));
    }

    #[test]
    fn test_is_beads_tracked_false() {
        let temp = TempDir::new().unwrap();
        assert!(!is_beads_tracked(temp.path()));
    }

    #[test]
    fn test_is_beads_tracked_nested() {
        let temp = TempDir::new().unwrap();
        let beads_dir = temp.path().join(".beads");
        fs::create_dir(&beads_dir).unwrap();

        let nested = temp.path().join("deep").join("nested").join("path");
        fs::create_dir_all(&nested).unwrap();

        assert!(is_beads_tracked(&nested));
    }

    #[test]
    fn test_metadata_keys() {
        assert_eq!(metadata_keys::BEADS_ROOT, "beads.root");
        assert_eq!(metadata_keys::BEADS_DETECTED, "beads.detected");
        assert_eq!(metadata_keys::BEADS_REPO_ROOT, "beads.repo_root");
        assert_eq!(metadata_keys::BEADS_READY_COUNT, "beads.ready_count");
        assert_eq!(metadata_keys::BEADS_DAEMON_AVAILABLE, "beads.daemon_available");
        assert_eq!(metadata_keys::BEADS_LAST_REFRESH, "beads.last_refresh");
    }

    // ==================== BeadsClient Tests (FEAT-058) ====================

    #[test]
    fn test_beads_client_new_no_beads_dir() {
        let temp = TempDir::new().unwrap();
        // No .beads directory
        let client = BeadsClient::new(temp.path(), 1000);
        assert!(client.is_none());
    }

    #[test]
    fn test_beads_client_new_no_socket() {
        let temp = TempDir::new().unwrap();
        let beads_dir = temp.path().join(".beads");
        fs::create_dir(&beads_dir).unwrap();
        // .beads exists but no bd.sock
        let client = BeadsClient::new(temp.path(), 1000);
        assert!(client.is_none());
    }

    #[test]
    fn test_beads_client_new_with_socket() {
        let temp = TempDir::new().unwrap();
        let beads_dir = temp.path().join(".beads");
        fs::create_dir(&beads_dir).unwrap();
        // Create a fake socket file (won't actually work as a socket)
        let socket_path = beads_dir.join("bd.sock");
        fs::write(&socket_path, "").unwrap();

        let client = BeadsClient::new(temp.path(), 1000);
        assert!(client.is_some());
        let client = client.unwrap();
        assert!(client.is_available());
    }

    #[test]
    fn test_beads_client_discover_socket_nested() {
        let temp = TempDir::new().unwrap();
        let beads_dir = temp.path().join(".beads");
        fs::create_dir(&beads_dir).unwrap();
        let socket_path = beads_dir.join("bd.sock");
        fs::write(&socket_path, "").unwrap();

        // Create nested directory
        let nested = temp.path().join("src").join("lib");
        fs::create_dir_all(&nested).unwrap();

        // Should find socket from nested path
        let client = BeadsClient::new(&nested, 1000);
        assert!(client.is_some());
    }

    #[test]
    fn test_beads_error_display() {
        let errors = [
            BeadsError::SocketNotFound,
            BeadsError::ConnectionFailed("refused".to_string()),
            BeadsError::Timeout,
            BeadsError::Protocol("invalid json".to_string()),
            BeadsError::DaemonError("not found".to_string()),
        ];

        for err in errors {
            let display = format!("{}", err);
            assert!(!display.is_empty());
        }
    }

    #[test]
    fn test_daemon_task_conversion() {
        let daemon_task = DaemonTask {
            id: "BUG-042".to_string(),
            title: "Fix login".to_string(),
            priority: 1,
            status: "open".to_string(),
            issue_type: "bug".to_string(),
            assignee: Some("alice".to_string()),
            labels: vec!["auth".to_string()],
        };

        let beads_task: BeadsTask = daemon_task.into();
        assert_eq!(beads_task.id, "BUG-042");
        assert_eq!(beads_task.title, "Fix login");
        assert_eq!(beads_task.priority, 1);
        assert_eq!(beads_task.status, "open");
        assert_eq!(beads_task.issue_type, "bug");
        assert_eq!(beads_task.assignee, Some("alice".to_string()));
        assert_eq!(beads_task.labels, vec!["auth".to_string()]);
    }

    #[test]
    fn test_rpc_request_serialization() {
        let request = RpcRequest {
            operation: "ready".to_string(),
            args: serde_json::json!({"limit": 10}),
            cwd: Some("/path/to/repo".to_string()),
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"operation\":\"ready\""));
        assert!(json.contains("\"limit\":10"));
        assert!(json.contains("\"cwd\":\"/path/to/repo\""));
    }

    #[test]
    fn test_rpc_request_serialization_no_cwd() {
        let request = RpcRequest {
            operation: "ping".to_string(),
            args: serde_json::json!(null),
            cwd: None,
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"operation\":\"ping\""));
        assert!(!json.contains("cwd")); // skip_serializing_if = None
    }

    #[test]
    fn test_rpc_response_deserialization_success() {
        let json = r#"{"success": true, "data": [{"id": "BUG-1", "title": "Test"}]}"#;
        let response: RpcResponse = serde_json::from_str(json).unwrap();
        assert!(response.success);
        assert!(response.error.is_none());
    }

    #[test]
    fn test_rpc_response_deserialization_error() {
        let json = r#"{"success": false, "error": "Not found"}"#;
        let response: RpcResponse = serde_json::from_str(json).unwrap();
        assert!(!response.success);
        assert_eq!(response.error, Some("Not found".to_string()));
    }
}
