use std::collections::HashMap;
use std::path::{Path, PathBuf};
use uuid::Uuid;
use fugue_utils::{CcmuxError, Result};

use super::{MirrorRegistry, Pane, Session, Window};
use crate::orchestration::WorktreeDetector;

/// Manages all sessions
#[derive(Debug, Default)]
pub struct SessionManager {
    sessions: HashMap<Uuid, Session>,
    /// Map session name to ID for lookup
    name_to_id: HashMap<String, Uuid>,
    /// The currently active session (used when no session is explicitly specified)
    active_session_id: Option<Uuid>,
    /// Mirror pane registry for tracking mirror relationships (FEAT-062)
    mirror_registry: MirrorRegistry,
}

impl SessionManager {
    /// Create a new session manager
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a new session
    pub fn create_session(&mut self, name: impl Into<String>) -> Result<&Session> {
        let name = name.into();

        if self.name_to_id.contains_key(&name) {
            return Err(CcmuxError::SessionExists(name));
        }

        let session = Session::new(&name);
        let session_id = session.id();

        self.name_to_id.insert(name, session_id);
        self.sessions.insert(session_id, session);

        Ok(self.sessions.get(&session_id).unwrap())
    }

    /// Add a restored session
    ///
    /// Used during crash recovery to add sessions with preserved IDs.
    pub fn add_restored_session(&mut self, session: Session) {
        let session_id = session.id();
        let name = session.name().to_string();
        self.name_to_id.insert(name, session_id);
        self.sessions.insert(session_id, session);
    }

    /// Get session by ID
    pub fn get_session(&self, session_id: Uuid) -> Option<&Session> {
        self.sessions.get(&session_id)
    }

    /// Get mutable session by ID
    pub fn get_session_mut(&mut self, session_id: Uuid) -> Option<&mut Session> {
        self.sessions.get_mut(&session_id)
    }

    /// Get session by name
    pub fn get_session_by_name(&self, name: &str) -> Option<&Session> {
        self.name_to_id
            .get(name)
            .and_then(|id| self.sessions.get(id))
    }

    /// Remove a session
    pub fn remove_session(&mut self, session_id: Uuid) -> Option<Session> {
        if let Some(session) = self.sessions.remove(&session_id) {
            self.name_to_id.remove(session.name());
            Some(session)
        } else {
            None
        }
    }

    /// Rename a session
    ///
    /// Changes the session's name to the new name. Returns error if:
    /// - The session doesn't exist
    /// - The new name is already in use by another session
    ///
    /// Renaming to the same name is a no-op and succeeds.
    pub fn rename_session(&mut self, session_id: Uuid, new_name: impl Into<String>) -> Result<String> {
        let new_name = new_name.into();

        // Get the session to check it exists and get its current name
        let session = self.sessions.get(&session_id)
            .ok_or_else(|| CcmuxError::SessionNotFound(session_id.to_string()))?;

        let old_name = session.name().to_string();

        // If renaming to same name, it's a no-op
        if old_name == new_name {
            return Ok(old_name);
        }

        // Check if new name is already in use by another session
        if let Some(&existing_id) = self.name_to_id.get(&new_name) {
            if existing_id != session_id {
                return Err(CcmuxError::SessionExists(new_name));
            }
        }

        // Update the name_to_id mapping
        self.name_to_id.remove(&old_name);
        self.name_to_id.insert(new_name.clone(), session_id);

        // Update the session's name
        let session = self.sessions.get_mut(&session_id).unwrap();
        session.set_name(&new_name);

        Ok(old_name)
    }

    /// Get mutable session by name
    pub fn get_session_by_name_mut(&mut self, name: &str) -> Option<&mut Session> {
        self.name_to_id
            .get(name)
            .copied()
            .and_then(|id| self.sessions.get_mut(&id))
    }

    /// List all sessions
    ///
    /// Sessions are returned sorted by creation time (oldest first).
    /// This ensures deterministic ordering when callers use `.first()`.
    pub fn list_sessions(&self) -> Vec<&Session> {
        let mut sessions: Vec<&Session> = self.sessions.values().collect();
        sessions.sort_by_key(|s| s.created_at_millis());
        sessions
    }

    /// Get session count
    pub fn session_count(&self) -> usize {
        self.sessions.len()
    }

    /// Get the active session ID
    ///
    /// Returns the session that should be used when no session is explicitly specified.
    /// Priority order (BUG-034 fix + FEAT-036 session-aware defaults):
    /// 1. Explicitly set active session (via select_session MCP command)
    /// 2. Session with most attached clients (indicates active TUI usage)
    /// 3. If tie on attached clients, prefer most recently created
    /// 4. If no sessions have attached clients, return most recent session
    ///
    /// This ensures MCP commands default to the session the user is actively using,
    /// whether set explicitly via select_session or implicitly via TUI attachment.
    pub fn active_session_id(&self) -> Option<Uuid> {
        if self.sessions.is_empty() {
            return None;
        }

        // First, check if there's an explicitly set active session (BUG-034)
        if let Some(explicit_id) = self.active_session_id {
            // Verify the session still exists
            if self.sessions.contains_key(&explicit_id) {
                return Some(explicit_id);
            }
            // If session was deleted, fall through to heuristics
        }

        // Find sessions with attached clients, sorted by client count (desc) then creation time (desc)
        let mut sessions_with_clients: Vec<_> = self.sessions
            .values()
            .filter(|s| s.attached_clients() > 0)
            .collect();

        if !sessions_with_clients.is_empty() {
            // Sort by attached_clients descending, then by creation time descending (most recent first)
            sessions_with_clients.sort_by(|a, b| {
                b.attached_clients().cmp(&a.attached_clients())
                    .then_with(|| b.created_at_millis().cmp(&a.created_at_millis()))
            });
            return Some(sessions_with_clients[0].id());
        }

        // No sessions have attached clients - fall back to most recent session
        // (list_sessions returns sorted by creation time, oldest first)
        self.list_sessions().last().map(|s| s.id())
    }

