use std::collections::HashMap;
use std::time::SystemTime;
use uuid::Uuid;
use ccmux_protocol::SessionInfo;

use super::Window;

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
        }
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

    /// Get creation timestamp as Unix time
    pub fn created_at_unix(&self) -> u64 {
        self.created_at
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_secs())
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
}
