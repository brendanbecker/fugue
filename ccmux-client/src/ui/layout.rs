//! Layout system for pane arrangement
//!
//! Provides a flexible layout system supporting horizontal/vertical splits,
//! nested layouts, and common layout presets.

// Allow unused code that's part of the public API for future features
#![allow(dead_code)]

use std::collections::HashMap;

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

/// Layout policy for dynamic resizing
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LayoutPolicy {
    /// Use fixed ratios as specified in the layout
    #[default]
    Fixed,
    /// Distribute space equally among all children
    Balanced,
    /// Adjust space dynamically based on pane activity and focus
    Adaptive,
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
    pub fn calculate_rects(&self, area: Rect, policy: LayoutPolicy, weights: &HashMap<Uuid, f32>) -> Vec<(Uuid, Rect)> {
        match self {
            LayoutNode::Pane { id } => vec![(*id, area)],
            LayoutNode::Split { direction, children } => {
                let constraints: Vec<Constraint> = match policy {
                    LayoutPolicy::Fixed => children
                        .iter()
                        .map(|(_, ratio)| Constraint::Ratio((*ratio * 100.0) as u32, 100))
                        .collect(),
                    LayoutPolicy::Balanced => {
                        let count = children.len() as u32;
                        children.iter().map(|_| Constraint::Ratio(1, count)).collect()
                    }
                    LayoutPolicy::Adaptive => {
                        // Calculate total weight for each child branch
                        let branch_weights: Vec<f32> = children.iter().map(|(child, ratio)| {
                            let pane_ids = child.pane_ids();
                            let total_weight: f32 = pane_ids.iter()
                                .map(|id| weights.get(id).copied().unwrap_or(1.0))
                                .sum();
                            // Average weight of panes in this branch, weighted by the branch's base ratio
                            let avg_weight = total_weight / pane_ids.len().max(1) as f32;
                            *ratio * avg_weight
                        }).collect();

                        let sum: f32 = branch_weights.iter().sum();
                        branch_weights.iter().map(|&w| {
                            let pct = if sum > 0.0 {
                                (w / sum * 100.0) as u32
                            } else {
                                (100 / children.len()) as u32
                            };
                            Constraint::Percentage(pct as u16)
                        }).collect()
                    }
                };

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
                    .flat_map(|((child, _), &rect)| child.calculate_rects(rect, policy, weights))
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

    /// Prune the layout tree by collapsing single-child splits
    /// This should be called after remove_pane to clean up the tree structure
    pub fn prune(&mut self) {
        match self {
            LayoutNode::Pane { .. } => {
                // Nothing to prune for a leaf node
            }
            LayoutNode::Split { children, .. } => {
                // First, recursively prune all children
                for (child, _) in children.iter_mut() {
                    child.prune();
                }

                // After pruning children, check if any child splits now have single children
                // and collapse them inline
                for (child, _) in children.iter_mut() {
                    if let LayoutNode::Split {
                        children: child_children,
                        ..
                    } = child
                    {
                        if child_children.len() == 1 {
                            // Replace the split with its single child
                            let (single_child, _) = child_children.remove(0);
                            *child = single_child;
                        }
                    }
                }

                // If this split itself has only one child after pruning, we can't collapse
                // ourselves here (we'd need the parent to do it), but LayoutManager will handle it
            }
        }
    }

    /// Check if this node is a single-child split (needs to be collapsed)
    pub fn is_single_child_split(&self) -> bool {
        matches!(self, LayoutNode::Split { children, .. } if children.len() == 1)
    }

    /// If this is a single-child split, return the unwrapped single child
    /// Otherwise return None
    pub fn unwrap_single_child(self) -> Option<LayoutNode> {
        match self {
            LayoutNode::Split { mut children, .. } if children.len() == 1 => {
                Some(children.remove(0).0)
            }
            _ => None,
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
    /// Layout policy
    policy: LayoutPolicy,
}

impl LayoutManager {
    /// Create a new layout manager with a single pane
    pub fn new(pane_id: Uuid) -> Self {
        Self {
            root: LayoutNode::pane(pane_id),
            active_pane_id: Some(pane_id),
            policy: LayoutPolicy::default(),
        }
    }

    /// Create from a preset
    pub fn from_preset(preset: LayoutPreset, pane_ids: Vec<Uuid>) -> Self {
        let root = preset.create_layout(pane_ids.clone());
        let active_pane_id = pane_ids.first().copied();
        Self {
            root,
            active_pane_id,
            policy: LayoutPolicy::default(),
        }
    }

    /// Get the root layout node
    pub fn root(&self) -> &LayoutNode {
        &self.root
    }

    /// Get the root layout node mutably
    pub fn root_mut(&mut self) -> &mut LayoutNode {
        &mut self.root
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

    /// Get the current policy
    pub fn policy(&self) -> LayoutPolicy {
        self.policy
    }

    /// Set the layout policy
    pub fn set_policy(&mut self, policy: LayoutPolicy) {
        self.policy = policy;
    }

    /// Calculate all pane rectangles for a given area
    pub fn calculate_rects(&self, area: Rect, weights: &HashMap<Uuid, f32>) -> Vec<(Uuid, Rect)> {
        self.root.calculate_rects(area, self.policy, weights)
    }

    /// Get rectangle for a specific pane
    pub fn get_pane_rect(&self, area: Rect, pane_id: Uuid, weights: &HashMap<Uuid, f32>) -> Option<Rect> {
        self.calculate_rects(area, weights)
            .into_iter()
            .find(|(id, _)| *id == pane_id)
            .map(|(_, rect)| rect)
    }

    /// Get rectangle for the active pane
    pub fn active_pane_rect(&self, area: Rect, weights: &HashMap<Uuid, f32>) -> Option<Rect> {
        self.active_pane_id
            .and_then(|id| self.get_pane_rect(area, id, weights))
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
            // Prune the tree to collapse single-child splits
            self.root.prune();

            // Handle the case where root itself is now a single-child split
            // We need to collapse it to avoid unnecessary nesting
            if self.root.is_single_child_split() {
                // Replace root with its single child
                let old_root = std::mem::replace(&mut self.root, LayoutNode::pane(Uuid::nil()));
                if let Some(new_root) = old_root.unwrap_single_child() {
                    self.root = new_root;
                }
            }

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
        let weights = HashMap::new();

        let rects = node.calculate_rects(area, LayoutPolicy::Fixed, &weights);
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
        let weights = HashMap::new();

        let rects = node.calculate_rects(area, LayoutPolicy::Fixed, &weights);
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
        let weights = HashMap::new();

        let rects = node.calculate_rects(area, LayoutPolicy::Fixed, &weights);
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
        let weights = HashMap::new();

        let rect = manager.get_pane_rect(area, id1, &weights);
        assert!(rect.is_some());
        let rect = rect.unwrap();
        assert!(rect.width >= 45 && rect.width <= 55);
    }

    // ==================== BUG-015 Tests: Layout Tree Pruning ====================

    #[test]
    fn test_prune_single_child_nested() {
        // Create: Split(V) { [Pane(A), Split(H) { [Pane(B), Pane(C)] }] }
        let id_a = Uuid::new_v4();
        let id_b = Uuid::new_v4();
        let id_c = Uuid::new_v4();

        let mut layout = LayoutNode::vertical_split(vec![
            LayoutNode::pane(id_a),
            LayoutNode::horizontal_split(vec![LayoutNode::pane(id_b), LayoutNode::pane(id_c)]),
        ]);

        // Remove Pane(C): Split(V) { [Pane(A), Split(H) { [Pane(B)] }] }
        assert!(layout.remove_pane(id_c));
        assert_eq!(layout.pane_count(), 2);

        // After pruning: Split(V) { [Pane(A), Pane(B)] }
        layout.prune();

        // Verify the nested split was collapsed
        assert_eq!(layout.pane_count(), 2);
        let ids = layout.pane_ids();
        assert!(ids.contains(&id_a));
        assert!(ids.contains(&id_b));

        // Verify structure: should be a split with two pane children (not nested)
        if let LayoutNode::Split { children, .. } = &layout {
            assert_eq!(children.len(), 2);
            for (child, _) in children {
                assert!(matches!(child, LayoutNode::Pane { .. }));
            }
        } else {
            panic!("Expected Split at root");
        }
    }

    #[test]
    fn test_prune_deeply_nested() {
        // Create deeply nested structure:
        // Split(V) { [Pane(A), Split(H) { [Pane(B), Split(V) { [Pane(C), Pane(D)] }] }] }
        let id_a = Uuid::new_v4();
        let id_b = Uuid::new_v4();
        let id_c = Uuid::new_v4();
        let id_d = Uuid::new_v4();

        let mut layout = LayoutNode::vertical_split(vec![
            LayoutNode::pane(id_a),
            LayoutNode::horizontal_split(vec![
                LayoutNode::pane(id_b),
                LayoutNode::vertical_split(vec![LayoutNode::pane(id_c), LayoutNode::pane(id_d)]),
            ]),
        ]);

        assert_eq!(layout.pane_count(), 4);

        // Remove Pane(D), leaving single-child Split(V) { [Pane(C)] }
        assert!(layout.remove_pane(id_d));
        assert_eq!(layout.pane_count(), 3);

        // After pruning: innermost single-child split should be collapsed
        layout.prune();
        assert_eq!(layout.pane_count(), 3);

        let ids = layout.pane_ids();
        assert!(ids.contains(&id_a));
        assert!(ids.contains(&id_b));
        assert!(ids.contains(&id_c));
    }

    #[test]
    fn test_remove_pane_with_manager_prunes_tree() {
        // Test that LayoutManager::remove_pane properly prunes the tree
        let id_a = Uuid::new_v4();
        let id_b = Uuid::new_v4();
        let id_c = Uuid::new_v4();

        // Create: Split(V) { [Pane(A), Split(H) { [Pane(B), Pane(C)] }] }
        let mut manager = LayoutManager::new(id_a);
        manager.root_mut().add_pane(id_a, id_b, SplitDirection::Vertical);

        // Now add C by splitting B horizontally
        manager.set_active_pane(id_b);
        manager.root_mut().add_pane(id_b, id_c, SplitDirection::Horizontal);

        assert_eq!(manager.pane_count(), 3);

        // Remove C - tree should be pruned automatically
        assert!(manager.remove_pane(id_c));
        assert_eq!(manager.pane_count(), 2);

        // Verify pruning happened - should have 2 panes in a simple split
        let ids = manager.pane_ids();
        assert!(ids.contains(&id_a));
        assert!(ids.contains(&id_b));
    }

    #[test]
    fn test_remove_to_single_pane() {
        // Start with 2 panes, remove one, should collapse to single pane
        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();
        let mut manager = LayoutManager::from_preset(LayoutPreset::SplitHorizontal, vec![id1, id2]);

        assert_eq!(manager.pane_count(), 2);

        // Remove id1
        assert!(manager.remove_pane(id1));
        assert_eq!(manager.pane_count(), 1);

        // Root should now be a single Pane, not a Split
        assert!(matches!(manager.root(), LayoutNode::Pane { id } if *id == id2));
    }

    #[test]
    fn test_quadrant_layout_close_three() {
        // Simulate the exact bug scenario: 4 panes in quadrant layout, close 3
        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();
        let id3 = Uuid::new_v4();
        let id4 = Uuid::new_v4();

        let mut manager = LayoutManager::from_preset(LayoutPreset::Grid2x2, vec![id1, id2, id3, id4]);

        assert_eq!(manager.pane_count(), 4);

        // Close pane 2, 3, and 4
        assert!(manager.remove_pane(id2));
        assert_eq!(manager.pane_count(), 3);

        assert!(manager.remove_pane(id3));
        assert_eq!(manager.pane_count(), 2);

        assert!(manager.remove_pane(id4));
        assert_eq!(manager.pane_count(), 1);

        // Only id1 should remain, and it should fill the full area
        let ids = manager.pane_ids();
        assert_eq!(ids, vec![id1]);

        // Root should be a single Pane
        assert!(matches!(manager.root(), LayoutNode::Pane { id } if *id == id1));

        // Verify it uses full area
        let area = Rect::new(0, 0, 100, 50);
        let weights = HashMap::new();
        let rects = manager.calculate_rects(area, &weights);
        assert_eq!(rects.len(), 1);
        assert_eq!(rects[0].0, id1);
        assert_eq!(rects[0].1, area);
    }

    #[test]
    fn test_adaptive_layout_weights() {
        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();
        let mut manager = LayoutManager::from_preset(
            LayoutPreset::SplitHorizontal,
            vec![id1, id2],
        );
        manager.set_policy(LayoutPolicy::Adaptive);
        
        let area = Rect::new(0, 0, 100, 50);
        
        // Equal weights
        let mut weights = HashMap::new();
        weights.insert(id1, 1.0);
        weights.insert(id2, 1.0);
        
        let rects = manager.calculate_rects(area, &weights);
        let rect1 = rects.iter().find(|(id, _)| *id == id1).unwrap().1;
        let rect2 = rects.iter().find(|(id, _)| *id == id2).unwrap().1;
        assert!(rect1.width >= 45 && rect1.width <= 55);
        assert!(rect2.width >= 45 && rect2.width <= 55);
        
        // id1 is twice as important as id2
        weights.insert(id1, 2.0);
        weights.insert(id2, 1.0);
        
        let rects = manager.calculate_rects(area, &weights);
        let rect1 = rects.iter().find(|(id, _)| *id == id1).unwrap().1;
        let rect2 = rects.iter().find(|(id, _)| *id == id2).unwrap().1;
        
        // Should be roughly 66% and 33%
        assert!(rect1.width >= 64 && rect1.width <= 68);
        assert!(rect2.width >= 31 && rect2.width <= 35);
    }

    #[test]
    fn test_is_single_child_split() {
        let id = Uuid::new_v4();

        // Single pane is not a single-child split
        let pane = LayoutNode::pane(id);
        assert!(!pane.is_single_child_split());

        // Split with one child is a single-child split
        let single_child = LayoutNode::Split {
            direction: SplitDirection::Horizontal,
            children: vec![(LayoutNode::pane(id), 1.0)],
        };
        assert!(single_child.is_single_child_split());

        // Split with two children is not
        let two_children = LayoutNode::horizontal_split(vec![
            LayoutNode::pane(Uuid::new_v4()),
            LayoutNode::pane(Uuid::new_v4()),
        ]);
        assert!(!two_children.is_single_child_split());
    }

    #[test]
    fn test_unwrap_single_child() {
        let id = Uuid::new_v4();

        // Single-child split should unwrap to its child
        let single_child = LayoutNode::Split {
            direction: SplitDirection::Horizontal,
            children: vec![(LayoutNode::pane(id), 1.0)],
        };

        let unwrapped = single_child.unwrap_single_child();
        assert!(unwrapped.is_some());
        let unwrapped = unwrapped.unwrap();
        assert!(matches!(unwrapped, LayoutNode::Pane { id: inner_id } if inner_id == id));

        // Regular pane should return None
        let pane = LayoutNode::pane(Uuid::new_v4());
        assert!(pane.unwrap_single_child().is_none());

        // Multi-child split should return None
        let multi_child = LayoutNode::horizontal_split(vec![
            LayoutNode::pane(Uuid::new_v4()),
            LayoutNode::pane(Uuid::new_v4()),
        ]);
        assert!(multi_child.unwrap_single_child().is_none());
    }
}
