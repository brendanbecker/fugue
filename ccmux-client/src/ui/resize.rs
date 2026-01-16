//! Resize handling for the terminal UI
//!
//! Provides utilities for handling terminal resize events, including
//! layout recalculation and maintaining pane proportions.

// Allow unused code that's part of the public API for future features
#![allow(dead_code)]

use ratatui::layout::Rect;
use uuid::Uuid;

use super::layout::LayoutManager;
use super::pane::PaneManager;

/// Minimum pane dimensions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MinimumSize {
    pub cols: u16,
    pub rows: u16,
}

impl Default for MinimumSize {
    fn default() -> Self {
        Self {
            cols: 10,  // Minimum 10 columns
            rows: 3,   // Minimum 3 rows (for title + content + border)
        }
    }
}

impl MinimumSize {
    /// Create a new minimum size
    pub fn new(cols: u16, rows: u16) -> Self {
        Self { cols, rows }
    }

    /// Check if a rect meets minimum size requirements
    pub fn is_valid(&self, rect: Rect) -> bool {
        rect.width >= self.cols && rect.height >= self.rows
    }

    /// Calculate how many panes can fit horizontally
    pub fn max_horizontal_panes(&self, width: u16) -> usize {
        if width < self.cols {
            0
        } else {
            (width / self.cols) as usize
        }
    }

    /// Calculate how many panes can fit vertically
    pub fn max_vertical_panes(&self, height: u16) -> usize {
        if height < self.rows {
            0
        } else {
            (height / self.rows) as usize
        }
    }
}

/// Resize event handler
pub struct ResizeHandler {
    /// Minimum pane size
    minimum_size: MinimumSize,
    /// Previous terminal size
    previous_size: Option<(u16, u16)>,
    /// Whether a resize is pending
    resize_pending: bool,
}

impl ResizeHandler {
    /// Create a new resize handler
    pub fn new() -> Self {
        Self {
            minimum_size: MinimumSize::default(),
            previous_size: None,
            resize_pending: false,
        }
    }

    /// Create with custom minimum size
    pub fn with_minimum_size(minimum_size: MinimumSize) -> Self {
        Self {
            minimum_size,
            previous_size: None,
            resize_pending: false,
        }
    }

    /// Get minimum size
    pub fn minimum_size(&self) -> MinimumSize {
        self.minimum_size
    }

    /// Set minimum size
    pub fn set_minimum_size(&mut self, size: MinimumSize) {
        self.minimum_size = size;
    }

    /// Check if resize is needed based on new size
    pub fn needs_resize(&self, new_cols: u16, new_rows: u16) -> bool {
        match self.previous_size {
            Some((old_cols, old_rows)) => old_cols != new_cols || old_rows != new_rows,
            None => true,
        }
    }

    /// Record a resize event
    pub fn on_resize(&mut self, cols: u16, rows: u16) {
        self.previous_size = Some((cols, rows));
        self.resize_pending = true;
    }

    /// Clear pending resize flag
    pub fn clear_pending(&mut self) {
        self.resize_pending = false;
    }

    /// Check if resize is pending
    pub fn is_pending(&self) -> bool {
        self.resize_pending
    }

    /// Get the previous size
    pub fn previous_size(&self) -> Option<(u16, u16)> {
        self.previous_size
    }

    /// Calculate new pane sizes after resize
    ///
    /// Returns a list of (pane_id, new_cols, new_rows) tuples
    pub fn calculate_pane_sizes(
        &self,
        layout: &LayoutManager,
        area: Rect,
    ) -> Vec<(Uuid, u16, u16)> {
        let weights = std::collections::HashMap::new();
        let rects = layout.calculate_rects(area, &weights);

        rects
            .into_iter()
            .map(|(id, rect)| {
                // Account for borders (1 char on each side)
                let inner_cols = rect.width.saturating_sub(2);
                let inner_rows = rect.height.saturating_sub(2);

                (id, inner_cols.max(1), inner_rows.max(1))
            })
            .collect()
    }

