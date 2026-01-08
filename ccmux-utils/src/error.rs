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

    #[test]
    fn test_error_display() {
        let err = CcmuxError::SessionNotFound("test".into());
        assert_eq!(err.to_string(), "Session not found: test");
    }

    #[test]
    fn test_retryable() {
        assert!(CcmuxError::ConnectionTimeout { seconds: 5 }.is_retryable());
        assert!(!CcmuxError::SessionNotFound("x".into()).is_retryable());
    }

    #[test]
    fn test_from_io_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "not found");
        let err: CcmuxError = io_err.into();
        assert!(matches!(err, CcmuxError::Io(_)));
    }
}
