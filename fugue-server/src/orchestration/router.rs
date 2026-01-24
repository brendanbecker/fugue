//! Message routing for cross-session communication

// Scaffolding for multi-session orchestration - not wired up to main server yet
#![allow(dead_code)]

use std::collections::{HashMap, HashSet};
use tokio::sync::mpsc;
use uuid::Uuid;

use fugue_protocol::{OrchestrationMessage, OrchestrationTarget};

/// Sender for orchestration messages (from_session_id, message)
pub type MessageSender = mpsc::Sender<(Uuid, OrchestrationMessage)>;
/// Receiver for orchestration messages
pub type MessageReceiver = mpsc::Receiver<(Uuid, OrchestrationMessage)>;

/// Routes messages between sessions using tag-based addressing
///
/// Sessions can be tagged with arbitrary strings (e.g., "orchestrator", "worker", "evaluator")
/// and messages can be routed to sessions with specific tags using `OrchestrationTarget::Tagged`.
pub struct MessageRouter {
    /// Registered session senders
    sessions: HashMap<Uuid, MessageSender>,
    /// Session to repository mapping
    session_repos: HashMap<Uuid, String>,
    /// Session to worktree path mapping
    session_worktrees: HashMap<Uuid, String>,
    /// Tags for each session
    session_tags: HashMap<Uuid, HashSet<String>>,
}

impl Default for MessageRouter {
    fn default() -> Self {
        Self::new()
    }
}

impl MessageRouter {
    pub fn new() -> Self {
        Self {
            sessions: HashMap::new(),
            session_repos: HashMap::new(),
            session_worktrees: HashMap::new(),
            session_tags: HashMap::new(),
        }
    }

    /// Register a session with the router
    ///
    /// # Arguments
    /// * `session_id` - Unique session identifier
    /// * `repo_id` - Optional repository identifier for repo-scoped routing
    /// * `worktree_path` - Optional worktree path for worktree-scoped routing
    /// * `tags` - Tags for tag-based routing (e.g., "orchestrator", "worker")
    ///
    /// Returns a receiver for incoming orchestration messages.
    pub fn register(
        &mut self,
        session_id: Uuid,
        repo_id: Option<String>,
        worktree_path: Option<String>,
        tags: HashSet<String>,
    ) -> MessageReceiver {
        let (tx, rx) = mpsc::channel(100);
        self.sessions.insert(session_id, tx);

        if let Some(repo) = repo_id {
            self.session_repos.insert(session_id, repo);
        }

        if let Some(worktree) = worktree_path {
            self.session_worktrees.insert(session_id, worktree);
        }

        self.session_tags.insert(session_id, tags);

        rx
    }

    /// Unregister a session from the router
    pub fn unregister(&mut self, session_id: Uuid) {
        self.sessions.remove(&session_id);
        self.session_worktrees.remove(&session_id);
        self.session_repos.remove(&session_id);
        self.session_tags.remove(&session_id);
    }

    /// Add a tag to a session
    pub fn add_tag(&mut self, session_id: Uuid, tag: impl Into<String>) -> bool {
        if let Some(tags) = self.session_tags.get_mut(&session_id) {
            tags.insert(tag.into());
            true
        } else {
            false
        }
    }

    /// Remove a tag from a session
    pub fn remove_tag(&mut self, session_id: Uuid, tag: &str) -> bool {
        if let Some(tags) = self.session_tags.get_mut(&session_id) {
            tags.remove(tag)
        } else {
            false
        }
    }

    /// Get all sessions with a specific tag
    pub fn sessions_with_tag(&self, tag: &str) -> Vec<Uuid> {
        self.session_tags
            .iter()
            .filter(|(_, tags)| tags.contains(tag))
            .map(|(id, _)| *id)
            .collect()
    }

    /// Get all tags for a session
    pub fn get_tags(&self, session_id: Uuid) -> Option<&HashSet<String>> {
        self.session_tags.get(&session_id)
    }

