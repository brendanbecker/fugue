//! Key translation for terminal input
//!
//! Translates crossterm key events to byte sequences that can be sent
//! to a PTY for proper terminal emulation. Also provides key binding
//! parsing for configurable keybindings.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::fmt;

/// Error type for key binding parsing
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyBindingError {
    /// Empty binding string
    Empty,
    /// Unknown modifier name
    UnknownModifier(String),
    /// Unknown key name
    UnknownKey(String),
    /// Invalid function key number
    InvalidFunctionKey(String),
}

impl fmt::Display for KeyBindingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            KeyBindingError::Empty => write!(f, "empty key binding string"),
            KeyBindingError::UnknownModifier(m) => write!(f, "unknown modifier: {}", m),
            KeyBindingError::UnknownKey(k) => write!(f, "unknown key: {}", k),
            KeyBindingError::InvalidFunctionKey(k) => write!(f, "invalid function key: {}", k),
        }
    }
}

impl std::error::Error for KeyBindingError {}

/// A parsed key binding that can match against KeyEvents
#[derive(Debug, Clone)]
pub struct KeyBinding {
    /// The key code to match
    pub code: KeyCode,
    /// The modifiers to match
    pub modifiers: KeyModifiers,
}

impl KeyBinding {
    /// Create a new key binding
    pub fn new(code: KeyCode, modifiers: KeyModifiers) -> Self {
        Self { code, modifiers }
    }

    /// Check if this binding matches a key event
    pub fn matches(&self, event: &KeyEvent) -> bool {
        // For character keys, we need to handle case sensitivity
        // The binding "Ctrl-PageDown" should match regardless of shift state
        // when not explicitly specified
        match (&self.code, &event.code) {
            (KeyCode::Char(a), KeyCode::Char(b)) => {
                // For character comparison, normalize case
                a.to_ascii_lowercase() == b.to_ascii_lowercase()
                    && self.modifiers == event.modifiers
            }
            _ => self.code == event.code && self.modifiers == event.modifiers,
        }
    }

    /// Parse a key binding from a string like "Ctrl-PageDown" or "Alt-Shift-a"
    pub fn parse(s: &str) -> Result<Self, KeyBindingError> {
        parse_key_binding(s)
    }
}

/// Parse a key binding string like "Ctrl-Tab" or "Alt-Shift-a"
///
/// Format:
/// - Single keys: `Tab`, `F1`, `Enter`, `Space`, `PageUp`, `PageDown`
/// - With modifiers: `Ctrl-Tab`, `Alt-a`, `Shift-F1`
/// - Multiple modifiers: `Ctrl-Shift-Tab`, `Ctrl-Alt-Delete`
/// - Case insensitive: `ctrl-tab`, `CTRL-TAB`, `Ctrl-Tab` all work
pub fn parse_key_binding(s: &str) -> Result<KeyBinding, KeyBindingError> {
    let s = s.trim();
    if s.is_empty() {
        return Err(KeyBindingError::Empty);
    }

    let parts: Vec<&str> = s.split('-').collect();
    let mut modifiers = KeyModifiers::empty();

    // All parts except the last are modifiers
    for part in &parts[..parts.len().saturating_sub(1)] {
        match part.to_lowercase().as_str() {
            "ctrl" | "control" => modifiers |= KeyModifiers::CONTROL,
            "alt" | "meta" | "option" => modifiers |= KeyModifiers::ALT,
            "shift" => modifiers |= KeyModifiers::SHIFT,
            "super" | "cmd" | "win" => modifiers |= KeyModifiers::SUPER,
            _ => return Err(KeyBindingError::UnknownModifier((*part).to_string())),
        }
    }

    // Last part is the key
    let key_part = parts.last().ok_or(KeyBindingError::Empty)?;
    let code = parse_key_code(key_part)?;

    Ok(KeyBinding::new(code, modifiers))
}

