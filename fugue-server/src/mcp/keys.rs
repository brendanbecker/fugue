//! Special key name to ANSI escape sequence mappings (FEAT-093)
//!
//! This module provides mappings from human-readable key names to their
//! corresponding ANSI escape sequences for sending to PTY.

use std::collections::HashMap;
use std::sync::LazyLock;

/// Mapping from key names to ANSI escape sequences
static KEY_SEQUENCES: LazyLock<HashMap<&'static str, &'static [u8]>> = LazyLock::new(|| {
    let mut m = HashMap::new();

    // Escape key
    m.insert("Escape", b"\x1b" as &[u8]);
    m.insert("Esc", b"\x1b");

    // Control sequences (Ctrl+letter produces ASCII control codes)
    m.insert("Ctrl+A", b"\x01");
    m.insert("Ctrl+B", b"\x02");
    m.insert("Ctrl+C", b"\x03");
    m.insert("Ctrl+D", b"\x04");
    m.insert("Ctrl+E", b"\x05");
    m.insert("Ctrl+F", b"\x06");
    m.insert("Ctrl+G", b"\x07");
    m.insert("Ctrl+H", b"\x08");
    m.insert("Ctrl+I", b"\x09"); // Tab
    m.insert("Ctrl+J", b"\x0a"); // Line feed
    m.insert("Ctrl+K", b"\x0b");
    m.insert("Ctrl+L", b"\x0c"); // Form feed / clear screen
    m.insert("Ctrl+M", b"\x0d"); // Carriage return
    m.insert("Ctrl+N", b"\x0e");
    m.insert("Ctrl+O", b"\x0f");
    m.insert("Ctrl+P", b"\x10");
    m.insert("Ctrl+Q", b"\x11"); // XON
    m.insert("Ctrl+R", b"\x12");
    m.insert("Ctrl+S", b"\x13"); // XOFF
    m.insert("Ctrl+T", b"\x14");
    m.insert("Ctrl+U", b"\x15");
    m.insert("Ctrl+V", b"\x16");
    m.insert("Ctrl+W", b"\x17");
    m.insert("Ctrl+X", b"\x18");
    m.insert("Ctrl+Y", b"\x19");
    m.insert("Ctrl+Z", b"\x1a"); // Suspend (SIGTSTP)

    // Lowercase variants for convenience
    m.insert("Ctrl+a", b"\x01");
    m.insert("Ctrl+b", b"\x02");
    m.insert("Ctrl+c", b"\x03");
    m.insert("Ctrl+d", b"\x04");
    m.insert("Ctrl+e", b"\x05");
    m.insert("Ctrl+f", b"\x06");
    m.insert("Ctrl+g", b"\x07");
    m.insert("Ctrl+h", b"\x08");
    m.insert("Ctrl+i", b"\x09");
    m.insert("Ctrl+j", b"\x0a");
    m.insert("Ctrl+k", b"\x0b");
    m.insert("Ctrl+l", b"\x0c");
    m.insert("Ctrl+m", b"\x0d");
    m.insert("Ctrl+n", b"\x0e");
    m.insert("Ctrl+o", b"\x0f");
    m.insert("Ctrl+p", b"\x10");
    m.insert("Ctrl+q", b"\x11");
    m.insert("Ctrl+r", b"\x12");
    m.insert("Ctrl+s", b"\x13");
    m.insert("Ctrl+t", b"\x14");
    m.insert("Ctrl+u", b"\x15");
    m.insert("Ctrl+v", b"\x16");
    m.insert("Ctrl+w", b"\x17");
    m.insert("Ctrl+x", b"\x18");
    m.insert("Ctrl+y", b"\x19");
    m.insert("Ctrl+z", b"\x1a");

    // Special control characters
    m.insert("Ctrl+[", b"\x1b"); // Escape
    m.insert("Ctrl+\\", b"\x1c"); // SIGQUIT
    m.insert("Ctrl+]", b"\x1d");
    m.insert("Ctrl+^", b"\x1e");
    m.insert("Ctrl+_", b"\x1f");

    // Arrow keys (standard VT100/ANSI)
    m.insert("ArrowUp", b"\x1b[A");
    m.insert("ArrowDown", b"\x1b[B");
    m.insert("ArrowRight", b"\x1b[C");
    m.insert("ArrowLeft", b"\x1b[D");
    m.insert("Up", b"\x1b[A");
    m.insert("Down", b"\x1b[B");
    m.insert("Right", b"\x1b[C");
    m.insert("Left", b"\x1b[D");

    // Function keys (VT100/xterm)
    m.insert("F1", b"\x1bOP");
    m.insert("F2", b"\x1bOQ");
    m.insert("F3", b"\x1bOR");
    m.insert("F4", b"\x1bOS");
    m.insert("F5", b"\x1b[15~");
    m.insert("F6", b"\x1b[17~");
    m.insert("F7", b"\x1b[18~");
    m.insert("F8", b"\x1b[19~");
    m.insert("F9", b"\x1b[20~");
    m.insert("F10", b"\x1b[21~");
    m.insert("F11", b"\x1b[23~");
    m.insert("F12", b"\x1b[24~");

    // Navigation keys
    m.insert("Home", b"\x1b[H");
    m.insert("End", b"\x1b[F");
    m.insert("Insert", b"\x1b[2~");
    m.insert("Delete", b"\x1b[3~");
    m.insert("PageUp", b"\x1b[5~");
    m.insert("PageDown", b"\x1b[6~");

    // Common aliases
    m.insert("Tab", b"\x09");
    m.insert("Enter", b"\x0d");
    m.insert("Return", b"\x0d");
    m.insert("Backspace", b"\x7f");
    m.insert("Space", b" ");

    m
});

