use std::collections::{HashMap, HashSet};
use std::time::SystemTime;
use uuid::Uuid;
use fugue_protocol::{SessionInfo, WorktreeInfo as ProtocolWorktreeInfo, OrchestrationMessage};

use super::Window;
use crate::orchestration::WorktreeInfo;

/// A session containing one or more windows
#[derive(Debug)]
pub struct Session {
    /// Unique session identifier
    id: Uuid,
    /// Session name
    name: String,
    /// Windows in this session
    windows: HashMap<Uuid, Window>,
    /// Order of window IDs
    window_order: Vec<Uuid>,
    /// Currently active window
    active_window_id: Option<Uuid>,
    /// Number of attached clients
    attached_clients: usize,
    /// When created
    created_at: SystemTime,
    /// Associated worktree (if any)
    worktree: Option<WorktreeInfo>,
    /// Tags for session classification and routing (e.g., "orchestrator", "worker", "evaluator")
    tags: HashSet<String>,
    /// Session-level environment variables (inherited by new panes)
    environment: HashMap<String, String>,
    /// Arbitrary key-value metadata for application use
    metadata: HashMap<String, String>,
    /// Inbox for orchestration messages (FEAT-097)
    inbox: Vec<(Uuid, OrchestrationMessage)>,
    /// Stored worker status (FEAT-097)
    status: Option<serde_json::Value>,
}

