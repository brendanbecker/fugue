//! Layout system for pane arrangement
//!
//! Provides a flexible layout system supporting horizontal/vertical splits,
//! nested layouts, and common layout presets.

// Allow unused code that's part of the public API for future features
#![allow(dead_code)]

use ratatui::layout::{Constraint, Direction, Layout as RatatuiLayout, Rect};
use uuid::Uuid;

/// Split direction for layouts
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitDirection {
    Horizontal,
    Vertical,
}

impl From<ccmux_protocol::SplitDirection> for SplitDirection {
    fn from(dir: ccmux_protocol::SplitDirection) -> Self {
        match dir {
            ccmux_protocol::SplitDirection::Horizontal => SplitDirection::Horizontal,
            ccmux_protocol::SplitDirection::Vertical => SplitDirection::Vertical,
        }
    }
}

/// A node in the layout tree
#[derive(Debug, Clone)]
pub enum LayoutNode {
    /// A leaf node representing a single pane
    Pane { id: Uuid },
    /// A split containing multiple child layouts
    Split {
        direction: SplitDirection,
        /// Children and their relative sizes (0.0 - 1.0, must sum to 1.0)
        children: Vec<(LayoutNode, f32)>,
    },
}

impl LayoutNode {
    /// Create a single pane layout
    pub fn pane(id: Uuid) -> Self {
        Self::Pane { id }
    }

    /// Create a horizontal split with equal-sized children
    pub fn horizontal_split(children: Vec<LayoutNode>) -> Self {
        let count = children.len();
        if count == 0 {
            return Self::Pane { id: Uuid::nil() };
        }
        let ratio = 1.0 / count as f32;
        Self::Split {
            direction: SplitDirection::Horizontal,
            children: children.into_iter().map(|c| (c, ratio)).collect(),
        }
    }

    /// Create a vertical split with equal-sized children
    pub fn vertical_split(children: Vec<LayoutNode>) -> Self {
        let count = children.len();
        if count == 0 {
            return Self::Pane { id: Uuid::nil() };
        }
        let ratio = 1.0 / count as f32;
        Self::Split {
            direction: SplitDirection::Vertical,
            children: children.into_iter().map(|c| (c, ratio)).collect(),
        }
    }

    /// Create a split with custom ratios
    pub fn split_with_ratios(direction: SplitDirection, children: Vec<(LayoutNode, f32)>) -> Self {
        if children.is_empty() {
            return Self::Pane { id: Uuid::nil() };
        }
        Self::Split {
            direction,
            children,
        }
    }

    /// Get all pane IDs in this layout
    pub fn pane_ids(&self) -> Vec<Uuid> {
        match self {
            LayoutNode::Pane { id } => vec![*id],
            LayoutNode::Split { children, .. } => {
                children.iter().flat_map(|(child, _)| child.pane_ids()).collect()
            }
        }
    }

    /// Count total number of panes
    pub fn pane_count(&self) -> usize {
        match self {
            LayoutNode::Pane { .. } => 1,
            LayoutNode::Split { children, .. } => {
                children.iter().map(|(child, _)| child.pane_count()).sum()
            }
        }
    }

    /// Calculate rectangles for all panes given a bounding rectangle
    pub fn calculate_rects(&self, area: Rect) -> Vec<(Uuid, Rect)> {
        match self {
            LayoutNode::Pane { id } => vec![(*id, area)],
            LayoutNode::Split { direction, children } => {
                let constraints: Vec<Constraint> = children
                    .iter()
                    .map(|(_, ratio)| Constraint::Ratio((*ratio * 100.0) as u32, 100))
                    .collect();

                let ratatui_direction = match direction {
                    SplitDirection::Horizontal => Direction::Horizontal,
                    SplitDirection::Vertical => Direction::Vertical,
                };

                let chunks = RatatuiLayout::default()
                    .direction(ratatui_direction)
                    .constraints(constraints)
                    .split(area);

                children
                    .iter()
                    .zip(chunks.iter())
                    .flat_map(|((child, _), &rect)| child.calculate_rects(rect))
                    .collect()
            }
        }
    }

