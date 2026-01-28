//! FEAT-125: Filesystem-based mail system for async agent communication
//!
//! Messages are stored in `.mail/{recipient}/` directories with YAML frontmatter.
//! See FEAT-124 for the storage format specification.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};

use crate::mcp::error::McpError;
use crate::mcp::protocol::ToolResult;

/// Mail message types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageType {
    Status,
    Alert,
    Task,
    Question,
    Response,
}

impl std::fmt::Display for MessageType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MessageType::Status => write!(f, "status"),
            MessageType::Alert => write!(f, "alert"),
            MessageType::Task => write!(f, "task"),
            MessageType::Question => write!(f, "question"),
            MessageType::Response => write!(f, "response"),
        }
    }
}

impl std::str::FromStr for MessageType {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "status" => Ok(MessageType::Status),
            "alert" => Ok(MessageType::Alert),
            "task" => Ok(MessageType::Task),
            "question" => Ok(MessageType::Question),
            "response" => Ok(MessageType::Response),
            _ => Err(format!("Unknown message type: {}", s)),
        }
    }
}

/// Mail priority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Priority {
    Urgent,
    #[default]
    Normal,
    Low,
}

impl std::fmt::Display for Priority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Priority::Urgent => write!(f, "urgent"),
            Priority::Normal => write!(f, "normal"),
            Priority::Low => write!(f, "low"),
        }
    }
}

impl std::str::FromStr for Priority {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "urgent" => Ok(Priority::Urgent),
            "normal" => Ok(Priority::Normal),
            "low" => Ok(Priority::Low),
            _ => Err(format!("Unknown priority: {}", s)),
        }
    }
}

/// Mail message metadata (YAML frontmatter)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageMetadata {
    pub from: String,
    pub to: String,
    #[serde(rename = "type")]
    pub msg_type: MessageType,
    pub timestamp: DateTime<Utc>,
    pub subject: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub needs_response: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub priority: Option<Priority>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub in_reply_to: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thread_id: Option<String>,
}

/// Full mail message with metadata and body
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MailMessage {
    #[serde(flatten)]
    pub metadata: MessageMetadata,
    pub body: String,
    pub filename: String,
}

/// Mail summary for check/list operations
#[derive(Debug, Clone, Serialize)]
pub struct MessageSummary {
    pub filename: String,
    pub from: String,
    #[serde(rename = "type")]
    pub msg_type: String,
    pub subject: String,
    pub priority: String,
    pub needs_response: bool,
    pub timestamp: String,
}

/// Resolves the mail directory path
fn get_mail_dir() -> PathBuf {
    // Use current working directory's .mail
    PathBuf::from(".mail")
}

/// Get the mailbox directory for a recipient
fn get_mailbox_path(recipient: &str) -> PathBuf {
    get_mail_dir().join(recipient)
}

/// Generate a filename for a new message
fn generate_filename(from: &str, msg_type: MessageType) -> String {
    let timestamp = Utc::now().format("%Y-%m-%dT%H-%M-%S");
    // Sanitize 'from' for filename (replace invalid chars)
    let safe_from: String = from
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '-' })
        .collect();
    format!("{}_{}_{}.md", timestamp, safe_from, msg_type)
}

/// Parse a mail message from a file
fn parse_message(path: &Path) -> Result<MailMessage, McpError> {
    let content = fs::read_to_string(path)
        .map_err(|e| McpError::Internal(format!("Failed to read message: {}", e)))?;

    // Split frontmatter and body
    let parts: Vec<&str> = content.splitn(3, "---").collect();
    if parts.len() < 3 {
        return Err(McpError::Internal("Invalid message format: missing frontmatter".into()));
    }

    let frontmatter = parts[1].trim();
    let body = parts[2].trim().to_string();

    let metadata: MessageMetadata = serde_yaml::from_str(frontmatter)
        .map_err(|e| McpError::Internal(format!("Failed to parse frontmatter: {}", e)))?;

    let filename = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_string();

    Ok(MailMessage {
        metadata,
        body,
        filename,
    })
}

/// Parse just the metadata from a file (faster for listings)
fn parse_metadata(path: &Path) -> Result<(MessageMetadata, String), McpError> {
    let content = fs::read_to_string(path)
        .map_err(|e| McpError::Internal(format!("Failed to read message: {}", e)))?;

    let parts: Vec<&str> = content.splitn(3, "---").collect();
    if parts.len() < 3 {
        return Err(McpError::Internal("Invalid message format".into()));
    }

    let frontmatter = parts[1].trim();
    let metadata: MessageMetadata = serde_yaml::from_str(frontmatter)
        .map_err(|e| McpError::Internal(format!("Failed to parse frontmatter: {}", e)))?;

    let filename = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_string();

    Ok((metadata, filename))
}