    /// Route a message to its target(s)
    ///
    /// Returns the number of sessions that received the message, or an error.
    pub async fn route(
        &self,
        from_session_id: Uuid,
        target: OrchestrationTarget,
        message: OrchestrationMessage,
    ) -> Result<usize, RouterError> {
        let repo = self.session_repos.get(&from_session_id);

        let targets: Vec<Uuid> = match target {
            OrchestrationTarget::Tagged(ref tag) => {
                // Find all sessions with this tag (excluding sender)
                self.sessions_with_tag(tag)
                    .into_iter()
                    .filter(|id| *id != from_session_id)
                    .collect()
            }
            OrchestrationTarget::Session(id) => vec![id],
            OrchestrationTarget::Broadcast => {
                let repo = repo.ok_or(RouterError::NoRepository)?;
                self.session_repos
                    .iter()
                    .filter(|(_, r)| *r == repo)
                    .map(|(id, _)| *id)
                    .filter(|id| *id != from_session_id)
                    .collect()
            }
            OrchestrationTarget::Worktree(ref path) => {
                // Find sessions in specific worktree
                self.session_worktrees
                    .iter()
                    .filter(|(_, wt)| *wt == path)
                    .map(|(id, _)| *id)
                    .filter(|id| *id != from_session_id)
                    .collect()
            }
        };

        let mut delivered = 0;
        for target_id in targets {
            if let Some(sender) = self.sessions.get(&target_id) {
                if sender
                    .send((from_session_id, message.clone()))
                    .await
                    .is_ok()
                {
                    delivered += 1;
                }
            }
        }

        // For non-broadcast targets, it's an error if no one received the message
        if delivered == 0 && !matches!(target, OrchestrationTarget::Broadcast) {
            return Err(RouterError::NoRecipients);
        }

        Ok(delivered)
    }

    /// Check if a session is registered
    pub fn is_registered(&self, session_id: Uuid) -> bool {
        self.sessions.contains_key(&session_id)
    }

    /// Get all sessions in a repository
    pub fn sessions_in_repo(&self, repo_id: &str) -> Vec<Uuid> {
        self.session_repos
            .iter()
            .filter(|(_, r)| *r == repo_id)
            .map(|(id, _)| *id)
            .collect()
    }

    /// Get all registered session IDs
    pub fn all_sessions(&self) -> Vec<Uuid> {
        self.sessions.keys().copied().collect()
    }
}

/// Errors that can occur during message routing
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RouterError {
    /// Session not associated with a repository
    NoRepository,
    /// No recipients for message
    NoRecipients,
}

impl std::fmt::Display for RouterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RouterError::NoRepository => write!(f, "Session not associated with a repository"),
            RouterError::NoRecipients => write!(f, "No recipients for message"),
        }
    }
}

