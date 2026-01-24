//! Logging infrastructure for fugue
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
    /// Log level filter (e.g., "info", "debug", "fugue=debug,tokio=warn")
    pub filter: String,
    /// Include span events (enter/exit)
    pub span_events: bool,
    /// Include file/line in logs
    pub file_line: bool,
    /// Optional custom log file name (defaults to "fugue.log")
    pub file_name: Option<String>,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            output: LogOutput::Stderr,
            filter: "info".into(),
            span_events: false,
            file_line: false,
            file_name: None,
        }
    }
}

impl LogConfig {
    /// Create config for client (file logging, since TUI owns the terminal)
    pub fn client() -> Self {
        Self {
            output: LogOutput::File,
            filter: std::env::var("FUGUE_LOG")
                .unwrap_or_else(|_| "warn".into()),
            span_events: false,
            file_line: false,
            file_name: None,
        }
    }

    /// Create config for server daemon (file logging)
    pub fn server() -> Self {
        Self {
            output: LogOutput::File,
            filter: std::env::var("FUGUE_LOG")
                .unwrap_or_else(|_| "info".into()),
            span_events: true,
            file_line: true,
            file_name: None,
        }
    }

    /// Create config for MCP bridge (file logging, separate file)
    pub fn mcp_bridge() -> Self {
        Self {
            output: LogOutput::File,
            filter: std::env::var("FUGUE_MCP_LOG")
                .or_else(|_| std::env::var("FUGUE_LOG"))
                .unwrap_or_else(|_| "info".into()),
            span_events: true,
            file_line: true,
            file_name: Some("mcp-bridge.log".into()),
        }
    }

    /// Create config for standalone MCP server (file logging, separate file)
    pub fn mcp_server() -> Self {
        Self {
            output: LogOutput::File,
            filter: std::env::var("FUGUE_MCP_LOG")
                .or_else(|_| std::env::var("FUGUE_LOG"))
                .unwrap_or_else(|_| "info".into()),
            span_events: true,
            file_line: true,
            file_name: Some("mcp-server.log".into()),
        }
    }

    /// Create config for development (verbose stderr)
    pub fn development() -> Self {
        Self {
            output: LogOutput::Stderr,
            filter: "debug".into(),
            span_events: true,
            file_line: true,
            file_name: None,
        }
    }
}

