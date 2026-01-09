//! Message routing for cross-session communication

use std::collections::HashMap;
use tokio::sync::mpsc;
use uuid::Uuid;

use ccmux_protocol::{OrchestrationMessage, OrchestrationTarget};

/// Sender for orchestration messages (from_session_id, message)
pub type MessageSender = mpsc::Sender<(Uuid, OrchestrationMessage)>;
/// Receiver for orchestration messages
pub type MessageReceiver = mpsc::Receiver<(Uuid, OrchestrationMessage)>;

/// Routes messages between sessions
pub struct MessageRouter {
    /// Registered session senders
    sessions: HashMap<Uuid, MessageSender>,
    /// Session to repository mapping
    session_repos: HashMap<Uuid, String>,
    /// Session to worktree path mapping
    session_worktrees: HashMap<Uuid, String>,
    /// Orchestrator session per repo
    orchestrators: HashMap<String, Uuid>,
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
            orchestrators: HashMap::new(),
        }
    }

    /// Register a session with the router
    ///
    /// Returns a receiver for incoming orchestration messages.
    pub fn register(
        &mut self,
        session_id: Uuid,
        repo_id: Option<String>,
        worktree_path: Option<String>,
        is_orchestrator: bool,
    ) -> MessageReceiver {
        let (tx, rx) = mpsc::channel(100);
        self.sessions.insert(session_id, tx);

        if let Some(repo) = repo_id {
            self.session_repos.insert(session_id, repo.clone());
            if is_orchestrator {
                self.orchestrators.insert(repo, session_id);
            }
        }

        if let Some(worktree) = worktree_path {
            self.session_worktrees.insert(session_id, worktree);
        }

        rx
    }

    /// Unregister a session from the router
    pub fn unregister(&mut self, session_id: Uuid) {
        self.sessions.remove(&session_id);
        self.session_worktrees.remove(&session_id);
        if let Some(repo) = self.session_repos.remove(&session_id) {
            if self.orchestrators.get(&repo) == Some(&session_id) {
                self.orchestrators.remove(&repo);
            }
        }
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
            OrchestrationTarget::Orchestrator => {
                let repo = repo.ok_or(RouterError::NoRepository)?;
                self.orchestrators
                    .get(repo)
                    .copied()
                    .into_iter()
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

    /// Get the orchestrator for a repository
    pub fn get_orchestrator(&self, repo_id: &str) -> Option<Uuid> {
        self.orchestrators.get(repo_id).copied()
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
    use ccmux_protocol::WorkerStatus;

    #[tokio::test]
    async fn test_router_register_unregister() {
        let mut router = MessageRouter::new();

        let session_id = Uuid::new_v4();
        let _rx = router.register(session_id, Some("repo1".to_string()), None, false);

        assert!(router.is_registered(session_id));

        router.unregister(session_id);
        assert!(!router.is_registered(session_id));
    }

    #[tokio::test]
    async fn test_router_orchestrator_registration() {
        let mut router = MessageRouter::new();

        let orch_id = Uuid::new_v4();
        let _rx = router.register(orch_id, Some("repo1".to_string()), None, true);

        assert_eq!(router.get_orchestrator("repo1"), Some(orch_id));
    }

    #[tokio::test]
    async fn test_router_send_to_orchestrator() {
        let mut router = MessageRouter::new();

        let orch_id = Uuid::new_v4();
        let worker_id = Uuid::new_v4();

        let mut orch_rx = router.register(orch_id, Some("repo1".to_string()), None, true);
        let _worker_rx = router.register(worker_id, Some("repo1".to_string()), None, false);

        let msg = OrchestrationMessage::StatusUpdate {
            session_id: worker_id,
            status: WorkerStatus::Idle,
            message: None,
        };

        let delivered = router
            .route(worker_id, OrchestrationTarget::Orchestrator, msg.clone())
            .await
            .unwrap();

        assert_eq!(delivered, 1);

        let (from, received) = orch_rx.recv().await.unwrap();
        assert_eq!(from, worker_id);
        assert_eq!(received, msg);
    }

    #[tokio::test]
    async fn test_router_send_to_specific_session() {
        let mut router = MessageRouter::new();

        let session1 = Uuid::new_v4();
        let session2 = Uuid::new_v4();

        let _rx1 = router.register(session1, Some("repo1".to_string()), None, false);
        let mut rx2 = router.register(session2, Some("repo1".to_string()), None, false);

        let msg = OrchestrationMessage::TaskAssignment {
            task_id: Uuid::new_v4(),
            description: "Fix the bug".to_string(),
            files: vec!["src/main.rs".to_string()],
        };

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

        let _rx1 = router.register(session1, Some("repo1".to_string()), None, true);
        let mut rx2 = router.register(session2, Some("repo1".to_string()), None, false);
        let mut rx3 = router.register(session3, Some("repo1".to_string()), None, false);

        let msg = OrchestrationMessage::Broadcast {
            from_session_id: session1,
            message: "Hello all".to_string(),
        };

        let delivered = router
            .route(session1, OrchestrationTarget::Broadcast, msg)
            .await
            .unwrap();

        assert_eq!(delivered, 2); // Broadcast doesn't send to self

        // Both should receive
        assert!(rx2.recv().await.is_some());
        assert!(rx3.recv().await.is_some());
    }

    #[tokio::test]
    async fn test_router_broadcast_different_repos() {
        let mut router = MessageRouter::new();

        let session1 = Uuid::new_v4();
        let session2 = Uuid::new_v4();
        let session3 = Uuid::new_v4();

        let _rx1 = router.register(session1, Some("repo1".to_string()), None, false);
        let mut rx2 = router.register(session2, Some("repo1".to_string()), None, false);
        let _rx3 = router.register(session3, Some("repo2".to_string()), None, false); // Different repo

        let msg = OrchestrationMessage::SyncRequest;

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
            false,
        );
        let mut rx2 = router.register(
            session2,
            Some("repo1".to_string()),
            Some("/repo/worktree1".to_string()),
            false,
        );
        let _rx3 = router.register(
            session3,
            Some("repo1".to_string()),
            Some("/repo/worktree2".to_string()),
            false,
        );

        let msg = OrchestrationMessage::StatusUpdate {
            session_id: session1,
            status: WorkerStatus::Working,
            message: Some("On it".to_string()),
        };

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
        let _rx = router.register(session_id, None, None, false);

        let msg = OrchestrationMessage::SyncRequest;

        let result = router
            .route(session_id, OrchestrationTarget::Orchestrator, msg)
            .await;

        assert_eq!(result, Err(RouterError::NoRepository));
    }

    #[tokio::test]
    async fn test_router_no_recipients_error() {
        let mut router = MessageRouter::new();

        let session_id = Uuid::new_v4();
        let _rx = router.register(session_id, Some("repo1".to_string()), None, false);

        let nonexistent = Uuid::new_v4();
        let msg = OrchestrationMessage::SyncRequest;

        let result = router
            .route(session_id, OrchestrationTarget::Session(nonexistent), msg)
            .await;

        assert_eq!(result, Err(RouterError::NoRecipients));
    }

    #[tokio::test]
    async fn test_router_no_orchestrator() {
        let mut router = MessageRouter::new();

        let session_id = Uuid::new_v4();
        // Register without orchestrator
        let _rx = router.register(session_id, Some("repo1".to_string()), None, false);

        let msg = OrchestrationMessage::StatusUpdate {
            session_id,
            status: WorkerStatus::Idle,
            message: None,
        };

        let result = router
            .route(session_id, OrchestrationTarget::Orchestrator, msg)
            .await;

        assert_eq!(result, Err(RouterError::NoRecipients));
    }

    #[tokio::test]
    async fn test_router_empty_broadcast_ok() {
        let mut router = MessageRouter::new();

        let session_id = Uuid::new_v4();
        let _rx = router.register(session_id, Some("repo1".to_string()), None, false);

        let msg = OrchestrationMessage::SyncRequest;

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

        let _rx1 = router.register(session1, Some("repo1".to_string()), None, false);
        let _rx2 = router.register(session2, Some("repo1".to_string()), None, false);
        let _rx3 = router.register(session3, Some("repo2".to_string()), None, false);

        let repo1_sessions = router.sessions_in_repo("repo1");
        assert_eq!(repo1_sessions.len(), 2);
        assert!(repo1_sessions.contains(&session1));
        assert!(repo1_sessions.contains(&session2));

        let repo2_sessions = router.sessions_in_repo("repo2");
        assert_eq!(repo2_sessions.len(), 1);
        assert!(repo2_sessions.contains(&session3));
    }

    #[tokio::test]
    async fn test_router_unregister_orchestrator() {
        let mut router = MessageRouter::new();

        let orch_id = Uuid::new_v4();
        let _rx = router.register(orch_id, Some("repo1".to_string()), None, true);

        assert_eq!(router.get_orchestrator("repo1"), Some(orch_id));

        router.unregister(orch_id);

        assert_eq!(router.get_orchestrator("repo1"), None);
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