    /// Find and update a pane's size ratio in its parent split
    /// Returns true if the pane was found and updated
    pub fn resize_pane(&mut self, pane_id: Uuid, delta: f32) -> bool {
        match self {
            LayoutNode::Pane { .. } => false,
            LayoutNode::Split { children, .. } => {
                // Find the child containing the pane
                for (i, (child, _)) in children.iter().enumerate() {
                    if matches!(child, LayoutNode::Pane { id } if *id == pane_id) {
                        // Found the pane, adjust its size
                        if children.len() > 1 && i + 1 < children.len() {
                            let current = children[i].1;
                            let next = children[i + 1].1;

                            // Calculate new sizes
                            let new_current = (current + delta).clamp(0.1, 0.9);
                            let diff = new_current - current;
                            let new_next = (next - diff).clamp(0.1, 0.9);

                            children[i].1 = new_current;
                            children[i + 1].1 = new_next;
                            return true;
                        }
                        return false;
                    }
                }

                // Recurse into children
                for (child, _) in children.iter_mut() {
                    if child.resize_pane(pane_id, delta) {
                        return true;
                    }
                }

                false
            }
        }
    }

    /// Add a pane by splitting an existing pane
    pub fn add_pane(&mut self, target_pane_id: Uuid, new_pane_id: Uuid, direction: SplitDirection) -> bool {
        match self {
            LayoutNode::Pane { id } if *id == target_pane_id => {
                // Split this pane
                let old_pane = LayoutNode::Pane { id: *id };
                let new_pane = LayoutNode::Pane { id: new_pane_id };
                *self = LayoutNode::Split {
                    direction,
                    children: vec![(old_pane, 0.5), (new_pane, 0.5)],
                };
                true
            }
            LayoutNode::Pane { .. } => false,
            LayoutNode::Split { children, .. } => {
                // Recurse into children
                for (child, _) in children.iter_mut() {
                    if child.add_pane(target_pane_id, new_pane_id, direction) {
                        return true;
                    }
                }
                false
            }
        }
    }

    /// Remove a pane from the layout
    /// Returns true if the pane was found and removed
    pub fn remove_pane(&mut self, pane_id: Uuid) -> bool {
        match self {
            LayoutNode::Pane { id } => *id == pane_id,
            LayoutNode::Split { children, .. } => {
                // Find and remove the child
                let mut found_idx = None;
                for (i, (child, _)) in children.iter().enumerate() {
                    if matches!(child, LayoutNode::Pane { id } if *id == pane_id) {
                        found_idx = Some(i);
                        break;
                    }
                }

                if let Some(idx) = found_idx {
                    children.remove(idx);
                    // Rebalance remaining children
                    if !children.is_empty() {
                        let ratio = 1.0 / children.len() as f32;
                        for (_, r) in children.iter_mut() {
                            *r = ratio;
                        }
                    }
                    return true;
                }

                // Recurse into children
                for (child, _) in children.iter_mut() {
                    if child.remove_pane(pane_id) {
                        return true;
                    }
                }

                // Clean up empty splits
                children.retain(|(child, _)| {
                    match child {
                        LayoutNode::Split { children: c, .. } => !c.is_empty(),
                        _ => true,
                    }
                });

                false
            }
        }
    }
}

/// Layout presets for common arrangements
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutPreset {
    /// Single pane filling the entire area
    Single,
    /// Two panes side by side
    SplitHorizontal,
    /// Two panes stacked vertically
    SplitVertical,
    /// Four panes in a 2x2 grid
    Grid2x2,
    /// Main pane on left, two smaller on right
    MainLeft,
    /// Main pane on top, two smaller on bottom
    MainTop,
}

