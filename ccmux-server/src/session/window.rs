// Allow unused fields that are reserved for future use
#![allow(dead_code)]

use std::collections::HashMap;
use std::time::SystemTime;
use uuid::Uuid;
use ccmux_protocol::WindowInfo;

use super::Pane;

/// A window containing one or more panes
#[derive(Debug)]
pub struct Window {
    /// Unique window identifier
    id: Uuid,
    /// Parent session ID
    session_id: Uuid,
    /// Window name
    name: String,
    /// Index within the session
    index: usize,
    /// Panes in this window
    panes: HashMap<Uuid, Pane>,
    /// Order of pane IDs
    pane_order: Vec<Uuid>,
    /// Currently active pane
    active_pane_id: Option<Uuid>,
    /// When created
    created_at: SystemTime,
}

impl Window {
    /// Create a new window
    pub fn new(session_id: Uuid, index: usize, name: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            session_id,
            name: name.into(),
            index,
            panes: HashMap::new(),
            pane_order: Vec::new(),
            active_pane_id: None,
            created_at: SystemTime::now(),
        }
    }

    /// Restore a window from persisted state
    ///
    /// Used during crash recovery to recreate window with original ID.
    pub fn restore(
        id: Uuid,
        session_id: Uuid,
        index: usize,
        name: impl Into<String>,
        created_at: u64,
    ) -> Self {
        Self {
            id,
            session_id,
            name: name.into(),
            index,
            panes: HashMap::new(),
            pane_order: Vec::new(),
            active_pane_id: None,
            created_at: SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(created_at),
        }
    }

    /// Add a restored pane to this window
    ///
    /// Used during crash recovery to add panes with preserved IDs.
    pub fn add_restored_pane(&mut self, pane: Pane) {
        let pane_id = pane.id();
        self.panes.insert(pane_id, pane);
        self.pane_order.push(pane_id);
    }

    /// Set active pane ID directly (for restoration)
    pub fn set_active_pane_id(&mut self, pane_id: Option<Uuid>) {
        self.active_pane_id = pane_id;
    }

    /// Get window ID
    pub fn id(&self) -> Uuid {
        self.id
    }

    /// Get session ID
    pub fn session_id(&self) -> Uuid {
        self.session_id
    }

    /// Get window name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Set window name
    pub fn set_name(&mut self, name: impl Into<String>) {
        self.name = name.into();
    }

    /// Get window index
    pub fn index(&self) -> usize {
        self.index
    }

    /// Set window index
    pub fn set_index(&mut self, index: usize) {
        self.index = index;
    }

    /// Get number of panes
    pub fn pane_count(&self) -> usize {
        self.panes.len()
    }

    /// Get active pane ID
    pub fn active_pane_id(&self) -> Option<Uuid> {
        self.active_pane_id
    }

    /// Set active pane
    pub fn set_active_pane(&mut self, pane_id: Uuid) -> bool {
        if self.panes.contains_key(&pane_id) {
            self.active_pane_id = Some(pane_id);
            true
        } else {
            false
        }
    }

    /// Create a new pane in this window
    pub fn create_pane(&mut self) -> &Pane {
        let index = self.panes.len();
        let pane = Pane::new(self.id, index);
        let pane_id = pane.id();

        self.panes.insert(pane_id, pane);
        self.pane_order.push(pane_id);

        // Set as active if first pane
        if self.active_pane_id.is_none() {
            self.active_pane_id = Some(pane_id);
        }

        self.panes.get(&pane_id).unwrap()
    }

    /// Get a pane by ID
    pub fn get_pane(&self, pane_id: Uuid) -> Option<&Pane> {
        self.panes.get(&pane_id)
    }

    /// Get a mutable pane by ID
    pub fn get_pane_mut(&mut self, pane_id: Uuid) -> Option<&mut Pane> {
        self.panes.get_mut(&pane_id)
    }

    /// Remove a pane
    pub fn remove_pane(&mut self, pane_id: Uuid) -> Option<Pane> {
        if let Some(pane) = self.panes.remove(&pane_id) {
            self.pane_order.retain(|&id| id != pane_id);

            // Update active pane if needed
            if self.active_pane_id == Some(pane_id) {
                self.active_pane_id = self.pane_order.first().copied();
            }

            // Reindex remaining panes
            for (i, &id) in self.pane_order.iter().enumerate() {
                if let Some(p) = self.panes.get_mut(&id) {
                    p.set_index(i);
                }
            }

            Some(pane)
        } else {
            None
        }
    }

    /// Iterate over panes
    pub fn panes(&self) -> impl Iterator<Item = &Pane> {
        self.pane_order.iter().filter_map(|id| self.panes.get(id))
    }

    /// Get all pane IDs
    pub fn pane_ids(&self) -> &[Uuid] {
        &self.pane_order
    }

    /// Check if window is empty
    pub fn is_empty(&self) -> bool {
        self.panes.is_empty()
    }

    /// Get creation timestamp as Unix time
    pub fn created_at_unix(&self) -> u64 {
        self.created_at
            .duration_since(std::time::SystemTime::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    }

    /// Convert to protocol WindowInfo
    pub fn to_info(&self) -> WindowInfo {
        WindowInfo {
            id: self.id,
            session_id: self.session_id,
            name: self.name.clone(),
            index: self.index,
            pane_count: self.panes.len(),
            active_pane_id: self.active_pane_id,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_window_creation() {
        let session_id = Uuid::new_v4();
        let window = Window::new(session_id, 0, "main");

        assert_eq!(window.session_id(), session_id);
        assert_eq!(window.name(), "main");
        assert_eq!(window.index(), 0);
        assert!(window.is_empty());
    }

    #[test]
    fn test_window_create_pane() {
        let session_id = Uuid::new_v4();
        let mut window = Window::new(session_id, 0, "main");

        let pane = window.create_pane();
        let pane_id = pane.id();

        assert_eq!(window.pane_count(), 1);
        assert_eq!(window.active_pane_id(), Some(pane_id));
    }

    #[test]
    fn test_window_remove_pane() {
        let session_id = Uuid::new_v4();
        let mut window = Window::new(session_id, 0, "main");

        let pane1 = window.create_pane();
        let pane1_id = pane1.id();
        let pane2 = window.create_pane();
        let pane2_id = pane2.id();

        window.remove_pane(pane1_id);

        assert_eq!(window.pane_count(), 1);
        assert_eq!(window.active_pane_id(), Some(pane2_id));
    }

    #[test]
    fn test_window_id_is_unique() {
        let session_id = Uuid::new_v4();
        let window1 = Window::new(session_id, 0, "main");
        let window2 = Window::new(session_id, 1, "other");

        assert_ne!(window1.id(), window2.id());
    }

    #[test]
    fn test_window_set_name() {
        let session_id = Uuid::new_v4();
        let mut window = Window::new(session_id, 0, "main");

        assert_eq!(window.name(), "main");
        window.set_name("new-name");
        assert_eq!(window.name(), "new-name");
    }

    #[test]
    fn test_window_set_index() {
        let session_id = Uuid::new_v4();
        let mut window = Window::new(session_id, 0, "main");

        assert_eq!(window.index(), 0);
        window.set_index(5);
        assert_eq!(window.index(), 5);
    }

    #[test]
    fn test_window_set_active_pane_success() {
        let session_id = Uuid::new_v4();
        let mut window = Window::new(session_id, 0, "main");

        let pane1 = window.create_pane();
        let pane1_id = pane1.id();
        let pane2 = window.create_pane();
        let pane2_id = pane2.id();

        assert_eq!(window.active_pane_id(), Some(pane1_id));

        let result = window.set_active_pane(pane2_id);
        assert!(result);
        assert_eq!(window.active_pane_id(), Some(pane2_id));
    }

    #[test]
    fn test_window_set_active_pane_nonexistent() {
        let session_id = Uuid::new_v4();
        let mut window = Window::new(session_id, 0, "main");

        let _ = window.create_pane();
        let nonexistent_id = Uuid::new_v4();

        let result = window.set_active_pane(nonexistent_id);
        assert!(!result);
    }

    #[test]
    fn test_window_get_pane() {
        let session_id = Uuid::new_v4();
        let mut window = Window::new(session_id, 0, "main");

        let pane = window.create_pane();
        let pane_id = pane.id();

        let retrieved = window.get_pane(pane_id);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id(), pane_id);
    }

    #[test]
    fn test_window_get_pane_nonexistent() {
        let session_id = Uuid::new_v4();
        let window = Window::new(session_id, 0, "main");

        let nonexistent_id = Uuid::new_v4();
        assert!(window.get_pane(nonexistent_id).is_none());
    }

    #[test]
    fn test_window_get_pane_mut() {
        let session_id = Uuid::new_v4();
        let mut window = Window::new(session_id, 0, "main");

        let pane = window.create_pane();
        let pane_id = pane.id();

        let pane_mut = window.get_pane_mut(pane_id).unwrap();
        pane_mut.resize(100, 50);

        let pane = window.get_pane(pane_id).unwrap();
        assert_eq!(pane.dimensions(), (100, 50));
    }

    #[test]
    fn test_window_panes_iterator() {
        let session_id = Uuid::new_v4();
        let mut window = Window::new(session_id, 0, "main");

        window.create_pane();
        window.create_pane();
        window.create_pane();

        let pane_count = window.panes().count();
        assert_eq!(pane_count, 3);
    }

    #[test]
    fn test_window_panes_iterator_order() {
        let session_id = Uuid::new_v4();
        let mut window = Window::new(session_id, 0, "main");

        let pane1_id = window.create_pane().id();
        let pane2_id = window.create_pane().id();
        let pane3_id = window.create_pane().id();

        let ids: Vec<_> = window.panes().map(|p| p.id()).collect();
        assert_eq!(ids, vec![pane1_id, pane2_id, pane3_id]);
    }

    #[test]
    fn test_window_pane_ids() {
        let session_id = Uuid::new_v4();
        let mut window = Window::new(session_id, 0, "main");

        let pane1_id = window.create_pane().id();
        let pane2_id = window.create_pane().id();

        let ids = window.pane_ids();
        assert_eq!(ids.len(), 2);
        assert_eq!(ids[0], pane1_id);
        assert_eq!(ids[1], pane2_id);
    }

    #[test]
    fn test_window_is_empty() {
        let session_id = Uuid::new_v4();
        let mut window = Window::new(session_id, 0, "main");

        assert!(window.is_empty());

        let pane = window.create_pane();
        let pane_id = pane.id();
        assert!(!window.is_empty());

        window.remove_pane(pane_id);
        assert!(window.is_empty());
    }

    #[test]
    fn test_window_to_info() {
        let session_id = Uuid::new_v4();
        let mut window = Window::new(session_id, 3, "test-window");

        let pane = window.create_pane();
        let pane_id = pane.id();

        let info = window.to_info();

        assert_eq!(info.id, window.id());
        assert_eq!(info.session_id, session_id);
        assert_eq!(info.name, "test-window");
        assert_eq!(info.index, 3);
        assert_eq!(info.pane_count, 1);
        assert_eq!(info.active_pane_id, Some(pane_id));
    }

    #[test]
    fn test_window_to_info_empty() {
        let session_id = Uuid::new_v4();
        let window = Window::new(session_id, 0, "empty");

        let info = window.to_info();

        assert_eq!(info.pane_count, 0);
        assert_eq!(info.active_pane_id, None);
    }

    #[test]
    fn test_window_remove_pane_nonexistent() {
        let session_id = Uuid::new_v4();
        let mut window = Window::new(session_id, 0, "main");

        let nonexistent_id = Uuid::new_v4();
        let result = window.remove_pane(nonexistent_id);
        assert!(result.is_none());
    }

    #[test]
    fn test_window_remove_pane_reindexes() {
        let session_id = Uuid::new_v4();
        let mut window = Window::new(session_id, 0, "main");

        let pane1_id = window.create_pane().id();
        let pane2_id = window.create_pane().id();
        let pane3_id = window.create_pane().id();

        // Remove middle pane
        window.remove_pane(pane2_id);

        // Check indices were updated
        let pane1 = window.get_pane(pane1_id).unwrap();
        let pane3 = window.get_pane(pane3_id).unwrap();

        assert_eq!(pane1.index(), 0);
        assert_eq!(pane3.index(), 1);
    }

    #[test]
    fn test_window_remove_active_pane_updates_active() {
        let session_id = Uuid::new_v4();
        let mut window = Window::new(session_id, 0, "main");

        let pane1_id = window.create_pane().id();
        let pane2_id = window.create_pane().id();

        // Active should be first pane
        assert_eq!(window.active_pane_id(), Some(pane1_id));

        // Remove active pane
        window.remove_pane(pane1_id);

        // Active should update to next available
        assert_eq!(window.active_pane_id(), Some(pane2_id));
    }

    #[test]
    fn test_window_remove_last_pane() {
        let session_id = Uuid::new_v4();
        let mut window = Window::new(session_id, 0, "main");

        let pane_id = window.create_pane().id();
        window.remove_pane(pane_id);

        assert!(window.is_empty());
        assert_eq!(window.active_pane_id(), None);
    }

    #[test]
    fn test_window_debug_format() {
        let session_id = Uuid::new_v4();
        let window = Window::new(session_id, 0, "main");

        let debug_str = format!("{:?}", window);
        assert!(debug_str.contains("Window"));
        assert!(debug_str.contains("main"));
    }

    #[test]
    fn test_window_multiple_panes_active_first() {
        let session_id = Uuid::new_v4();
        let mut window = Window::new(session_id, 0, "main");

        let pane1_id = window.create_pane().id();
        let _ = window.create_pane();
        let _ = window.create_pane();

        // First created pane should be active
        assert_eq!(window.active_pane_id(), Some(pane1_id));
    }

    #[test]
    fn test_window_name_with_special_characters() {
        let session_id = Uuid::new_v4();
        let mut window = Window::new(session_id, 0, "test window!");

        assert_eq!(window.name(), "test window!");

        window.set_name("new-name_123");
        assert_eq!(window.name(), "new-name_123");
    }
}
