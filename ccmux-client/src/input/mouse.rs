//! Mouse event handling for ccmux client
//!
//! Handles mouse clicks, scroll events, and coordinate translation for pane interaction.

use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};

use super::{InputAction, InputMode};

/// Default scroll lines per wheel event
const DEFAULT_SCROLL_LINES: usize = 3;

/// Handle a mouse event and return the appropriate action
pub fn handle_mouse_event(event: MouseEvent, mode: InputMode) -> InputAction {
    match event.kind {
        // Left click - focus pane at cursor position
        MouseEventKind::Down(MouseButton::Left) => {
            InputAction::FocusPane {
                x: event.column,
                y: event.row,
            }
        }

        // Right click - context menu or alternative action (future)
        MouseEventKind::Down(MouseButton::Right) => {
            // Currently no action for right click
            InputAction::None
        }

        // Middle click - paste from clipboard (future)
        MouseEventKind::Down(MouseButton::Middle) => {
            // Currently no action for middle click
            InputAction::None
        }

        // Scroll wheel
        MouseEventKind::ScrollUp => {
            match mode {
                InputMode::Copy => {
                    // In copy mode, scroll navigates history
                    InputAction::ScrollUp {
                        lines: DEFAULT_SCROLL_LINES,
                    }
                }
                _ => {
                    // In normal mode, scroll also scrolls (enters scroll mode implicitly)
                    InputAction::ScrollUp {
                        lines: DEFAULT_SCROLL_LINES,
                    }
                }
            }
        }

        MouseEventKind::ScrollDown => {
            match mode {
                InputMode::Copy => {
                    InputAction::ScrollDown {
                        lines: DEFAULT_SCROLL_LINES,
                    }
                }
                _ => {
                    InputAction::ScrollDown {
                        lines: DEFAULT_SCROLL_LINES,
                    }
                }
            }
        }

        // Horizontal scroll (if supported)
        MouseEventKind::ScrollLeft | MouseEventKind::ScrollRight => {
            // Could be used for horizontal pane navigation in the future
            InputAction::None
        }

        // Mouse drag - could be used for pane resizing or text selection
        MouseEventKind::Drag(MouseButton::Left) => {
            // Future: implement pane border dragging for resize
            // For now, no action
            InputAction::None
        }

        MouseEventKind::Drag(_) => InputAction::None,

        // Mouse up events
        MouseEventKind::Up(_) => InputAction::None,

        // Mouse movement (not dragging)
        MouseEventKind::Moved => InputAction::None,
    }
}

/// Translate terminal coordinates to pane-relative coordinates
///
/// Given a terminal position and the pane's bounding rectangle,
/// returns the position relative to the pane's content area.
///
/// Returns `None` if the position is outside the pane.
#[allow(dead_code)]
pub fn translate_to_pane_coords(
    term_x: u16,
    term_y: u16,
    pane_x: u16,
    pane_y: u16,
    pane_width: u16,
    pane_height: u16,
) -> Option<(u16, u16)> {
    // Check if position is within pane bounds
    if term_x < pane_x
        || term_y < pane_y
        || term_x >= pane_x + pane_width
        || term_y >= pane_y + pane_height
    {
        return None;
    }

    // Calculate relative position (accounting for 1-char border)
    let rel_x = term_x - pane_x;
    let rel_y = term_y - pane_y;

    Some((rel_x, rel_y))
}

/// Check if a position is on a pane border
///
/// Returns true if the position is on the border of the pane,
/// which could be used for resize drag operations.
#[allow(dead_code)]
pub fn is_on_pane_border(
    term_x: u16,
    term_y: u16,
    pane_x: u16,
    pane_y: u16,
    pane_width: u16,
    pane_height: u16,
) -> Option<BorderPosition> {
    // Check if outside pane area entirely
    if term_x < pane_x
        || term_y < pane_y
        || term_x >= pane_x + pane_width
        || term_y >= pane_y + pane_height
    {
        return None;
    }

    let on_left = term_x == pane_x;
    let on_right = term_x == pane_x + pane_width - 1;
    let on_top = term_y == pane_y;
    let on_bottom = term_y == pane_y + pane_height - 1;

    match (on_left, on_right, on_top, on_bottom) {
        (true, _, true, _) => Some(BorderPosition::TopLeft),
        (_, true, true, _) => Some(BorderPosition::TopRight),
        (true, _, _, true) => Some(BorderPosition::BottomLeft),
        (_, true, _, true) => Some(BorderPosition::BottomRight),
        (true, _, _, _) => Some(BorderPosition::Left),
        (_, true, _, _) => Some(BorderPosition::Right),
        (_, _, true, _) => Some(BorderPosition::Top),
        (_, _, _, true) => Some(BorderPosition::Bottom),
        _ => None,
    }
}

