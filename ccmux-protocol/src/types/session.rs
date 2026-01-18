use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

/// Worktree information for protocol messages
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorktreeInfo {
    /// Absolute path to the worktree
    pub path: String,
    /// Branch name (if any)
    pub branch: Option<String>,
    /// Whether this is the main worktree
    pub is_main: bool,
}

/// Session information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SessionInfo {
    pub id: Uuid,
    pub name: String,
    pub created_at: u64, // Unix timestamp
    pub window_count: usize,
    pub attached_clients: usize,
    /// Associated worktree (if any)
    pub worktree: Option<WorktreeInfo>,
    /// Tags for session classification and routing (e.g., "orchestrator", "worker", "evaluator")
    #[serde(default)]
    pub tags: HashSet<String>,
    /// Arbitrary key-value metadata for application use
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

impl SessionInfo {
    /// Check if session has a specific tag
    pub fn has_tag(&self, tag: &str) -> bool {
        self.tags.contains(tag)
    }

    /// Add a tag to the session
    pub fn add_tag(&mut self, tag: impl Into<String>) {
        self.tags.insert(tag.into());
    }

    /// Remove a tag from the session
    pub fn remove_tag(&mut self, tag: &str) -> bool {
        self.tags.remove(tag)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== WorktreeInfo Tests ====================

    #[test]
    fn test_worktree_info_creation() {
        let wt = WorktreeInfo {
            path: "/path/to/worktree".to_string(),
            branch: Some("feature-1".to_string()),
            is_main: false,
        };

        assert_eq!(wt.path, "/path/to/worktree");
        assert_eq!(wt.branch, Some("feature-1".to_string()));
        assert!(!wt.is_main);
    }

    #[test]
    fn test_worktree_info_main() {
        let wt = WorktreeInfo {
            path: "/path/to/repo".to_string(),
            branch: Some("main".to_string()),
            is_main: true,
        };

        assert!(wt.is_main);
    }

    #[test]
    fn test_worktree_info_no_branch() {
        let wt = WorktreeInfo {
            path: "/path/to/worktree".to_string(),
            branch: None,
            is_main: false,
        };

        assert!(wt.branch.is_none());
    }

    #[test]
    fn test_worktree_info_clone() {
        let wt = WorktreeInfo {
            path: "/path/to/worktree".to_string(),
            branch: Some("main".to_string()),
            is_main: true,
        };

        let cloned = wt.clone();
        assert_eq!(wt, cloned);
    }

    #[test]
    fn test_worktree_info_equality() {
        let wt1 = WorktreeInfo {
            path: "/path/a".to_string(),
            branch: Some("main".to_string()),
            is_main: true,
        };

        let wt2 = WorktreeInfo {
            path: "/path/a".to_string(),
            branch: Some("main".to_string()),
            is_main: true,
        };

        let wt3 = WorktreeInfo {
            path: "/path/b".to_string(),
            branch: Some("main".to_string()),
            is_main: true,
        };

        assert_eq!(wt1, wt2);
        assert_ne!(wt1, wt3);
    }

    #[test]
    fn test_worktree_info_debug() {
        let wt = WorktreeInfo {
            path: "/debug/path".to_string(),
            branch: Some("test".to_string()),
            is_main: false,
        };

        let debug = format!("{:?}", wt);
        assert!(debug.contains("WorktreeInfo"));
        assert!(debug.contains("/debug/path"));
    }

    #[test]
    fn test_worktree_info_serde() {
        let wt = WorktreeInfo {
            path: "/path/to/worktree".to_string(),
            branch: Some("feature".to_string()),
            is_main: false,
        };

        let serialized = bincode::serialize(&wt).unwrap();
        let deserialized: WorktreeInfo = bincode::deserialize(&serialized).unwrap();
        assert_eq!(wt, deserialized);
    }

    #[test]
    fn test_worktree_info_serde_no_branch() {
        let wt = WorktreeInfo {
            path: "/path/to/worktree".to_string(),
            branch: None,
            is_main: true,
        };

        let serialized = bincode::serialize(&wt).unwrap();
        let deserialized: WorktreeInfo = bincode::deserialize(&serialized).unwrap();
        assert_eq!(wt, deserialized);
    }

    // ==================== SessionInfo Tests ====================

    #[test]
    fn test_session_info_creation() {
        let id = Uuid::new_v4();

        let session = SessionInfo {
            id,
            name: "my-session".to_string(),
            created_at: 1704067200, // 2024-01-01 00:00:00 UTC
            window_count: 2,
            attached_clients: 1,
            worktree: None,
            tags: HashSet::new(),
            metadata: HashMap::new(),
        };

        assert_eq!(session.id, id);
        assert_eq!(session.name, "my-session");
        assert_eq!(session.created_at, 1704067200);
        assert_eq!(session.window_count, 2);
        assert_eq!(session.attached_clients, 1);
    }

    #[test]
    fn test_session_info_no_clients() {
        let session = SessionInfo {
            id: Uuid::new_v4(),
            name: "detached".to_string(),
            created_at: 0,
            window_count: 1,
            attached_clients: 0,
            worktree: None,
            tags: HashSet::new(),
            metadata: HashMap::new(),
        };

        assert_eq!(session.attached_clients, 0);
    }

    #[test]
    fn test_session_info_multiple_clients() {
        let session = SessionInfo {
            id: Uuid::new_v4(),
            name: "shared".to_string(),
            created_at: 0,
            window_count: 1,
            attached_clients: 5,
            worktree: None,
            tags: HashSet::new(),
            metadata: HashMap::new(),
        };

        assert_eq!(session.attached_clients, 5);
    }

    #[test]
    fn test_session_info_clone() {
        let session = SessionInfo {
            id: Uuid::new_v4(),
            name: "test".to_string(),
            created_at: 12345,
            window_count: 3,
            attached_clients: 2,
            worktree: None,
            tags: HashSet::new(),
            metadata: HashMap::new(),
        };

        let cloned = session.clone();
        assert_eq!(session, cloned);
    }

    #[test]
    fn test_session_info_equality() {
        let id = Uuid::new_v4();

        let session1 = SessionInfo {
            id,
            name: "test".to_string(),
            created_at: 1000,
            window_count: 1,
            attached_clients: 0,
            worktree: None,
            tags: HashSet::new(),
            metadata: HashMap::new(),
        };

        let session2 = SessionInfo {
            id,
            name: "test".to_string(),
            created_at: 1000,
            window_count: 1,
            attached_clients: 0,
            worktree: None,
            tags: HashSet::new(),
            metadata: HashMap::new(),
        };

        let session3 = SessionInfo {
            id,
            name: "different".to_string(),
            created_at: 1000,
            window_count: 1,
            attached_clients: 0,
            worktree: None,
            tags: HashSet::new(),
            metadata: HashMap::new(),
        };

        assert_eq!(session1, session2);
        assert_ne!(session1, session3);
    }

    #[test]
    fn test_session_info_debug() {
        let session = SessionInfo {
            id: Uuid::nil(),
            name: "debug-test".to_string(),
            created_at: 0,
            window_count: 0,
            attached_clients: 0,
            worktree: None,
            tags: HashSet::new(),
            metadata: HashMap::new(),
        };

        let debug = format!("{:?}", session);
        assert!(debug.contains("SessionInfo"));
        assert!(debug.contains("debug-test"));
    }

    #[test]
    fn test_session_info_serde() {
        let session = SessionInfo {
            id: Uuid::new_v4(),
            name: "test-session".to_string(),
            created_at: 1234567890,
            window_count: 3,
            attached_clients: 1,
            worktree: None,
            tags: HashSet::new(),
            metadata: HashMap::new(),
        };

        let serialized = bincode::serialize(&session).unwrap();
        let deserialized: SessionInfo = bincode::deserialize(&serialized).unwrap();
        assert_eq!(session, deserialized);
    }

    #[test]
    fn test_session_info_with_worktree() {
        let mut tags = HashSet::new();
        tags.insert("orchestrator".to_string());

        let session = SessionInfo {
            id: Uuid::new_v4(),
            name: "test".to_string(),
            created_at: 1234567890,
            window_count: 1,
            attached_clients: 0,
            worktree: Some(WorktreeInfo {
                path: "/path/to/repo".to_string(),
                branch: Some("main".to_string()),
                is_main: true,
            }),
            tags,
            metadata: HashMap::new(),
        };

        assert!(session.worktree.is_some());
        assert!(session.has_tag("orchestrator"));
    }

    #[test]
    fn test_session_info_without_worktree() {
        let session = SessionInfo {
            id: Uuid::new_v4(),
            name: "test".to_string(),
            created_at: 1234567890,
            window_count: 1,
            attached_clients: 0,
            worktree: None,
            tags: HashSet::new(),
            metadata: HashMap::new(),
        };

        assert!(session.worktree.is_none());
        assert!(!session.has_tag("orchestrator"));
    }

    #[test]
    fn test_session_info_tags() {
        let mut session = SessionInfo {
            id: Uuid::new_v4(),
            name: "test".to_string(),
            created_at: 0,
            window_count: 0,
            attached_clients: 0,
            worktree: None,
            tags: HashSet::new(),
            metadata: HashMap::new(),
        };

        // Initially no tags
        assert!(!session.has_tag("worker"));
        assert!(session.tags.is_empty());

        // Add a tag
        session.add_tag("worker");
        assert!(session.has_tag("worker"));
        assert_eq!(session.tags.len(), 1);

        // Add another tag
        session.add_tag("evaluator");
        assert!(session.has_tag("evaluator"));
        assert_eq!(session.tags.len(), 2);

        // Remove a tag
        assert!(session.remove_tag("worker"));
        assert!(!session.has_tag("worker"));
        assert_eq!(session.tags.len(), 1);

        // Removing non-existent tag returns false
        assert!(!session.remove_tag("nonexistent"));
    }

    #[test]
    fn test_session_info_tags_clone() {
        let mut tags = HashSet::new();
        tags.insert("tag1".to_string());
        tags.insert("tag2".to_string());

        let session = SessionInfo {
            id: Uuid::new_v4(),
            name: "test".to_string(),
            created_at: 0,
            window_count: 0,
            attached_clients: 0,
            worktree: None,
            tags,
            metadata: HashMap::new(),
        };

        let cloned = session.clone();
        assert_eq!(session.tags, cloned.tags);
    }

    #[test]
    fn test_session_info_with_worktree_serde() {
        let session = SessionInfo {
            id: Uuid::new_v4(),
            name: "test".to_string(),
            created_at: 1234567890,
            window_count: 2,
            attached_clients: 1,
            worktree: Some(WorktreeInfo {
                path: "/path/to/worktree".to_string(),
                branch: Some("feature".to_string()),
                is_main: false,
            }),
            tags: HashSet::new(),
            metadata: HashMap::new(),
        };

        let serialized = bincode::serialize(&session).unwrap();
        let deserialized: SessionInfo = bincode::deserialize(&serialized).unwrap();
        assert_eq!(session, deserialized);
    }
}