impl std::error::Error for RouterError {}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn make_msg(msg_type: &str) -> OrchestrationMessage {
        OrchestrationMessage::new(msg_type, json!({}))
    }

    fn make_tags(tags: &[&str]) -> HashSet<String> {
        tags.iter().map(|s| s.to_string()).collect()
    }

    #[tokio::test]
    async fn test_router_register_unregister() {
        let mut router = MessageRouter::new();

        let session_id = Uuid::new_v4();
        let _rx = router.register(
            session_id,
            Some("repo1".to_string()),
            None,
            HashSet::new(),
        );

        assert!(router.is_registered(session_id));

        router.unregister(session_id);
        assert!(!router.is_registered(session_id));
    }

    #[tokio::test]
    async fn test_router_tag_registration() {
        let mut router = MessageRouter::new();

        let session_id = Uuid::new_v4();
        let _rx = router.register(
            session_id,
            Some("repo1".to_string()),
            None,
            make_tags(&["orchestrator", "primary"]),
        );

        let tags = router.get_tags(session_id).unwrap();
        assert!(tags.contains("orchestrator"));
        assert!(tags.contains("primary"));
        assert_eq!(tags.len(), 2);
    }

    #[tokio::test]
    async fn test_router_send_to_tagged_session() {
        let mut router = MessageRouter::new();

        let orch_id = Uuid::new_v4();
        let worker_id = Uuid::new_v4();

        let mut orch_rx = router.register(
            orch_id,
            Some("repo1".to_string()),
            None,
            make_tags(&["orchestrator"]),
        );
        let _worker_rx = router.register(
            worker_id,
            Some("repo1".to_string()),
            None,
            make_tags(&["worker"]),
        );

        let msg = make_msg("status.update");

        let delivered = router
            .route(
                worker_id,
                OrchestrationTarget::Tagged("orchestrator".to_string()),
                msg.clone(),
            )
            .await
            .unwrap();

        assert_eq!(delivered, 1);

        let (from, received) = orch_rx.recv().await.unwrap();
        assert_eq!(from, worker_id);
        assert_eq!(received, msg);
    }

    #[tokio::test]
    async fn test_router_send_to_multiple_tagged_sessions() {
        let mut router = MessageRouter::new();

        let orch_id = Uuid::new_v4();
        let worker1_id = Uuid::new_v4();
        let worker2_id = Uuid::new_v4();

        let _orch_rx = router.register(
            orch_id,
            Some("repo1".to_string()),
            None,
            make_tags(&["orchestrator"]),
        );
        let mut worker1_rx = router.register(
            worker1_id,
            Some("repo1".to_string()),
            None,
            make_tags(&["worker"]),
        );
        let mut worker2_rx = router.register(
            worker2_id,
            Some("repo1".to_string()),
            None,
            make_tags(&["worker"]),
        );

        let msg = make_msg("task.broadcast");

        let delivered = router
            .route(
                orch_id,
                OrchestrationTarget::Tagged("worker".to_string()),
                msg,
            )
            .await
            .unwrap();

        assert_eq!(delivered, 2);

        assert!(worker1_rx.recv().await.is_some());
        assert!(worker2_rx.recv().await.is_some());
    }

    #[tokio::test]
    async fn test_router_send_to_specific_session() {
        let mut router = MessageRouter::new();

        let session1 = Uuid::new_v4();
        let session2 = Uuid::new_v4();

        let _rx1 = router.register(
            session1,
            Some("repo1".to_string()),
            None,
            HashSet::new(),
        );
        let mut rx2 = router.register(
            session2,
            Some("repo1".to_string()),
            None,
            HashSet::new(),
        );

        let msg = OrchestrationMessage::new(
            "task.assigned",
            json!({
                "task_id": "abc123",
                "description": "Fix the bug",
                "files": ["src/main.rs"]
            }),
        );

        let delivered = router
            .route(session1, OrchestrationTarget::Session(session2), msg.clone())
            .await
            .unwrap();

        assert_eq!(delivered, 1);

        let (from, received) = rx2.recv().await.unwrap();
        assert_eq!(from, session1);
        assert_eq!(received, msg);
    }

    #[tokio::test]
    async fn test_router_broadcast() {
        let mut router = MessageRouter::new();

        let session1 = Uuid::new_v4();
        let session2 = Uuid::new_v4();
        let session3 = Uuid::new_v4();

        let _rx1 = router.register(
            session1,
            Some("repo1".to_string()),
            None,
            make_tags(&["orchestrator"]),
        );
        let mut rx2 = router.register(
            session2,
            Some("repo1".to_string()),
            None,
            make_tags(&["worker"]),
        );
        let mut rx3 = router.register(
            session3,
            Some("repo1".to_string()),
            None,
            make_tags(&["worker"]),
        );

        let msg = make_msg("broadcast.pause");

        let delivered = router
            .route(session1, OrchestrationTarget::Broadcast, msg)
            .await
            .unwrap();

        assert_eq!(delivered, 2); // Broadcast doesn't send to self

        assert!(rx2.recv().await.is_some());
        assert!(rx3.recv().await.is_some());
    }

    #[tokio::test]
    async fn test_router_broadcast_different_repos() {
        let mut router = MessageRouter::new();

        let session1 = Uuid::new_v4();
        let session2 = Uuid::new_v4();
        let session3 = Uuid::new_v4();

        let _rx1 = router.register(
            session1,
            Some("repo1".to_string()),
            None,
            HashSet::new(),
        );
        let mut rx2 = router.register(
            session2,
            Some("repo1".to_string()),
            None,
            HashSet::new(),
        );
        let _rx3 = router.register(
            session3,
            Some("repo2".to_string()),
            None,
            HashSet::new(),
        ); // Different repo

        let msg = make_msg("sync.request");

        let delivered = router
            .route(session1, OrchestrationTarget::Broadcast, msg)
            .await
            .unwrap();

        // Only session2 should receive (same repo, not sender)
        assert_eq!(delivered, 1);
        assert!(rx2.recv().await.is_some());
    }

    #[tokio::test]
    async fn test_router_worktree_routing() {
        let mut router = MessageRouter::new();

        let session1 = Uuid::new_v4();
        let session2 = Uuid::new_v4();
        let session3 = Uuid::new_v4();

        let _rx1 = router.register(
            session1,
            Some("repo1".to_string()),
            Some("/repo/worktree1".to_string()),
            HashSet::new(),
        );
        let mut rx2 = router.register(
            session2,
            Some("repo1".to_string()),
            Some("/repo/worktree1".to_string()),
            HashSet::new(),
        );
        let _rx3 = router.register(
            session3,
            Some("repo1".to_string()),
            Some("/repo/worktree2".to_string()),
            HashSet::new(),
        );

        let msg = make_msg("status.update");

        let delivered = router
            .route(
                session1,
                OrchestrationTarget::Worktree("/repo/worktree1".to_string()),
                msg,
            )
            .await
            .unwrap();

        // Only session2 should receive (same worktree, not sender)
        assert_eq!(delivered, 1);
        assert!(rx2.recv().await.is_some());
    }

    #[tokio::test]
    async fn test_router_no_repository_error() {
        let mut router = MessageRouter::new();

        let session_id = Uuid::new_v4();
        let _rx = router.register(session_id, None, None, HashSet::new());

        let msg = make_msg("sync.request");

        let result = router
            .route(session_id, OrchestrationTarget::Broadcast, msg)
            .await;

        assert_eq!(result, Err(RouterError::NoRepository));
    }

    #[tokio::test]
    async fn test_router_no_recipients_error() {
        let mut router = MessageRouter::new();

        let session_id = Uuid::new_v4();
        let _rx = router.register(
            session_id,
            Some("repo1".to_string()),
            None,
            HashSet::new(),
        );

        let nonexistent = Uuid::new_v4();
        let msg = make_msg("ping");

        let result = router
            .route(session_id, OrchestrationTarget::Session(nonexistent), msg)
            .await;

        assert_eq!(result, Err(RouterError::NoRecipients));
    }

    #[tokio::test]
    async fn test_router_no_tagged_sessions() {
        let mut router = MessageRouter::new();

        let session_id = Uuid::new_v4();
        // Register without orchestrator tag
        let _rx = router.register(
            session_id,
            Some("repo1".to_string()),
            None,
            make_tags(&["worker"]),
        );

        let msg = make_msg("status.update");

        // Try to send to non-existent tag
        let result = router
            .route(
                session_id,
                OrchestrationTarget::Tagged("orchestrator".to_string()),
                msg,
            )
            .await;

        assert_eq!(result, Err(RouterError::NoRecipients));
    }

    #[tokio::test]
    async fn test_router_empty_broadcast_ok() {
        let mut router = MessageRouter::new();

        let session_id = Uuid::new_v4();
        let _rx = router.register(
            session_id,
            Some("repo1".to_string()),
            None,
            HashSet::new(),
        );

        let msg = make_msg("sync.request");

        // Broadcast to empty repo (only self) should succeed with 0 delivered
        let result = router
            .route(session_id, OrchestrationTarget::Broadcast, msg)
            .await;

        assert_eq!(result, Ok(0));
    }

    #[tokio::test]
    async fn test_router_sessions_in_repo() {
        let mut router = MessageRouter::new();

        let session1 = Uuid::new_v4();
        let session2 = Uuid::new_v4();
        let session3 = Uuid::new_v4();

        let _rx1 = router.register(
            session1,
            Some("repo1".to_string()),
            None,
            HashSet::new(),
        );
        let _rx2 = router.register(
            session2,
            Some("repo1".to_string()),
            None,
            HashSet::new(),
        );
        let _rx3 = router.register(
            session3,
            Some("repo2".to_string()),
            None,
            HashSet::new(),
        );

        let repo1_sessions = router.sessions_in_repo("repo1");
        assert_eq!(repo1_sessions.len(), 2);
        assert!(repo1_sessions.contains(&session1));
        assert!(repo1_sessions.contains(&session2));

        let repo2_sessions = router.sessions_in_repo("repo2");
        assert_eq!(repo2_sessions.len(), 1);
        assert!(repo2_sessions.contains(&session3));
    }

    #[tokio::test]
    async fn test_router_add_remove_tag() {
        let mut router = MessageRouter::new();

        let session_id = Uuid::new_v4();
        let _rx = router.register(
            session_id,
            Some("repo1".to_string()),
            None,
            make_tags(&["worker"]),
        );

        // Add a tag dynamically
        assert!(router.add_tag(session_id, "evaluator"));
        let tags = router.get_tags(session_id).unwrap();
        assert!(tags.contains("evaluator"));
        assert!(tags.contains("worker"));

        // Remove a tag
        assert!(router.remove_tag(session_id, "worker"));
        let tags = router.get_tags(session_id).unwrap();
        assert!(!tags.contains("worker"));
        assert!(tags.contains("evaluator"));

        // Can't remove non-existent tag
        assert!(!router.remove_tag(session_id, "nonexistent"));
    }

    #[tokio::test]
    async fn test_router_sessions_with_tag() {
        let mut router = MessageRouter::new();

        let session1 = Uuid::new_v4();
        let session2 = Uuid::new_v4();
        let session3 = Uuid::new_v4();

        let _rx1 = router.register(
            session1,
            Some("repo1".to_string()),
            None,
            make_tags(&["worker", "evaluator"]),
        );
        let _rx2 = router.register(
            session2,
            Some("repo1".to_string()),
            None,
            make_tags(&["worker"]),
        );
        let _rx3 = router.register(
            session3,
            Some("repo1".to_string()),
            None,
            make_tags(&["orchestrator"]),
        );

        let workers = router.sessions_with_tag("worker");
        assert_eq!(workers.len(), 2);
        assert!(workers.contains(&session1));
        assert!(workers.contains(&session2));

        let evaluators = router.sessions_with_tag("evaluator");
        assert_eq!(evaluators.len(), 1);
        assert!(evaluators.contains(&session1));

        let orchestrators = router.sessions_with_tag("orchestrator");
        assert_eq!(orchestrators.len(), 1);
        assert!(orchestrators.contains(&session3));

        let empty = router.sessions_with_tag("nonexistent");
        assert!(empty.is_empty());
    }

    #[tokio::test]
    async fn test_router_unregister_cleans_tags() {
        let mut router = MessageRouter::new();

        let session_id = Uuid::new_v4();
        let _rx = router.register(
            session_id,
            Some("repo1".to_string()),
            None,
            make_tags(&["orchestrator"]),
        );

        assert_eq!(router.sessions_with_tag("orchestrator").len(), 1);

        router.unregister(session_id);

        assert!(router.sessions_with_tag("orchestrator").is_empty());
        assert!(router.get_tags(session_id).is_none());
    }

    #[tokio::test]
    async fn test_router_multi_tag_routing() {
        let mut router = MessageRouter::new();

        let orch_id = Uuid::new_v4();
        let worker_id = Uuid::new_v4();

        // Session with multiple tags
        let mut rx = router.register(
            orch_id,
            Some("repo1".to_string()),
            None,
            make_tags(&["orchestrator", "primary"]),
        );
        let _worker_rx = router.register(
            worker_id,
            Some("repo1".to_string()),
            None,
            make_tags(&["worker"]),
        );

        // Can receive via either tag
        let msg1 = make_msg("via.orchestrator");
        let delivered1 = router
            .route(
                worker_id,
                OrchestrationTarget::Tagged("orchestrator".to_string()),
                msg1,
            )
            .await
            .unwrap();
        assert_eq!(delivered1, 1);
        assert!(rx.recv().await.is_some());

        let msg2 = make_msg("via.primary");
        let delivered2 = router
            .route(
                worker_id,
                OrchestrationTarget::Tagged("primary".to_string()),
                msg2,
            )
            .await
            .unwrap();
        assert_eq!(delivered2, 1);
        assert!(rx.recv().await.is_some());
    }

    #[test]
    fn test_router_error_display() {
        assert_eq!(
            RouterError::NoRepository.to_string(),
            "Session not associated with a repository"
        );
        assert_eq!(
            RouterError::NoRecipients.to_string(),
            "No recipients for message"
        );
    }

    #[test]
    fn test_message_router_default() {
        let router = MessageRouter::default();
        assert!(router.all_sessions().is_empty());
    }
}
