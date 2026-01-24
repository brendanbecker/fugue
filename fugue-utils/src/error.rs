//! Error types for ccmux
//!
//! Provides a unified error type used across all ccmux crates.

use std::path::PathBuf;

/// Main error type for ccmux operations
#[derive(Debug, thiserror::Error)]
pub enum CcmuxError {
    // === IO Errors ===

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Failed to read file {path}: {source}")]
    FileRead {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("Failed to write file {path}: {source}")]
    FileWrite {
        path: PathBuf,
        source: std::io::Error,
    },

    // === Connection Errors ===

    #[error("Connection failed: {0}")]
    Connection(String),

    #[error("Server not running at {path}")]
    ServerNotRunning { path: PathBuf },

    #[error("Connection timeout after {seconds}s")]
    ConnectionTimeout { seconds: u64 },

    #[error("Connection closed unexpectedly")]
    ConnectionClosed,

    // === Protocol Errors ===

    #[error("Protocol error: {0}")]
    Protocol(String),

    #[error("Protocol version mismatch: client={client}, server={server}")]
    ProtocolMismatch { client: u32, server: u32 },

    #[error("Invalid message: {0}")]
    InvalidMessage(String),

    // === Configuration Errors ===

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Invalid configuration at {path}: {message}")]
    ConfigInvalid { path: PathBuf, message: String },

    #[error("Configuration file not found: {0}")]
    ConfigNotFound(PathBuf),

    // === Session Errors ===

    #[error("Session not found: {0}")]
    SessionNotFound(String),

    #[error("Window not found: {0}")]
    WindowNotFound(String),

    #[error("Pane not found: {0}")]
    PaneNotFound(String),

    #[error("Session already exists: {0}")]
    SessionExists(String),

    // === PTY Errors ===

    #[error("PTY error: {0}")]
    Pty(String),

    #[error("Failed to spawn process: {0}")]
    ProcessSpawn(String),

    // === Persistence Errors ===

    #[error("Persistence error: {0}")]
    Persistence(String),

    #[error("Recovery failed: {0}")]
    Recovery(String),

    // === Internal Errors ===

    #[error("Internal error: {0}")]
    Internal(String),
}

impl CcmuxError {
    /// Create a connection error
    pub fn connection(msg: impl Into<String>) -> Self {
        Self::Connection(msg.into())
    }

    /// Create a protocol error
    pub fn protocol(msg: impl Into<String>) -> Self {
        Self::Protocol(msg.into())
    }

    /// Create a config error
    pub fn config(msg: impl Into<String>) -> Self {
        Self::Config(msg.into())
    }

    /// Create a PTY error
    pub fn pty(msg: impl Into<String>) -> Self {
        Self::Pty(msg.into())
    }

    /// Create a persistence error
    pub fn persistence(msg: impl Into<String>) -> Self {
        Self::Persistence(msg.into())
    }

    /// Create an internal error
    pub fn internal(msg: impl Into<String>) -> Self {
        Self::Internal(msg.into())
    }

    /// Check if this error is retryable
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::ConnectionTimeout { .. }
            | Self::Connection(_)
        )
    }
}

