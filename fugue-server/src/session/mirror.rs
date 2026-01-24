//! Mirror pane registry for ccmux server (FEAT-062)
//!
//! Tracks mirror pane relationships for output forwarding and cleanup.
//! When a pane produces output, the registry is consulted to forward
//! that output to any mirror panes.

use std::collections::{HashMap, HashSet};
use uuid::Uuid;

/// Registry for tracking mirror pane relationships
///
/// The registry maintains bidirectional mappings:
/// - source_id -> Set of mirror_ids (for output forwarding)
/// - mirror_id -> source_id (for cleanup when mirror closes)
#[derive(Debug, Default)]
pub struct MirrorRegistry {
    /// Maps source pane ID to set of mirror pane IDs
    source_to_mirrors: HashMap<Uuid, HashSet<Uuid>>,
    /// Maps mirror pane ID to source pane ID
    mirror_to_source: HashMap<Uuid, Uuid>,
}

impl MirrorRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a new mirror relationship
    ///
    /// Returns false if the mirror_id was already registered.
    pub fn register(&mut self, source_id: Uuid, mirror_id: Uuid) -> bool {
        // Check if this mirror is already registered
        if self.mirror_to_source.contains_key(&mirror_id) {
            return false;
        }

        // Add to source -> mirrors mapping
        self.source_to_mirrors
            .entry(source_id)
            .or_default()
            .insert(mirror_id);

        // Add to mirror -> source mapping
        self.mirror_to_source.insert(mirror_id, source_id);

        tracing::debug!(
            source_id = %source_id,
            mirror_id = %mirror_id,
            "Registered mirror pane"
        );

        true
    }

    /// Unregister a mirror pane
    ///
    /// Returns the source pane ID if the mirror was registered.
    pub fn unregister_mirror(&mut self, mirror_id: Uuid) -> Option<Uuid> {
        let source_id = self.mirror_to_source.remove(&mirror_id)?;

        // Remove from source -> mirrors mapping
        if let Some(mirrors) = self.source_to_mirrors.get_mut(&source_id) {
            mirrors.remove(&mirror_id);
            if mirrors.is_empty() {
                self.source_to_mirrors.remove(&source_id);
            }
        }

        tracing::debug!(
            source_id = %source_id,
            mirror_id = %mirror_id,
            "Unregistered mirror pane"
        );

        Some(source_id)
    }

    /// Get all mirror pane IDs for a source pane
    ///
    /// Returns an empty slice if the source has no mirrors.
    pub fn get_mirrors(&self, source_id: Uuid) -> Vec<Uuid> {
        self.source_to_mirrors
            .get(&source_id)
            .map(|mirrors| mirrors.iter().copied().collect())
            .unwrap_or_default()
    }

    /// Get the source pane ID for a mirror pane
    pub fn get_source(&self, mirror_id: Uuid) -> Option<Uuid> {
        self.mirror_to_source.get(&mirror_id).copied()
    }

    /// Check if a pane is a mirror
    pub fn is_mirror(&self, pane_id: Uuid) -> bool {
        self.mirror_to_source.contains_key(&pane_id)
    }

    /// Check if a pane has any mirrors
    pub fn has_mirrors(&self, source_id: Uuid) -> bool {
        self.source_to_mirrors
            .get(&source_id)
            .map(|mirrors| !mirrors.is_empty())
            .unwrap_or(false)
    }

    /// Handle source pane closing
    ///
    /// Returns the list of mirror pane IDs that need to be notified.
    pub fn on_source_closed(&mut self, source_id: Uuid) -> Vec<Uuid> {
        let mirrors = self.source_to_mirrors.remove(&source_id);

        if let Some(mirrors) = &mirrors {
            // Remove all mirror -> source mappings
            for mirror_id in mirrors {
                self.mirror_to_source.remove(mirror_id);
            }

            tracing::debug!(
                source_id = %source_id,
                mirror_count = mirrors.len(),
                "Source pane closed, notifying mirrors"
            );
        }

        mirrors.into_iter().flatten().collect()
    }

    /// Get the number of registered mirrors
    pub fn mirror_count(&self) -> usize {
        self.mirror_to_source.len()
    }

    /// Get the number of source panes with mirrors
    pub fn source_count(&self) -> usize {
        self.source_to_mirrors.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_mirror() {
        let mut registry = MirrorRegistry::new();
        let source_id = Uuid::new_v4();
        let mirror_id = Uuid::new_v4();

        assert!(registry.register(source_id, mirror_id));
        assert_eq!(registry.get_source(mirror_id), Some(source_id));
        assert_eq!(registry.get_mirrors(source_id), vec![mirror_id]);
    }

    #[test]
    fn test_register_duplicate_mirror() {
        let mut registry = MirrorRegistry::new();
        let source_id = Uuid::new_v4();
        let mirror_id = Uuid::new_v4();

        assert!(registry.register(source_id, mirror_id));
        assert!(!registry.register(source_id, mirror_id));
    }

    #[test]
    fn test_multiple_mirrors() {
        let mut registry = MirrorRegistry::new();
        let source_id = Uuid::new_v4();
        let mirror_id1 = Uuid::new_v4();
        let mirror_id2 = Uuid::new_v4();

        registry.register(source_id, mirror_id1);
        registry.register(source_id, mirror_id2);

        let mirrors = registry.get_mirrors(source_id);
        assert_eq!(mirrors.len(), 2);
        assert!(mirrors.contains(&mirror_id1));
        assert!(mirrors.contains(&mirror_id2));
    }

    #[test]
    fn test_unregister_mirror() {
        let mut registry = MirrorRegistry::new();
        let source_id = Uuid::new_v4();
        let mirror_id = Uuid::new_v4();

        registry.register(source_id, mirror_id);
        assert_eq!(registry.unregister_mirror(mirror_id), Some(source_id));
        assert!(!registry.is_mirror(mirror_id));
        assert!(!registry.has_mirrors(source_id));
    }

    #[test]
    fn test_on_source_closed() {
        let mut registry = MirrorRegistry::new();
        let source_id = Uuid::new_v4();
        let mirror_id1 = Uuid::new_v4();
        let mirror_id2 = Uuid::new_v4();

        registry.register(source_id, mirror_id1);
        registry.register(source_id, mirror_id2);

        let affected = registry.on_source_closed(source_id);
        assert_eq!(affected.len(), 2);

        // All mappings should be cleaned up
        assert!(!registry.is_mirror(mirror_id1));
        assert!(!registry.is_mirror(mirror_id2));
        assert!(!registry.has_mirrors(source_id));
    }

    #[test]
    fn test_is_mirror() {
        let mut registry = MirrorRegistry::new();
        let source_id = Uuid::new_v4();
        let mirror_id = Uuid::new_v4();
        let other_id = Uuid::new_v4();

        registry.register(source_id, mirror_id);

        assert!(registry.is_mirror(mirror_id));
        assert!(!registry.is_mirror(source_id));
        assert!(!registry.is_mirror(other_id));
    }
}
