use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Window information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WindowInfo {
    pub id: Uuid,
    pub session_id: Uuid,
    pub name: String,
    pub index: usize,
    pub pane_count: usize,
    pub active_pane_id: Option<Uuid>,
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== WindowInfo Tests ====================

    #[test]
    fn test_window_info_minimal() {
        let window = WindowInfo {
            id: Uuid::new_v4(),
            session_id: Uuid::new_v4(),
            name: "main".to_string(),
            index: 0,
            pane_count: 1,
            active_pane_id: None,
        };

        assert_eq!(window.name, "main");
        assert_eq!(window.index, 0);
        assert_eq!(window.pane_count, 1);
        assert!(window.active_pane_id.is_none());
    }

    #[test]
    fn test_window_info_with_active_pane() {
        let pane_id = Uuid::new_v4();

        let window = WindowInfo {
            id: Uuid::new_v4(),
            session_id: Uuid::new_v4(),
            name: "editor".to_string(),
            index: 1,
            pane_count: 3,
            active_pane_id: Some(pane_id),
        };

        assert_eq!(window.active_pane_id, Some(pane_id));
        assert_eq!(window.pane_count, 3);
    }

    #[test]
    fn test_window_info_clone() {
        let window = WindowInfo {
            id: Uuid::new_v4(),
            session_id: Uuid::new_v4(),
            name: "test".to_string(),
            index: 0,
            pane_count: 2,
            active_pane_id: Some(Uuid::new_v4()),
        };

        let cloned = window.clone();
        assert_eq!(window, cloned);
    }

    #[test]
    fn test_window_info_equality() {
        let id = Uuid::new_v4();
        let session_id = Uuid::new_v4();

        let window1 = WindowInfo {
            id,
            session_id,
            name: "main".to_string(),
            index: 0,
            pane_count: 1,
            active_pane_id: None,
        };

        let window2 = WindowInfo {
            id,
            session_id,
            name: "main".to_string(),
            index: 0,
            pane_count: 1,
            active_pane_id: None,
        };

        let window3 = WindowInfo {
            id,
            session_id,
            name: "other".to_string(), // Different name
            index: 0,
            pane_count: 1,
            active_pane_id: None,
        };

        assert_eq!(window1, window2);
        assert_ne!(window1, window3);
    }

    #[test]
    fn test_window_info_serde() {
        let window = WindowInfo {
            id: Uuid::new_v4(),
            session_id: Uuid::new_v4(),
            name: "main".to_string(),
            index: 0,
            pane_count: 2,
            active_pane_id: Some(Uuid::new_v4()),
        };

        let serialized = bincode::serialize(&window).unwrap();
        let deserialized: WindowInfo = bincode::deserialize(&serialized).unwrap();
        assert_eq!(window, deserialized);
    }
}
