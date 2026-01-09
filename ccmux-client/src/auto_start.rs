//! Auto-start functionality for ccmux server
//!
//! Provides tmux-like behavior where running the client automatically
//! starts the server daemon if it's not already running.

use std::io::ErrorKind;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use ccmux_utils::{socket_path, CcmuxError, Result};

/// Configuration for auto-start behavior
#[derive(Debug, Clone)]
pub struct AutoStartConfig {
    /// Whether to auto-start the server if not running
    pub enabled: bool,
    /// Timeout for waiting for server to start (milliseconds)
    pub timeout_ms: u64,
    /// Delay between connection retries (milliseconds)
    pub retry_delay_ms: u64,
    /// Initial delay after spawning server (milliseconds)
    pub initial_delay_ms: u64,
}

impl Default for AutoStartConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            timeout_ms: 2000,
            retry_delay_ms: 200,
            initial_delay_ms: 100,
        }
    }
}

/// Server binary name
const SERVER_BINARY_NAME: &str = "ccmux-server";

/// Find the ccmux-server binary
///
/// Search order:
/// 1. Same directory as the current executable
/// 2. PATH environment variable
pub fn find_server_binary() -> Result<PathBuf> {
    // 1. Check same directory as current executable
    if let Ok(current_exe) = std::env::current_exe() {
        if let Some(parent) = current_exe.parent() {
            let server_path = parent.join(SERVER_BINARY_NAME);
            if server_path.is_file() {
                tracing::debug!("Found server binary at: {:?}", server_path);
                return Ok(server_path);
            }
        }
    }

    // 2. Search PATH
    if let Ok(path) = which::which(SERVER_BINARY_NAME) {
        tracing::debug!("Found server binary in PATH: {:?}", path);
        return Ok(path);
    }

    Err(CcmuxError::Internal(format!(
        "{} binary not found. Ensure it's in the same directory as ccmux or in your PATH.",
        SERVER_BINARY_NAME
    )))
}

/// Start the ccmux-server as a background daemon
///
/// The server is spawned as a detached process with no stdin/stdout/stderr
/// connections, allowing it to run independently of the client.
pub fn start_server_daemon() -> Result<()> {
    let server_path = find_server_binary()?;

    tracing::info!("Starting server daemon: {:?}", server_path);

    // Spawn as daemon (detached, no stdin/stdout/stderr)
    Command::new(&server_path)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| {
            CcmuxError::ProcessSpawn(format!(
                "Failed to start {}: {}. Check that the binary is executable.",
                SERVER_BINARY_NAME, e
            ))
        })?;

    tracing::info!("Server daemon started");
    Ok(())
}

/// Check if an error indicates the server is not running
///
/// Returns true for:
/// - Socket file not found (ENOENT)
/// - Connection refused (ECONNREFUSED)
/// - ServerNotRunning error from our code
///
/// This is useful for distinguishing between "server not running" errors
/// that can be resolved by auto-start vs other errors that cannot.
#[allow(dead_code)] // Public API for tests and future retry logic
pub fn is_server_not_running(error: &CcmuxError) -> bool {
    match error {
        CcmuxError::ServerNotRunning { .. } => true,
        CcmuxError::Io(io_err) => matches!(
            io_err.kind(),
            ErrorKind::NotFound | ErrorKind::ConnectionRefused
        ),
        CcmuxError::Connection(msg) => {
            // Check for common connection error messages
            msg.contains("Connection refused")
                || msg.contains("No such file or directory")
                || msg.contains("connect")
        }
        _ => false,
    }
}

/// Check if the server socket exists and is connectable
pub async fn check_server_available() -> bool {
    let path = socket_path();

    if !path.exists() {
        return false;
    }

    // Try to connect briefly
    match tokio::net::UnixStream::connect(&path).await {
        Ok(_) => true,
        Err(_) => false,
    }
}

/// Wait for server to become available with retries
pub async fn wait_for_server(config: &AutoStartConfig) -> Result<()> {
    let start = Instant::now();
    let timeout = Duration::from_millis(config.timeout_ms);
    let retry_delay = Duration::from_millis(config.retry_delay_ms);
    let initial_delay = Duration::from_millis(config.initial_delay_ms);

    // Initial delay for server startup
    tokio::time::sleep(initial_delay).await;

    loop {
        if check_server_available().await {
            tracing::debug!("Server is available after {:?}", start.elapsed());
            return Ok(());
        }

        if start.elapsed() >= timeout {
            return Err(CcmuxError::ConnectionTimeout {
                seconds: config.timeout_ms / 1000,
            });
        }

        tokio::time::sleep(retry_delay).await;
    }
}

/// Result of attempting to ensure server is running
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServerStartResult {
    /// Server was already running
    AlreadyRunning,
    /// Server was started by this client
    Started,
    /// Auto-start is disabled and server is not running
    NotRunning,
}

/// Ensure the server is running, starting it if necessary
///
/// Returns information about whether the server was already running
/// or was started by this call.
pub async fn ensure_server_running(config: &AutoStartConfig) -> Result<ServerStartResult> {
    // First check if server is already running
    if check_server_available().await {
        return Ok(ServerStartResult::AlreadyRunning);
    }

    // If auto-start is disabled, return early
    if !config.enabled {
        return Ok(ServerStartResult::NotRunning);
    }

    // Start the server
    start_server_daemon()?;

    // Wait for it to become available
    wait_for_server(config).await?;

    Ok(ServerStartResult::Started)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auto_start_config_default() {
        let config = AutoStartConfig::default();
        assert!(config.enabled);
        assert_eq!(config.timeout_ms, 2000);
        assert_eq!(config.retry_delay_ms, 200);
        assert_eq!(config.initial_delay_ms, 100);
    }

    #[test]
    fn test_is_server_not_running_server_not_running_error() {
        let err = CcmuxError::ServerNotRunning {
            path: PathBuf::from("/tmp/test.sock"),
        };
        assert!(is_server_not_running(&err));
    }

    #[test]
    fn test_is_server_not_running_io_not_found() {
        let io_err = std::io::Error::new(ErrorKind::NotFound, "not found");
        let err = CcmuxError::Io(io_err);
        assert!(is_server_not_running(&err));
    }

    #[test]
    fn test_is_server_not_running_io_connection_refused() {
        let io_err = std::io::Error::new(ErrorKind::ConnectionRefused, "refused");
        let err = CcmuxError::Io(io_err);
        assert!(is_server_not_running(&err));
    }

    #[test]
    fn test_is_server_not_running_connection_message() {
        let err = CcmuxError::Connection("Connection refused".into());
        assert!(is_server_not_running(&err));
    }

    #[test]
    fn test_is_server_not_running_other_errors() {
        let err = CcmuxError::Protocol("bad protocol".into());
        assert!(!is_server_not_running(&err));

        let err = CcmuxError::SessionNotFound("test".into());
        assert!(!is_server_not_running(&err));
    }

    #[test]
    fn test_server_start_result_variants() {
        assert_eq!(ServerStartResult::AlreadyRunning, ServerStartResult::AlreadyRunning);
        assert_ne!(ServerStartResult::Started, ServerStartResult::NotRunning);
    }

    // Note: Integration tests for find_server_binary and start_server_daemon
    // require the actual binary to be present and are better suited for
    // integration test suite
}