    /// Get the active session
    pub fn active_session(&self) -> Option<&Session> {
        self.active_session_id().and_then(|id| self.get_session(id))
    }

    /// Set the active session
    ///
    /// This is called when a TUI client attaches to a session, making it
    /// the default target for MCP commands that don't specify a session.
    pub fn set_active_session(&mut self, session_id: Uuid) {
        if self.sessions.contains_key(&session_id) {
            self.active_session_id = Some(session_id);
        }
    }

    /// Clear the active session
    pub fn clear_active_session(&mut self) {
        self.active_session_id = None;
    }

    /// Find pane by ID across all sessions
    pub fn find_pane(&self, pane_id: Uuid) -> Option<(&Session, &Window, &Pane)> {
        for session in self.sessions.values() {
            for window in session.windows() {
                if let Some(pane) = window.get_pane(pane_id) {
                    return Some((session, window, pane));
                }
            }
        }
        None
    }

    /// Find mutable pane by ID
    pub fn find_pane_mut(&mut self, pane_id: Uuid) -> Option<&mut Pane> {
        // First, find the location with immutable borrows
        let location = self.find_pane(pane_id).map(|(s, w, _)| (s.id(), w.id()));

        // Then use the IDs to get mutable access
        if let Some((session_id, window_id)) = location {
            if let Some(session) = self.sessions.get_mut(&session_id) {
                if let Some(window) = session.get_window_mut(window_id) {
                    return window.get_pane_mut(pane_id);
                }
            }
        }
        None
    }

    /// Find window by ID
    pub fn find_window(&self, window_id: Uuid) -> Option<(&Session, &Window)> {
        for session in self.sessions.values() {
            if let Some(window) = session.get_window(window_id) {
                return Some((session, window));
            }
        }
        None
    }

    /// Split a pane by creating a new pane in the same window
    ///
    /// Creates a new pane in the window containing `source_pane_id`.
    /// The new pane can optionally have a specified cwd.
    ///
    /// Returns `(session_id, window_id, new_pane)` on success.
    ///
    /// # Arguments
    /// * `source_pane_id` - The pane to split from (determines which window)
    /// * `cwd` - Optional working directory for the new pane
    ///
    /// # Errors
    /// Returns `PaneNotFound` if `source_pane_id` doesn't exist.
    pub fn split_pane(
        &mut self,
        source_pane_id: Uuid,
        cwd: Option<String>,
    ) -> Result<(Uuid, Uuid, &Pane)> {
        // Find the session and window containing the source pane
        let (session_id, window_id, source_cwd) = {
            let (session, window, pane) = self.find_pane(source_pane_id)
                .ok_or_else(|| CcmuxError::PaneNotFound(source_pane_id.to_string()))?;
            (session.id(), window.id(), pane.cwd().map(String::from))
        };

        // Get mutable access to create the new pane
        let session = self.sessions.get_mut(&session_id)
            .ok_or_else(|| CcmuxError::SessionNotFound(session_id.to_string()))?;
        let window = session.get_window_mut(window_id)
            .ok_or_else(|| CcmuxError::WindowNotFound(window_id.to_string()))?;

        // Create the new pane
        let new_pane = window.create_pane();
        let new_pane_id = new_pane.id();

        // Set the cwd - use provided cwd, or inherit from source pane
        let effective_cwd = cwd.or(source_cwd);
        if let Some(cwd_value) = effective_cwd {
            let pane_mut = window.get_pane_mut(new_pane_id).unwrap();
            pane_mut.set_cwd(Some(cwd_value));
        }

        // Return reference to the new pane
        let new_pane = window.get_pane(new_pane_id).unwrap();
        Ok((session_id, window_id, new_pane))
    }

    /// Find pane by name/title across all sessions
    pub fn find_pane_by_name(&self, name: &str) -> Option<(&Session, &Window, &Pane)> {
        for session in self.sessions.values() {
            for window in session.windows() {
                for pane in window.panes() {
                    if let Some(title) = pane.title() {
                        if title == name {
                            return Some((session, window, pane));
                        }
                    }
                }
            }
        }
        None
    }

    /// Find mutable pane by name/title
    pub fn find_pane_by_name_mut(&mut self, name: &str) -> Option<&mut Pane> {
        // First find the IDs
        let location = self.find_pane_by_name(name).map(|(s, w, p)| (s.id(), w.id(), p.id()));

        if let Some((session_id, window_id, pane_id)) = location {
            if let Some(session) = self.sessions.get_mut(&session_id) {
                if let Some(window) = session.get_window_mut(window_id) {
                    return window.get_pane_mut(pane_id);
                }
            }
        }
        None
    }

    /// Create a session with worktree detection
    ///
    /// Creates a session and automatically detects and binds any git worktree
    /// context for the given working directory.
    pub fn create_session_in_dir(&mut self, name: impl Into<String>, cwd: &Path) -> Result<&Session> {
        let name = name.into();

        if self.name_to_id.contains_key(&name) {
            return Err(CcmuxError::SessionExists(name));
        }

        let mut session = Session::new(&name);
        let session_id = session.id();

        // Detect worktree context
        if let Some(worktree_root) = WorktreeDetector::get_worktree_root(cwd) {
            let worktrees = WorktreeDetector::list_worktrees(cwd);

            if let Some(worktree) = worktrees.into_iter().find(|w| w.path == worktree_root) {
                let is_orchestrator = worktree.is_main;
                session.set_worktree_with_orchestrator(worktree, is_orchestrator);
            }
        }

        self.name_to_id.insert(name, session_id);
        self.sessions.insert(session_id, session);

        Ok(self.sessions.get(&session_id).unwrap())
    }