/// Result type alias using CcmuxError
pub type Result<T> = std::result::Result<T, CcmuxError>;

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== Display Tests ====================

    #[test]
    fn test_error_display() {
        let err = CcmuxError::SessionNotFound("test".into());
        assert_eq!(err.to_string(), "Session not found: test");
    }

    #[test]
    fn test_error_display_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let err = CcmuxError::Io(io_err);
        assert!(err.to_string().contains("IO error"));
    }

    #[test]
    fn test_error_display_file_read() {
        let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "permission denied");
        let err = CcmuxError::FileRead {
            path: PathBuf::from("/etc/passwd"),
            source: io_err,
        };
        let msg = err.to_string();
        assert!(msg.contains("Failed to read file"));
        assert!(msg.contains("/etc/passwd"));
    }

    #[test]
    fn test_error_display_file_write() {
        let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "permission denied");
        let err = CcmuxError::FileWrite {
            path: PathBuf::from("/root/test.txt"),
            source: io_err,
        };
        let msg = err.to_string();
        assert!(msg.contains("Failed to write file"));
        assert!(msg.contains("/root/test.txt"));
    }

    #[test]
    fn test_error_display_connection() {
        let err = CcmuxError::Connection("refused".into());
        assert_eq!(err.to_string(), "Connection failed: refused");
    }

    #[test]
    fn test_error_display_server_not_running() {
        let err = CcmuxError::ServerNotRunning {
            path: PathBuf::from("/tmp/ccmux.sock"),
        };
        let msg = err.to_string();
        assert!(msg.contains("Server not running"));
        assert!(msg.contains("/tmp/ccmux.sock"));
    }

    #[test]
    fn test_error_display_connection_timeout() {
        let err = CcmuxError::ConnectionTimeout { seconds: 30 };
        assert_eq!(err.to_string(), "Connection timeout after 30s");
    }

    #[test]
    fn test_error_display_connection_closed() {
        let err = CcmuxError::ConnectionClosed;
        assert_eq!(err.to_string(), "Connection closed unexpectedly");
    }

    #[test]
    fn test_error_display_protocol() {
        let err = CcmuxError::Protocol("invalid frame".into());
        assert_eq!(err.to_string(), "Protocol error: invalid frame");
    }

    #[test]
    fn test_error_display_protocol_mismatch() {
        let err = CcmuxError::ProtocolMismatch {
            client: 1,
            server: 2,
        };
        assert_eq!(
            err.to_string(),
            "Protocol version mismatch: client=1, server=2"
        );
    }

    #[test]
    fn test_error_display_invalid_message() {
        let err = CcmuxError::InvalidMessage("malformed JSON".into());
        assert_eq!(err.to_string(), "Invalid message: malformed JSON");
    }

    #[test]
    fn test_error_display_config() {
        let err = CcmuxError::Config("missing key".into());
        assert_eq!(err.to_string(), "Configuration error: missing key");
    }

    #[test]
    fn test_error_display_config_invalid() {
        let err = CcmuxError::ConfigInvalid {
            path: PathBuf::from("/home/user/.config/ccmux/config.toml"),
            message: "syntax error".into(),
        };
        let msg = err.to_string();
        assert!(msg.contains("Invalid configuration"));
        assert!(msg.contains("config.toml"));
        assert!(msg.contains("syntax error"));
    }

    #[test]
    fn test_error_display_config_not_found() {
        let err = CcmuxError::ConfigNotFound(PathBuf::from("/missing/config.toml"));
        let msg = err.to_string();
        assert!(msg.contains("Configuration file not found"));
        assert!(msg.contains("/missing/config.toml"));
    }

    #[test]
    fn test_error_display_window_not_found() {
        let err = CcmuxError::WindowNotFound("main".into());
        assert_eq!(err.to_string(), "Window not found: main");
    }

    #[test]
    fn test_error_display_pane_not_found() {
        let err = CcmuxError::PaneNotFound("abc-123".into());
        assert_eq!(err.to_string(), "Pane not found: abc-123");
    }

    #[test]
    fn test_error_display_session_exists() {
        let err = CcmuxError::SessionExists("my-session".into());
        assert_eq!(err.to_string(), "Session already exists: my-session");
    }

    #[test]
    fn test_error_display_pty() {
        let err = CcmuxError::Pty("failed to allocate PTY".into());
        assert_eq!(err.to_string(), "PTY error: failed to allocate PTY");
    }

    #[test]
    fn test_error_display_process_spawn() {
        let err = CcmuxError::ProcessSpawn("command not found".into());
        assert_eq!(err.to_string(), "Failed to spawn process: command not found");
    }

    #[test]
    fn test_error_display_persistence() {
        let err = CcmuxError::Persistence("WAL corrupted".into());
        assert_eq!(err.to_string(), "Persistence error: WAL corrupted");
    }

    #[test]
    fn test_error_display_recovery() {
        let err = CcmuxError::Recovery("checkpoint missing".into());
        assert_eq!(err.to_string(), "Recovery failed: checkpoint missing");
    }

    #[test]
    fn test_error_display_internal() {
        let err = CcmuxError::Internal("unexpected state".into());
        assert_eq!(err.to_string(), "Internal error: unexpected state");
    }

    // ==================== Retryable Tests ====================

    #[test]
    fn test_retryable() {
        assert!(CcmuxError::ConnectionTimeout { seconds: 5 }.is_retryable());
        assert!(!CcmuxError::SessionNotFound("x".into()).is_retryable());
    }

    #[test]
    fn test_retryable_connection() {
        assert!(CcmuxError::Connection("refused".into()).is_retryable());
    }

    #[test]
    fn test_retryable_timeout_various_durations() {
        for seconds in [1, 5, 10, 30, 60, 300] {
            assert!(CcmuxError::ConnectionTimeout { seconds }.is_retryable());
        }
    }

    #[test]
    fn test_not_retryable_errors() {
        let non_retryable = [
            CcmuxError::SessionNotFound("test".into()),
            CcmuxError::WindowNotFound("test".into()),
            CcmuxError::PaneNotFound("test".into()),
            CcmuxError::SessionExists("test".into()),
            CcmuxError::Protocol("error".into()),
            CcmuxError::ProtocolMismatch { client: 1, server: 2 },
            CcmuxError::InvalidMessage("bad".into()),
            CcmuxError::Config("bad".into()),
            CcmuxError::ConfigNotFound(PathBuf::from("/test")),
            CcmuxError::Pty("error".into()),
            CcmuxError::ProcessSpawn("error".into()),
            CcmuxError::Persistence("error".into()),
            CcmuxError::Recovery("error".into()),
            CcmuxError::Internal("error".into()),
            CcmuxError::ConnectionClosed,
            CcmuxError::ServerNotRunning { path: PathBuf::from("/tmp/sock") },
        ];

        for err in non_retryable {
            assert!(
                !err.is_retryable(),
                "Expected {:?} to NOT be retryable",
                err
            );
        }
    }

    // ==================== From Trait Tests ====================

    #[test]
    fn test_from_io_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "not found");
        let err: CcmuxError = io_err.into();
        assert!(matches!(err, CcmuxError::Io(_)));
    }

    #[test]
    fn test_from_io_error_preserves_kind() {
        let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied");
        let err: CcmuxError = io_err.into();
        if let CcmuxError::Io(inner) = err {
            assert_eq!(inner.kind(), std::io::ErrorKind::PermissionDenied);
        } else {
            panic!("Expected Io variant");
        }
    }

    // ==================== Helper Function Tests ====================

    #[test]
    fn test_connection_helper() {
        let err = CcmuxError::connection("connection refused");
        assert!(matches!(err, CcmuxError::Connection(_)));
        assert_eq!(err.to_string(), "Connection failed: connection refused");
    }

    #[test]
    fn test_connection_helper_with_string() {
        let msg = String::from("host unreachable");
        let err = CcmuxError::connection(msg);
        assert_eq!(err.to_string(), "Connection failed: host unreachable");
    }

    #[test]
    fn test_protocol_helper() {
        let err = CcmuxError::protocol("invalid frame header");
        assert!(matches!(err, CcmuxError::Protocol(_)));
        assert_eq!(err.to_string(), "Protocol error: invalid frame header");
    }

    #[test]
    fn test_config_helper() {
        let err = CcmuxError::config("missing required field 'name'");
        assert!(matches!(err, CcmuxError::Config(_)));
        assert!(err.to_string().contains("missing required field"));
    }

    #[test]
    fn test_pty_helper() {
        let err = CcmuxError::pty("no available PTY devices");
        assert!(matches!(err, CcmuxError::Pty(_)));
        assert_eq!(err.to_string(), "PTY error: no available PTY devices");
    }

    #[test]
    fn test_persistence_helper() {
        let err = CcmuxError::persistence("disk full");
        assert!(matches!(err, CcmuxError::Persistence(_)));
        assert_eq!(err.to_string(), "Persistence error: disk full");
    }

    #[test]
    fn test_internal_helper() {
        let err = CcmuxError::internal("invariant violated");
        assert!(matches!(err, CcmuxError::Internal(_)));
        assert_eq!(err.to_string(), "Internal error: invariant violated");
    }

    // ==================== Result Type Tests ====================

    #[test]
    fn test_result_ok() {
        let result: Result<i32> = Ok(42);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_result_err() {
        let result: Result<i32> = Err(CcmuxError::SessionNotFound("test".into()));
        assert!(result.is_err());
    }

    #[test]
    fn test_result_map() {
        let result: Result<i32> = Ok(21);
        let mapped = result.map(|x| x * 2);
        assert_eq!(mapped.unwrap(), 42);
    }

    #[test]
    fn test_result_and_then() {
        let result: Result<i32> = Ok(42);
        let chained = result.and_then(|x| {
            if x > 0 {
                Ok(x.to_string())
            } else {
                Err(CcmuxError::internal("negative"))
            }
        });
        assert_eq!(chained.unwrap(), "42");
    }

    // ==================== Debug Tests ====================

    #[test]
    fn test_error_debug() {
        let err = CcmuxError::SessionNotFound("my-session".into());
        let debug = format!("{:?}", err);
        assert!(debug.contains("SessionNotFound"));
        assert!(debug.contains("my-session"));
    }

    #[test]
    fn test_error_debug_complex() {
        let err = CcmuxError::ProtocolMismatch { client: 1, server: 2 };
        let debug = format!("{:?}", err);
        assert!(debug.contains("ProtocolMismatch"));
        assert!(debug.contains("client"));
        assert!(debug.contains("server"));
    }
}
