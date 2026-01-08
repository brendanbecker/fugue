use std::collections::HashMap;
use uuid::Uuid;
use ccmux_utils::{CcmuxError, Result};

use super::{Pane, Session, Window};

/// Manages all sessions
#[derive(Debug, Default)]
pub struct SessionManager {
    sessions: HashMap<Uuid, Session>,
    /// Map session name to ID for lookup
    name_to_id: HashMap<String, Uuid>,
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

    /// List all sessions
    pub fn list_sessions(&self) -> Vec<&Session> {
        self.sessions.values().collect()
    }

    /// Get session count
    pub fn session_count(&self) -> usize {
        self.sessions.len()
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
}