    /// Find sessions by worktree path
    pub fn sessions_for_worktree(&self, worktree_path: &Path) -> Vec<&Session> {
        self.sessions
            .values()
            .filter(|s| {
                s.worktree()
                    .map(|w| w.path == worktree_path)
                    .unwrap_or(false)
            })
            .collect()
    }

    /// Find the orchestrator session for a repository
    ///
    /// Returns the session that is marked as orchestrator and belongs to
    /// the same repository as the given path.
    pub fn orchestrator_session(&self, repo_path: &Path) -> Option<&Session> {
        // Get main repo root
        let main_root = WorktreeDetector::get_main_repo_root(repo_path)?;

        self.sessions.values().find(|s| {
            s.is_orchestrator()
                && s.worktree()
                    .map(|w| {
                        WorktreeDetector::get_main_repo_root(&w.path)
                            .map(|r| r == main_root)
                            .unwrap_or(false)
                    })
                    .unwrap_or(false)
        })
    }

    /// Get all sessions grouped by repository
    ///
    /// Returns a map from main repository root path to all sessions
    /// associated with worktrees of that repository.
    pub fn sessions_by_repo(&self) -> HashMap<PathBuf, Vec<&Session>> {
        let mut by_repo: HashMap<PathBuf, Vec<&Session>> = HashMap::new();

        for session in self.sessions.values() {
            if let Some(worktree) = session.worktree() {
                if let Some(main_root) = WorktreeDetector::get_main_repo_root(&worktree.path) {
                    by_repo.entry(main_root).or_default().push(session);
                }
            }
        }

        by_repo
    }

    // ==================== Mirror Registry (FEAT-062) ====================

    /// Get a reference to the mirror registry
    pub fn mirror_registry(&self) -> &MirrorRegistry {
        &self.mirror_registry
    }

    /// Get a mutable reference to the mirror registry
    pub fn mirror_registry_mut(&mut self) -> &mut MirrorRegistry {
        &mut self.mirror_registry
    }

    /// Get all mirror pane IDs for a source pane
    pub fn get_mirrors_for_pane(&self, source_id: Uuid) -> Vec<Uuid> {
        self.mirror_registry.get_mirrors(source_id)
    }

    /// Check if a pane is a mirror
    pub fn is_mirror_pane(&self, pane_id: Uuid) -> bool {
        self.mirror_registry.is_mirror(pane_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manager_create_session() {
        let mut manager = SessionManager::new();

        let session = manager.create_session("work").unwrap();
        assert_eq!(session.name(), "work");
        assert_eq!(manager.session_count(), 1);
    }

    #[test]
    fn test_manager_duplicate_session_name() {
        let mut manager = SessionManager::new();

        manager.create_session("work").unwrap();
        let result = manager.create_session("work");

        assert!(matches!(result, Err(CcmuxError::SessionExists(_))));
    }

    #[test]
    fn test_manager_get_by_name() {
        let mut manager = SessionManager::new();

        manager.create_session("work").unwrap();

        let session = manager.get_session_by_name("work");
        assert!(session.is_some());
        assert_eq!(session.unwrap().name(), "work");
    }

    #[test]
    fn test_manager_find_pane() {
        let mut manager = SessionManager::new();

        let session = manager.create_session("work").unwrap();
        let session_id = session.id();

        let session = manager.get_session_mut(session_id).unwrap();
        let window = session.create_window(None);
        let window_id = window.id();

        let window = session.get_window_mut(window_id).unwrap();
        let pane = window.create_pane();
        let pane_id = pane.id();

        let found = manager.find_pane(pane_id);
        assert!(found.is_some());
    }

    #[test]
    fn test_manager_default() {
        let manager = SessionManager::default();
        assert_eq!(manager.session_count(), 0);
    }

    #[test]
    fn test_manager_get_session() {
        let mut manager = SessionManager::new();

        let session = manager.create_session("work").unwrap();
        let session_id = session.id();

        let retrieved = manager.get_session(session_id);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name(), "work");
    }

    #[test]
    fn test_manager_get_session_nonexistent() {
        let manager = SessionManager::new();

        let nonexistent_id = Uuid::new_v4();
        assert!(manager.get_session(nonexistent_id).is_none());
    }

    #[test]
    fn test_manager_get_session_mut() {
        let mut manager = SessionManager::new();

        let session = manager.create_session("work").unwrap();
        let session_id = session.id();

        let session_mut = manager.get_session_mut(session_id).unwrap();
        session_mut.set_name("renamed");

        let session = manager.get_session(session_id).unwrap();
        assert_eq!(session.name(), "renamed");
    }

    #[test]
    fn test_manager_get_session_by_name_nonexistent() {
        let manager = SessionManager::new();

        assert!(manager.get_session_by_name("nonexistent").is_none());
    }

    #[test]
    fn test_manager_remove_session() {
        let mut manager = SessionManager::new();

        let session = manager.create_session("work").unwrap();
        let session_id = session.id();

        assert_eq!(manager.session_count(), 1);

        let removed = manager.remove_session(session_id);
        assert!(removed.is_some());
        assert_eq!(manager.session_count(), 0);
        assert!(manager.get_session_by_name("work").is_none());
    }

    #[test]
    fn test_manager_remove_session_nonexistent() {
        let mut manager = SessionManager::new();

        let nonexistent_id = Uuid::new_v4();
        let result = manager.remove_session(nonexistent_id);
        assert!(result.is_none());
    }

    #[test]
    fn test_manager_list_sessions() {
        let mut manager = SessionManager::new();

        manager.create_session("work").unwrap();
        manager.create_session("personal").unwrap();
        manager.create_session("other").unwrap();

        let sessions = manager.list_sessions();
        assert_eq!(sessions.len(), 3);
    }

    #[test]
    fn test_manager_list_sessions_ordered_by_creation() {
        let mut manager = SessionManager::new();

        // Create sessions in order
        manager.create_session("first").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(10)); // Ensure different timestamps
        manager.create_session("second").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(10));
        manager.create_session("third").unwrap();

