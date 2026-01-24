use super::common::JsonValue;
use serde::{Deserialize, Serialize};

// ==================== Generic Widget System (FEAT-083) ====================

/// A generic widget for displaying data in the TUI (FEAT-083)
///
/// Widgets are agent-agnostic data containers that replace hardcoded types like BeadsTask.
/// This follows the "dumb pipe" strategy from ADR-001, allowing any external system
/// to push data through fugue without requiring protocol changes.
///
/// # Examples
/// ```rust
/// use fugue_protocol::types::widget::Widget;
/// use serde_json::json;
///
/// let widget = Widget::new("beads.task", json!({"id": "BUG-042", "title": "Fix login"}))
///     .with_priority(1)
///     .with_expires_at(1704067200);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Widget {
    /// Type identifier for the widget (e.g., "beads.task", "progress.bar")
    pub widget_type: String,
    /// Arbitrary JSON payload - structure defined by the widget type
    pub data: JsonValue,
    /// Optional ordering hint (lower = higher priority)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub priority: Option<i32>,
    /// Optional expiration timestamp (Unix timestamp)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<u64>,
}

impl Widget {
    /// Create a new widget with the given type and data
    pub fn new(widget_type: impl Into<String>, data: serde_json::Value) -> Self {
        Self {
            widget_type: widget_type.into(),
            data: JsonValue::new(data),
            priority: None,
            expires_at: None,
        }
    }

    /// Set the priority for this widget (builder pattern)
    pub fn with_priority(mut self, priority: i32) -> Self {
        self.priority = Some(priority);
        self
    }

    /// Set the expiration timestamp for this widget (builder pattern)
    pub fn with_expires_at(mut self, expires_at: u64) -> Self {
        self.expires_at = Some(expires_at);
        self
    }

    /// Get the data as a serde_json::Value reference
    pub fn data(&self) -> &serde_json::Value {
        self.data.inner()
    }
}

/// A widget update containing metadata and a collection of widgets (FEAT-083)
///
/// This is the response type for widget queries, containing status information
/// about the data source and the actual widget items.
///
/// # Examples
/// ```rust
/// use fugue_protocol::types::widget::{Widget, WidgetUpdate};
/// use serde_json::json;
///
/// let update = WidgetUpdate::new("beads.status", json!({"daemon_available": true}))
///     .with_widgets(vec![
///         Widget::new("beads.task", json!({"id": "BUG-042"})),
///     ]);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WidgetUpdate {
    /// Type identifier for this update (e.g., "beads.status", "progress.status")
    pub update_type: String,
    /// Metadata about the data source (e.g., daemon_available, last_refresh)
    pub metadata: JsonValue,
    /// The widget items in this update
    #[serde(default)]
    pub widgets: Vec<Widget>,
}

impl WidgetUpdate {
    /// Create a new widget update with the given type and metadata
    pub fn new(update_type: impl Into<String>, metadata: serde_json::Value) -> Self {
        Self {
            update_type: update_type.into(),
            metadata: JsonValue::new(metadata),
            widgets: Vec::new(),
        }
    }

    /// Set the widgets for this update (builder pattern)
    pub fn with_widgets(mut self, widgets: Vec<Widget>) -> Self {
        self.widgets = widgets;
        self
    }

    /// Add a single widget to this update
    pub fn add_widget(&mut self, widget: Widget) {
        self.widgets.push(widget);
    }

    /// Get the metadata as a serde_json::Value reference
    pub fn metadata(&self) -> &serde_json::Value {
        self.metadata.inner()
    }

    /// Check if this update has any widgets
    pub fn is_empty(&self) -> bool {
        self.widgets.is_empty()
    }

    /// Get the number of widgets in this update
    pub fn len(&self) -> usize {
        self.widgets.len()
    }
}

impl Default for WidgetUpdate {
    fn default() -> Self {
        Self {
            update_type: String::new(),
            metadata: JsonValue::new(serde_json::Value::Null),
            widgets: Vec::new(),
        }
    }
}