/// Send a mail message
pub fn mail_send(
    from: &str,
    to: &str,
    msg_type: &str,
    subject: &str,
    body: &str,
    needs_response: Option<bool>,
    priority: Option<&str>,
    tags: Vec<String>,
    in_reply_to: Option<&str>,
) -> Result<ToolResult, McpError> {
    let msg_type: MessageType = msg_type.parse()
        .map_err(|e: String| McpError::InvalidParams(e))?;

    let priority: Option<Priority> = priority.map(|p| p.parse())
        .transpose()
        .map_err(|e: String| McpError::InvalidParams(e))?;

    let timestamp = Utc::now();

    let metadata = MessageMetadata {
        from: from.to_string(),
        to: to.to_string(),
        msg_type,
        timestamp,
        subject: subject.to_string(),
        needs_response,
        priority,
        tags,
        in_reply_to: in_reply_to.map(String::from),
        thread_id: None,
    };

    // Generate filename and full message content
    let filename = generate_filename(from, msg_type);
    let frontmatter = serde_yaml::to_string(&metadata)
        .map_err(|e| McpError::Internal(format!("Failed to serialize metadata: {}", e)))?;

    let message_content = format!("---\n{}---\n\n{}", frontmatter, body);

    // Create mailbox directory if needed
    let mailbox_path = get_mailbox_path(to);
    fs::create_dir_all(&mailbox_path)
        .map_err(|e| McpError::Internal(format!("Failed to create mailbox directory: {}", e)))?;

    // Write atomically: write to temp file, then rename
    let final_path = mailbox_path.join(&filename);
    let temp_path = mailbox_path.join(format!(".{}.tmp", filename));

    {
        let mut file = fs::File::create(&temp_path)
            .map_err(|e| McpError::Internal(format!("Failed to create temp file: {}", e)))?;
        file.write_all(message_content.as_bytes())
            .map_err(|e| McpError::Internal(format!("Failed to write message: {}", e)))?;
        file.sync_all()
            .map_err(|e| McpError::Internal(format!("Failed to sync file: {}", e)))?;
    }

    fs::rename(&temp_path, &final_path)
        .map_err(|e| McpError::Internal(format!("Failed to finalize message: {}", e)))?;

    info!(to = %to, filename = %filename, "Mail sent successfully");

    let result = serde_json::json!({
        "success": true,
        "filename": filename,
        "mailbox": format!(".mail/{}/", to),
    });

    let json = serde_json::to_string_pretty(&result)
        .map_err(|e| McpError::Internal(e.to_string()))?;
    Ok(ToolResult::text(json))
}

/// Check mailbox for unread messages (summary only)
pub fn mail_check(
    mailbox: &str,
    type_filter: Option<&str>,
    priority_filter: Option<&str>,
    needs_response_filter: Option<bool>,
) -> Result<ToolResult, McpError> {
    let mailbox_path = get_mailbox_path(mailbox);

    if !mailbox_path.exists() {
        let result = serde_json::json!({
            "mailbox": mailbox,
            "unread_count": 0,
            "messages": [],
        });
        let json = serde_json::to_string_pretty(&result)
            .map_err(|e| McpError::Internal(e.to_string()))?;
        return Ok(ToolResult::text(json));
    }

    let mut messages = Vec::new();

    // Read all .md files in the mailbox (excluding subdirs like read/, archive/)
    let entries = fs::read_dir(&mailbox_path)
        .map_err(|e| McpError::Internal(format!("Failed to read mailbox: {}", e)))?;

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() && path.extension().map_or(false, |e| e == "md") {
            // Skip hidden files (temp files)
            if path.file_name().and_then(|n| n.to_str()).map_or(false, |n| n.starts_with('.')) {
                continue;
            }

            match parse_metadata(&path) {
                Ok((metadata, filename)) => {
                    // Apply filters
                    if let Some(type_f) = type_filter {
                        if metadata.msg_type.to_string() != type_f {
                            continue;
                        }
                    }
                    if let Some(priority_f) = priority_filter {
                        let msg_priority = metadata.priority.unwrap_or_default().to_string();
                        if msg_priority != priority_f {
                            continue;
                        }
                    }
                    if let Some(needs_resp) = needs_response_filter {
                        if metadata.needs_response.unwrap_or(false) != needs_resp {
                            continue;
                        }
                    }

                    messages.push(MessageSummary {
                        filename,
                        from: metadata.from,
                        msg_type: metadata.msg_type.to_string(),
                        subject: metadata.subject,
                        priority: metadata.priority.unwrap_or_default().to_string(),
                        needs_response: metadata.needs_response.unwrap_or(false),
                        timestamp: metadata.timestamp.to_rfc3339(),
                    });
                }
                Err(e) => {
                    warn!(path = ?path, error = %e, "Failed to parse message metadata");
                }
            }
        }
    }

    // Sort by timestamp (newest first)
    messages.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

    let result = serde_json::json!({
        "mailbox": mailbox,
        "unread_count": messages.len(),
        "messages": messages,
    });

    let json = serde_json::to_string_pretty(&result)
        .map_err(|e| McpError::Internal(e.to_string()))?;
    Ok(ToolResult::text(json))
}

