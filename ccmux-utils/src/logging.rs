//! Logging infrastructure for ccmux
//!
//! Provides unified logging setup using the tracing ecosystem.

use tracing_subscriber::{
    fmt::{self, format::FmtSpan},
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter,
};

use crate::{paths, Result, CcmuxError};

/// Log output destination
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogOutput {
    /// Log to stderr (for client)
    Stderr,
    /// Log to file (for server daemon)
    File,
    /// Log to both stderr and file
    Both,
}

/// Logging configuration
#[derive(Debug, Clone)]
pub struct LogConfig {
    /// Output destination
    pub output: LogOutput,
    /// Log level filter (e.g., "info", "debug", "ccmux=debug,tokio=warn")
    pub filter: String,
    /// Include span events (enter/exit)
    pub span_events: bool,
    /// Include file/line in logs
    pub file_line: bool,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            output: LogOutput::Stderr,
            filter: "info".into(),
            span_events: false,
            file_line: false,
        }
    }
}

impl LogConfig {
    /// Create config for client (stderr only)
    pub fn client() -> Self {
        Self {
            output: LogOutput::Stderr,
            filter: std::env::var("CCMUX_LOG")
                .unwrap_or_else(|_| "warn".into()),
            span_events: false,
            file_line: false,
        }
    }

    /// Create config for server daemon (file logging)
    pub fn server() -> Self {
        Self {
            output: LogOutput::File,
            filter: std::env::var("CCMUX_LOG")
                .unwrap_or_else(|_| "info".into()),
            span_events: true,
            file_line: true,
        }
    }

    /// Create config for development (verbose stderr)
    pub fn development() -> Self {
        Self {
            output: LogOutput::Stderr,
            filter: "debug".into(),
            span_events: true,
            file_line: true,
        }
    }
}

/// Initialize logging with default configuration
///
/// Uses CCMUX_LOG env var for filter, defaults to "info"
pub fn init_logging() -> Result<()> {
    init_logging_with_config(LogConfig::default())
}

/// Initialize logging with custom configuration
pub fn init_logging_with_config(config: LogConfig) -> Result<()> {
    let filter = EnvFilter::try_new(&config.filter)
        .map_err(|e| CcmuxError::config(format!("Invalid log filter: {}", e)))?;

    let fmt_layer = fmt::layer()
        .with_target(true)
        .with_thread_ids(false)
        .with_thread_names(false);

    let fmt_layer = if config.span_events {
        fmt_layer.with_span_events(FmtSpan::ENTER | FmtSpan::EXIT)
    } else {
        fmt_layer
    };

    let fmt_layer = if config.file_line {
        fmt_layer.with_file(true).with_line_number(true)
    } else {
        fmt_layer.with_file(false).with_line_number(false)
    };

    match config.output {
        LogOutput::Stderr => {
            tracing_subscriber::registry()
                .with(filter)
                .with(fmt_layer.with_writer(std::io::stderr))
                .try_init()
                .map_err(|e| CcmuxError::internal(format!("Failed to init logging: {}", e)))?;
        }
        LogOutput::File => {
            let log_dir = paths::log_dir();
            std::fs::create_dir_all(&log_dir)
                .map_err(|e| CcmuxError::FileWrite {
                    path: log_dir.clone(),
                    source: e,
                })?;

            let log_path = log_dir.join("ccmux.log");
            let file = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&log_path)
                .map_err(|e| CcmuxError::FileWrite {
                    path: log_path,
                    source: e,
                })?;

            tracing_subscriber::registry()
                .with(filter)
                .with(fmt_layer.with_writer(file).with_ansi(false))
                .try_init()
                .map_err(|e| CcmuxError::internal(format!("Failed to init logging: {}", e)))?;
        }
        LogOutput::Both => {
            let log_dir = paths::log_dir();
            std::fs::create_dir_all(&log_dir)
                .map_err(|e| CcmuxError::FileWrite {
                    path: log_dir.clone(),
                    source: e,
                })?;

            let log_path = log_dir.join("ccmux.log");
            let file = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&log_path)
                .map_err(|e| CcmuxError::FileWrite {
                    path: log_path,
                    source: e,
                })?;

            let file_layer = fmt::layer()
                .with_writer(file)
                .with_ansi(false)
                .with_target(true);

            tracing_subscriber::registry()
                .with(filter)
                .with(fmt_layer.with_writer(std::io::stderr))
                .with(file_layer)
                .try_init()
                .map_err(|e| CcmuxError::internal(format!("Failed to init logging: {}", e)))?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_config_defaults() {
        let config = LogConfig::default();
        assert_eq!(config.output, LogOutput::Stderr);
        assert_eq!(config.filter, "info");
    }

    #[test]
    fn test_log_config_client() {
        let config = LogConfig::client();
        assert_eq!(config.output, LogOutput::Stderr);
    }

    #[test]
    fn test_log_config_server() {
        let config = LogConfig::server();
        assert_eq!(config.output, LogOutput::File);
        assert!(config.span_events);
    }
}