    /// Apply resize to all panes
    ///
    /// Updates pane terminals with new dimensions
    pub fn apply_resize(
        &self,
        layout: &LayoutManager,
        panes: &mut PaneManager,
        area: Rect,
    ) {
        let sizes = self.calculate_pane_sizes(layout, area);

        for (pane_id, cols, rows) in sizes {
            if cols >= self.minimum_size.cols && rows >= self.minimum_size.rows {
                panes.resize_pane(pane_id, rows, cols);
            }
        }
    }

    /// Check if layout is valid for the given area
    pub fn is_layout_valid(&self, layout: &LayoutManager, area: Rect) -> bool {
        let weights = std::collections::HashMap::new();
        let rects = layout.calculate_rects(area, &weights);

        for (_, rect) in rects {
            if !self.minimum_size.is_valid(rect) {
                return false;
            }
        }

        true
    }

    /// Calculate the maximum number of panes that can fit
    pub fn max_panes(&self, area: Rect) -> usize {
        let max_h = self.minimum_size.max_horizontal_panes(area.width);
        let max_v = self.minimum_size.max_vertical_panes(area.height);
        max_h * max_v
    }
}

impl Default for ResizeHandler {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper to calculate inner area after accounting for borders
pub fn inner_area(outer: Rect) -> Rect {
    Rect {
        x: outer.x + 1,
        y: outer.y + 1,
        width: outer.width.saturating_sub(2),
        height: outer.height.saturating_sub(2),
    }
}

/// Helper to calculate status bar area at the bottom
pub fn status_bar_area(outer: Rect, height: u16) -> (Rect, Rect) {
    let main_height = outer.height.saturating_sub(height);

    let main_area = Rect {
        x: outer.x,
        y: outer.y,
        width: outer.width,
        height: main_height,
    };

    let status_area = Rect {
        x: outer.x,
        y: outer.y + main_height,
        width: outer.width,
        height,
    };

    (main_area, status_area)
}

/// Calculate proportional resize
///
/// When resizing, maintain the relative proportions of panes
pub fn proportional_resize(
    old_size: (u16, u16),
    new_size: (u16, u16),
    pane_size: (u16, u16),
) -> (u16, u16) {
    let (old_cols, old_rows) = old_size;
    let (new_cols, new_rows) = new_size;
    let (pane_cols, pane_rows) = pane_size;

    // Calculate scale factors
    let scale_x = if old_cols > 0 {
        new_cols as f32 / old_cols as f32
    } else {
        1.0
    };
    let scale_y = if old_rows > 0 {
        new_rows as f32 / old_rows as f32
    } else {
        1.0
    };

    // Apply scaling
    let new_pane_cols = ((pane_cols as f32) * scale_x).round() as u16;
    let new_pane_rows = ((pane_rows as f32) * scale_y).round() as u16;

    (new_pane_cols.max(1), new_pane_rows.max(1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_minimum_size_default() {
        let min = MinimumSize::default();
        assert_eq!(min.cols, 10);
        assert_eq!(min.rows, 3);
    }

    #[test]
    fn test_minimum_size_is_valid() {
        let min = MinimumSize::new(10, 5);

        assert!(min.is_valid(Rect::new(0, 0, 10, 5)));
        assert!(min.is_valid(Rect::new(0, 0, 20, 10)));
        assert!(!min.is_valid(Rect::new(0, 0, 5, 5)));
        assert!(!min.is_valid(Rect::new(0, 0, 10, 3)));
    }

    #[test]
    fn test_minimum_size_max_panes() {
        let min = MinimumSize::new(10, 5);

        assert_eq!(min.max_horizontal_panes(100), 10);
        assert_eq!(min.max_vertical_panes(25), 5);
        assert_eq!(min.max_horizontal_panes(5), 0);
    }

    #[test]
    fn test_resize_handler_new() {
        let handler = ResizeHandler::new();
        assert!(!handler.is_pending());
        assert!(handler.previous_size().is_none());
    }

    #[test]
    fn test_resize_handler_needs_resize() {
        let mut handler = ResizeHandler::new();

        // First resize always needed
        assert!(handler.needs_resize(80, 24));

        handler.on_resize(80, 24);

        // Same size doesn't need resize
        assert!(!handler.needs_resize(80, 24));

        // Different size needs resize
        assert!(handler.needs_resize(100, 24));
        assert!(handler.needs_resize(80, 30));
    }

    #[test]
    fn test_resize_handler_on_resize() {
        let mut handler = ResizeHandler::new();

        handler.on_resize(80, 24);
        assert!(handler.is_pending());
        assert_eq!(handler.previous_size(), Some((80, 24)));

        handler.clear_pending();
        assert!(!handler.is_pending());
    }

    #[test]
    fn test_resize_handler_with_minimum() {
        let min = MinimumSize::new(20, 10);
        let handler = ResizeHandler::with_minimum_size(min);

        assert_eq!(handler.minimum_size().cols, 20);
        assert_eq!(handler.minimum_size().rows, 10);
    }

    #[test]
    fn test_resize_handler_max_panes() {
        let handler = ResizeHandler::new();
        let area = Rect::new(0, 0, 80, 24);

        let max = handler.max_panes(area);
        assert!(max > 0);
    }

    #[test]
    fn test_inner_area() {
        let outer = Rect::new(0, 0, 80, 24);
        let inner = inner_area(outer);

        assert_eq!(inner.x, 1);
        assert_eq!(inner.y, 1);
        assert_eq!(inner.width, 78);
        assert_eq!(inner.height, 22);
    }

    #[test]
    fn test_inner_area_small() {
        let outer = Rect::new(0, 0, 2, 2);
        let inner = inner_area(outer);

        assert_eq!(inner.width, 0);
        assert_eq!(inner.height, 0);
    }

    #[test]
    fn test_status_bar_area() {
        let outer = Rect::new(0, 0, 80, 24);
        let (main, status) = status_bar_area(outer, 1);

        assert_eq!(main.height, 23);
        assert_eq!(status.height, 1);
        assert_eq!(status.y, 23);
    }

    #[test]
    fn test_proportional_resize() {
        let old_size = (80, 24);
        let new_size = (160, 48);
        let pane_size = (40, 12);

        let (new_cols, new_rows) = proportional_resize(old_size, new_size, pane_size);

        // Should roughly double
        assert_eq!(new_cols, 80);
        assert_eq!(new_rows, 24);
    }

    #[test]
    fn test_proportional_resize_shrink() {
        let old_size = (100, 50);
        let new_size = (50, 25);
        let pane_size = (50, 25);

        let (new_cols, new_rows) = proportional_resize(old_size, new_size, pane_size);

        // Should roughly halve
        assert_eq!(new_cols, 25);
        assert_eq!(new_rows, 13); // Rounding
    }

    #[test]
    fn test_proportional_resize_minimum() {
        let old_size = (100, 100);
        let new_size = (10, 10);
        let pane_size = (5, 5);

        let (new_cols, new_rows) = proportional_resize(old_size, new_size, pane_size);

        // Should not go below 1
        assert!(new_cols >= 1);
        assert!(new_rows >= 1);
    }

    #[test]
    fn test_proportional_resize_zero_old() {
        let old_size = (0, 0);
        let new_size = (80, 24);
        let pane_size = (40, 12);

        let (new_cols, new_rows) = proportional_resize(old_size, new_size, pane_size);

        // Should keep original size when old is 0
        assert_eq!(new_cols, 40);
        assert_eq!(new_rows, 12);
    }

    #[test]
    fn test_calculate_pane_sizes() {
        let handler = ResizeHandler::new();
        let layout = LayoutManager::new(Uuid::new_v4());
        let area = Rect::new(0, 0, 80, 24);

        let sizes = handler.calculate_pane_sizes(&layout, area);

        assert_eq!(sizes.len(), 1);
        // Accounts for borders
        assert_eq!(sizes[0].1, 78);
        assert_eq!(sizes[0].2, 22);
    }
}
