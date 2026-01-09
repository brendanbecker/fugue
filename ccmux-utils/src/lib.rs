//! ccmux-utils: Common utilities shared across ccmux crates
//!
//! This crate provides:
//! - Unified error types ([`CcmuxError`], [`Result`])
//! - Logging infrastructure ([`init_logging`], [`LogConfig`])
//! - Per-session logging ([`SessionLogger`], [`SessionLogLevel`])
//! - XDG-compliant path utilities ([`paths`] module)

pub mod error;
pub mod logging;
pub mod paths;
pub mod session_logging;

// Re-export main types at crate root for convenience
pub use error::{CcmuxError, Result};
pub use logging::{init_logging, init_logging_with_config, LogConfig, LogOutput};
pub use session_logging::{
    LogEntry, LogEventType, SessionLogConfig, SessionLogLevel, SessionLogger,
};

// Re-export commonly used path functions
pub use paths::{
    cache_dir, checkpoints_dir, config_dir, config_file, data_dir, ensure_all_dirs, log_dir,
    pid_file, runtime_dir, session_log_dir, socket_path, state_dir, wal_dir,
};