impl LayoutPreset {
    /// Create a layout tree from this preset with the given pane IDs
    /// If not enough pane IDs are provided, new UUIDs will be generated
    pub fn create_layout(self, mut pane_ids: Vec<Uuid>) -> LayoutNode {
        // Ensure we have enough pane IDs
        let needed = match self {
            LayoutPreset::Single => 1,
            LayoutPreset::SplitHorizontal | LayoutPreset::SplitVertical => 2,
            LayoutPreset::Grid2x2 | LayoutPreset::MainLeft | LayoutPreset::MainTop => 4,
        };

        while pane_ids.len() < needed {
            pane_ids.push(Uuid::new_v4());
        }

        match self {
            LayoutPreset::Single => LayoutNode::pane(pane_ids[0]),
            LayoutPreset::SplitHorizontal => LayoutNode::horizontal_split(vec![
                LayoutNode::pane(pane_ids[0]),
                LayoutNode::pane(pane_ids[1]),
            ]),
            LayoutPreset::SplitVertical => LayoutNode::vertical_split(vec![
                LayoutNode::pane(pane_ids[0]),
                LayoutNode::pane(pane_ids[1]),
            ]),
            LayoutPreset::Grid2x2 => LayoutNode::vertical_split(vec![
                LayoutNode::horizontal_split(vec![
                    LayoutNode::pane(pane_ids[0]),
                    LayoutNode::pane(pane_ids[1]),
                ]),
                LayoutNode::horizontal_split(vec![
                    LayoutNode::pane(pane_ids[2]),
                    LayoutNode::pane(pane_ids[3]),
                ]),
            ]),
            LayoutPreset::MainLeft => LayoutNode::split_with_ratios(
                SplitDirection::Horizontal,
                vec![
                    (LayoutNode::pane(pane_ids[0]), 0.6),
                    (
                        LayoutNode::vertical_split(vec![
                            LayoutNode::pane(pane_ids[1]),
                            LayoutNode::pane(pane_ids[2]),
                        ]),
                        0.4,
                    ),
                ],
            ),
            LayoutPreset::MainTop => LayoutNode::split_with_ratios(
                SplitDirection::Vertical,
                vec![
                    (LayoutNode::pane(pane_ids[0]), 0.6),
                    (
                        LayoutNode::horizontal_split(vec![
                            LayoutNode::pane(pane_ids[1]),
                            LayoutNode::pane(pane_ids[2]),
                        ]),
                        0.4,
                    ),
                ],
            ),
        }
    }
}

/// Layout manager for the application
#[derive(Debug)]
pub struct LayoutManager {
    /// Current layout tree
    root: LayoutNode,
    /// Active pane ID
    active_pane_id: Option<Uuid>,
}

impl LayoutManager {
    /// Create a new layout manager with a single pane
    pub fn new(pane_id: Uuid) -> Self {
        Self {
            root: LayoutNode::pane(pane_id),
            active_pane_id: Some(pane_id),
        }
    }

    /// Create from a preset
    pub fn from_preset(preset: LayoutPreset, pane_ids: Vec<Uuid>) -> Self {
        let root = preset.create_layout(pane_ids.clone());
        let active_pane_id = pane_ids.first().copied();
        Self {
            root,
            active_pane_id,
        }
    }

    /// Get the root layout node
    pub fn root(&self) -> &LayoutNode {
        &self.root
    }

    /// Get the active pane ID
    pub fn active_pane_id(&self) -> Option<Uuid> {
        self.active_pane_id
    }

    /// Set the active pane
    pub fn set_active_pane(&mut self, pane_id: Uuid) {
        if self.root.pane_ids().contains(&pane_id) {
            self.active_pane_id = Some(pane_id);
        }
    }

    /// Calculate all pane rectangles for a given area
    pub fn calculate_rects(&self, area: Rect) -> Vec<(Uuid, Rect)> {
        self.root.calculate_rects(area)
    }

    /// Get rectangle for a specific pane
    pub fn get_pane_rect(&self, area: Rect, pane_id: Uuid) -> Option<Rect> {
        self.calculate_rects(area)
            .into_iter()
            .find(|(id, _)| *id == pane_id)
            .map(|(_, rect)| rect)
    }

    /// Get rectangle for the active pane
    pub fn active_pane_rect(&self, area: Rect) -> Option<Rect> {
        self.active_pane_id
            .and_then(|id| self.get_pane_rect(area, id))
    }

    /// Split the active pane
    pub fn split_active(&mut self, direction: SplitDirection) -> Option<Uuid> {
        let active_id = self.active_pane_id?;
        let new_id = Uuid::new_v4();
        if self.root.add_pane(active_id, new_id, direction) {
            Some(new_id)
        } else {
            None
        }
    }

    /// Remove a pane from the layout
    pub fn remove_pane(&mut self, pane_id: Uuid) -> bool {
        if self.root.remove_pane(pane_id) {
            // Update active pane if needed
            if self.active_pane_id == Some(pane_id) {
                self.active_pane_id = self.root.pane_ids().first().copied();
            }
            true
        } else {
            false
        }
    }