/// Get the ANSI escape sequence for a key name
///
/// Returns `None` if the key name is not recognized.
pub fn get_key_sequence(key_name: &str) -> Option<&'static [u8]> {
    KEY_SEQUENCES.get(key_name).copied()
}

/// Get a list of all supported key names
#[allow(dead_code)] // Utility function for key sequence discovery
pub fn supported_keys() -> Vec<&'static str> {
    let mut keys: Vec<_> = KEY_SEQUENCES.keys().copied().collect();
    keys.sort();
    keys
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escape_key() {
        assert_eq!(get_key_sequence("Escape"), Some(b"\x1b" as &[u8]));
        assert_eq!(get_key_sequence("Esc"), Some(b"\x1b" as &[u8]));
    }

    #[test]
    fn test_ctrl_sequences() {
        assert_eq!(get_key_sequence("Ctrl+C"), Some(b"\x03" as &[u8]));
        assert_eq!(get_key_sequence("Ctrl+c"), Some(b"\x03" as &[u8]));
        assert_eq!(get_key_sequence("Ctrl+D"), Some(b"\x04" as &[u8]));
        assert_eq!(get_key_sequence("Ctrl+Z"), Some(b"\x1a" as &[u8]));
        assert_eq!(get_key_sequence("Ctrl+L"), Some(b"\x0c" as &[u8]));
    }

    #[test]
    fn test_arrow_keys() {
        assert_eq!(get_key_sequence("ArrowUp"), Some(b"\x1b[A" as &[u8]));
        assert_eq!(get_key_sequence("ArrowDown"), Some(b"\x1b[B" as &[u8]));
        assert_eq!(get_key_sequence("ArrowRight"), Some(b"\x1b[C" as &[u8]));
        assert_eq!(get_key_sequence("ArrowLeft"), Some(b"\x1b[D" as &[u8]));
        // Short aliases
        assert_eq!(get_key_sequence("Up"), Some(b"\x1b[A" as &[u8]));
        assert_eq!(get_key_sequence("Down"), Some(b"\x1b[B" as &[u8]));
    }

    #[test]
    fn test_function_keys() {
        assert_eq!(get_key_sequence("F1"), Some(b"\x1bOP" as &[u8]));
        assert_eq!(get_key_sequence("F5"), Some(b"\x1b[15~" as &[u8]));
        assert_eq!(get_key_sequence("F12"), Some(b"\x1b[24~" as &[u8]));
    }

    #[test]
    fn test_navigation_keys() {
        assert_eq!(get_key_sequence("Home"), Some(b"\x1b[H" as &[u8]));
        assert_eq!(get_key_sequence("End"), Some(b"\x1b[F" as &[u8]));
        assert_eq!(get_key_sequence("PageUp"), Some(b"\x1b[5~" as &[u8]));
        assert_eq!(get_key_sequence("PageDown"), Some(b"\x1b[6~" as &[u8]));
        assert_eq!(get_key_sequence("Delete"), Some(b"\x1b[3~" as &[u8]));
        assert_eq!(get_key_sequence("Insert"), Some(b"\x1b[2~" as &[u8]));
    }

    #[test]
    fn test_common_keys() {
        assert_eq!(get_key_sequence("Tab"), Some(b"\x09" as &[u8]));
        assert_eq!(get_key_sequence("Enter"), Some(b"\x0d" as &[u8]));
        assert_eq!(get_key_sequence("Return"), Some(b"\x0d" as &[u8]));
        assert_eq!(get_key_sequence("Backspace"), Some(b"\x7f" as &[u8]));
        assert_eq!(get_key_sequence("Space"), Some(b" " as &[u8]));
    }

    #[test]
    fn test_unknown_key() {
        assert_eq!(get_key_sequence("UnknownKey"), None);
        assert_eq!(get_key_sequence(""), None);
    }

    #[test]
    fn test_supported_keys_not_empty() {
        let keys = supported_keys();
        assert!(!keys.is_empty());
        assert!(keys.len() > 50); // We have many keys defined
    }

    #[test]
    fn test_supported_keys_contains_expected() {
        let keys = supported_keys();
        assert!(keys.contains(&"Escape"));
        assert!(keys.contains(&"Ctrl+C"));
        assert!(keys.contains(&"ArrowUp"));
        assert!(keys.contains(&"F1"));
    }
}
