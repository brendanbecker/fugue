//! Per-session logging infrastructure for ccmux
//!
//! Provides session-specific logging with configurable levels,
//! structured output, log rotation, and audit trail separation.

use serde::{Deserialize, Serialize};
use std::fs::{self, File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use uuid::Uuid;

use crate::{paths, CcmuxError, Result};

/// Log level for per-session logging
///
/// Each level includes all events from lower levels.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SessionLogLevel {
    /// Minimal: lifecycle events only (spawn, terminate)
    Spawns,
    /// Spawns + completions, errors, signals
    #[default]
    Signals,
    /// Signals + initial prompts
    Prompts,
    /// Complete transcripts (everything)
    Full,
}

impl std::fmt::Display for SessionLogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Spawns => write!(f, "spawns"),
            Self::Signals => write!(f, "signals"),
            Self::Prompts => write!(f, "prompts"),
            Self::Full => write!(f, "full"),
        }
    }
}

impl std::str::FromStr for SessionLogLevel {
    type Err = CcmuxError;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "spawns" => Ok(Self::Spawns),
            "signals" => Ok(Self::Signals),
            "prompts" => Ok(Self::Prompts),
            "full" => Ok(Self::Full),
            _ => Err(CcmuxError::config(format!("Invalid session log level: {}", s))),
        }
    }
}

/// Type of log event
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LogEventType {
    // Spawns level
    SessionCreated,
    SessionTerminated,
    WindowCreated,
    WindowClosed,
    PaneCreated,
    PaneClosed,

    // Signals level
    ProcessStarted,
    ProcessExited,
    Error,
    Warning,
    ClientAttached,
    ClientDetached,

    // Prompts level
    PromptDetected,
    CommandStarted,

    // Full level
    Output,
    Input,
}

impl LogEventType {
    /// Get the minimum log level required for this event type
    pub fn min_level(&self) -> SessionLogLevel {
        match self {
            Self::SessionCreated
            | Self::SessionTerminated
            | Self::WindowCreated
            | Self::WindowClosed
            | Self::PaneCreated
            | Self::PaneClosed => SessionLogLevel::Spawns,

            Self::ProcessStarted
            | Self::ProcessExited
            | Self::Error
            | Self::Warning
            | Self::ClientAttached
            | Self::ClientDetached => SessionLogLevel::Signals,

            Self::PromptDetected
            | Self::CommandStarted => SessionLogLevel::Prompts,

            Self::Output
            | Self::Input => SessionLogLevel::Full,
        }
    }
}

/// A structured log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    /// ISO 8601 timestamp
    pub timestamp: String,
    /// Session ID
    pub session_id: Uuid,
    /// Event type
    pub event_type: LogEventType,
    /// Optional window ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub window_id: Option<Uuid>,
    /// Optional pane ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pane_id: Option<Uuid>,
    /// Event-specific payload
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload: Option<serde_json::Value>,
}

impl LogEntry {
    /// Create a new log entry with the current timestamp
    pub fn new(session_id: Uuid, event_type: LogEventType) -> Self {
        Self {
            timestamp: Self::iso8601_now(),
            session_id,
            event_type,
            window_id: None,
            pane_id: None,
            payload: None,
        }
    }

    /// Set the window ID
    pub fn with_window(mut self, window_id: Uuid) -> Self {
        self.window_id = Some(window_id);
        self
    }

    /// Set the pane ID
    pub fn with_pane(mut self, pane_id: Uuid) -> Self {
        self.pane_id = Some(pane_id);
        self
    }

    /// Set the payload
    pub fn with_payload(mut self, payload: impl Serialize) -> Self {
        self.payload = serde_json::to_value(payload).ok();
        self
    }