    /// Navigate to the next pane in order
    pub fn next_pane(&mut self) {
        let panes = self.root.pane_ids();
        if let Some(current) = self.active_pane_id {
            if let Some(idx) = panes.iter().position(|&id| id == current) {
                let next_idx = (idx + 1) % panes.len();
                self.active_pane_id = Some(panes[next_idx]);
            }
        } else if !panes.is_empty() {
            self.active_pane_id = Some(panes[0]);
        }
    }

    /// Navigate to the previous pane in order
    pub fn prev_pane(&mut self) {
        let panes = self.root.pane_ids();
        if let Some(current) = self.active_pane_id {
            if let Some(idx) = panes.iter().position(|&id| id == current) {
                let prev_idx = if idx == 0 { panes.len() - 1 } else { idx - 1 };
                self.active_pane_id = Some(panes[prev_idx]);
            }
        } else if !panes.is_empty() {
            self.active_pane_id = Some(panes[panes.len() - 1]);
        }
    }

    /// Resize the active pane
    pub fn resize_active(&mut self, delta: f32) -> bool {
        if let Some(pane_id) = self.active_pane_id {
            self.root.resize_pane(pane_id, delta)
        } else {
            false
        }
    }

    /// Get all pane IDs
    pub fn pane_ids(&self) -> Vec<Uuid> {
        self.root.pane_ids()
    }

    /// Get pane count
    pub fn pane_count(&self) -> usize {
        self.root.pane_count()
    }
}