/// Read a specific message
pub fn mail_read(
    mailbox: &str,
    filename: &str,
    mark_read: bool,
) -> Result<ToolResult, McpError> {
    let mailbox_path = get_mailbox_path(mailbox);
    let message_path = mailbox_path.join(filename);

    // Also check read/ subdirectory
    let read_path = mailbox_path.join("read").join(filename);

    let actual_path = if message_path.exists() {
        message_path.clone()
    } else if read_path.exists() {
        read_path
    } else {
        return Err(McpError::InvalidParams(format!(
            "Message '{}' not found in mailbox '{}'",
            filename, mailbox
        )));
    };

    let message = parse_message(&actual_path)?;

    // Move to read/ if requested and not already there
    if mark_read && actual_path == message_path {
        let read_dir = mailbox_path.join("read");
        fs::create_dir_all(&read_dir)
            .map_err(|e| McpError::Internal(format!("Failed to create read directory: {}", e)))?;

        let dest = read_dir.join(filename);
        fs::rename(&message_path, &dest)
            .map_err(|e| McpError::Internal(format!("Failed to move message to read/: {}", e)))?;

        debug!(filename = %filename, "Marked message as read");
    }

    let result = serde_json::json!({
        "filename": message.filename,
        "from": message.metadata.from,
        "to": message.metadata.to,
        "type": message.metadata.msg_type.to_string(),
        "timestamp": message.metadata.timestamp.to_rfc3339(),
        "subject": message.metadata.subject,
        "needs_response": message.metadata.needs_response.unwrap_or(false),
        "priority": message.metadata.priority.unwrap_or_default().to_string(),
        "tags": message.metadata.tags,
        "in_reply_to": message.metadata.in_reply_to,
        "body": message.body,
    });

    let json = serde_json::to_string_pretty(&result)
        .map_err(|e| McpError::Internal(e.to_string()))?;
    Ok(ToolResult::text(json))
}

/// List messages in a mailbox with optional filters
pub fn mail_list(
    mailbox: &str,
    include_read: bool,
    from_filter: Option<&str>,
    type_filter: Option<&str>,
    since: Option<&str>,
    limit: usize,
) -> Result<ToolResult, McpError> {
    let mailbox_path = get_mailbox_path(mailbox);
    let mut messages = Vec::new();

    // Parse 'since' timestamp if provided
    let since_time: Option<DateTime<Utc>> = since
        .map(|s| {
            chrono::DateTime::parse_from_rfc3339(s)
                .map(|dt: chrono::DateTime<chrono::FixedOffset>| dt.with_timezone(&Utc))
                .map_err(|e| McpError::InvalidParams(format!("Invalid timestamp: {}", e)))
        })
        .transpose()?;

    // Collect messages from main directory
    if mailbox_path.exists() {
        collect_messages_from_dir(&mailbox_path, &mut messages, from_filter, type_filter, since_time.as_ref())?;
    }

    // Collect from read/ if requested
    if include_read {
        let read_path = mailbox_path.join("read");
        if read_path.exists() {
            collect_messages_from_dir(&read_path, &mut messages, from_filter, type_filter, since_time.as_ref())?;
        }
    }

    // Sort by timestamp (newest first) and limit
    messages.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
    messages.truncate(limit);

    let result = serde_json::json!({
        "mailbox": mailbox,
        "total_count": messages.len(),
        "include_read": include_read,
        "messages": messages,
    });

    let json = serde_json::to_string_pretty(&result)
        .map_err(|e| McpError::Internal(e.to_string()))?;
    Ok(ToolResult::text(json))
}