    /// Get current time as ISO 8601 string
    fn iso8601_now() -> String {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();

        // Format as ISO 8601 with milliseconds
        let secs = now.as_secs();
        let millis = now.subsec_millis();

        // Calculate date/time components
        let days_since_epoch = secs / 86400;
        let time_of_day = secs % 86400;
        let hours = time_of_day / 3600;
        let minutes = (time_of_day % 3600) / 60;
        let seconds = time_of_day % 60;

        // Simple year/month/day calculation (valid for 1970-2099)
        let mut year = 1970;
        let mut remaining_days = days_since_epoch as i64;

        loop {
            let days_in_year = if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) {
                366
            } else {
                365
            };
            if remaining_days < days_in_year {
                break;
            }
            remaining_days -= days_in_year;
            year += 1;
        }

        let is_leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
        let days_in_months = if is_leap {
            [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
        } else {
            [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
        };

        let mut month = 1;
        for &days_in_month in &days_in_months {
            if remaining_days < days_in_month as i64 {
                break;
            }
            remaining_days -= days_in_month as i64;
            month += 1;
        }
        let day = remaining_days + 1;

        format!(
            "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:03}Z",
            year, month, day, hours, minutes, seconds, millis
        )
    }
}

/// Configuration for session logging
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SessionLogConfig {
    /// Default log level for new sessions
    pub default_level: SessionLogLevel,
    /// Maximum log file size in bytes before rotation (default: 10MB)
    pub max_file_size: u64,
    /// Maximum number of rotated log files to keep (default: 5)
    pub max_rotated_files: u32,
    /// Retention period in seconds (default: 7 days)
    pub retention_secs: u64,
    /// Whether to separate audit trail (user actions vs system events)
    pub separate_audit_trail: bool,
}

impl Default for SessionLogConfig {
    fn default() -> Self {
        Self {
            default_level: SessionLogLevel::Signals,
            max_file_size: 10 * 1024 * 1024, // 10MB
            max_rotated_files: 5,
            retention_secs: 7 * 24 * 60 * 60, // 7 days
            separate_audit_trail: true,
        }
    }
}

/// Per-session logger with log rotation and structured output
pub struct SessionLogger {
    session_id: Uuid,
    log_dir: PathBuf,
    level: Mutex<SessionLogLevel>,
    config: SessionLogConfig,

    // System events log
    system_log: Mutex<Option<BufWriter<File>>>,
    system_log_size: AtomicU64,

    // User actions audit trail (separate file)
    audit_log: Mutex<Option<BufWriter<File>>>,
    audit_log_size: AtomicU64,
}

impl std::fmt::Debug for SessionLogger {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SessionLogger")
            .field("session_id", &self.session_id)
            .field("log_dir", &self.log_dir)
            .field("level", &*self.level.lock().unwrap())
            .field("config", &self.config)
            .finish()
    }
}

impl SessionLogger {
    /// Create a new session logger
    pub fn new(session_id: Uuid, config: SessionLogConfig) -> Result<Self> {
        let log_dir = paths::session_log_dir(session_id);
        fs::create_dir_all(&log_dir).map_err(|e| CcmuxError::FileWrite {
            path: log_dir.clone(),
            source: e,
        })?;

        let level = std::env::var("CCMUX_SESSION_LOG")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(config.default_level);

        let system_log_path = log_dir.join("system.jsonl");
        let system_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&system_log_path)
            .map_err(|e| CcmuxError::FileWrite {
                path: system_log_path.clone(),
                source: e,
            })?;
        let system_size = system_file.metadata().map(|m| m.len()).unwrap_or(0);