// ==================== Beads Query Types (FEAT-058) ====================

/// A task from the beads daemon work queue
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BeadsTask {
    /// Task ID (e.g., "BUG-042", "FEAT-015")
    pub id: String,
    /// Task title/summary
    pub title: String,
    /// Priority level (0 = highest, higher = lower priority)
    pub priority: i32,
    /// Current status (e.g., "open", "in_progress")
    pub status: String,
    /// Issue type (e.g., "bug", "feature")
    pub issue_type: String,
    /// Assigned user (if any)
    pub assignee: Option<String>,
    /// Labels attached to the task
    #[serde(default)]
    pub labels: Vec<String>,
}

impl BeadsTask {
    /// Check if this task has a specific label
    pub fn has_label(&self, label: &str) -> bool {
        self.labels.iter().any(|l| l.eq_ignore_ascii_case(label))
    }

    /// Get a short display string for the task
    pub fn short_display(&self) -> String {
        format!("{} P{} {}", self.id, self.priority, self.title)
    }
}

/// Beads daemon status for a pane's repository
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct BeadsStatus {
    /// Whether the daemon is available and responding
    pub daemon_available: bool,
    /// Number of ready tasks (no blockers)
    pub ready_count: usize,
    /// Ready tasks (may be limited/summarized)
    #[serde(default)]
    pub ready_tasks: Vec<BeadsTask>,
    /// Unix timestamp of last successful refresh
    pub last_refresh: Option<u64>,
    /// Error message if daemon unavailable
    pub error: Option<String>,
}

impl BeadsStatus {
    /// Create a status indicating daemon is unavailable
    pub fn unavailable() -> Self {
        Self {
            daemon_available: false,
            ready_count: 0,
            ready_tasks: Vec::new(),
            last_refresh: None,
            error: None,
        }
    }

    /// Create a status with an error message
    pub fn with_error(error: impl Into<String>) -> Self {
        Self {
            daemon_available: false,
            ready_count: 0,
            ready_tasks: Vec::new(),
            last_refresh: None,
            error: Some(error.into()),
        }
    }

    /// Create a successful status with tasks
    pub fn with_tasks(tasks: Vec<BeadsTask>, timestamp: u64) -> Self {
        let ready_count = tasks.len();
        Self {
            daemon_available: true,
            ready_count,
            ready_tasks: tasks,
            last_refresh: Some(timestamp),
            error: None,
        }
    }
}

// ==================== Widget Conversions (FEAT-083) ====================

/// Error type for widget conversion failures
#[derive(Debug, Clone, PartialEq)]
pub struct WidgetConversionError {
    pub message: String,
}

impl std::fmt::Display for WidgetConversionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Widget conversion error: {}", self.message)
    }
}

impl std::error::Error for WidgetConversionError {}

impl WidgetConversionError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

/// Convert BeadsTask to Widget
impl From<BeadsTask> for Widget {
    fn from(task: BeadsTask) -> Self {
        // Serialize BeadsTask as JSON for the widget data
        let data = serde_json::json!({
            "id": task.id,
            "title": task.title,
            "priority": task.priority,
            "status": task.status,
            "issue_type": task.issue_type,
            "assignee": task.assignee,
            "labels": task.labels,
        });

        Widget {
            widget_type: "beads.task".to_string(),
            data: JsonValue::new(data),
            priority: Some(task.priority),
            expires_at: None,
        }
    }
}

/// Convert Widget to BeadsTask
impl TryFrom<Widget> for BeadsTask {
    type Error = WidgetConversionError;

