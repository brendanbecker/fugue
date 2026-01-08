//! ccmux-utils: Common utilities shared across ccmux crates
//!
//! This crate provides:
//! - Unified error types ([`CcmuxError`], [`Result`])
//! - Logging infrastructure ([`init_logging`], [`LogConfig`])
//! - XDG-compliant path utilities ([`paths`] module)

pub mod error;
pub mod logging;
pub mod paths;

// Re-export main types at crate root for convenience
pub use error::{CcmuxError, Result};
pub use logging::{init_logging, init_logging_with_config, LogConfig, LogOutput};

// Re-export commonly used path functions
pub use paths::{
    cache_dir, checkpoints_dir, config_dir, config_file, data_dir, ensure_all_dirs, log_dir,
    pid_file, runtime_dir, socket_path, state_dir, wal_dir,
};