        let (audit_log, audit_size) = if config.separate_audit_trail {
            let audit_log_path = log_dir.join("audit.jsonl");
            let audit_file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&audit_log_path)
                .map_err(|e| CcmuxError::FileWrite {
                    path: audit_log_path.clone(),
                    source: e,
                })?;
            let size = audit_file.metadata().map(|m| m.len()).unwrap_or(0);
            (Some(BufWriter::new(audit_file)), size)
        } else {
            (None, 0)
        };

        Ok(Self {
            session_id,
            log_dir,
            level: Mutex::new(level),
            config,
            system_log: Mutex::new(Some(BufWriter::new(system_file))),
            system_log_size: AtomicU64::new(system_size),
            audit_log: Mutex::new(audit_log),
            audit_log_size: AtomicU64::new(audit_size),
        })
    }

    /// Get the session ID
    pub fn session_id(&self) -> Uuid {
        self.session_id
    }

    /// Get the log directory path
    pub fn log_dir(&self) -> &PathBuf {
        &self.log_dir
    }

    /// Get the current log level
    pub fn level(&self) -> SessionLogLevel {
        *self.level.lock().unwrap()
    }

    /// Set the log level at runtime
    pub fn set_level(&self, level: SessionLogLevel) {
        *self.level.lock().unwrap() = level;
    }

    /// Check if an event type should be logged at the current level
    pub fn should_log(&self, event_type: LogEventType) -> bool {
        self.level() >= event_type.min_level()
    }

    /// Log an entry if it passes the level filter
    pub fn log(&self, entry: LogEntry) -> Result<()> {
        if !self.should_log(entry.event_type) {
            return Ok(());
        }

        let json = serde_json::to_string(&entry)
            .map_err(|e| CcmuxError::internal(format!("Failed to serialize log entry: {}", e)))?;
        let line = format!("{}\n", json);
        let line_bytes = line.as_bytes();

        // Determine which log file to use
        let is_audit = self.config.separate_audit_trail && Self::is_audit_event(entry.event_type);

        if is_audit {
            self.write_to_audit(line_bytes)?;
        } else {
            self.write_to_system(line_bytes)?;
        }

        Ok(())
    }

    /// Log a session lifecycle event
    pub fn log_lifecycle(&self, event_type: LogEventType) -> Result<()> {
        let entry = LogEntry::new(self.session_id, event_type);
        self.log(entry)
    }

    /// Log an event with window context
    pub fn log_window_event(&self, event_type: LogEventType, window_id: Uuid) -> Result<()> {
        let entry = LogEntry::new(self.session_id, event_type).with_window(window_id);
        self.log(entry)
    }

    /// Log an event with pane context
    pub fn log_pane_event(
        &self,
        event_type: LogEventType,
        window_id: Uuid,
        pane_id: Uuid,
    ) -> Result<()> {
        let entry = LogEntry::new(self.session_id, event_type)
            .with_window(window_id)
            .with_pane(pane_id);
        self.log(entry)
    }

    /// Log an event with payload
    pub fn log_with_payload(
        &self,
        event_type: LogEventType,
        payload: impl Serialize,
    ) -> Result<()> {
        let entry = LogEntry::new(self.session_id, event_type).with_payload(payload);
        self.log(entry)
    }

    /// Flush all log buffers
    pub fn flush(&self) -> Result<()> {
        if let Some(ref mut writer) = *self.system_log.lock().unwrap() {
            writer.flush().map_err(|e| CcmuxError::FileWrite {
                path: self.log_dir.join("system.jsonl"),
                source: e,
            })?;
        }

        if let Some(ref mut writer) = *self.audit_log.lock().unwrap() {
            writer.flush().map_err(|e| CcmuxError::FileWrite {
                path: self.log_dir.join("audit.jsonl"),
                source: e,
            })?;
        }

        Ok(())
    }

    /// Clean up old rotated log files based on retention policy
    pub fn cleanup_old_logs(&self) -> Result<()> {
        let now = SystemTime::now();
        let retention = Duration::from_secs(self.config.retention_secs);

        let entries = fs::read_dir(&self.log_dir).map_err(|e| CcmuxError::FileRead {
            path: self.log_dir.clone(),
            source: e,
        })?;

        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                // Only clean up rotated files (e.g., system.1.jsonl, audit.2.jsonl)
                if name.contains('.') && name.ends_with(".jsonl") {
                    let parts: Vec<&str> = name.split('.').collect();
                    if parts.len() == 3 && parts[1].parse::<u32>().is_ok() {
                        if let Ok(metadata) = path.metadata() {
                            if let Ok(modified) = metadata.modified() {
                                if let Ok(age) = now.duration_since(modified) {
                                    if age > retention {
                                        let _ = fs::remove_file(&path);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Check if an event type is considered an audit event (user action)
    fn is_audit_event(event_type: LogEventType) -> bool {
        matches!(
            event_type,
            LogEventType::Input
                | LogEventType::CommandStarted
                | LogEventType::ClientAttached
                | LogEventType::ClientDetached
        )
    }

    /// Write to the system log with rotation
    fn write_to_system(&self, data: &[u8]) -> Result<()> {
        let mut guard = self.system_log.lock().unwrap();

        // Check if rotation is needed
        let current_size = self.system_log_size.load(Ordering::Relaxed);
        if current_size + data.len() as u64 > self.config.max_file_size {
            self.rotate_log(&mut guard, "system.jsonl", &self.system_log_size)?;
        }

        if let Some(ref mut writer) = *guard {
            writer.write_all(data).map_err(|e| CcmuxError::FileWrite {
                path: self.log_dir.join("system.jsonl"),
                source: e,
            })?;
            self.system_log_size.fetch_add(data.len() as u64, Ordering::Relaxed);
        }

        Ok(())
    }

    /// Write to the audit log with rotation
    fn write_to_audit(&self, data: &[u8]) -> Result<()> {
        let mut guard = self.audit_log.lock().unwrap();

        // Check if rotation is needed
        let current_size = self.audit_log_size.load(Ordering::Relaxed);
        if current_size + data.len() as u64 > self.config.max_file_size {
            self.rotate_log(&mut guard, "audit.jsonl", &self.audit_log_size)?;
        }

        if let Some(ref mut writer) = *guard {
            writer.write_all(data).map_err(|e| CcmuxError::FileWrite {
                path: self.log_dir.join("audit.jsonl"),
                source: e,
            })?;
            self.audit_log_size.fetch_add(data.len() as u64, Ordering::Relaxed);
        }

        Ok(())
    }

    /// Rotate a log file
    fn rotate_log(
        &self,
        writer: &mut Option<BufWriter<File>>,
        filename: &str,
        size_counter: &AtomicU64,
    ) -> Result<()> {
        // Flush and close current file
        if let Some(ref mut w) = writer {
            let _ = w.flush();
        }
        *writer = None;

        let base_path = self.log_dir.join(filename);
        let stem = filename.trim_end_matches(".jsonl");

        // Shift existing rotated files
        for i in (1..self.config.max_rotated_files).rev() {
            let old_path = self.log_dir.join(format!("{}.{}.jsonl", stem, i));
            let new_path = self.log_dir.join(format!("{}.{}.jsonl", stem, i + 1));
            if old_path.exists() {
                let _ = fs::rename(&old_path, &new_path);
            }
        }

        // Remove oldest if at limit
        let oldest = self.log_dir.join(format!(
            "{}.{}.jsonl",
            stem,
            self.config.max_rotated_files
        ));
        if oldest.exists() {
            let _ = fs::remove_file(&oldest);
        }

        // Rotate current file to .1
        if base_path.exists() {
            let rotated = self.log_dir.join(format!("{}.1.jsonl", stem));
            fs::rename(&base_path, &rotated).map_err(|e| CcmuxError::FileWrite {
                path: rotated,
                source: e,
            })?;
        }

        // Open new file
        let new_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&base_path)
            .map_err(|e| CcmuxError::FileWrite {
                path: base_path,
                source: e,
            })?;

        *writer = Some(BufWriter::new(new_file));
        size_counter.store(0, Ordering::Relaxed);

        Ok(())
    }
}

impl Drop for SessionLogger {
    fn drop(&mut self) {
        let _ = self.flush();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use tempfile::TempDir;

    fn setup_test_logger() -> (TempDir, SessionLogger) {
        let temp_dir = TempDir::new().unwrap();
        let session_id = Uuid::new_v4();

        // Override paths for testing
        env::set_var("XDG_STATE_HOME", temp_dir.path());

        let config = SessionLogConfig {
            max_file_size: 1024, // Small for testing rotation
            max_rotated_files: 3,
            retention_secs: 1,
            ..Default::default()
        };

        let logger = SessionLogger::new(session_id, config).unwrap();
        (temp_dir, logger)
    }

    // ==================== SessionLogLevel Tests ====================

    #[test]
    fn test_session_log_level_default() {
        assert_eq!(SessionLogLevel::default(), SessionLogLevel::Signals);
    }

    #[test]
    fn test_session_log_level_ordering() {
        assert!(SessionLogLevel::Spawns < SessionLogLevel::Signals);
        assert!(SessionLogLevel::Signals < SessionLogLevel::Prompts);
        assert!(SessionLogLevel::Prompts < SessionLogLevel::Full);
    }

    #[test]
    fn test_session_log_level_display() {
        assert_eq!(format!("{}", SessionLogLevel::Spawns), "spawns");
        assert_eq!(format!("{}", SessionLogLevel::Signals), "signals");
        assert_eq!(format!("{}", SessionLogLevel::Prompts), "prompts");
        assert_eq!(format!("{}", SessionLogLevel::Full), "full");
    }

    #[test]
    fn test_session_log_level_from_str() {
        assert_eq!("spawns".parse::<SessionLogLevel>().unwrap(), SessionLogLevel::Spawns);
        assert_eq!("signals".parse::<SessionLogLevel>().unwrap(), SessionLogLevel::Signals);
        assert_eq!("prompts".parse::<SessionLogLevel>().unwrap(), SessionLogLevel::Prompts);
        assert_eq!("full".parse::<SessionLogLevel>().unwrap(), SessionLogLevel::Full);
        assert_eq!("FULL".parse::<SessionLogLevel>().unwrap(), SessionLogLevel::Full);
        assert!("invalid".parse::<SessionLogLevel>().is_err());
    }

    #[test]
    fn test_session_log_level_serialize() {
        let level = SessionLogLevel::Prompts;
        let json = serde_json::to_string(&level).unwrap();
        assert_eq!(json, "\"prompts\"");
    }

    #[test]
    fn test_session_log_level_deserialize() {
        let level: SessionLogLevel = serde_json::from_str("\"signals\"").unwrap();
        assert_eq!(level, SessionLogLevel::Signals);
    }

    // ==================== LogEventType Tests ====================

    #[test]
    fn test_log_event_type_min_level_spawns() {
        assert_eq!(LogEventType::SessionCreated.min_level(), SessionLogLevel::Spawns);
        assert_eq!(LogEventType::SessionTerminated.min_level(), SessionLogLevel::Spawns);
        assert_eq!(LogEventType::WindowCreated.min_level(), SessionLogLevel::Spawns);
        assert_eq!(LogEventType::WindowClosed.min_level(), SessionLogLevel::Spawns);
        assert_eq!(LogEventType::PaneCreated.min_level(), SessionLogLevel::Spawns);
        assert_eq!(LogEventType::PaneClosed.min_level(), SessionLogLevel::Spawns);
    }

    #[test]
    fn test_log_event_type_min_level_signals() {
        assert_eq!(LogEventType::ProcessStarted.min_level(), SessionLogLevel::Signals);
        assert_eq!(LogEventType::ProcessExited.min_level(), SessionLogLevel::Signals);
        assert_eq!(LogEventType::Error.min_level(), SessionLogLevel::Signals);
        assert_eq!(LogEventType::Warning.min_level(), SessionLogLevel::Signals);
        assert_eq!(LogEventType::ClientAttached.min_level(), SessionLogLevel::Signals);
        assert_eq!(LogEventType::ClientDetached.min_level(), SessionLogLevel::Signals);
    }

    #[test]
    fn test_log_event_type_min_level_prompts() {
        assert_eq!(LogEventType::PromptDetected.min_level(), SessionLogLevel::Prompts);
        assert_eq!(LogEventType::CommandStarted.min_level(), SessionLogLevel::Prompts);
    }

    #[test]
    fn test_log_event_type_min_level_full() {
        assert_eq!(LogEventType::Output.min_level(), SessionLogLevel::Full);
        assert_eq!(LogEventType::Input.min_level(), SessionLogLevel::Full);
    }

    // ==================== LogEntry Tests ====================

    #[test]
    fn test_log_entry_new() {
        let session_id = Uuid::new_v4();
        let entry = LogEntry::new(session_id, LogEventType::SessionCreated);

        assert_eq!(entry.session_id, session_id);
        assert_eq!(entry.event_type, LogEventType::SessionCreated);
        assert!(entry.window_id.is_none());
        assert!(entry.pane_id.is_none());
        assert!(entry.payload.is_none());
        assert!(!entry.timestamp.is_empty());
    }

    #[test]
    fn test_log_entry_with_window() {
        let session_id = Uuid::new_v4();
        let window_id = Uuid::new_v4();
        let entry = LogEntry::new(session_id, LogEventType::WindowCreated)
            .with_window(window_id);

        assert_eq!(entry.window_id, Some(window_id));
    }

    #[test]
    fn test_log_entry_with_pane() {
        let session_id = Uuid::new_v4();
        let pane_id = Uuid::new_v4();
        let entry = LogEntry::new(session_id, LogEventType::PaneCreated)
            .with_pane(pane_id);

        assert_eq!(entry.pane_id, Some(pane_id));
    }

    #[test]
    fn test_log_entry_with_payload() {
        let session_id = Uuid::new_v4();
        let entry = LogEntry::new(session_id, LogEventType::ProcessExited)
            .with_payload(serde_json::json!({"exit_code": 0}));

        assert!(entry.payload.is_some());
        let payload = entry.payload.unwrap();
        assert_eq!(payload["exit_code"], 0);
    }

    #[test]
    fn test_log_entry_timestamp_format() {
        let session_id = Uuid::new_v4();
        let entry = LogEntry::new(session_id, LogEventType::SessionCreated);

        // Check ISO 8601 format: YYYY-MM-DDTHH:MM:SS.mmmZ
        assert!(entry.timestamp.len() == 24);
        assert!(entry.timestamp.contains('T'));
        assert!(entry.timestamp.ends_with('Z'));
    }

    #[test]
    fn test_log_entry_serialize() {
        let session_id = Uuid::new_v4();
        let entry = LogEntry::new(session_id, LogEventType::SessionCreated);

        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains("session_created"));
        assert!(json.contains(&session_id.to_string()));
    }

    // ==================== SessionLogConfig Tests ====================

    #[test]
    fn test_session_log_config_default() {
        let config = SessionLogConfig::default();

        assert_eq!(config.default_level, SessionLogLevel::Signals);
        assert_eq!(config.max_file_size, 10 * 1024 * 1024);
        assert_eq!(config.max_rotated_files, 5);
        assert_eq!(config.retention_secs, 7 * 24 * 60 * 60);
        assert!(config.separate_audit_trail);
    }

    #[test]
    fn test_session_log_config_serialize() {
        let config = SessionLogConfig::default();
        let json = serde_json::to_string(&config).unwrap();

        assert!(json.contains("default_level"));
        assert!(json.contains("max_file_size"));
    }

    // ==================== SessionLogger Tests ====================

    #[test]
    fn test_session_logger_creation() {
        let (_temp_dir, logger) = setup_test_logger();

        assert!(logger.log_dir().exists());
        assert!(logger.log_dir().join("system.jsonl").exists());
        assert!(logger.log_dir().join("audit.jsonl").exists());
    }

    #[test]
    fn test_session_logger_level() {
        let (_temp_dir, logger) = setup_test_logger();

        assert_eq!(logger.level(), SessionLogLevel::Signals);

        logger.set_level(SessionLogLevel::Full);
        assert_eq!(logger.level(), SessionLogLevel::Full);
    }

    #[test]
    fn test_session_logger_should_log() {
        let (_temp_dir, logger) = setup_test_logger();

        // Default level is Signals
        assert!(logger.should_log(LogEventType::SessionCreated)); // Spawns level
        assert!(logger.should_log(LogEventType::ProcessExited));  // Signals level
        assert!(!logger.should_log(LogEventType::PromptDetected)); // Prompts level
        assert!(!logger.should_log(LogEventType::Output));         // Full level

        logger.set_level(SessionLogLevel::Full);
        assert!(logger.should_log(LogEventType::Output));
    }

    #[test]
    fn test_session_logger_log_lifecycle() {
        let (_temp_dir, logger) = setup_test_logger();

        logger.log_lifecycle(LogEventType::SessionCreated).unwrap();
        logger.flush().unwrap();

        let contents = fs::read_to_string(logger.log_dir().join("system.jsonl")).unwrap();
        assert!(contents.contains("session_created"));
    }

    #[test]
    fn test_session_logger_log_with_payload() {
        let (_temp_dir, logger) = setup_test_logger();

        logger.log_with_payload(
            LogEventType::ProcessExited,
            serde_json::json!({"exit_code": 0, "signal": null}),
        ).unwrap();
        logger.flush().unwrap();

        let contents = fs::read_to_string(logger.log_dir().join("system.jsonl")).unwrap();
        assert!(contents.contains("exit_code"));
    }

    #[test]
    fn test_session_logger_audit_separation() {
        let (_temp_dir, logger) = setup_test_logger();
        logger.set_level(SessionLogLevel::Full);

        // System event
        logger.log_lifecycle(LogEventType::SessionCreated).unwrap();
        // Audit event (user input)
        logger.log_with_payload(LogEventType::Input, "ls -la").unwrap();
        logger.flush().unwrap();

        let system = fs::read_to_string(logger.log_dir().join("system.jsonl")).unwrap();
        let audit = fs::read_to_string(logger.log_dir().join("audit.jsonl")).unwrap();

        assert!(system.contains("session_created"));
        assert!(!system.contains("input"));
        assert!(audit.contains("input"));
    }

    #[test]
    fn test_session_logger_rotation() {
        let (temp_dir, logger) = setup_test_logger();

        // Write enough to trigger rotation (config has 1024 byte limit)
        for i in 0..50 {
            logger.log_with_payload(
                LogEventType::ProcessExited,
                serde_json::json!({"iteration": i, "data": "x".repeat(100)}),
            ).unwrap();
        }
        logger.flush().unwrap();

        // Check that rotated files were created
        let rotated = temp_dir.path()
            .join("ccmux")
            .join("log")
            .join(logger.session_id().to_string())
            .join("system.1.jsonl");

        assert!(rotated.exists(), "Rotated log file should exist");
    }

    #[test]
    fn test_session_logger_debug() {
        let (_temp_dir, logger) = setup_test_logger();

        let debug = format!("{:?}", logger);
        assert!(debug.contains("SessionLogger"));
        assert!(debug.contains(&logger.session_id().to_string()));
    }

    // ==================== Integration Tests ====================

    #[test]
    fn test_full_logging_workflow() {
        let (_temp_dir, logger) = setup_test_logger();
        logger.set_level(SessionLogLevel::Full);

        let window_id = Uuid::new_v4();
        let pane_id = Uuid::new_v4();

        // Log session lifecycle
        logger.log_lifecycle(LogEventType::SessionCreated).unwrap();

        // Log window creation
        logger.log_window_event(LogEventType::WindowCreated, window_id).unwrap();

        // Log pane creation
        logger.log_pane_event(LogEventType::PaneCreated, window_id, pane_id).unwrap();

        // Log process start
        logger.log_pane_event(LogEventType::ProcessStarted, window_id, pane_id).unwrap();

        // Log some output
        logger.log_with_payload(LogEventType::Output, "$ hello world\n").unwrap();

        // Log input (goes to audit)
        logger.log_with_payload(LogEventType::Input, "echo hello").unwrap();

        logger.flush().unwrap();

        // Verify system log
        let system = fs::read_to_string(logger.log_dir().join("system.jsonl")).unwrap();
        let system_lines: Vec<&str> = system.lines().collect();
        assert!(system_lines.len() >= 4);

        // Verify audit log
        let audit = fs::read_to_string(logger.log_dir().join("audit.jsonl")).unwrap();
        assert!(audit.contains("echo hello"));
    }
}