/// Parse a key code from a string
fn parse_key_code(s: &str) -> Result<KeyCode, KeyBindingError> {
    let lower = s.to_lowercase();
    match lower.as_str() {
        // Whitespace and control
        "tab" => Ok(KeyCode::Tab),
        "enter" | "return" | "cr" => Ok(KeyCode::Enter),
        "space" | "spc" => Ok(KeyCode::Char(' ')),
        "backspace" | "bs" => Ok(KeyCode::Backspace),
        "delete" | "del" => Ok(KeyCode::Delete),
        "insert" | "ins" => Ok(KeyCode::Insert),
        "esc" | "escape" => Ok(KeyCode::Esc),

        // Navigation
        "home" => Ok(KeyCode::Home),
        "end" => Ok(KeyCode::End),
        "pageup" | "pgup" | "prior" => Ok(KeyCode::PageUp),
        "pagedown" | "pgdn" | "pgdown" | "next" => Ok(KeyCode::PageDown),

        // Arrows
        "up" | "uparrow" => Ok(KeyCode::Up),
        "down" | "downarrow" => Ok(KeyCode::Down),
        "left" | "leftarrow" => Ok(KeyCode::Left),
        "right" | "rightarrow" => Ok(KeyCode::Right),

        // Function keys
        s if s.starts_with('f') && s.len() > 1 => {
            let num_str = &s[1..];
            let num: u8 = num_str
                .parse()
                .map_err(|_| KeyBindingError::InvalidFunctionKey(s.to_string()))?;
            if num == 0 || num > 24 {
                return Err(KeyBindingError::InvalidFunctionKey(s.to_string()));
            }
            Ok(KeyCode::F(num))
        }

        // Single character
        s if s.chars().count() == 1 => Ok(KeyCode::Char(s.chars().next().unwrap())),

        _ => Err(KeyBindingError::UnknownKey(s.to_string())),
    }
}

/// Translate a key event to its terminal byte sequence
///
/// Returns `None` for keys that don't have a terminal representation
/// (e.g., modifier-only keys like Shift)
pub fn translate_key(key: &KeyEvent) -> Option<Vec<u8>> {
    let modifiers = key.modifiers;

    match key.code {
        // Character keys
        KeyCode::Char(c) => translate_char(c, modifiers),

        // Whitespace and control
        KeyCode::Enter => Some(vec![b'\r']),
        KeyCode::Tab => {
            if modifiers.contains(KeyModifiers::SHIFT) {
                // Shift+Tab -> CSI Z (backtab)
                Some(b"\x1b[Z".to_vec())
            } else {
                Some(vec![b'\t'])
            }
        }
        KeyCode::BackTab => Some(b"\x1b[Z".to_vec()),
        KeyCode::Backspace => {
            if modifiers.contains(KeyModifiers::ALT) {
                // Alt+Backspace - delete word
                Some(vec![0x1b, 0x7f])
            } else {
                Some(vec![0x7f])
            }
        }
        KeyCode::Esc => Some(vec![0x1b]),

        // Arrow keys
        KeyCode::Up => Some(translate_arrow('A', modifiers)),
        KeyCode::Down => Some(translate_arrow('B', modifiers)),
        KeyCode::Right => Some(translate_arrow('C', modifiers)),
        KeyCode::Left => Some(translate_arrow('D', modifiers)),

        // Navigation keys
        KeyCode::Home => {
            if modifiers.is_empty() {
                Some(b"\x1b[H".to_vec())
            } else {
                Some(translate_modified_key(1, 'H', modifiers))
            }
        }
        KeyCode::End => {
            if modifiers.is_empty() {
                Some(b"\x1b[F".to_vec())
            } else {
                Some(translate_modified_key(1, 'F', modifiers))
            }
        }
        KeyCode::PageUp => {
            if modifiers.is_empty() {
                Some(b"\x1b[5~".to_vec())
            } else {
                Some(translate_tilde_key(5, modifiers))
            }
        }
        KeyCode::PageDown => {
            if modifiers.is_empty() {
                Some(b"\x1b[6~".to_vec())
            } else {
                Some(translate_tilde_key(6, modifiers))
            }
        }
        KeyCode::Insert => {
            if modifiers.is_empty() {
                Some(b"\x1b[2~".to_vec())
            } else {
                Some(translate_tilde_key(2, modifiers))
            }
        }
        KeyCode::Delete => {
            if modifiers.is_empty() {
                Some(b"\x1b[3~".to_vec())
            } else {
                Some(translate_tilde_key(3, modifiers))
            }
        }

        // Function keys
        KeyCode::F(n) => Some(translate_function_key(n, modifiers)),

        // Null key
        KeyCode::Null => Some(vec![0]),

        // Keys that don't produce output
        KeyCode::CapsLock
        | KeyCode::ScrollLock
        | KeyCode::NumLock
        | KeyCode::PrintScreen
        | KeyCode::Pause
        | KeyCode::Menu
        | KeyCode::KeypadBegin => None,

        // Modifier-only keys
        KeyCode::Modifier(_) => None,

        // Media keys - no terminal representation
        KeyCode::Media(_) => None,

        // Catch-all for any other keys
        _ => None,
    }
}