    fn try_from(widget: Widget) -> Result<Self, Self::Error> {
        if widget.widget_type != "beads.task" {
            return Err(WidgetConversionError::new(format!(
                "Expected widget_type 'beads.task', got '{}'",
                widget.widget_type
            )));
        }

        let data = widget.data.inner();

        let id = data["id"]
            .as_str()
            .ok_or_else(|| WidgetConversionError::new("Missing or invalid 'id' field"))?
            .to_string();

        let title = data["title"]
            .as_str()
            .ok_or_else(|| WidgetConversionError::new("Missing or invalid 'title' field"))?
            .to_string();

        let priority = data["priority"]
            .as_i64()
            .ok_or_else(|| WidgetConversionError::new("Missing or invalid 'priority' field"))?
            as i32;

        let status = data["status"]
            .as_str()
            .ok_or_else(|| WidgetConversionError::new("Missing or invalid 'status' field"))?
            .to_string();

        let issue_type = data["issue_type"]
            .as_str()
            .ok_or_else(|| WidgetConversionError::new("Missing or invalid 'issue_type' field"))?
            .to_string();

        let assignee = data["assignee"].as_str().map(|s| s.to_string());

        let labels = data["labels"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        Ok(BeadsTask {
            id,
            title,
            priority,
            status,
            issue_type,
            assignee,
            labels,
        })
    }
}

/// Convert BeadsStatus to WidgetUpdate
impl From<BeadsStatus> for WidgetUpdate {
    fn from(status: BeadsStatus) -> Self {
        let metadata = serde_json::json!({
            "daemon_available": status.daemon_available,
            "ready_count": status.ready_count,
            "last_refresh": status.last_refresh,
            "error": status.error,
        });

        let widgets: Vec<Widget> = status
            .ready_tasks
            .into_iter()
            .map(Widget::from)
            .collect();

        WidgetUpdate {
            update_type: "beads.status".to_string(),
            metadata: JsonValue::new(metadata),
            widgets,
        }
    }
}

/// Convert WidgetUpdate to BeadsStatus
impl TryFrom<WidgetUpdate> for BeadsStatus {
    type Error = WidgetConversionError;