        // List should return in creation order (oldest first)
        let sessions = manager.list_sessions();
        assert_eq!(sessions.len(), 3);
        assert_eq!(sessions[0].name(), "first");
        assert_eq!(sessions[1].name(), "second");
        assert_eq!(sessions[2].name(), "third");

        // .first() should always return the oldest session
        assert_eq!(sessions.first().unwrap().name(), "first");
    }

    #[test]
    fn test_manager_list_sessions_empty() {
        let manager = SessionManager::new();

        let sessions = manager.list_sessions();
        assert!(sessions.is_empty());
    }

    #[test]
    fn test_manager_find_pane_nonexistent() {
        let manager = SessionManager::new();

        let nonexistent_id = Uuid::new_v4();
        assert!(manager.find_pane(nonexistent_id).is_none());
    }

    #[test]
    fn test_manager_find_pane_mut() {
        let mut manager = SessionManager::new();

        let session = manager.create_session("work").unwrap();
        let session_id = session.id();

        let session = manager.get_session_mut(session_id).unwrap();
        let window = session.create_window(None);
        let window_id = window.id();

        let window = session.get_window_mut(window_id).unwrap();
        let pane = window.create_pane();
        let pane_id = pane.id();

        // Get mutable pane and modify
        let pane_mut = manager.find_pane_mut(pane_id).unwrap();
        pane_mut.resize(100, 50);

        // Verify modification
        let (_, _, pane) = manager.find_pane(pane_id).unwrap();
        assert_eq!(pane.dimensions(), (100, 50));
    }

    #[test]
    fn test_manager_find_pane_mut_nonexistent() {
        let mut manager = SessionManager::new();

        let nonexistent_id = Uuid::new_v4();
        assert!(manager.find_pane_mut(nonexistent_id).is_none());
    }

    #[test]
    fn test_manager_find_window() {
        let mut manager = SessionManager::new();

        let session = manager.create_session("work").unwrap();
        let session_id = session.id();

        let session = manager.get_session_mut(session_id).unwrap();
        let window = session.create_window(Some("main".into()));
        let window_id = window.id();

        let found = manager.find_window(window_id);
        assert!(found.is_some());
        let (session, window) = found.unwrap();
        assert_eq!(session.name(), "work");
        assert_eq!(window.name(), "main");
    }

    #[test]
    fn test_manager_find_window_nonexistent() {
        let manager = SessionManager::new();

        let nonexistent_id = Uuid::new_v4();
        assert!(manager.find_window(nonexistent_id).is_none());
    }

    #[test]
    fn test_manager_debug_format() {
        let manager = SessionManager::new();

        let debug_str = format!("{:?}", manager);
        assert!(debug_str.contains("SessionManager"));
    }

    #[test]
    fn test_manager_multiple_sessions() {
        let mut manager = SessionManager::new();

        for i in 0..10 {
            manager.create_session(format!("session-{}", i)).unwrap();
        }

        assert_eq!(manager.session_count(), 10);
    }

    #[test]
    fn test_manager_find_pane_in_multiple_sessions() {
        let mut manager = SessionManager::new();

        // Create multiple sessions with windows and panes
        let session1 = manager.create_session("session1").unwrap();
        let session1_id = session1.id();

        let session2 = manager.create_session("session2").unwrap();
        let session2_id = session2.id();

        // Add panes to session1
        let session = manager.get_session_mut(session1_id).unwrap();
        let window = session.create_window(None);
        let window_id = window.id();
        let window = session.get_window_mut(window_id).unwrap();
        window.create_pane();

        // Add panes to session2
        let session = manager.get_session_mut(session2_id).unwrap();
        let window = session.create_window(None);
        let window_id = window.id();
        let window = session.get_window_mut(window_id).unwrap();
        let pane = window.create_pane();
        let pane_id = pane.id();

        // Find pane in session2
        let found = manager.find_pane(pane_id);
        assert!(found.is_some());
        let (session, _, _) = found.unwrap();
        assert_eq!(session.name(), "session2");
    }

    #[test]
    fn test_manager_session_name_uniqueness() {
        let mut manager = SessionManager::new();

        manager.create_session("unique").unwrap();

        // Try to create with same name
        let result = manager.create_session("unique");
        assert!(result.is_err());

        // Different name should work
        let result = manager.create_session("different");
        assert!(result.is_ok());
    }

    #[test]
    fn test_manager_remove_and_recreate_session() {
        let mut manager = SessionManager::new();

        let session = manager.create_session("recyclable").unwrap();
        let session_id = session.id();

        manager.remove_session(session_id);
        assert_eq!(manager.session_count(), 0);

        // Should be able to recreate with same name
        let result = manager.create_session("recyclable");
        assert!(result.is_ok());
        assert_eq!(manager.session_count(), 1);
    }

    #[test]
    fn test_manager_find_window_in_correct_session() {
        let mut manager = SessionManager::new();

        let session1 = manager.create_session("session1").unwrap();
        let session1_id = session1.id();

        let session2 = manager.create_session("session2").unwrap();
        let session2_id = session2.id();

        // Create window in session2
        let session = manager.get_session_mut(session2_id).unwrap();
        let window = session.create_window(Some("target".into()));
        let window_id = window.id();

        // Create window in session1
        let session = manager.get_session_mut(session1_id).unwrap();
        session.create_window(Some("other".into()));

        // Find should return session2
        let (session, window) = manager.find_window(window_id).unwrap();
        assert_eq!(session.name(), "session2");
        assert_eq!(window.name(), "target");
    }

    // ==================== Find Pane by Name Tests ====================

    #[test]
    fn test_manager_find_pane_by_name() {
        let mut manager = SessionManager::new();

        let session = manager.create_session("work").unwrap();
        let session_id = session.id();

        let session = manager.get_session_mut(session_id).unwrap();
        let window = session.create_window(None);
        let window_id = window.id();

        let window = session.get_window_mut(window_id).unwrap();
        let pane = window.create_pane();
        let pane_id = pane.id();

        // Set a title on the pane
        let pane_mut = window.get_pane_mut(pane_id).unwrap();
        pane_mut.set_title(Some("worker-3".to_string()));

        // Find pane by name
        let found = manager.find_pane_by_name("worker-3");
        assert!(found.is_some());
        let (_, _, pane) = found.unwrap();
        assert_eq!(pane.id(), pane_id);
    }

    #[test]
    fn test_manager_find_pane_by_name_nonexistent() {
        let manager = SessionManager::new();

        let found = manager.find_pane_by_name("nonexistent");
        assert!(found.is_none());
    }

    #[test]
    fn test_manager_find_pane_by_name_no_title() {
        let mut manager = SessionManager::new();

        let session = manager.create_session("work").unwrap();
        let session_id = session.id();

        let session = manager.get_session_mut(session_id).unwrap();
        let window = session.create_window(None);
        let window_id = window.id();

        let window = session.get_window_mut(window_id).unwrap();
        window.create_pane(); // Pane without title

        // Can't find a pane with no title
        let found = manager.find_pane_by_name("anything");
        assert!(found.is_none());
    }

    #[test]
    fn test_manager_find_pane_by_name_mut() {
        let mut manager = SessionManager::new();

        let session = manager.create_session("work").unwrap();
        let session_id = session.id();

        let session = manager.get_session_mut(session_id).unwrap();
        let window = session.create_window(None);
        let window_id = window.id();

        let window = session.get_window_mut(window_id).unwrap();
        let pane = window.create_pane();
        let pane_id = pane.id();

        // Set a title on the pane
        let pane_mut = window.get_pane_mut(pane_id).unwrap();
        pane_mut.set_title(Some("worker-3".to_string()));

        // Find and modify pane
        let pane_mut = manager.find_pane_by_name_mut("worker-3").unwrap();
        pane_mut.resize(100, 50);

        // Verify modification
        let (_, _, pane) = manager.find_pane_by_name("worker-3").unwrap();
        assert_eq!(pane.dimensions(), (100, 50));
    }

    #[test]
    fn test_manager_find_pane_by_name_mut_nonexistent() {
        let mut manager = SessionManager::new();

        let found = manager.find_pane_by_name_mut("nonexistent");
        assert!(found.is_none());
    }

    #[test]
    fn test_manager_find_pane_by_name_multiple_sessions() {
        let mut manager = SessionManager::new();

        let session1 = manager.create_session("session1").unwrap();
        let session1_id = session1.id();

        let session2 = manager.create_session("session2").unwrap();
        let session2_id = session2.id();

        // Create pane in session1
        let session = manager.get_session_mut(session1_id).unwrap();
        let window = session.create_window(None);
        let window_id = window.id();
        let window = session.get_window_mut(window_id).unwrap();
        let pane = window.create_pane();
        let pane_id = pane.id();
        let pane_mut = window.get_pane_mut(pane_id).unwrap();
        pane_mut.set_title(Some("other-worker".to_string()));

        // Create pane in session2 with target name
        let session = manager.get_session_mut(session2_id).unwrap();
        let window = session.create_window(None);
        let window_id = window.id();
        let window = session.get_window_mut(window_id).unwrap();
        let pane = window.create_pane();
        let pane_id = pane.id();
        let pane_mut = window.get_pane_mut(pane_id).unwrap();
        pane_mut.set_title(Some("target-worker".to_string()));

        // Find should find the correct pane
        let found = manager.find_pane_by_name("target-worker");
        assert!(found.is_some());
        let (session, _, pane) = found.unwrap();
        assert_eq!(pane.title(), Some("target-worker"));
        assert_eq!(session.name(), "session2");
    }

    // ==================== Worktree Binding Tests ====================

    #[test]
    fn test_create_session_in_dir() {
        use std::env;

        let mut manager = SessionManager::new();
        let cwd = env::current_dir().unwrap();

        let session = manager.create_session_in_dir("test", &cwd).unwrap();

        // Session should be created
        assert_eq!(session.name(), "test");

        // In a git repo, worktree should be detected
        // Note: This test assumes we're in a git repo
        if session.worktree().is_some() {
            let wt = session.worktree().unwrap();
            assert!(wt.path.exists());
        }
    }

    #[test]
    fn test_create_session_in_dir_duplicate_name() {
        use std::env;

        let mut manager = SessionManager::new();
        let cwd = env::current_dir().unwrap();

        manager.create_session_in_dir("unique", &cwd).unwrap();
        let result = manager.create_session_in_dir("unique", &cwd);

        assert!(matches!(result, Err(CcmuxError::SessionExists(_))));
    }

    #[test]
    fn test_sessions_for_worktree() {
        use crate::orchestration::WorktreeInfo;

        let mut manager = SessionManager::new();

        // Create sessions with worktrees manually
        let session1 = manager.create_session("session1").unwrap();
        let session1_id = session1.id();

        let session2 = manager.create_session("session2").unwrap();
        let session2_id = session2.id();

        // Set worktrees
        let worktree_path = PathBuf::from("/test/repo");
        let worktree = WorktreeInfo {
            path: worktree_path.clone(),
            branch: Some("main".to_string()),
            head: "abc123".to_string(),
            is_main: true,
        };

        manager.get_session_mut(session1_id).unwrap().set_worktree_with_orchestrator(worktree.clone(), true);
        manager.get_session_mut(session2_id).unwrap().set_worktree_with_orchestrator(worktree, false);

        // Both sessions should be found for the worktree
        let sessions = manager.sessions_for_worktree(&worktree_path);
        assert_eq!(sessions.len(), 2);
    }

    #[test]
    fn test_sessions_for_worktree_no_match() {
        let manager = SessionManager::new();

        let sessions = manager.sessions_for_worktree(Path::new("/nonexistent/path"));
        assert!(sessions.is_empty());
    }

    #[test]
    fn test_sessions_by_repo_empty() {
        let manager = SessionManager::new();

        let by_repo = manager.sessions_by_repo();
        assert!(by_repo.is_empty());
    }

    #[test]
    fn test_orchestrator_detection() {
        use std::env;

        let mut manager = SessionManager::new();
        let cwd = env::current_dir().unwrap();

        let session = manager.create_session_in_dir("main", &cwd).unwrap();

        // If we're in the main worktree, session should be orchestrator
        if session.worktree().map(|w| w.is_main).unwrap_or(false) {
            assert!(session.is_orchestrator());
        }
    }

    // ==================== Split Pane Tests ====================

    #[test]
    fn test_split_pane_basic() {
        let mut manager = SessionManager::new();

        // Create a session with a window and pane
        let session = manager.create_session("work").unwrap();
        let session_id = session.id();

        let session = manager.get_session_mut(session_id).unwrap();
        let window = session.create_window(None);
        let window_id = window.id();

        let window = session.get_window_mut(window_id).unwrap();
        let source_pane = window.create_pane();
        let source_pane_id = source_pane.id();

        // Split the pane
        let (result_session_id, result_window_id, new_pane) =
            manager.split_pane(source_pane_id, None).unwrap();

        // Verify the new pane is in the same session/window
        assert_eq!(result_session_id, session_id);
        assert_eq!(result_window_id, window_id);
        assert_ne!(new_pane.id(), source_pane_id);

        // Verify window now has 2 panes
        let (_, window) = manager.find_window(window_id).unwrap();
        assert_eq!(window.pane_count(), 2);
    }

    #[test]
    fn test_split_pane_with_cwd() {
        let mut manager = SessionManager::new();

        // Create a session with a window and pane
        let session = manager.create_session("work").unwrap();
        let session_id = session.id();

        let session = manager.get_session_mut(session_id).unwrap();
        let window = session.create_window(None);
        let window_id = window.id();

        let window = session.get_window_mut(window_id).unwrap();
        let source_pane = window.create_pane();
        let source_pane_id = source_pane.id();

        // Split with specific cwd
        let (_, _, new_pane) =
            manager.split_pane(source_pane_id, Some("/custom/path".to_string())).unwrap();

        assert_eq!(new_pane.cwd(), Some("/custom/path"));
    }

    #[test]
    fn test_split_pane_inherits_cwd() {
        let mut manager = SessionManager::new();

        // Create a session with a window and pane
        let session = manager.create_session("work").unwrap();
        let session_id = session.id();

        let session = manager.get_session_mut(session_id).unwrap();
        let window = session.create_window(None);
        let window_id = window.id();

        let window = session.get_window_mut(window_id).unwrap();
        let source_pane = window.create_pane();
        let source_pane_id = source_pane.id();

        // Set cwd on source pane
        let source_pane_mut = window.get_pane_mut(source_pane_id).unwrap();
        source_pane_mut.set_cwd(Some("/source/cwd".to_string()));

        // Split without cwd - should inherit from source
        let (_, _, new_pane) = manager.split_pane(source_pane_id, None).unwrap();

        assert_eq!(new_pane.cwd(), Some("/source/cwd"));
    }

    #[test]
    fn test_split_pane_explicit_cwd_overrides_inherited() {
        let mut manager = SessionManager::new();

        // Create a session with a window and pane
        let session = manager.create_session("work").unwrap();
        let session_id = session.id();

        let session = manager.get_session_mut(session_id).unwrap();
        let window = session.create_window(None);
        let window_id = window.id();

        let window = session.get_window_mut(window_id).unwrap();
        let source_pane = window.create_pane();
        let source_pane_id = source_pane.id();

        // Set cwd on source pane
        let source_pane_mut = window.get_pane_mut(source_pane_id).unwrap();
        source_pane_mut.set_cwd(Some("/source/cwd".to_string()));

        // Split with explicit cwd - should override inherited
        let (_, _, new_pane) =
            manager.split_pane(source_pane_id, Some("/explicit/cwd".to_string())).unwrap();

        assert_eq!(new_pane.cwd(), Some("/explicit/cwd"));
    }

    #[test]
    fn test_split_pane_nonexistent_source() {
        let mut manager = SessionManager::new();

        let nonexistent_id = Uuid::new_v4();
        let result = manager.split_pane(nonexistent_id, None);

        assert!(matches!(result, Err(CcmuxError::PaneNotFound(_))));
    }

    #[test]
    fn test_split_pane_multiple_times() {
        let mut manager = SessionManager::new();

        // Create a session with a window and pane
        let session = manager.create_session("work").unwrap();
        let session_id = session.id();

        let session = manager.get_session_mut(session_id).unwrap();
        let window = session.create_window(None);
        let window_id = window.id();

        let window = session.get_window_mut(window_id).unwrap();
        let source_pane = window.create_pane();
        let source_pane_id = source_pane.id();

        // Split multiple times
        let (_, _, pane2) = manager.split_pane(source_pane_id, None).unwrap();
        let pane2_id = pane2.id();
        let (_, _, _pane3) = manager.split_pane(pane2_id, None).unwrap();
        let (_, _, _pane4) = manager.split_pane(source_pane_id, None).unwrap();

        // Verify window now has 4 panes
        let (_, window) = manager.find_window(window_id).unwrap();
        assert_eq!(window.pane_count(), 4);

        // All panes should be different
        let pane_ids: Vec<_> = window.panes().map(|p| p.id()).collect();
        assert_eq!(pane_ids.len(), 4);
        let unique_ids: std::collections::HashSet<_> = pane_ids.iter().collect();
        assert_eq!(unique_ids.len(), 4);
    }

    // ==================== Rename Session Tests ====================

    #[test]
    fn test_rename_session_basic() {
        let mut manager = SessionManager::new();

        let session = manager.create_session("original").unwrap();
        let session_id = session.id();

        let old_name = manager.rename_session(session_id, "renamed").unwrap();
        assert_eq!(old_name, "original");

        // Session should have new name
        let session = manager.get_session(session_id).unwrap();
        assert_eq!(session.name(), "renamed");

        // Old name should not work
        assert!(manager.get_session_by_name("original").is_none());

        // New name should work
        assert!(manager.get_session_by_name("renamed").is_some());
    }

    #[test]
    fn test_rename_session_same_name() {
        let mut manager = SessionManager::new();

        let session = manager.create_session("same").unwrap();
        let session_id = session.id();

        // Renaming to same name should be no-op
        let old_name = manager.rename_session(session_id, "same").unwrap();
        assert_eq!(old_name, "same");

        let session = manager.get_session(session_id).unwrap();
        assert_eq!(session.name(), "same");
    }

    #[test]
    fn test_rename_session_duplicate_name() {
        let mut manager = SessionManager::new();

        manager.create_session("session1").unwrap();
        let session2 = manager.create_session("session2").unwrap();
        let session2_id = session2.id();

        // Try to rename session2 to session1's name
        let result = manager.rename_session(session2_id, "session1");
        assert!(matches!(result, Err(CcmuxError::SessionExists(_))));

        // session2 should still have its original name
        let session = manager.get_session(session2_id).unwrap();
        assert_eq!(session.name(), "session2");
    }

    #[test]
    fn test_rename_session_not_found() {
        let mut manager = SessionManager::new();

        let nonexistent_id = Uuid::new_v4();
        let result = manager.rename_session(nonexistent_id, "new-name");
        assert!(matches!(result, Err(CcmuxError::SessionNotFound(_))));
    }

    #[test]
    fn test_rename_session_updates_name_lookup() {
        let mut manager = SessionManager::new();

        let session = manager.create_session("lookup-test").unwrap();
        let session_id = session.id();

        manager.rename_session(session_id, "new-lookup-test").unwrap();

        // Lookup by new name should return the same session
        let found = manager.get_session_by_name("new-lookup-test").unwrap();
        assert_eq!(found.id(), session_id);
    }

    #[test]
    fn test_get_session_by_name_mut() {
        let mut manager = SessionManager::new();

        let session = manager.create_session("mutable").unwrap();
        let session_id = session.id();

        // Get mutable and modify
        let session_mut = manager.get_session_by_name_mut("mutable").unwrap();
        session_mut.create_window(Some("test-window".to_string()));

        // Verify modification
        let session = manager.get_session(session_id).unwrap();
        assert_eq!(session.window_count(), 1);
    }

    #[test]
    fn test_get_session_by_name_mut_nonexistent() {
        let mut manager = SessionManager::new();

        assert!(manager.get_session_by_name_mut("nonexistent").is_none());
    }

    // ==================== FEAT-036: Active Session Tests ====================

    #[test]
    fn test_active_session_prefers_attached_clients() {
        let mut manager = SessionManager::new();

        // Create two sessions
        let session1 = manager.create_session("orphaned").unwrap();
        let session1_id = session1.id();

        std::thread::sleep(std::time::Duration::from_millis(10));

        let session2 = manager.create_session("active").unwrap();
        let session2_id = session2.id();

        // Attach a client to session2
        manager.get_session_mut(session2_id).unwrap().attach_client();

        // Active session should be session2 (has attached client)
        assert_eq!(manager.active_session_id(), Some(session2_id));

        // session1 has no attached clients, but is older
        let session1 = manager.get_session(session1_id).unwrap();
        assert_eq!(session1.attached_clients(), 0);
    }

    #[test]
    fn test_active_session_most_attached_clients_wins() {
        let mut manager = SessionManager::new();

        // Create two sessions
        let session1 = manager.create_session("session1").unwrap();
        let session1_id = session1.id();

        std::thread::sleep(std::time::Duration::from_millis(10));

        let session2 = manager.create_session("session2").unwrap();
        let session2_id = session2.id();

        // Attach 2 clients to session1, 1 client to session2
        manager.get_session_mut(session1_id).unwrap().attach_client();
        manager.get_session_mut(session1_id).unwrap().attach_client();
        manager.get_session_mut(session2_id).unwrap().attach_client();

        // Active session should be session1 (has more attached clients)
        assert_eq!(manager.active_session_id(), Some(session1_id));
    }

    #[test]
    fn test_active_session_tie_prefers_recent() {
        let mut manager = SessionManager::new();

        // Create two sessions with gap for timestamp difference
        let session1 = manager.create_session("session1").unwrap();
        let session1_id = session1.id();

        std::thread::sleep(std::time::Duration::from_millis(10));

        let session2 = manager.create_session("session2").unwrap();
        let session2_id = session2.id();

        // Attach 1 client to each (tie)
        manager.get_session_mut(session1_id).unwrap().attach_client();
        manager.get_session_mut(session2_id).unwrap().attach_client();

        // Active session should be session2 (more recent, same attached count)
        assert_eq!(manager.active_session_id(), Some(session2_id));
    }

    #[test]
    fn test_active_session_no_attached_clients_falls_back_to_recent() {
        let mut manager = SessionManager::new();

        // Create two sessions with no attached clients
        let _session1 = manager.create_session("session1").unwrap();

        std::thread::sleep(std::time::Duration::from_millis(10));

        let session2 = manager.create_session("session2").unwrap();
        let session2_id = session2.id();

        // No attached clients - should fall back to most recent session
        assert_eq!(manager.active_session_id(), Some(session2_id));
    }

    #[test]
    fn test_active_session_empty() {
        let manager = SessionManager::new();
        assert_eq!(manager.active_session_id(), None);
    }

    #[test]
    fn test_active_session_detach_updates_selection() {
        let mut manager = SessionManager::new();

        // Create two sessions
        let session1 = manager.create_session("session1").unwrap();
        let session1_id = session1.id();

        std::thread::sleep(std::time::Duration::from_millis(10));

        let session2 = manager.create_session("session2").unwrap();
        let session2_id = session2.id();

        // Attach clients to session1
        manager.get_session_mut(session1_id).unwrap().attach_client();

        // Active should be session1
        assert_eq!(manager.active_session_id(), Some(session1_id));

        // Detach client from session1
        manager.get_session_mut(session1_id).unwrap().detach_client();

        // Now no attached clients - should fall back to most recent (session2)
        assert_eq!(manager.active_session_id(), Some(session2_id));
    }

    // ==================== BUG-034: Explicit Session Selection Tests ====================

    #[test]
    fn test_bug034_set_active_session_takes_priority() {
        let mut manager = SessionManager::new();

        // Create two sessions
        let session1 = manager.create_session("session1").unwrap();
        let session1_id = session1.id();

        std::thread::sleep(std::time::Duration::from_millis(10));

        let session2 = manager.create_session("session2").unwrap();
        let session2_id = session2.id();

        // Without explicit selection, should return session2 (most recent, no attached clients)
        assert_eq!(manager.active_session_id(), Some(session2_id));

        // Explicitly select session1
        manager.set_active_session(session1_id);

        // Now should return session1 (explicitly selected, even though session2 is more recent)
        assert_eq!(manager.active_session_id(), Some(session1_id));
    }

    #[test]
    fn test_bug034_explicit_selection_overrides_attached_clients() {
        let mut manager = SessionManager::new();

        // Create two sessions
        let session1 = manager.create_session("session1").unwrap();
        let session1_id = session1.id();

        let session2 = manager.create_session("session2").unwrap();
        let session2_id = session2.id();

        // Attach multiple clients to session2
        manager.get_session_mut(session2_id).unwrap().attach_client();
        manager.get_session_mut(session2_id).unwrap().attach_client();

        // Without explicit selection, session2 should be active (has most attached clients)
        assert_eq!(manager.active_session_id(), Some(session2_id));

        // Explicitly select session1
        manager.set_active_session(session1_id);

        // Now should return session1 (explicit selection takes priority over attached clients)
        assert_eq!(manager.active_session_id(), Some(session1_id));
    }

    #[test]
    fn test_bug034_clear_active_session_falls_back_to_heuristics() {
        let mut manager = SessionManager::new();

        // Create two sessions
        let session1 = manager.create_session("session1").unwrap();
        let session1_id = session1.id();

        std::thread::sleep(std::time::Duration::from_millis(10));

        let session2 = manager.create_session("session2").unwrap();
        let session2_id = session2.id();

        // Attach client to session1
        manager.get_session_mut(session1_id).unwrap().attach_client();

        // Explicitly select session2
        manager.set_active_session(session2_id);
        assert_eq!(manager.active_session_id(), Some(session2_id));

        // Clear explicit selection
        manager.clear_active_session();

        // Should fall back to session1 (has attached clients)
        assert_eq!(manager.active_session_id(), Some(session1_id));
    }

    #[test]
    fn test_bug034_deleted_session_falls_back_to_heuristics() {
        let mut manager = SessionManager::new();

        // Create two sessions
        let session1 = manager.create_session("session1").unwrap();
        let session1_id = session1.id();

        std::thread::sleep(std::time::Duration::from_millis(10));

        let session2 = manager.create_session("session2").unwrap();
        let session2_id = session2.id();

        // Explicitly select session1
        manager.set_active_session(session1_id);
        assert_eq!(manager.active_session_id(), Some(session1_id));

        // Delete session1
        manager.remove_session(session1_id);

        // Should fall back to session2 (session1 no longer exists)
        assert_eq!(manager.active_session_id(), Some(session2_id));
    }

    #[test]
    fn test_bug034_set_active_session_invalid_id() {
        let mut manager = SessionManager::new();

        // Create a session
        let session1 = manager.create_session("session1").unwrap();
        let session1_id = session1.id();

        // Try to set an invalid session as active
        let invalid_id = Uuid::new_v4();
        manager.set_active_session(invalid_id);

        // Should fall back to the existing session
        assert_eq!(manager.active_session_id(), Some(session1_id));
    }
}