/// Translate a character with modifiers
fn translate_char(c: char, modifiers: KeyModifiers) -> Option<Vec<u8>> {
    // Check for Alt modifier first (including Ctrl+Alt combinations)
    if modifiers.contains(KeyModifiers::ALT) {
        // Alt + key sends ESC followed by the key
        let mut bytes = vec![0x1b];
        if modifiers.contains(KeyModifiers::CONTROL) && c.is_ascii_alphabetic() {
            // Ctrl+Alt+key
            let code = (c.to_ascii_lowercase() as u8) - b'a' + 1;
            bytes.push(code);
        } else {
            let mut buf = [0u8; 4];
            let s = c.encode_utf8(&mut buf);
            bytes.extend_from_slice(s.as_bytes());
        }
        return Some(bytes);
    }

    // Check for Control modifier (without Alt)
    if modifiers.contains(KeyModifiers::CONTROL) {
        // Control characters
        if c.is_ascii_alphabetic() {
            // Ctrl+A = 0x01, Ctrl+B = 0x02, etc.
            let code = (c.to_ascii_lowercase() as u8) - b'a' + 1;
            return Some(vec![code]);
        }
        // Special control key combinations
        match c {
            '@' => return Some(vec![0x00]), // Ctrl+@
            '[' => return Some(vec![0x1b]), // Ctrl+[ = Escape
            '\\' => return Some(vec![0x1c]),
            ']' => return Some(vec![0x1d]),
            '^' => return Some(vec![0x1e]),
            '_' => return Some(vec![0x1f]),
            '?' => return Some(vec![0x7f]), // Ctrl+? = DEL
            ' ' => return Some(vec![0x00]), // Ctrl+Space = NUL
            _ => {}
        }
    }

    // Regular character
    let mut buf = [0u8; 4];
    let s = c.encode_utf8(&mut buf);
    Some(s.as_bytes().to_vec())
}

/// Translate arrow keys with modifiers
fn translate_arrow(direction: char, modifiers: KeyModifiers) -> Vec<u8> {
    if modifiers.is_empty() {
        format!("\x1b[{}", direction).into_bytes()
    } else {
        translate_modified_key(1, direction, modifiers)
    }
}

/// Translate a modified key (CSI 1;modifier key)
fn translate_modified_key(code: u8, suffix: char, modifiers: KeyModifiers) -> Vec<u8> {
    let modifier = get_modifier_code(modifiers);
    format!("\x1b[{};{}{}", code, modifier, suffix).into_bytes()
}

/// Translate tilde-style keys with modifiers (CSI code;modifier ~)
fn translate_tilde_key(code: u8, modifiers: KeyModifiers) -> Vec<u8> {
    let modifier = get_modifier_code(modifiers);
    format!("\x1b[{};{}~", code, modifier).into_bytes()
}