    fn try_from(update: WidgetUpdate) -> Result<Self, Self::Error> {
        if update.update_type != "beads.status" {
            return Err(WidgetConversionError::new(format!(
                "Expected update_type 'beads.status', got '{}'",
                update.update_type
            )));
        }

        let metadata = update.metadata.inner();

        let daemon_available = metadata["daemon_available"]
            .as_bool()
            .ok_or_else(|| WidgetConversionError::new("Missing or invalid 'daemon_available' field"))?;

        let ready_count = metadata["ready_count"]
            .as_u64()
            .ok_or_else(|| WidgetConversionError::new("Missing or invalid 'ready_count' field"))?
            as usize;

        let last_refresh = metadata["last_refresh"].as_u64();

        let error = metadata["error"].as_str().map(|s| s.to_string());

        // Convert widgets back to BeadsTask
        // Note: We silently skip widgets that fail conversion to avoid
        // failing the entire status conversion due to one bad widget
        let ready_tasks: Vec<BeadsTask> = update
            .widgets
            .into_iter()
            .filter_map(|widget| BeadsTask::try_from(widget).ok())
            .collect();

        Ok(BeadsStatus {
            daemon_available,
            ready_count,
            ready_tasks,
            last_refresh,
            error,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== BeadsTask Tests (FEAT-058) ====================

    #[test]
    fn test_beads_task_creation() {
        let task = BeadsTask {
            id: "BUG-042".to_string(),
            title: "Fix login timeout".to_string(),
            priority: 1,
            status: "open".to_string(),
            issue_type: "bug".to_string(),
            assignee: Some("alice@example.com".to_string()),
            labels: vec!["auth".to_string(), "urgent".to_string()],
        };

        assert_eq!(task.id, "BUG-042");
        assert_eq!(task.priority, 1);
        assert!(task.assignee.is_some());
        assert_eq!(task.labels.len(), 2);
    }

    #[test]
    fn test_beads_task_has_label() {
        let task = BeadsTask {
            id: "FEAT-015".to_string(),
            title: "Add dark mode".to_string(),
            priority: 2,
            status: "open".to_string(),
            issue_type: "feature".to_string(),
            assignee: None,
            labels: vec!["UI".to_string(), "enhancement".to_string()],
        };

        assert!(task.has_label("ui")); // Case insensitive
        assert!(task.has_label("UI"));
        assert!(task.has_label("enhancement"));
        assert!(!task.has_label("bug"));
    }

    #[test]
    fn test_beads_task_short_display() {
        let task = BeadsTask {
            id: "BUG-042".to_string(),
            title: "Fix login timeout".to_string(),
            priority: 1,
            status: "open".to_string(),
            issue_type: "bug".to_string(),
            assignee: None,
            labels: vec![],
        };

        let display = task.short_display();
        assert!(display.contains("BUG-042"));
        assert!(display.contains("P1"));
        assert!(display.contains("Fix login timeout"));
    }

    #[test]
    fn test_beads_task_clone() {
        let task = BeadsTask {
            id: "TEST-001".to_string(),
            title: "Test task".to_string(),
            priority: 0,
            status: "open".to_string(),
            issue_type: "test".to_string(),
            assignee: Some("bob".to_string()),
            labels: vec!["test".to_string()],
        };

        let cloned = task.clone();
        assert_eq!(task, cloned);
    }

    #[test]
    fn test_beads_task_serde() {
        let task = BeadsTask {
            id: "FEAT-100".to_string(),
            title: "New feature".to_string(),
            priority: 2,
            status: "in_progress".to_string(),
            issue_type: "feature".to_string(),
            assignee: Some("dev@example.com".to_string()),
            labels: vec!["backend".to_string()],
        };

        let serialized = bincode::serialize(&task).unwrap();
        let deserialized: BeadsTask = bincode::deserialize(&serialized).unwrap();
        assert_eq!(task, deserialized);
    }

    // ==================== BeadsStatus Tests (FEAT-058) ====================

    #[test]
    fn test_beads_status_default() {
        let status = BeadsStatus::default();

        assert!(!status.daemon_available);
        assert_eq!(status.ready_count, 0);
        assert!(status.ready_tasks.is_empty());
        assert!(status.last_refresh.is_none());
        assert!(status.error.is_none());
    }

    #[test]
    fn test_beads_status_unavailable() {
        let status = BeadsStatus::unavailable();

        assert!(!status.daemon_available);
        assert_eq!(status.ready_count, 0);
        assert!(status.error.is_none());
    }

    #[test]
    fn test_beads_status_with_error() {
        let status = BeadsStatus::with_error("Connection refused");

        assert!(!status.daemon_available);
        assert_eq!(status.error, Some("Connection refused".to_string()));
    }

    #[test]
    fn test_beads_status_with_tasks() {
        let tasks = vec![
            BeadsTask {
                id: "BUG-001".to_string(),
                title: "First bug".to_string(),
                priority: 1,
                status: "open".to_string(),
                issue_type: "bug".to_string(),
                assignee: None,
                labels: vec![],
            },
            BeadsTask {
                id: "FEAT-002".to_string(),
                title: "Second feature".to_string(),
                priority: 2,
                status: "open".to_string(),
                issue_type: "feature".to_string(),
                assignee: None,
                labels: vec![],
            },
        ];

        let status = BeadsStatus::with_tasks(tasks.clone(), 1704067200);

        assert!(status.daemon_available);
        assert_eq!(status.ready_count, 2);
        assert_eq!(status.ready_tasks.len(), 2);
        assert_eq!(status.last_refresh, Some(1704067200));
        assert!(status.error.is_none());
    }

    #[test]
    fn test_beads_status_clone() {
        let status = BeadsStatus::with_tasks(vec![], 1234567890);
        let cloned = status.clone();
        assert_eq!(status, cloned);
    }

    #[test]
    fn test_beads_status_serde() {
        let status = BeadsStatus {
            daemon_available: true,
            ready_count: 5,
            ready_tasks: vec![BeadsTask {
                id: "TEST-001".to_string(),
                title: "Test".to_string(),
                priority: 1,
                status: "open".to_string(),
                issue_type: "test".to_string(),
                assignee: None,
                labels: vec![],
            }],
            last_refresh: Some(1704067200),
            error: None,
        };

        let serialized = bincode::serialize(&status).unwrap();
        let deserialized: BeadsStatus = bincode::deserialize(&serialized).unwrap();
        assert_eq!(status, deserialized);
    }

    #[test]
    fn test_beads_status_equality() {
        let status1 = BeadsStatus::unavailable();
        let status2 = BeadsStatus::unavailable();
        let status3 = BeadsStatus::with_error("error");

        assert_eq!(status1, status2);
        assert_ne!(status1, status3);
    }

    // ==================== Widget Tests (FEAT-083) ====================

    #[test]
    fn test_widget_new() {
        let widget = Widget::new("beads.task", serde_json::json!({"id": "BUG-042"}));

        assert_eq!(widget.widget_type, "beads.task");
        assert_eq!(widget.data()["id"], "BUG-042");
        assert!(widget.priority.is_none());
        assert!(widget.expires_at.is_none());
    }

    #[test]
    fn test_widget_builder() {
        let widget = Widget::new("progress.bar", serde_json::json!({"percent": 50}))
            .with_priority(1)
            .with_expires_at(1704067200);

        assert_eq!(widget.widget_type, "progress.bar");
        assert_eq!(widget.priority, Some(1));
        assert_eq!(widget.expires_at, Some(1704067200));
    }

    #[test]
    fn test_widget_clone() {
        let widget = Widget::new("test", serde_json::json!({})).with_priority(5);
        let cloned = widget.clone();
        assert_eq!(widget, cloned);
    }

    #[test]
    fn test_widget_serde() {
        let widget = Widget::new("beads.task", serde_json::json!({"id": "TEST-001"}))
            .with_priority(2)
            .with_expires_at(1000);

        let serialized = bincode::serialize(&widget).unwrap();
        let deserialized: Widget = bincode::deserialize(&serialized).unwrap();
        assert_eq!(widget, deserialized);
    }

    #[test]
    fn test_widget_update_new() {
        let update = WidgetUpdate::new("beads.status", serde_json::json!({"daemon_available": true}));

        assert_eq!(update.update_type, "beads.status");
        assert_eq!(update.metadata()["daemon_available"], true);
        assert!(update.widgets.is_empty());
        assert!(update.is_empty());
        assert_eq!(update.len(), 0);
    }

    #[test]
    fn test_widget_update_with_widgets() {
        let update = WidgetUpdate::new("beads.status", serde_json::json!({}))
            .with_widgets(vec![
                Widget::new("beads.task", serde_json::json!({"id": "BUG-001"})),
                Widget::new("beads.task", serde_json::json!({"id": "BUG-002"})),
            ]);

        assert!(!update.is_empty());
        assert_eq!(update.len(), 2);
    }

    #[test]
    fn test_widget_update_add_widget() {
        let mut update = WidgetUpdate::new("test", serde_json::json!({}));
        assert!(update.is_empty());

        update.add_widget(Widget::new("item", serde_json::json!({})));
        assert_eq!(update.len(), 1);

        update.add_widget(Widget::new("item", serde_json::json!({})));
        assert_eq!(update.len(), 2);
    }

    #[test]
    fn test_widget_update_default() {
        let update = WidgetUpdate::default();

        assert!(update.update_type.is_empty());
        assert!(update.metadata().is_null());
        assert!(update.widgets.is_empty());
    }

    #[test]
    fn test_widget_update_clone() {
        let update = WidgetUpdate::new("test", serde_json::json!({"key": "value"}))
            .with_widgets(vec![Widget::new("item", serde_json::json!({}))]);

        let cloned = update.clone();
        assert_eq!(update, cloned);
    }

    #[test]
    fn test_widget_update_serde() {
        // Test with empty widgets first (simpler case)
        let update_empty = WidgetUpdate::new("beads.status", serde_json::json!({"daemon_available": true}));
        let serialized_empty = bincode::serialize(&update_empty).unwrap();
        let deserialized_empty: WidgetUpdate = bincode::deserialize(&serialized_empty).unwrap();
        assert_eq!(update_empty, deserialized_empty);

        // Test with widgets using JSON serialization (more compatible with nested JsonValue)
        let update = WidgetUpdate::new("beads.status", serde_json::json!({"daemon_available": true}))
            .with_widgets(vec![
                Widget::new("beads.task", serde_json::json!({"id": "BUG-001"})).with_priority(1),
            ]);

        // Use JSON roundtrip for complex nested structures
        let json_serialized = serde_json::to_string(&update).unwrap();
        let json_deserialized: WidgetUpdate = serde_json::from_str(&json_serialized).unwrap();
        assert_eq!(update.update_type, json_deserialized.update_type);
        assert_eq!(update.widgets.len(), json_deserialized.widgets.len());
        assert_eq!(update.widgets[0].widget_type, json_deserialized.widgets[0].widget_type);
    }

    // ==================== Widget Conversion Tests (FEAT-083) ====================

    #[test]
    fn test_beads_task_to_widget() {
        let task = BeadsTask {
            id: "BUG-042".to_string(),
            title: "Fix login timeout".to_string(),
            priority: 1,
            status: "open".to_string(),
            issue_type: "bug".to_string(),
            assignee: Some("alice".to_string()),
            labels: vec!["auth".to_string()],
        };

        let widget: Widget = task.into();

        assert_eq!(widget.widget_type, "beads.task");
        assert_eq!(widget.priority, Some(1));
        assert_eq!(widget.data()["id"], "BUG-042");
        assert_eq!(widget.data()["title"], "Fix login timeout");
        assert_eq!(widget.data()["status"], "open");
        assert_eq!(widget.data()["issue_type"], "bug");
        assert_eq!(widget.data()["assignee"], "alice");
    }

    #[test]
    fn test_widget_to_beads_task() {
        let widget = Widget::new(
            "beads.task",
            serde_json::json!({
                "id": "FEAT-015",
                "title": "Add dark mode",
                "priority": 2,
                "status": "open",
                "issue_type": "feature",
                "assignee": null,
                "labels": ["ui", "enhancement"]
            }),
        );

        let task: BeadsTask = widget.try_into().unwrap();

        assert_eq!(task.id, "FEAT-015");
        assert_eq!(task.title, "Add dark mode");
        assert_eq!(task.priority, 2);
        assert_eq!(task.status, "open");
        assert_eq!(task.issue_type, "feature");
        assert!(task.assignee.is_none());
        assert_eq!(task.labels, vec!["ui", "enhancement"]);
    }

    #[test]
    fn test_widget_to_beads_task_wrong_type() {
        let widget = Widget::new("progress.bar", serde_json::json!({"percent": 50}));

        let result: Result<BeadsTask, _> = widget.try_into();
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("Expected widget_type 'beads.task'"));
    }

    #[test]
    fn test_widget_to_beads_task_missing_field() {
        let widget = Widget::new(
            "beads.task",
            serde_json::json!({
                "id": "BUG-001"
                // Missing title, priority, etc.
            }),
        );

        let result: Result<BeadsTask, _> = widget.try_into();
        assert!(result.is_err());
    }

    #[test]
    fn test_beads_task_widget_roundtrip() {
        let original = BeadsTask {
            id: "TEST-001".to_string(),
            title: "Test roundtrip".to_string(),
            priority: 3,
            status: "in_progress".to_string(),
            issue_type: "test".to_string(),
            assignee: Some("bob".to_string()),
            labels: vec!["roundtrip".to_string(), "test".to_string()],
        };

        let widget: Widget = original.clone().into();
        let recovered: BeadsTask = widget.try_into().unwrap();

        assert_eq!(original, recovered);
    }

    #[test]
    fn test_beads_status_to_widget_update() {
        let status = BeadsStatus::with_tasks(
            vec![BeadsTask {
                id: "BUG-001".to_string(),
                title: "First bug".to_string(),
                priority: 1,
                status: "open".to_string(),
                issue_type: "bug".to_string(),
                assignee: None,
                labels: vec![],
            }],
            1704067200,
        );

        let update: WidgetUpdate = status.into();

        assert_eq!(update.update_type, "beads.status");
        assert_eq!(update.metadata()["daemon_available"], true);
        assert_eq!(update.metadata()["ready_count"], 1);
        assert_eq!(update.metadata()["last_refresh"], 1704067200);
        assert!(update.metadata()["error"].is_null());
        assert_eq!(update.len(), 1);
        assert_eq!(update.widgets[0].widget_type, "beads.task");
    }

    #[test]
    fn test_widget_update_to_beads_status() {
        let update = WidgetUpdate::new(
            "beads.status",
            serde_json::json!({
                "daemon_available": true,
                "ready_count": 2,
                "last_refresh": 1704067200,
                "error": null
            }),
        )
        .with_widgets(vec![
            Widget::new(
                "beads.task",
                serde_json::json!({
                    "id": "BUG-001",
                    "title": "First",
                    "priority": 1,
                    "status": "open",
                    "issue_type": "bug",
                    "assignee": null,
                    "labels": []
                }),
            ),
            Widget::new(
                "beads.task",
                serde_json::json!({
                    "id": "BUG-002",
                    "title": "Second",
                    "priority": 2,
                    "status": "open",
                    "issue_type": "bug",
                    "assignee": null,
                    "labels": []
                }),
            ),
        ]);

        let status: BeadsStatus = update.try_into().unwrap();

        assert!(status.daemon_available);
        assert_eq!(status.ready_count, 2);
        assert_eq!(status.last_refresh, Some(1704067200));
        assert!(status.error.is_none());
        assert_eq!(status.ready_tasks.len(), 2);
        assert_eq!(status.ready_tasks[0].id, "BUG-001");
        assert_eq!(status.ready_tasks[1].id, "BUG-002");
    }

    #[test]
    fn test_widget_update_to_beads_status_wrong_type() {
        let update = WidgetUpdate::new("progress.status", serde_json::json!({}));

        let result: Result<BeadsStatus, _> = update.try_into();
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("Expected update_type 'beads.status'"));
    }

    #[test]
    fn test_beads_status_widget_update_roundtrip() {
        let original = BeadsStatus::with_tasks(
            vec![
                BeadsTask {
                    id: "BUG-001".to_string(),
                    title: "First".to_string(),
                    priority: 1,
                    status: "open".to_string(),
                    issue_type: "bug".to_string(),
                    assignee: Some("alice".to_string()),
                    labels: vec!["urgent".to_string()],
                },
                BeadsTask {
                    id: "FEAT-002".to_string(),
                    title: "Second".to_string(),
                    priority: 2,
                    status: "open".to_string(),
                    issue_type: "feature".to_string(),
                    assignee: None,
                    labels: vec![],
                },
            ],
            1704067200,
        );

        let update: WidgetUpdate = original.clone().into();
        let recovered: BeadsStatus = update.try_into().unwrap();

        assert_eq!(original.daemon_available, recovered.daemon_available);
        assert_eq!(original.ready_count, recovered.ready_count);
        assert_eq!(original.last_refresh, recovered.last_refresh);
        assert_eq!(original.error, recovered.error);
        assert_eq!(original.ready_tasks, recovered.ready_tasks);
    }

    #[test]
    fn test_widget_conversion_error() {
        let err = WidgetConversionError::new("Test error message");
        assert_eq!(err.message, "Test error message");
        assert!(format!("{}", err).contains("Test error message"));
    }
}