/// Position on a pane border
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BorderPosition {
    Top,
    Bottom,
    Left,
    Right,
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

/// Generate SGR mouse protocol bytes for passing mouse events to applications
///
/// This is useful when applications inside panes want to receive mouse events
/// (like vim, less, etc.)
#[allow(dead_code)]
pub fn encode_sgr_mouse(event: &MouseEvent) -> Option<Vec<u8>> {
    let button = match event.kind {
        MouseEventKind::Down(MouseButton::Left) => 0,
        MouseEventKind::Down(MouseButton::Middle) => 1,
        MouseEventKind::Down(MouseButton::Right) => 2,
        MouseEventKind::Up(MouseButton::Left) => 0,
        MouseEventKind::Up(MouseButton::Middle) => 1,
        MouseEventKind::Up(MouseButton::Right) => 2,
        MouseEventKind::Drag(MouseButton::Left) => 32,
        MouseEventKind::Drag(MouseButton::Middle) => 33,
        MouseEventKind::Drag(MouseButton::Right) => 34,
        MouseEventKind::Moved => 35,
        MouseEventKind::ScrollUp => 64,
        MouseEventKind::ScrollDown => 65,
        MouseEventKind::ScrollLeft => 66,
        MouseEventKind::ScrollRight => 67,
    };

    let suffix = match event.kind {
        MouseEventKind::Up(_) => 'm',
        _ => 'M',
    };

    // SGR format: CSI < button ; x ; y M/m
    // Note: SGR uses 1-based coordinates
    Some(
        format!(
            "\x1b[<{};{};{}{}",
            button,
            event.column + 1,
            event.row + 1,
            suffix
        )
        .into_bytes(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyModifiers;

    fn make_mouse_event(kind: MouseEventKind, col: u16, row: u16) -> MouseEvent {
        MouseEvent {
            kind,
            column: col,
            row,
            modifiers: KeyModifiers::empty(),
        }
    }

    #[test]
    fn test_left_click() {
        let event = make_mouse_event(MouseEventKind::Down(MouseButton::Left), 10, 5);
        let result = handle_mouse_event(event, InputMode::Normal);
        assert_eq!(result, InputAction::FocusPane { x: 10, y: 5 });
    }

    #[test]
    fn test_right_click() {
        let event = make_mouse_event(MouseEventKind::Down(MouseButton::Right), 10, 5);
        let result = handle_mouse_event(event, InputMode::Normal);
        assert_eq!(result, InputAction::None);
    }

    #[test]
    fn test_scroll_up_normal() {
        let event = make_mouse_event(MouseEventKind::ScrollUp, 10, 5);
        let result = handle_mouse_event(event, InputMode::Normal);
        assert_eq!(
            result,
            InputAction::ScrollUp {
                lines: DEFAULT_SCROLL_LINES
            }
        );
    }

    #[test]
    fn test_scroll_down_normal() {
        let event = make_mouse_event(MouseEventKind::ScrollDown, 10, 5);
        let result = handle_mouse_event(event, InputMode::Normal);
        assert_eq!(
            result,
            InputAction::ScrollDown {
                lines: DEFAULT_SCROLL_LINES
            }
        );
    }

    #[test]
    fn test_scroll_up_copy_mode() {
        let event = make_mouse_event(MouseEventKind::ScrollUp, 10, 5);
        let result = handle_mouse_event(event, InputMode::Copy);
        assert_eq!(
            result,
            InputAction::ScrollUp {
                lines: DEFAULT_SCROLL_LINES
            }
        );
    }

    #[test]
    fn test_translate_to_pane_coords_inside() {
        let result = translate_to_pane_coords(15, 10, 10, 5, 20, 10);
        assert_eq!(result, Some((5, 5)));
    }

    #[test]
    fn test_translate_to_pane_coords_outside_left() {
        let result = translate_to_pane_coords(5, 10, 10, 5, 20, 10);
        assert_eq!(result, None);
    }

    #[test]
    fn test_translate_to_pane_coords_outside_top() {
        let result = translate_to_pane_coords(15, 2, 10, 5, 20, 10);
        assert_eq!(result, None);
    }

    #[test]
    fn test_translate_to_pane_coords_at_origin() {
        let result = translate_to_pane_coords(10, 5, 10, 5, 20, 10);
        assert_eq!(result, Some((0, 0)));
    }

    #[test]
    fn test_is_on_border_left() {
        let result = is_on_pane_border(10, 10, 10, 5, 20, 15);
        assert_eq!(result, Some(BorderPosition::Left));
    }

    #[test]
    fn test_is_on_border_right() {
        let result = is_on_pane_border(29, 10, 10, 5, 20, 15);
        assert_eq!(result, Some(BorderPosition::Right));
    }

    #[test]
    fn test_is_on_border_top() {
        let result = is_on_pane_border(20, 5, 10, 5, 20, 15);
        assert_eq!(result, Some(BorderPosition::Top));
    }

    #[test]
    fn test_is_on_border_bottom() {
        let result = is_on_pane_border(20, 19, 10, 5, 20, 15);
        assert_eq!(result, Some(BorderPosition::Bottom));
    }

    #[test]
    fn test_is_on_border_corner() {
        let result = is_on_pane_border(10, 5, 10, 5, 20, 15);
        assert_eq!(result, Some(BorderPosition::TopLeft));
    }

    #[test]
    fn test_is_on_border_inside() {
        let result = is_on_pane_border(15, 10, 10, 5, 20, 15);
        assert_eq!(result, None);
    }

    #[test]
    fn test_is_on_border_outside() {
        let result = is_on_pane_border(5, 2, 10, 5, 20, 15);
        assert_eq!(result, None);
    }

    #[test]
    fn test_encode_sgr_mouse_left_down() {
        let event = make_mouse_event(MouseEventKind::Down(MouseButton::Left), 10, 5);
        let result = encode_sgr_mouse(&event);
        assert_eq!(result, Some(b"\x1b[<0;11;6M".to_vec()));
    }

    #[test]
    fn test_encode_sgr_mouse_left_up() {
        let event = make_mouse_event(MouseEventKind::Up(MouseButton::Left), 10, 5);
        let result = encode_sgr_mouse(&event);
        assert_eq!(result, Some(b"\x1b[<0;11;6m".to_vec()));
    }

    #[test]
    fn test_encode_sgr_mouse_scroll_up() {
        let event = make_mouse_event(MouseEventKind::ScrollUp, 10, 5);
        let result = encode_sgr_mouse(&event);
        assert_eq!(result, Some(b"\x1b[<64;11;6M".to_vec()));
    }

    #[test]
    fn test_encode_sgr_mouse_drag() {
        let event = make_mouse_event(MouseEventKind::Drag(MouseButton::Left), 10, 5);
        let result = encode_sgr_mouse(&event);
        assert_eq!(result, Some(b"\x1b[<32;11;6M".to_vec()));
    }

    #[test]
    fn test_mouse_up_no_action() {
        let event = make_mouse_event(MouseEventKind::Up(MouseButton::Left), 10, 5);
        let result = handle_mouse_event(event, InputMode::Normal);
        assert_eq!(result, InputAction::None);
    }

    #[test]
    fn test_mouse_moved_no_action() {
        let event = make_mouse_event(MouseEventKind::Moved, 10, 5);
        let result = handle_mouse_event(event, InputMode::Normal);
        assert_eq!(result, InputAction::None);
    }
}