impl Session {
    /// Create a new session
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            windows: HashMap::new(),
            window_order: Vec::new(),
            active_window_id: None,
            attached_clients: 0,
            created_at: SystemTime::now(),
            worktree: None,
            tags: HashSet::new(),
            environment: HashMap::new(),
            metadata: HashMap::new(),
            inbox: Vec::new(),
            status: None,
        }
    }

    /// Restore a session from persisted state
    ///
    /// Used during crash recovery to recreate session with original ID.
    pub fn restore(id: Uuid, name: impl Into<String>, created_at: u64) -> Self {
        Self {
            id,
            name: name.into(),
            windows: HashMap::new(),
            window_order: Vec::new(),
            active_window_id: None,
            attached_clients: 0,
            created_at: SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(created_at),
            worktree: None,
            tags: HashSet::new(),
            environment: HashMap::new(),
            metadata: HashMap::new(),
            inbox: Vec::new(),
            status: None,
        }
    }

    /// Restore a session with metadata from persisted state
    ///
    /// Used during crash recovery to recreate session with original ID and metadata.
    pub fn restore_with_metadata(
        id: Uuid,
        name: impl Into<String>,
        created_at: u64,
        metadata: HashMap<String, String>,
    ) -> Self {
        Self {
            id,
            name: name.into(),
            windows: HashMap::new(),
            window_order: Vec::new(),
            active_window_id: None,
            attached_clients: 0,
            created_at: SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(created_at),
            worktree: None,
            tags: HashSet::new(),
            environment: HashMap::new(),
            metadata,
            inbox: Vec::new(),
            status: None,
        }
    }

    /// Add a restored window to this session
    ///
    /// Used during crash recovery to add windows with preserved IDs.
    pub fn add_restored_window(&mut self, window: Window) {
        let window_id = window.id();
        self.windows.insert(window_id, window);
        self.window_order.push(window_id);
    }

    /// Set active window ID directly (for restoration)
    pub fn set_active_window_id(&mut self, window_id: Option<Uuid>) {
        self.active_window_id = window_id;
    }

    /// Get session ID
    pub fn id(&self) -> Uuid {
        self.id
    }

    /// Get session name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Set session name
    pub fn set_name(&mut self, name: impl Into<String>) {
        self.name = name.into();
    }

    /// Get window count
    pub fn window_count(&self) -> usize {
        self.windows.len()
    }

    /// Get attached client count
    pub fn attached_clients(&self) -> usize {
        self.attached_clients
    }

    /// Increment attached clients
    pub fn attach_client(&mut self) {
        self.attached_clients += 1;
    }

    /// Decrement attached clients
    pub fn detach_client(&mut self) {
        self.attached_clients = self.attached_clients.saturating_sub(1);
    }

    /// Get active window ID
    pub fn active_window_id(&self) -> Option<Uuid> {
        self.active_window_id
    }

    /// Set active window
    pub fn set_active_window(&mut self, window_id: Uuid) -> bool {
        if self.windows.contains_key(&window_id) {
            self.active_window_id = Some(window_id);
            true
        } else {
            false
        }
    }

    /// Get associated worktree
    pub fn worktree(&self) -> Option<&WorktreeInfo> {
        self.worktree.as_ref()
    }

    /// Check if session has a specific tag
    pub fn has_tag(&self, tag: &str) -> bool {
        self.tags.contains(tag)
    }

    /// Check if this session has the orchestrator tag (convenience method)
    pub fn is_orchestrator(&self) -> bool {
        self.has_tag("orchestrator")
    }

    /// Get all tags for this session
    pub fn tags(&self) -> &HashSet<String> {
        &self.tags
    }

    /// Add a tag to this session
    pub fn add_tag(&mut self, tag: impl Into<String>) {
        self.tags.insert(tag.into());
    }

    /// Remove a tag from this session
    pub fn remove_tag(&mut self, tag: &str) -> bool {
        self.tags.remove(tag)
    }

    /// Set worktree association with tags
    pub fn set_worktree(&mut self, worktree: WorktreeInfo, tags: HashSet<String>) {
        self.tags = tags;
        self.worktree = Some(worktree);
    }

    /// Set worktree association (legacy convenience method)
    pub fn set_worktree_with_orchestrator(&mut self, worktree: WorktreeInfo, is_orchestrator: bool) {
        if is_orchestrator {
            self.tags.insert("orchestrator".to_string());
        }
        self.worktree = Some(worktree);
    }

    /// Set an environment variable on this session
    ///
    /// Environment variables are inherited by newly created panes.
    pub fn set_env(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.environment.insert(key.into(), value.into());
    }

    /// Remove an environment variable from this session
    pub fn unset_env(&mut self, key: &str) -> Option<String> {
        self.environment.remove(key)
    }

    /// Get an environment variable value
    pub fn get_env(&self, key: &str) -> Option<&String> {
        self.environment.get(key)
    }

    /// Get all session environment variables
    pub fn environment(&self) -> &HashMap<String, String> {
        &self.environment
    }

    /// Set a metadata value on this session
    pub fn set_metadata(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.metadata.insert(key.into(), value.into());
    }

    /// Remove a metadata key from this session
    pub fn remove_metadata(&mut self, key: &str) -> Option<String> {
        self.metadata.remove(key)
    }

    /// Get a metadata value
    pub fn get_metadata(&self, key: &str) -> Option<&String> {
        self.metadata.get(key)
    }

    /// Get all session metadata
    pub fn all_metadata(&self) -> &HashMap<String, String> {
        &self.metadata
    }

    /// Push an orchestration message to the inbox
    pub fn push_message(&mut self, from: Uuid, msg: OrchestrationMessage) {
        self.inbox.push((from, msg));
    }

    /// Poll all messages from the inbox, clearing it
    pub fn poll_messages(&mut self) -> Vec<(Uuid, OrchestrationMessage)> {
        std::mem::take(&mut self.inbox)
    }

    /// Set the worker status
    pub fn set_status(&mut self, status: serde_json::Value) {
        self.status = Some(status);
    }

    /// Get the worker status
    pub fn get_status(&self) -> Option<&serde_json::Value> {
        self.status.as_ref()
    }

    /// Create a new window
    pub fn create_window(&mut self, name: Option<String>) -> &Window {
        let index = self.windows.len();
        let window_name = name.unwrap_or_else(|| format!("{}", index));
        let window = Window::new(self.id, index, window_name);
        let window_id = window.id();

        self.windows.insert(window_id, window);
        self.window_order.push(window_id);

        // Set as active if first window
        if self.active_window_id.is_none() {
            self.active_window_id = Some(window_id);
        }

        self.windows.get(&window_id).unwrap()
    }

    /// Get a window by ID
    pub fn get_window(&self, window_id: Uuid) -> Option<&Window> {
        self.windows.get(&window_id)
    }

    /// Get a mutable window by ID
    pub fn get_window_mut(&mut self, window_id: Uuid) -> Option<&mut Window> {
        self.windows.get_mut(&window_id)
    }

    /// Remove a window
    pub fn remove_window(&mut self, window_id: Uuid) -> Option<Window> {
        if let Some(window) = self.windows.remove(&window_id) {
            self.window_order.retain(|&id| id != window_id);

            if self.active_window_id == Some(window_id) {
                self.active_window_id = self.window_order.first().copied();
            }

            // Reindex remaining windows
            for (i, &id) in self.window_order.iter().enumerate() {
                if let Some(w) = self.windows.get_mut(&id) {
                    w.set_index(i);
                }
            }

            Some(window)
        } else {
            None
        }
    }

    /// Iterate over windows
    pub fn windows(&self) -> impl Iterator<Item = &Window> {
        self.window_order.iter().filter_map(|id| self.windows.get(id))
    }

    /// Get window IDs
    pub fn window_ids(&self) -> &[Uuid] {
        &self.window_order
    }

    /// Check if session is empty
    pub fn is_empty(&self) -> bool {
        self.windows.is_empty()
    }

    /// Get creation timestamp as Unix time (seconds)
    pub fn created_at_unix(&self) -> u64 {
        self.created_at
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    }

    /// Get creation timestamp in milliseconds for high-resolution ordering
    pub fn created_at_millis(&self) -> u128 {
        self.created_at
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0)
    }

    /// Convert to protocol SessionInfo
    pub fn to_info(&self) -> SessionInfo {
        SessionInfo {
            id: self.id,
            name: self.name.clone(),
            created_at: self.created_at_unix(),
            window_count: self.windows.len(),
            attached_clients: self.attached_clients,
            worktree: self.worktree.as_ref().map(|w| ProtocolWorktreeInfo {
                path: w.path.to_string_lossy().into_owned(),
                branch: w.branch.clone(),
                is_main: w.is_main,
            }),
            tags: self.tags.clone(),
            metadata: self.metadata.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_creation() {
        let session = Session::new("work");

        assert_eq!(session.name(), "work");
        assert!(session.is_empty());
        assert_eq!(session.attached_clients(), 0);
    }

    #[test]
    fn test_session_create_window() {
        let mut session = Session::new("work");

        let window = session.create_window(Some("main".into()));
        let window_id = window.id();

        assert_eq!(session.window_count(), 1);
        assert_eq!(session.active_window_id(), Some(window_id));
    }

    #[test]
    fn test_session_attach_detach() {
        let mut session = Session::new("work");

        session.attach_client();
        assert_eq!(session.attached_clients(), 1);

        session.attach_client();
        assert_eq!(session.attached_clients(), 2);

        session.detach_client();
        assert_eq!(session.attached_clients(), 1);
    }

    #[test]
    fn test_session_id_is_unique() {
        let session1 = Session::new("work1");
        let session2 = Session::new("work2");

        assert_ne!(session1.id(), session2.id());
    }

    #[test]
    fn test_session_set_name() {
        let mut session = Session::new("work");

        assert_eq!(session.name(), "work");
        session.set_name("new-name");
        assert_eq!(session.name(), "new-name");
    }

    #[test]
    fn test_session_set_active_window_success() {
        let mut session = Session::new("work");

        let window1_id = session.create_window(Some("w1".into())).id();
        let window2_id = session.create_window(Some("w2".into())).id();

        assert_eq!(session.active_window_id(), Some(window1_id));

        let result = session.set_active_window(window2_id);
        assert!(result);
        assert_eq!(session.active_window_id(), Some(window2_id));
    }

    #[test]
    fn test_session_set_active_window_nonexistent() {
        let mut session = Session::new("work");
        session.create_window(None);

        let nonexistent_id = Uuid::new_v4();
        let result = session.set_active_window(nonexistent_id);
        assert!(!result);
    }

    #[test]
    fn test_session_get_window() {
        let mut session = Session::new("work");
        let window_id = session.create_window(Some("main".into())).id();

        let window = session.get_window(window_id);
        assert!(window.is_some());
        assert_eq!(window.unwrap().name(), "main");
    }

    #[test]
    fn test_session_get_window_nonexistent() {
        let session = Session::new("work");

        let nonexistent_id = Uuid::new_v4();
        assert!(session.get_window(nonexistent_id).is_none());
    }

    #[test]
    fn test_session_get_window_mut() {
        let mut session = Session::new("work");
        let window_id = session.create_window(Some("main".into())).id();

        let window = session.get_window_mut(window_id).unwrap();
        window.set_name("renamed");

        let window = session.get_window(window_id).unwrap();
        assert_eq!(window.name(), "renamed");
    }

    #[test]
    fn test_session_remove_window() {
        let mut session = Session::new("work");
        let window1_id = session.create_window(Some("w1".into())).id();
        let window2_id = session.create_window(Some("w2".into())).id();

        assert_eq!(session.active_window_id(), Some(window1_id));

        let removed = session.remove_window(window1_id);
        assert!(removed.is_some());
        assert_eq!(session.window_count(), 1);
        assert_eq!(session.active_window_id(), Some(window2_id));
    }

    #[test]
    fn test_session_remove_window_nonexistent() {
        let mut session = Session::new("work");
        session.create_window(None);

        let nonexistent_id = Uuid::new_v4();
        let result = session.remove_window(nonexistent_id);
        assert!(result.is_none());
    }

    #[test]
    fn test_session_remove_window_reindexes() {
        let mut session = Session::new("work");
        let window1_id = session.create_window(Some("w1".into())).id();
        let window2_id = session.create_window(Some("w2".into())).id();
        let window3_id = session.create_window(Some("w3".into())).id();

        session.remove_window(window2_id);

        let window1 = session.get_window(window1_id).unwrap();
        let window3 = session.get_window(window3_id).unwrap();

        assert_eq!(window1.index(), 0);
        assert_eq!(window3.index(), 1);
    }

    #[test]
    fn test_session_windows_iterator() {
        let mut session = Session::new("work");
        session.create_window(None);
        session.create_window(None);
        session.create_window(None);

        let count = session.windows().count();
        assert_eq!(count, 3);
    }

    #[test]
    fn test_session_windows_iterator_order() {
        let mut session = Session::new("work");
        let w1_id = session.create_window(Some("w1".into())).id();
        let w2_id = session.create_window(Some("w2".into())).id();
        let w3_id = session.create_window(Some("w3".into())).id();

        let ids: Vec<_> = session.windows().map(|w| w.id()).collect();
        assert_eq!(ids, vec![w1_id, w2_id, w3_id]);
    }

    #[test]
    fn test_session_window_ids() {
        let mut session = Session::new("work");
        let w1_id = session.create_window(None).id();
        let w2_id = session.create_window(None).id();

        let ids = session.window_ids();
        assert_eq!(ids.len(), 2);
        assert_eq!(ids[0], w1_id);
        assert_eq!(ids[1], w2_id);
    }

    #[test]
    fn test_session_is_empty() {
        let mut session = Session::new("work");
        assert!(session.is_empty());

        let window_id = session.create_window(None).id();
        assert!(!session.is_empty());

        session.remove_window(window_id);
        assert!(session.is_empty());
    }

    #[test]
    fn test_session_created_at_unix() {
        let session = Session::new("work");
        let timestamp = session.created_at_unix();

        // Should be a reasonable timestamp (after year 2020)
        assert!(timestamp > 1577836800); // Jan 1, 2020
    }

    #[test]
    fn test_session_to_info() {
        let mut session = Session::new("test-session");
        session.attach_client();
        session.create_window(None);
        session.create_window(None);

        let info = session.to_info();

        assert_eq!(info.id, session.id());
        assert_eq!(info.name, "test-session");
        assert_eq!(info.window_count, 2);
        assert_eq!(info.attached_clients, 1);
        assert!(info.created_at > 0);
        assert!(info.worktree.is_none());
        assert!(info.tags.is_empty());
    }

    #[test]
    fn test_session_to_info_empty() {
        let session = Session::new("empty");

        let info = session.to_info();

        assert_eq!(info.window_count, 0);
        assert_eq!(info.attached_clients, 0);
        assert!(info.worktree.is_none());
        assert!(info.tags.is_empty());
    }

    #[test]
    fn test_session_detach_client_saturates() {
        let mut session = Session::new("work");

        // Detach when no clients should stay at 0
        session.detach_client();
        assert_eq!(session.attached_clients(), 0);

        session.attach_client();
        session.detach_client();
        assert_eq!(session.attached_clients(), 0);

        session.detach_client();
        assert_eq!(session.attached_clients(), 0);
    }

    #[test]
    fn test_session_create_window_auto_name() {
        let mut session = Session::new("work");

        let window1 = session.create_window(None);
        assert_eq!(window1.name(), "0");

        let window2 = session.create_window(None);
        assert_eq!(window2.name(), "1");
    }

    #[test]
    fn test_session_debug_format() {
        let session = Session::new("debug-test");

        let debug_str = format!("{:?}", session);
        assert!(debug_str.contains("Session"));
        assert!(debug_str.contains("debug-test"));
    }

    #[test]
    fn test_session_remove_last_window() {
        let mut session = Session::new("work");
        let window_id = session.create_window(None).id();

        session.remove_window(window_id);

        assert!(session.is_empty());
        assert_eq!(session.active_window_id(), None);
    }

    #[test]
    fn test_session_multiple_attach_detach() {
        let mut session = Session::new("work");

        for _ in 0..10 {
            session.attach_client();
        }
        assert_eq!(session.attached_clients(), 10);

        for _ in 0..5 {
            session.detach_client();
        }
        assert_eq!(session.attached_clients(), 5);
    }

    #[test]
    fn test_session_name_with_special_characters() {
        let mut session = Session::new("test session!");

        assert_eq!(session.name(), "test session!");

        session.set_name("new-name_123");
        assert_eq!(session.name(), "new-name_123");
    }

    // ==================== Worktree Tests ====================

    #[test]
    fn test_session_worktree_default() {
        let session = Session::new("work");

        assert!(session.worktree().is_none());
        assert!(!session.is_orchestrator());
        assert!(session.tags().is_empty());
    }

    #[test]
    fn test_session_set_worktree() {
        use std::path::PathBuf;

        let mut session = Session::new("work");

        let worktree = WorktreeInfo {
            path: PathBuf::from("/path/to/worktree"),
            branch: Some("feature-1".to_string()),
            head: "def456".to_string(),
            is_main: false,
        };

        session.set_worktree(worktree.clone(), HashSet::new());

        let wt = session.worktree().unwrap();
        assert_eq!(wt.path, PathBuf::from("/path/to/worktree"));
        assert_eq!(wt.branch, Some("feature-1".to_string()));
        assert!(!wt.is_main);
        assert!(!session.is_orchestrator());
    }

    #[test]
    fn test_session_set_worktree_as_orchestrator() {
        use std::path::PathBuf;

        let mut session = Session::new("orchestrator");

        let worktree = WorktreeInfo {
            path: PathBuf::from("/path/to/main"),
            branch: Some("main".to_string()),
            head: "abc123".to_string(),
            is_main: true,
        };

        let mut tags = HashSet::new();
        tags.insert("orchestrator".to_string());
        session.set_worktree(worktree, tags);

        assert!(session.worktree().is_some());
        assert!(session.is_orchestrator());
    }

    #[test]
    fn test_session_to_info_with_worktree() {
        use std::path::PathBuf;

        let mut session = Session::new("worker");

        let worktree = WorktreeInfo {
            path: PathBuf::from("/path/to/worktree"),
            branch: Some("feature-1".to_string()),
            head: "def456".to_string(),
            is_main: false,
        };

        session.set_worktree(worktree, HashSet::new());

        let info = session.to_info();

        assert!(info.worktree.is_some());
        let proto_wt = info.worktree.unwrap();
        assert_eq!(proto_wt.path, "/path/to/worktree");
        assert_eq!(proto_wt.branch, Some("feature-1".to_string()));
        assert!(!proto_wt.is_main);
        assert!(info.tags.is_empty());
    }

    #[test]
    fn test_session_to_info_orchestrator() {
        use std::path::PathBuf;

        let mut session = Session::new("main");

        let worktree = WorktreeInfo {
            path: PathBuf::from("/path/to/main"),
            branch: Some("main".to_string()),
            head: "abc123".to_string(),
            is_main: true,
        };

        let mut tags = HashSet::new();
        tags.insert("orchestrator".to_string());
        session.set_worktree(worktree, tags);

        let info = session.to_info();

        assert!(info.worktree.is_some());
        assert!(info.has_tag("orchestrator"));
    }

    // ==================== Tags Tests ====================

    #[test]
    fn test_session_tags_management() {
        let mut session = Session::new("work");

        // Initially no tags
        assert!(session.tags().is_empty());
        assert!(!session.has_tag("worker"));

        // Add a tag
        session.add_tag("worker");
        assert!(session.has_tag("worker"));
        assert_eq!(session.tags().len(), 1);

        // Add another tag
        session.add_tag("evaluator");
        assert!(session.has_tag("evaluator"));
        assert_eq!(session.tags().len(), 2);

        // Remove a tag
        assert!(session.remove_tag("worker"));
        assert!(!session.has_tag("worker"));
        assert_eq!(session.tags().len(), 1);

        // Removing non-existent tag returns false
        assert!(!session.remove_tag("nonexistent"));
    }

    #[test]
    fn test_session_multiple_tags() {
        use std::path::PathBuf;

        let mut session = Session::new("multi-role");

        let worktree = WorktreeInfo {
            path: PathBuf::from("/path/to/repo"),
            branch: Some("main".to_string()),
            head: "abc123".to_string(),
            is_main: true,
        };

        let mut tags = HashSet::new();
        tags.insert("orchestrator".to_string());
        tags.insert("primary".to_string());
        session.set_worktree(worktree, tags);

        assert!(session.has_tag("orchestrator"));
        assert!(session.has_tag("primary"));
        assert!(session.is_orchestrator());
        assert_eq!(session.tags().len(), 2);

        let info = session.to_info();
        assert!(info.has_tag("orchestrator"));
        assert!(info.has_tag("primary"));
    }
}