fn collect_messages_from_dir(
    dir: &Path,
    messages: &mut Vec<MessageSummary>,
    from_filter: Option<&str>,
    type_filter: Option<&str>,
    since: Option<&DateTime<Utc>>,
) -> Result<(), McpError> {
    let entries = fs::read_dir(dir)
        .map_err(|e| McpError::Internal(format!("Failed to read directory: {}", e)))?;

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() && path.extension().map_or(false, |e| e == "md") {
            // Skip hidden files
            if path.file_name().and_then(|n| n.to_str()).map_or(false, |n| n.starts_with('.')) {
                continue;
            }

            match parse_metadata(&path) {
                Ok((metadata, filename)) => {
                    // Apply filters
                    if let Some(from_f) = from_filter {
                        if metadata.from != from_f {
                            continue;
                        }
                    }
                    if let Some(type_f) = type_filter {
                        if metadata.msg_type.to_string() != type_f {
                            continue;
                        }
                    }
                    if let Some(since_time) = since {
                        if metadata.timestamp < *since_time {
                            continue;
                        }
                    }

                    messages.push(MessageSummary {
                        filename,
                        from: metadata.from,
                        msg_type: metadata.msg_type.to_string(),
                        subject: metadata.subject,
                        priority: metadata.priority.unwrap_or_default().to_string(),
                        needs_response: metadata.needs_response.unwrap_or(false),
                        timestamp: metadata.timestamp.to_rfc3339(),
                    });
                }
                Err(e) => {
                    warn!(path = ?path, error = %e, "Failed to parse message");
                }
            }
        }
    }

    Ok(())
}

/// Delete or archive a message
pub fn mail_delete(
    mailbox: &str,
    filename: &str,
    archive: bool,
) -> Result<ToolResult, McpError> {
    let mailbox_path = get_mailbox_path(mailbox);

    // Check both unread and read directories
    let unread_path = mailbox_path.join(filename);
    let read_path = mailbox_path.join("read").join(filename);

    let source_path = if unread_path.exists() {
        unread_path
    } else if read_path.exists() {
        read_path
    } else {
        return Err(McpError::InvalidParams(format!(
            "Message '{}' not found in mailbox '{}'",
            filename, mailbox
        )));
    };

    if archive {
        // Move to archive/
        let archive_dir = mailbox_path.join("archive");
        fs::create_dir_all(&archive_dir)
            .map_err(|e| McpError::Internal(format!("Failed to create archive directory: {}", e)))?;

        let dest = archive_dir.join(filename);
        fs::rename(&source_path, &dest)
            .map_err(|e| McpError::Internal(format!("Failed to archive message: {}", e)))?;

        info!(filename = %filename, mailbox = %mailbox, "Message archived");

        let result = serde_json::json!({
            "success": true,
            "action": "archived",
            "mailbox": mailbox,
            "filename": filename,
            "archive_path": format!(".mail/{}/archive/{}", mailbox, filename),
        });

        let json = serde_json::to_string_pretty(&result)
            .map_err(|e| McpError::Internal(e.to_string()))?;
        Ok(ToolResult::text(json))
    } else {
        // Permanently delete
        fs::remove_file(&source_path)
            .map_err(|e| McpError::Internal(format!("Failed to delete message: {}", e)))?;

        info!(filename = %filename, mailbox = %mailbox, "Message deleted permanently");

        let result = serde_json::json!({
            "success": true,
            "action": "deleted",
            "mailbox": mailbox,
            "filename": filename,
        });

        let json = serde_json::to_string_pretty(&result)
            .map_err(|e| McpError::Internal(e.to_string()))?;
        Ok(ToolResult::text(json))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_filename() {
        let filename = generate_filename("worker-bug-069", MessageType::Status);
        assert!(filename.contains("worker-bug-069"));
        assert!(filename.contains("status"));
        assert!(filename.ends_with(".md"));
    }

    #[test]
    fn test_message_type_parsing() {
        assert_eq!("status".parse::<MessageType>().unwrap(), MessageType::Status);
        assert_eq!("alert".parse::<MessageType>().unwrap(), MessageType::Alert);
        assert_eq!("task".parse::<MessageType>().unwrap(), MessageType::Task);
        assert!("invalid".parse::<MessageType>().is_err());
    }

    #[test]
    fn test_priority_parsing() {
        assert_eq!("urgent".parse::<Priority>().unwrap(), Priority::Urgent);
        assert_eq!("normal".parse::<Priority>().unwrap(), Priority::Normal);
        assert_eq!("low".parse::<Priority>().unwrap(), Priority::Low);
        assert!("invalid".parse::<Priority>().is_err());
    }

    #[test]
    fn test_message_metadata_serialization() {
        let metadata = MessageMetadata {
            from: "worker-test".to_string(),
            to: "orchestrator".to_string(),
            msg_type: MessageType::Status,
            timestamp: Utc::now(),
            subject: "Test Subject".to_string(),
            needs_response: Some(false),
            priority: Some(Priority::Normal),
            tags: vec!["test".to_string()],
            in_reply_to: None,
            thread_id: None,
        };

        // Test serialization round-trip
        let yaml = serde_yaml::to_string(&metadata).unwrap();
        assert!(yaml.contains("from: worker-test"));
        assert!(yaml.contains("to: orchestrator"));
        assert!(yaml.contains("type: status"));

        let parsed: MessageMetadata = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(parsed.from, "worker-test");
        assert_eq!(parsed.to, "orchestrator");
        assert_eq!(parsed.msg_type, MessageType::Status);
    }
}