/// Initialize logging with default configuration
///
/// Uses FUGUE_LOG env var for filter, defaults to "info"
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

    let file_name = config.file_name.as_deref().unwrap_or("fugue.log");

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

            let log_path = log_dir.join(file_name);
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

            let log_path = log_dir.join(file_name);
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
    use std::env;

    // ==================== LogOutput Tests ====================

    #[test]
    fn test_log_output_stderr() {
        let output = LogOutput::Stderr;
        assert_eq!(output, LogOutput::Stderr);
    }

    #[test]
    fn test_log_output_file() {
        let output = LogOutput::File;
        assert_eq!(output, LogOutput::File);
    }

    #[test]
    fn test_log_output_both() {
        let output = LogOutput::Both;
        assert_eq!(output, LogOutput::Both);
    }

    #[test]
    fn test_log_output_equality() {
        assert_eq!(LogOutput::Stderr, LogOutput::Stderr);
        assert_eq!(LogOutput::File, LogOutput::File);
        assert_eq!(LogOutput::Both, LogOutput::Both);

        assert_ne!(LogOutput::Stderr, LogOutput::File);
        assert_ne!(LogOutput::File, LogOutput::Both);
        assert_ne!(LogOutput::Both, LogOutput::Stderr);
    }

    #[test]
    fn test_log_output_clone() {
        let output = LogOutput::File;
        let cloned = output.clone();
        assert_eq!(output, cloned);
    }

    #[test]
    fn test_log_output_copy() {
        let output = LogOutput::Both;
        let copied = output; // Copy semantics
        assert_eq!(output, copied);
    }

    #[test]
    fn test_log_output_debug() {
        assert_eq!(format!("{:?}", LogOutput::Stderr), "Stderr");
        assert_eq!(format!("{:?}", LogOutput::File), "File");
        assert_eq!(format!("{:?}", LogOutput::Both), "Both");
    }

    // ==================== LogConfig Default Tests ====================

    #[test]
    fn test_log_config_defaults() {
        let config = LogConfig::default();
        assert_eq!(config.output, LogOutput::Stderr);
        assert_eq!(config.filter, "info");
    }

    #[test]
    fn test_log_config_default_span_events() {
        let config = LogConfig::default();
        assert!(!config.span_events);
    }

    #[test]
    fn test_log_config_default_file_line() {
        let config = LogConfig::default();
        assert!(!config.file_line);
    }

    // ==================== LogConfig::client() Tests ====================

    #[test]
    fn test_log_config_client() {
        let config = LogConfig::client();
        assert_eq!(config.output, LogOutput::File);
    }

    #[test]
    fn test_log_config_client_default_filter() {
        // Save original
        let original = env::var("FUGUE_LOG").ok();
        env::remove_var("FUGUE_LOG");

        let config = LogConfig::client();
        assert_eq!(config.filter, "warn");

        // Restore
        if let Some(val) = original {
            env::set_var("FUGUE_LOG", val);
        }
    }

    #[test]
    fn test_log_config_client_with_env() {
        // Save original
        let original = env::var("FUGUE_LOG").ok();
        env::set_var("FUGUE_LOG", "debug");

        let config = LogConfig::client();
        assert_eq!(config.filter, "debug");

        // Restore
        match original {
            Some(val) => env::set_var("FUGUE_LOG", val),
            None => env::remove_var("FUGUE_LOG"),
        }
    }

    #[test]
    fn test_log_config_client_no_span_events() {
        let config = LogConfig::client();
        assert!(!config.span_events);
    }

    #[test]
    fn test_log_config_client_no_file_line() {
        let config = LogConfig::client();
        assert!(!config.file_line);
    }

    // ==================== LogConfig::server() Tests ====================

    #[test]
    fn test_log_config_server() {
        let config = LogConfig::server();
        assert_eq!(config.output, LogOutput::File);
        assert!(config.span_events);
    }

    #[test]
    fn test_log_config_server_default_filter() {
        // Save original
        let original = env::var("FUGUE_LOG").ok();
        env::remove_var("FUGUE_LOG");

        let config = LogConfig::server();
        assert_eq!(config.filter, "info");

        // Restore
        if let Some(val) = original {
            env::set_var("FUGUE_LOG", val);
        }
    }

    #[test]
    fn test_log_config_server_with_env() {
        // Save original
        let original = env::var("FUGUE_LOG").ok();
        env::set_var("FUGUE_LOG", "trace");

        let config = LogConfig::server();
        assert_eq!(config.filter, "trace");

        // Restore
        match original {
            Some(val) => env::set_var("FUGUE_LOG", val),
            None => env::remove_var("FUGUE_LOG"),
        }
    }

    #[test]
    fn test_log_config_server_span_events() {
        let config = LogConfig::server();
        assert!(config.span_events);
    }

    #[test]
    fn test_log_config_server_file_line() {
        let config = LogConfig::server();
        assert!(config.file_line);
    }

    // ==================== LogConfig::mcp_bridge() Tests ====================

    #[test]
    fn test_log_config_mcp_bridge() {
        let config = LogConfig::mcp_bridge();
        assert_eq!(config.output, LogOutput::File);
        assert_eq!(config.file_name, Some("mcp-bridge.log".into()));
        assert!(config.span_events);
        assert!(config.file_line);
    }

    #[test]
    fn test_log_config_mcp_bridge_default_filter() {
        let _original_mcp = env::var("FUGUE_MCP_LOG").ok();
        let _original_fugue = env::var("FUGUE_LOG").ok();
        env::remove_var("FUGUE_MCP_LOG");
        env::remove_var("FUGUE_LOG");

        let config = LogConfig::mcp_bridge();
        assert_eq!(config.filter, "info");

        if let Some(val) = _original_mcp { env::set_var("FUGUE_MCP_LOG", val); }
        if let Some(val) = _original_fugue { env::set_var("FUGUE_LOG", val); }
    }

    #[test]
    fn test_log_config_mcp_bridge_with_env() {
        let _original = env::var("FUGUE_MCP_LOG").ok();
        env::set_var("FUGUE_MCP_LOG", "trace");

        let config = LogConfig::mcp_bridge();
        assert_eq!(config.filter, "trace");

        match _original {
            Some(val) => env::set_var("FUGUE_MCP_LOG", val),
            None => env::remove_var("FUGUE_MCP_LOG"),
        }
    }

    // ==================== LogConfig::mcp_server() Tests ====================

    #[test]
    fn test_log_config_mcp_server() {
        let config = LogConfig::mcp_server();
        assert_eq!(config.output, LogOutput::File);
        assert_eq!(config.file_name, Some("mcp-server.log".into()));
    }

    // ==================== LogConfig::development() Tests ====================

    #[test]
    fn test_log_config_development() {
        let config = LogConfig::development();
        assert_eq!(config.output, LogOutput::Stderr);
        assert_eq!(config.filter, "debug");
        assert!(config.span_events);
        assert!(config.file_line);
    }

    #[test]
    fn test_log_config_development_verbose() {
        let config = LogConfig::development();
        // Development should be verbose
        assert!(config.span_events);
        assert!(config.file_line);
    }

    // ==================== LogConfig Clone Tests ====================

    #[test]
    fn test_log_config_clone() {
        let config = LogConfig {
            output: LogOutput::Both,
            filter: "fugue=debug,tokio=warn".to_string(),
            span_events: true,
            file_line: true,
            file_name: Some("test.log".into()),
        };

        let cloned = config.clone();
        assert_eq!(config.output, cloned.output);
        assert_eq!(config.filter, cloned.filter);
        assert_eq!(config.span_events, cloned.span_events);
        assert_eq!(config.file_line, cloned.file_line);
        assert_eq!(config.file_name, cloned.file_name);
    }

    // ==================== LogConfig Debug Tests ====================

    #[test]
    fn test_log_config_debug() {
        let config = LogConfig::default();
        let debug = format!("{:?}", config);
        assert!(debug.contains("LogConfig"));
        assert!(debug.contains("Stderr"));
        assert!(debug.contains("info"));
    }

    // ==================== LogConfig Custom Tests ====================

    #[test]
    fn test_log_config_custom_filter() {
        let config = LogConfig {
            filter: "fugue=trace,hyper=warn".to_string(),
            ..LogConfig::default()
        };
        assert_eq!(config.filter, "fugue=trace,hyper=warn");
    }

    #[test]
    fn test_log_config_custom_output() {
        let config = LogConfig {
            output: LogOutput::Both,
            ..LogConfig::default()
        };
        assert_eq!(config.output, LogOutput::Both);
    }

    #[test]
    fn test_log_config_all_options() {
        let config = LogConfig {
            output: LogOutput::File,
            filter: "error".to_string(),
            span_events: true,
            file_line: false,
            file_name: Some("all.log".into()),
        };

        assert_eq!(config.output, LogOutput::File);
        assert_eq!(config.filter, "error");
        assert!(config.span_events);
        assert!(!config.file_line);
        assert_eq!(config.file_name, Some("all.log".into()));
    }

    // ==================== Config Comparison Tests ====================

    #[test]
    fn test_client_vs_server_config() {
        let client = LogConfig::client();
        let server = LogConfig::server();

        // Both use file logging (client can't use stderr since TUI owns terminal)
        assert_eq!(client.output, LogOutput::File);
        assert_eq!(server.output, LogOutput::File);

        // Server should be more verbose with spans and file info
        assert!(!client.span_events);
        assert!(server.span_events);
        assert!(!client.file_line);
        assert!(server.file_line);
    }

    #[test]
    fn test_development_vs_default() {
        let dev = LogConfig::development();
        let default = LogConfig::default();

        // Both should use stderr
        assert_eq!(dev.output, LogOutput::Stderr);
        assert_eq!(default.output, LogOutput::Stderr);

        // Development should be more verbose
        assert_eq!(dev.filter, "debug");
        assert_eq!(default.filter, "info");

        // Development should have more detail
        assert!(dev.span_events);
        assert!(!default.span_events);
        assert!(dev.file_line);
        assert!(!default.file_line);
    }

    // ==================== Filter String Tests ====================

    #[test]
    fn test_log_config_various_filters() {
        let filters = [
            "info",
            "debug",
            "warn",
            "error",
            "trace",
            "fugue=debug",
            "fugue=trace,tokio=warn",
            "fugue::server=debug,fugue::client=info",
        ];

        for filter_str in filters {
            let config = LogConfig {
                filter: filter_str.to_string(),
                ..LogConfig::default()
            };
            assert_eq!(config.filter, filter_str);
        }
    }

    // Note: We cannot easily test init_logging() in unit tests because:
    // 1. The tracing subscriber can only be initialized once per process
    // 2. Tests run in parallel in the same process
    // 3. Testing would require mocking file system operations
    //
    // Integration tests would be more appropriate for testing init_logging()
}