/// Get the modifier code for xterm-style modified keys
fn get_modifier_code(modifiers: KeyModifiers) -> u8 {
    let mut code = 1;
    if modifiers.contains(KeyModifiers::SHIFT) {
        code += 1;
    }
    if modifiers.contains(KeyModifiers::ALT) {
        code += 2;
    }
    if modifiers.contains(KeyModifiers::CONTROL) {
        code += 4;
    }
    code
}

/// Translate function keys (F1-F12 and beyond)
fn translate_function_key(n: u8, modifiers: KeyModifiers) -> Vec<u8> {
    // Base sequences for function keys
    let (base_seq, is_ss3) = match n {
        1 => ("P", true),
        2 => ("Q", true),
        3 => ("R", true),
        4 => ("S", true),
        5 => ("15~", false),
        6 => ("17~", false),
        7 => ("18~", false),
        8 => ("19~", false),
        9 => ("20~", false),
        10 => ("21~", false),
        11 => ("23~", false),
        12 => ("24~", false),
        // Extended function keys (F13-F24)
        13 => ("25~", false),
        14 => ("26~", false),
        15 => ("28~", false),
        16 => ("29~", false),
        17 => ("31~", false),
        18 => ("32~", false),
        19 => ("33~", false),
        20 => ("34~", false),
        _ => return vec![],
    };

    if modifiers.is_empty() {
        if is_ss3 {
            format!("\x1bO{}", base_seq).into_bytes()
        } else {
            format!("\x1b[{}", base_seq).into_bytes()
        }
    } else {
        // With modifiers, use CSI format
        let modifier = get_modifier_code(modifiers);
        if is_ss3 {
            // F1-F4 with modifiers use CSI 1;modifier P/Q/R/S
            let key_code = match n {
                1 => 'P',
                2 => 'Q',
                3 => 'R',
                4 => 'S',
                _ => return vec![],
            };
            format!("\x1b[1;{}{}", modifier, key_code).into_bytes()
        } else {
            // F5+ with modifiers
            let key_num = match n {
                5 => 15,
                6 => 17,
                7 => 18,
                8 => 19,
                9 => 20,
                10 => 21,
                11 => 23,
                12 => 24,
                _ => return format!("\x1b[{}", base_seq).into_bytes(),
            };
            format!("\x1b[{};{}~", key_num, modifier).into_bytes()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_regular_char() {
        let key = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::empty());
        assert_eq!(translate_key(&key), Some(vec![b'a']));
    }

    #[test]
    fn test_uppercase_char() {
        let key = KeyEvent::new(KeyCode::Char('A'), KeyModifiers::SHIFT);
        let result = translate_key(&key);
        assert_eq!(result, Some(vec![b'A']));
    }

    #[test]
    fn test_unicode_char() {
        let key = KeyEvent::new(KeyCode::Char('ñ'), KeyModifiers::empty());
        let result = translate_key(&key);
        assert_eq!(result, Some("ñ".as_bytes().to_vec()));
    }

    #[test]
    fn test_ctrl_c() {
        let key = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
        assert_eq!(translate_key(&key), Some(vec![0x03])); // ETX
    }

    #[test]
    fn test_ctrl_a() {
        let key = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::CONTROL);
        assert_eq!(translate_key(&key), Some(vec![0x01])); // SOH
    }

    #[test]
    fn test_ctrl_z() {
        let key = KeyEvent::new(KeyCode::Char('z'), KeyModifiers::CONTROL);
        assert_eq!(translate_key(&key), Some(vec![0x1a])); // SUB
    }

    #[test]
    fn test_ctrl_bracket() {
        let key = KeyEvent::new(KeyCode::Char('['), KeyModifiers::CONTROL);
        assert_eq!(translate_key(&key), Some(vec![0x1b])); // ESC
    }

    #[test]
    fn test_ctrl_space() {
        let key = KeyEvent::new(KeyCode::Char(' '), KeyModifiers::CONTROL);
        assert_eq!(translate_key(&key), Some(vec![0x00])); // NUL
    }

    #[test]
    fn test_alt_a() {
        let key = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::ALT);
        assert_eq!(translate_key(&key), Some(vec![0x1b, b'a']));
    }

    #[test]
    fn test_ctrl_alt_a() {
        let key = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::CONTROL | KeyModifiers::ALT);
        assert_eq!(translate_key(&key), Some(vec![0x1b, 0x01]));
    }

    #[test]
    fn test_enter() {
        let key = KeyEvent::new(KeyCode::Enter, KeyModifiers::empty());
        assert_eq!(translate_key(&key), Some(vec![b'\r']));
    }

    #[test]
    fn test_tab() {
        let key = KeyEvent::new(KeyCode::Tab, KeyModifiers::empty());
        assert_eq!(translate_key(&key), Some(vec![b'\t']));
    }

    #[test]
    fn test_shift_tab() {
        let key = KeyEvent::new(KeyCode::Tab, KeyModifiers::SHIFT);
        assert_eq!(translate_key(&key), Some(b"\x1b[Z".to_vec()));
    }

    #[test]
    fn test_backspace() {
        let key = KeyEvent::new(KeyCode::Backspace, KeyModifiers::empty());
        assert_eq!(translate_key(&key), Some(vec![0x7f]));
    }

    #[test]
    fn test_alt_backspace() {
        let key = KeyEvent::new(KeyCode::Backspace, KeyModifiers::ALT);
        assert_eq!(translate_key(&key), Some(vec![0x1b, 0x7f]));
    }

    #[test]
    fn test_escape() {
        let key = KeyEvent::new(KeyCode::Esc, KeyModifiers::empty());
        assert_eq!(translate_key(&key), Some(vec![0x1b]));
    }

    #[test]
    fn test_arrow_up() {
        let key = KeyEvent::new(KeyCode::Up, KeyModifiers::empty());
        assert_eq!(translate_key(&key), Some(b"\x1b[A".to_vec()));
    }

    #[test]
    fn test_arrow_down() {
        let key = KeyEvent::new(KeyCode::Down, KeyModifiers::empty());
        assert_eq!(translate_key(&key), Some(b"\x1b[B".to_vec()));
    }

    #[test]
    fn test_arrow_right() {
        let key = KeyEvent::new(KeyCode::Right, KeyModifiers::empty());
        assert_eq!(translate_key(&key), Some(b"\x1b[C".to_vec()));
    }

    #[test]
    fn test_arrow_left() {
        let key = KeyEvent::new(KeyCode::Left, KeyModifiers::empty());
        assert_eq!(translate_key(&key), Some(b"\x1b[D".to_vec()));
    }

    #[test]
    fn test_shift_arrow_up() {
        let key = KeyEvent::new(KeyCode::Up, KeyModifiers::SHIFT);
        assert_eq!(translate_key(&key), Some(b"\x1b[1;2A".to_vec()));
    }

    #[test]
    fn test_ctrl_arrow_right() {
        let key = KeyEvent::new(KeyCode::Right, KeyModifiers::CONTROL);
        assert_eq!(translate_key(&key), Some(b"\x1b[1;5C".to_vec()));
    }

    #[test]
    fn test_alt_arrow_left() {
        let key = KeyEvent::new(KeyCode::Left, KeyModifiers::ALT);
        assert_eq!(translate_key(&key), Some(b"\x1b[1;3D".to_vec()));
    }

    #[test]
    fn test_ctrl_shift_arrow_down() {
        let key = KeyEvent::new(KeyCode::Down, KeyModifiers::CONTROL | KeyModifiers::SHIFT);
        assert_eq!(translate_key(&key), Some(b"\x1b[1;6B".to_vec()));
    }

    #[test]
    fn test_home() {
        let key = KeyEvent::new(KeyCode::Home, KeyModifiers::empty());
        assert_eq!(translate_key(&key), Some(b"\x1b[H".to_vec()));
    }

    #[test]
    fn test_end() {
        let key = KeyEvent::new(KeyCode::End, KeyModifiers::empty());
        assert_eq!(translate_key(&key), Some(b"\x1b[F".to_vec()));
    }

    #[test]
    fn test_page_up() {
        let key = KeyEvent::new(KeyCode::PageUp, KeyModifiers::empty());
        assert_eq!(translate_key(&key), Some(b"\x1b[5~".to_vec()));
    }

    #[test]
    fn test_page_down() {
        let key = KeyEvent::new(KeyCode::PageDown, KeyModifiers::empty());
        assert_eq!(translate_key(&key), Some(b"\x1b[6~".to_vec()));
    }

    #[test]
    fn test_insert() {
        let key = KeyEvent::new(KeyCode::Insert, KeyModifiers::empty());
        assert_eq!(translate_key(&key), Some(b"\x1b[2~".to_vec()));
    }

    #[test]
    fn test_delete() {
        let key = KeyEvent::new(KeyCode::Delete, KeyModifiers::empty());
        assert_eq!(translate_key(&key), Some(b"\x1b[3~".to_vec()));
    }

    #[test]
    fn test_ctrl_delete() {
        let key = KeyEvent::new(KeyCode::Delete, KeyModifiers::CONTROL);
        assert_eq!(translate_key(&key), Some(b"\x1b[3;5~".to_vec()));
    }

    #[test]
    fn test_f1() {
        let key = KeyEvent::new(KeyCode::F(1), KeyModifiers::empty());
        assert_eq!(translate_key(&key), Some(b"\x1bOP".to_vec()));
    }

    #[test]
    fn test_f2() {
        let key = KeyEvent::new(KeyCode::F(2), KeyModifiers::empty());
        assert_eq!(translate_key(&key), Some(b"\x1bOQ".to_vec()));
    }

    #[test]
    fn test_f3() {
        let key = KeyEvent::new(KeyCode::F(3), KeyModifiers::empty());
        assert_eq!(translate_key(&key), Some(b"\x1bOR".to_vec()));
    }

    #[test]
    fn test_f4() {
        let key = KeyEvent::new(KeyCode::F(4), KeyModifiers::empty());
        assert_eq!(translate_key(&key), Some(b"\x1bOS".to_vec()));
    }

    #[test]
    fn test_f5() {
        let key = KeyEvent::new(KeyCode::F(5), KeyModifiers::empty());
        assert_eq!(translate_key(&key), Some(b"\x1b[15~".to_vec()));
    }

    #[test]
    fn test_f12() {
        let key = KeyEvent::new(KeyCode::F(12), KeyModifiers::empty());
        assert_eq!(translate_key(&key), Some(b"\x1b[24~".to_vec()));
    }

    #[test]
    fn test_shift_f1() {
        let key = KeyEvent::new(KeyCode::F(1), KeyModifiers::SHIFT);
        assert_eq!(translate_key(&key), Some(b"\x1b[1;2P".to_vec()));
    }

    #[test]
    fn test_ctrl_f5() {
        let key = KeyEvent::new(KeyCode::F(5), KeyModifiers::CONTROL);
        assert_eq!(translate_key(&key), Some(b"\x1b[15;5~".to_vec()));
    }

    #[test]
    fn test_null_key() {
        let key = KeyEvent::new(KeyCode::Null, KeyModifiers::empty());
        assert_eq!(translate_key(&key), Some(vec![0]));
    }

    #[test]
    fn test_modifier_only() {
        use crossterm::event::ModifierKeyCode;
        let key = KeyEvent::new(KeyCode::Modifier(ModifierKeyCode::LeftShift), KeyModifiers::SHIFT);
        assert_eq!(translate_key(&key), None);
    }

    #[test]
    fn test_modifier_code_calculation() {
        assert_eq!(get_modifier_code(KeyModifiers::empty()), 1);
        assert_eq!(get_modifier_code(KeyModifiers::SHIFT), 2);
        assert_eq!(get_modifier_code(KeyModifiers::ALT), 3);
        assert_eq!(get_modifier_code(KeyModifiers::SHIFT | KeyModifiers::ALT), 4);
        assert_eq!(get_modifier_code(KeyModifiers::CONTROL), 5);
        assert_eq!(get_modifier_code(KeyModifiers::SHIFT | KeyModifiers::CONTROL), 6);
        assert_eq!(get_modifier_code(KeyModifiers::ALT | KeyModifiers::CONTROL), 7);
        assert_eq!(
            get_modifier_code(KeyModifiers::SHIFT | KeyModifiers::ALT | KeyModifiers::CONTROL),
            8
        );
    }

    // ==================== Key Binding Parser Tests ====================

    #[test]
    fn test_parse_simple_key() {
        let binding = parse_key_binding("Tab").unwrap();
        assert_eq!(binding.code, KeyCode::Tab);
        assert_eq!(binding.modifiers, KeyModifiers::empty());
    }

    #[test]
    fn test_parse_ctrl_key() {
        let binding = parse_key_binding("Ctrl-Tab").unwrap();
        assert_eq!(binding.code, KeyCode::Tab);
        assert_eq!(binding.modifiers, KeyModifiers::CONTROL);
    }

    #[test]
    fn test_parse_ctrl_pagedown() {
        let binding = parse_key_binding("Ctrl-PageDown").unwrap();
        assert_eq!(binding.code, KeyCode::PageDown);
        assert_eq!(binding.modifiers, KeyModifiers::CONTROL);
    }

    #[test]
    fn test_parse_ctrl_shift_pageup() {
        let binding = parse_key_binding("Ctrl-Shift-PageUp").unwrap();
        assert_eq!(binding.code, KeyCode::PageUp);
        assert_eq!(binding.modifiers, KeyModifiers::CONTROL | KeyModifiers::SHIFT);
    }

    #[test]
    fn test_parse_case_insensitive() {
        let lower = parse_key_binding("ctrl-pagedown").unwrap();
        let upper = parse_key_binding("CTRL-PAGEDOWN").unwrap();
        let mixed = parse_key_binding("Ctrl-PageDown").unwrap();

        assert_eq!(lower.code, upper.code);
        assert_eq!(lower.modifiers, upper.modifiers);
        assert_eq!(lower.code, mixed.code);
        assert_eq!(lower.modifiers, mixed.modifiers);
    }

    #[test]
    fn test_parse_function_keys() {
        let f1 = parse_key_binding("F1").unwrap();
        assert_eq!(f1.code, KeyCode::F(1));

        let f12 = parse_key_binding("F12").unwrap();
        assert_eq!(f12.code, KeyCode::F(12));

        let shift_f7 = parse_key_binding("Shift-F7").unwrap();
        assert_eq!(shift_f7.code, KeyCode::F(7));
        assert_eq!(shift_f7.modifiers, KeyModifiers::SHIFT);
    }

    #[test]
    fn test_parse_single_char() {
        let a = parse_key_binding("a").unwrap();
        assert_eq!(a.code, KeyCode::Char('a'));

        let ctrl_a = parse_key_binding("Ctrl-a").unwrap();
        assert_eq!(ctrl_a.code, KeyCode::Char('a'));
        assert_eq!(ctrl_a.modifiers, KeyModifiers::CONTROL);
    }

    #[test]
    fn test_parse_navigation_keys() {
        assert_eq!(parse_key_binding("Home").unwrap().code, KeyCode::Home);
        assert_eq!(parse_key_binding("End").unwrap().code, KeyCode::End);
        assert_eq!(parse_key_binding("PgUp").unwrap().code, KeyCode::PageUp);
        assert_eq!(parse_key_binding("PgDn").unwrap().code, KeyCode::PageDown);
        assert_eq!(parse_key_binding("Insert").unwrap().code, KeyCode::Insert);
        assert_eq!(parse_key_binding("Delete").unwrap().code, KeyCode::Delete);
    }

    #[test]
    fn test_parse_arrow_keys() {
        assert_eq!(parse_key_binding("Up").unwrap().code, KeyCode::Up);
        assert_eq!(parse_key_binding("Down").unwrap().code, KeyCode::Down);
        assert_eq!(parse_key_binding("Left").unwrap().code, KeyCode::Left);
        assert_eq!(parse_key_binding("Right").unwrap().code, KeyCode::Right);
    }

    #[test]
    fn test_parse_special_keys() {
        assert_eq!(parse_key_binding("Enter").unwrap().code, KeyCode::Enter);
        assert_eq!(parse_key_binding("Space").unwrap().code, KeyCode::Char(' '));
        assert_eq!(parse_key_binding("Backspace").unwrap().code, KeyCode::Backspace);
        assert_eq!(parse_key_binding("Esc").unwrap().code, KeyCode::Esc);
    }

    #[test]
    fn test_parse_alt_modifier() {
        let binding = parse_key_binding("Alt-Tab").unwrap();
        assert_eq!(binding.modifiers, KeyModifiers::ALT);

        let meta = parse_key_binding("Meta-a").unwrap();
        assert_eq!(meta.modifiers, KeyModifiers::ALT);
    }

    #[test]
    fn test_parse_empty_string() {
        assert!(matches!(
            parse_key_binding(""),
            Err(KeyBindingError::Empty)
        ));
        assert!(matches!(
            parse_key_binding("  "),
            Err(KeyBindingError::Empty)
        ));
    }

    #[test]
    fn test_parse_unknown_modifier() {
        assert!(matches!(
            parse_key_binding("Foo-Tab"),
            Err(KeyBindingError::UnknownModifier(_))
        ));
    }

    #[test]
    fn test_parse_unknown_key() {
        assert!(matches!(
            parse_key_binding("Ctrl-Unknown"),
            Err(KeyBindingError::UnknownKey(_))
        ));
    }

    #[test]
    fn test_parse_invalid_function_key() {
        assert!(matches!(
            parse_key_binding("F0"),
            Err(KeyBindingError::InvalidFunctionKey(_))
        ));
        assert!(matches!(
            parse_key_binding("F99"),
            Err(KeyBindingError::InvalidFunctionKey(_))
        ));
    }

    #[test]
    fn test_keybinding_matches() {
        let binding = parse_key_binding("Ctrl-PageDown").unwrap();

        let matching_event = KeyEvent::new(KeyCode::PageDown, KeyModifiers::CONTROL);
        assert!(binding.matches(&matching_event));

        let wrong_modifier = KeyEvent::new(KeyCode::PageDown, KeyModifiers::ALT);
        assert!(!binding.matches(&wrong_modifier));

        let wrong_key = KeyEvent::new(KeyCode::PageUp, KeyModifiers::CONTROL);
        assert!(!binding.matches(&wrong_key));

        let no_modifier = KeyEvent::new(KeyCode::PageDown, KeyModifiers::empty());
        assert!(!binding.matches(&no_modifier));
    }

    #[test]
    fn test_keybinding_matches_shift_combo() {
        let binding = parse_key_binding("Ctrl-Shift-PageUp").unwrap();
        let event = KeyEvent::new(KeyCode::PageUp, KeyModifiers::CONTROL | KeyModifiers::SHIFT);
        assert!(binding.matches(&event));

        // Wrong - missing shift
        let wrong = KeyEvent::new(KeyCode::PageUp, KeyModifiers::CONTROL);
        assert!(!binding.matches(&wrong));
    }

    #[test]
    fn test_keybinding_error_display() {
        assert_eq!(
            format!("{}", KeyBindingError::Empty),
            "empty key binding string"
        );
        assert_eq!(
            format!("{}", KeyBindingError::UnknownModifier("foo".into())),
            "unknown modifier: foo"
        );
        assert_eq!(
            format!("{}", KeyBindingError::UnknownKey("bar".into())),
            "unknown key: bar"
        );
    }
}