impl Default for LayoutManager {
    fn default() -> Self {
        Self::new(Uuid::new_v4())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_pane_layout() {
        let id = Uuid::new_v4();
        let node = LayoutNode::pane(id);

        assert_eq!(node.pane_count(), 1);
        assert_eq!(node.pane_ids(), vec![id]);
    }

    #[test]
    fn test_horizontal_split() {
        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();
        let node = LayoutNode::horizontal_split(vec![
            LayoutNode::pane(id1),
            LayoutNode::pane(id2),
        ]);

        assert_eq!(node.pane_count(), 2);
        let ids = node.pane_ids();
        assert!(ids.contains(&id1));
        assert!(ids.contains(&id2));
    }

    #[test]
    fn test_vertical_split() {
        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();
        let node = LayoutNode::vertical_split(vec![
            LayoutNode::pane(id1),
            LayoutNode::pane(id2),
        ]);

        assert_eq!(node.pane_count(), 2);
    }

    #[test]
    fn test_calculate_rects_single() {
        let id = Uuid::new_v4();
        let node = LayoutNode::pane(id);
        let area = Rect::new(0, 0, 100, 50);

        let rects = node.calculate_rects(area);
        assert_eq!(rects.len(), 1);
        assert_eq!(rects[0].0, id);
        assert_eq!(rects[0].1, area);
    }

    #[test]
    fn test_calculate_rects_horizontal_split() {
        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();
        let node = LayoutNode::horizontal_split(vec![
            LayoutNode::pane(id1),
            LayoutNode::pane(id2),
        ]);
        let area = Rect::new(0, 0, 100, 50);

        let rects = node.calculate_rects(area);
        assert_eq!(rects.len(), 2);

        // Each pane should be roughly half the width
        let rect1 = rects.iter().find(|(id, _)| *id == id1).unwrap().1;
        let rect2 = rects.iter().find(|(id, _)| *id == id2).unwrap().1;

        assert!(rect1.width >= 45 && rect1.width <= 55);
        assert!(rect2.width >= 45 && rect2.width <= 55);
        assert_eq!(rect1.height, 50);
        assert_eq!(rect2.height, 50);
    }

    #[test]
    fn test_calculate_rects_vertical_split() {
        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();
        let node = LayoutNode::vertical_split(vec![
            LayoutNode::pane(id1),
            LayoutNode::pane(id2),
        ]);
        let area = Rect::new(0, 0, 100, 50);

        let rects = node.calculate_rects(area);
        assert_eq!(rects.len(), 2);

        // Each pane should be roughly half the height
        let rect1 = rects.iter().find(|(id, _)| *id == id1).unwrap().1;
        let rect2 = rects.iter().find(|(id, _)| *id == id2).unwrap().1;

        assert!(rect1.height >= 23 && rect1.height <= 27);
        assert!(rect2.height >= 23 && rect2.height <= 27);
        assert_eq!(rect1.width, 100);
        assert_eq!(rect2.width, 100);
    }

    #[test]
    fn test_nested_layout() {
        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();
        let id3 = Uuid::new_v4();

        let node = LayoutNode::horizontal_split(vec![
            LayoutNode::pane(id1),
            LayoutNode::vertical_split(vec![
                LayoutNode::pane(id2),
                LayoutNode::pane(id3),
            ]),
        ]);

        assert_eq!(node.pane_count(), 3);
        let ids = node.pane_ids();
        assert!(ids.contains(&id1));
        assert!(ids.contains(&id2));
        assert!(ids.contains(&id3));
    }

    #[test]
    fn test_add_pane() {
        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();
        let mut node = LayoutNode::pane(id1);

        assert!(node.add_pane(id1, id2, SplitDirection::Horizontal));
        assert_eq!(node.pane_count(), 2);
    }

    #[test]
    fn test_remove_pane() {
        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();
        let mut node = LayoutNode::horizontal_split(vec![
            LayoutNode::pane(id1),
            LayoutNode::pane(id2),
        ]);

        assert!(node.remove_pane(id1));
        assert_eq!(node.pane_count(), 1);
        assert!(node.pane_ids().contains(&id2));
    }

    #[test]
    fn test_layout_preset_single() {
        let ids = vec![Uuid::new_v4()];
        let layout = LayoutPreset::Single.create_layout(ids.clone());
        assert_eq!(layout.pane_count(), 1);
        assert_eq!(layout.pane_ids(), ids);
    }

    #[test]
    fn test_layout_preset_split_horizontal() {
        let ids = vec![Uuid::new_v4(), Uuid::new_v4()];
        let layout = LayoutPreset::SplitHorizontal.create_layout(ids.clone());
        assert_eq!(layout.pane_count(), 2);
    }

    #[test]
    fn test_layout_preset_grid() {
        let ids = vec![Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4()];
        let layout = LayoutPreset::Grid2x2.create_layout(ids.clone());
        assert_eq!(layout.pane_count(), 4);
    }

    #[test]
    fn test_layout_manager_new() {
        let id = Uuid::new_v4();
        let manager = LayoutManager::new(id);

        assert_eq!(manager.active_pane_id(), Some(id));
        assert_eq!(manager.pane_count(), 1);
    }

    #[test]
    fn test_layout_manager_split() {
        let id = Uuid::new_v4();
        let mut manager = LayoutManager::new(id);

        let new_id = manager.split_active(SplitDirection::Horizontal);
        assert!(new_id.is_some());
        assert_eq!(manager.pane_count(), 2);
    }

    #[test]
    fn test_layout_manager_navigation() {
        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();
        let mut manager = LayoutManager::from_preset(
            LayoutPreset::SplitHorizontal,
            vec![id1, id2],
        );

        assert_eq!(manager.active_pane_id(), Some(id1));

        manager.next_pane();
        assert_eq!(manager.active_pane_id(), Some(id2));

        manager.next_pane();
        assert_eq!(manager.active_pane_id(), Some(id1)); // Wrap around

        manager.prev_pane();
        assert_eq!(manager.active_pane_id(), Some(id2));
    }

    #[test]
    fn test_layout_manager_remove() {
        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();
        let mut manager = LayoutManager::from_preset(
            LayoutPreset::SplitHorizontal,
            vec![id1, id2],
        );

        manager.set_active_pane(id1);
        assert!(manager.remove_pane(id1));
        assert_eq!(manager.pane_count(), 1);
        // Active pane should update to remaining pane
        assert_eq!(manager.active_pane_id(), Some(id2));
    }

    #[test]
    fn test_split_direction_from_protocol() {
        let proto_h = ccmux_protocol::SplitDirection::Horizontal;
        let proto_v = ccmux_protocol::SplitDirection::Vertical;

        assert_eq!(SplitDirection::from(proto_h), SplitDirection::Horizontal);
        assert_eq!(SplitDirection::from(proto_v), SplitDirection::Vertical);
    }

    #[test]
    fn test_get_pane_rect() {
        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();
        let manager = LayoutManager::from_preset(
            LayoutPreset::SplitHorizontal,
            vec![id1, id2],
        );
        let area = Rect::new(0, 0, 100, 50);

        let rect = manager.get_pane_rect(area, id1);
        assert!(rect.is_some());
        let rect = rect.unwrap();
        assert!(rect.width >= 45 && rect.width <= 55);
    }
}
