//! Input handling for ccmux client
//!
//! This module provides comprehensive keyboard and mouse input handling,
//! including a state machine for prefix key combinations (similar to tmux).

// Allow unused code that's part of the public API for future features
#![allow(dead_code)]

mod commands;
mod keys;
mod mouse;

pub use commands::{ClientCommand, CommandHandler};
pub use keys::translate_key;
pub use mouse::handle_mouse_event;

use std::time::{Duration, Instant};

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers, MouseEvent};
use uuid::Uuid;

/// Default prefix key timeout in milliseconds
const DEFAULT_PREFIX_TIMEOUT_MS: u64 = 500;

/// Input handling mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    /// Normal mode - input goes to active pane
    Normal,
    /// Prefix key pressed - waiting for command key
    PrefixPending,
    /// Command entry mode (for : commands)
    Command,
    /// Copy/scroll mode (for browsing history)
    Copy,
}

/// Result of processing an input event
#[derive(Debug, Clone, PartialEq)]
pub enum InputAction {
    /// No action needed
    None,
    /// Send bytes to the active pane's PTY
    SendToPane(Vec<u8>),
    /// Execute a client command
    Command(ClientCommand),
    /// Focus pane at coordinates
    FocusPane { x: u16, y: u16 },
    /// Scroll the active pane up
    ScrollUp { lines: usize },
    /// Scroll the active pane down
    ScrollDown { lines: usize },
    /// Terminal resize event
    Resize { cols: u16, rows: u16 },
    /// Detach from session
    Detach,
    /// Quit the client
    Quit,
}

/// Main input handler with prefix key state machine
pub struct InputHandler {
    /// Current input mode
    mode: InputMode,
    /// Prefix key (default: Ctrl+B like tmux)
    prefix: KeyEvent,
    /// Timeout for prefix key
    prefix_timeout: Duration,
    /// Time when prefix was pressed
    prefix_time: Option<Instant>,
    /// Command buffer for command mode
    command_buffer: String,
    /// Active pane ID for context
    active_pane_id: Option<Uuid>,
    /// Scroll offset in copy mode
    scroll_offset: usize,
    /// Mouse capture enabled
    mouse_enabled: bool,
}

impl Default for InputHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl InputHandler {
    /// Create a new input handler with default settings
    pub fn new() -> Self {
        Self {
            mode: InputMode::Normal,
            prefix: KeyEvent::new(KeyCode::Char('b'), KeyModifiers::CONTROL),
            prefix_timeout: Duration::from_millis(DEFAULT_PREFIX_TIMEOUT_MS),
            prefix_time: None,
            command_buffer: String::new(),
            active_pane_id: None,
            scroll_offset: 0,
            mouse_enabled: true,
        }
    }

    /// Create with custom prefix key
    pub fn with_prefix(prefix: KeyEvent) -> Self {
        let mut handler = Self::new();
        handler.prefix = prefix;
        handler
    }

    /// Get current input mode
    pub fn mode(&self) -> InputMode {
        self.mode
    }

    /// Set the active pane ID
    pub fn set_active_pane(&mut self, pane_id: Option<Uuid>) {
        self.active_pane_id = pane_id;
    }

    /// Enable or disable mouse capture
    pub fn set_mouse_enabled(&mut self, enabled: bool) {
        self.mouse_enabled = enabled;
    }

    /// Check if mouse is enabled
    pub fn is_mouse_enabled(&self) -> bool {
        self.mouse_enabled
    }

    /// Get the command buffer contents (for command mode)
    pub fn command_buffer(&self) -> &str {
        &self.command_buffer
    }

    /// Get scroll offset (for copy mode)
    pub fn scroll_offset(&self) -> usize {
        self.scroll_offset
    }

    /// Process a crossterm event and return the appropriate action
    pub fn handle_event(&mut self, event: Event) -> InputAction {
        match event {
            Event::Key(key) => self.handle_key(key),
            Event::Mouse(mouse) => self.handle_mouse(mouse),
            Event::Resize(cols, rows) => InputAction::Resize { cols, rows },
            Event::FocusGained | Event::FocusLost => InputAction::None,
            Event::Paste(text) => {
                // Handle paste as input to pane
                InputAction::SendToPane(text.into_bytes())
            }
        }
    }

    /// Handle a key event
    fn handle_key(&mut self, key: KeyEvent) -> InputAction {
        // Check prefix timeout
        self.check_prefix_timeout();

        match self.mode {
            InputMode::Normal => self.handle_normal_key(key),
            InputMode::PrefixPending => self.handle_prefix_key(key),
            InputMode::Command => self.handle_command_key(key),
            InputMode::Copy => self.handle_copy_key(key),
        }
    }

    /// Check if prefix has timed out
    fn check_prefix_timeout(&mut self) {
        if self.mode == InputMode::PrefixPending {
            if let Some(time) = self.prefix_time {
                if time.elapsed() > self.prefix_timeout {
                    self.mode = InputMode::Normal;
                    self.prefix_time = None;
                }
            }
        }
    }

    /// Handle key in normal mode
    fn handle_normal_key(&mut self, key: KeyEvent) -> InputAction {
        // Global quit binding (Ctrl+Q)
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('q') {
            return InputAction::Quit;
        }

        // Check for prefix key
        if self.is_prefix_key(&key) {
            self.mode = InputMode::PrefixPending;
            self.prefix_time = Some(Instant::now());
            return InputAction::None;
        }

        // Translate key to bytes and send to pane
        if let Some(bytes) = translate_key(&key) {
            InputAction::SendToPane(bytes)
        } else {
            InputAction::None
        }
    }

    /// Handle key after prefix was pressed
    fn handle_prefix_key(&mut self, key: KeyEvent) -> InputAction {
        // Reset to normal mode
        self.mode = InputMode::Normal;
        self.prefix_time = None;

        // If prefix key pressed again, send literal prefix to pane
        if self.is_prefix_key(&key) {
            if let Some(bytes) = translate_key(&key) {
                return InputAction::SendToPane(bytes);
            }
        }

        // Handle command key
        match key.code {
            // Pane creation/management
            KeyCode::Char('c') => InputAction::Command(ClientCommand::CreatePane),
            KeyCode::Char('x') => InputAction::Command(ClientCommand::ClosePane),

            // Pane navigation
            KeyCode::Char('n') => InputAction::Command(ClientCommand::NextPane),
            KeyCode::Char('p') => InputAction::Command(ClientCommand::PreviousPane),
            KeyCode::Char('h') | KeyCode::Left => InputAction::Command(ClientCommand::PaneLeft),
            KeyCode::Char('j') | KeyCode::Down => InputAction::Command(ClientCommand::PaneDown),
            KeyCode::Char('k') | KeyCode::Up => InputAction::Command(ClientCommand::PaneUp),
            KeyCode::Char('l') | KeyCode::Right => InputAction::Command(ClientCommand::PaneRight),

            // Window management
            KeyCode::Char('w') => InputAction::Command(ClientCommand::ListWindows),

            // Session management
            KeyCode::Char('d') => InputAction::Detach,
            KeyCode::Char('s') => InputAction::Command(ClientCommand::ListSessions),

            // Modes
            KeyCode::Char(':') => {
                self.mode = InputMode::Command;
                self.command_buffer.clear();
                InputAction::None
            }
            KeyCode::Char('[') => {
                self.mode = InputMode::Copy;
                self.scroll_offset = 0;
                InputAction::Command(ClientCommand::EnterCopyMode)
            }

            // Split panes
            KeyCode::Char('%') => InputAction::Command(ClientCommand::SplitVertical),
            KeyCode::Char('"') => InputAction::Command(ClientCommand::SplitHorizontal),

            // Zoom/fullscreen pane
            KeyCode::Char('z') => InputAction::Command(ClientCommand::ToggleZoom),

            // Help
            KeyCode::Char('?') => InputAction::Command(ClientCommand::ShowHelp),

            _ => InputAction::None,
        }
    }

    /// Handle key in command mode
    fn handle_command_key(&mut self, key: KeyEvent) -> InputAction {
        match key.code {
            KeyCode::Esc => {
                self.mode = InputMode::Normal;
                self.command_buffer.clear();
                InputAction::None
            }
            KeyCode::Enter => {
                self.mode = InputMode::Normal;
                let command = std::mem::take(&mut self.command_buffer);
                if let Some(cmd) = CommandHandler::parse_command(&command) {
                    InputAction::Command(cmd)
                } else {
                    InputAction::None
                }
            }
            KeyCode::Backspace => {
                self.command_buffer.pop();
                InputAction::None
            }
            KeyCode::Char(c) => {
                self.command_buffer.push(c);
                InputAction::None
            }
            _ => InputAction::None,
        }
    }

    /// Handle key in copy mode
    fn handle_copy_key(&mut self, key: KeyEvent) -> InputAction {
        match key.code {
            // Exit copy mode
            KeyCode::Esc | KeyCode::Char('q') => {
                self.mode = InputMode::Normal;
                self.scroll_offset = 0;
                InputAction::Command(ClientCommand::ExitCopyMode)
            }

            // Navigation
            KeyCode::Up | KeyCode::Char('k') => {
                self.scroll_offset = self.scroll_offset.saturating_add(1);
                InputAction::ScrollUp { lines: 1 }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.scroll_offset > 0 {
                    self.scroll_offset = self.scroll_offset.saturating_sub(1);
                    InputAction::ScrollDown { lines: 1 }
                } else {
                    InputAction::None
                }
            }
            KeyCode::PageUp | KeyCode::Char('b') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.scroll_offset = self.scroll_offset.saturating_add(24);
                InputAction::ScrollUp { lines: 24 }
            }
            KeyCode::PageDown | KeyCode::Char('f') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.scroll_offset = self.scroll_offset.saturating_sub(24);
                InputAction::ScrollDown { lines: 24 }
            }
            KeyCode::Char('g') => {
                // Go to top
                self.scroll_offset = usize::MAX;
                InputAction::ScrollUp { lines: usize::MAX }
            }
            KeyCode::Char('G') => {
                // Go to bottom
                self.scroll_offset = 0;
                InputAction::ScrollDown { lines: usize::MAX }
            }

            _ => InputAction::None,
        }
    }

    /// Handle mouse event
    fn handle_mouse(&mut self, mouse: MouseEvent) -> InputAction {
        if !self.mouse_enabled {
            return InputAction::None;
        }
        handle_mouse_event(mouse, self.mode)
    }

    /// Check if a key matches the prefix key
    fn is_prefix_key(&self, key: &KeyEvent) -> bool {
        key.code == self.prefix.code && key.modifiers == self.prefix.modifiers
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{MouseButton, MouseEventKind};

    #[test]
    fn test_input_handler_default() {
        let handler = InputHandler::new();
        assert_eq!(handler.mode(), InputMode::Normal);
        assert!(handler.is_mouse_enabled());
    }

    #[test]
    fn test_prefix_key_detection() {
        let mut handler = InputHandler::new();

        // Prefix key (Ctrl+B)
        let prefix_key = KeyEvent::new(KeyCode::Char('b'), KeyModifiers::CONTROL);
        let result = handler.handle_key(prefix_key);

        assert_eq!(result, InputAction::None);
        assert_eq!(handler.mode(), InputMode::PrefixPending);
    }

    #[test]
    fn test_prefix_then_command() {
        let mut handler = InputHandler::new();

        // Press prefix
        let prefix = KeyEvent::new(KeyCode::Char('b'), KeyModifiers::CONTROL);
        handler.handle_key(prefix);
        assert_eq!(handler.mode(), InputMode::PrefixPending);

        // Press 'c' for create pane
        let c_key = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::empty());
        let result = handler.handle_key(c_key);

        assert_eq!(result, InputAction::Command(ClientCommand::CreatePane));
        assert_eq!(handler.mode(), InputMode::Normal);
    }

    #[test]
    fn test_double_prefix_sends_literal() {
        let mut handler = InputHandler::new();

        // Press prefix twice
        let prefix = KeyEvent::new(KeyCode::Char('b'), KeyModifiers::CONTROL);
        handler.handle_key(prefix.clone());
        let result = handler.handle_key(prefix);

        // Should send literal Ctrl+B
        assert!(matches!(result, InputAction::SendToPane(_)));
        assert_eq!(handler.mode(), InputMode::Normal);
    }

    #[test]
    fn test_normal_key_translation() {
        let mut handler = InputHandler::new();

        let key = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::empty());
        let result = handler.handle_key(key);

        assert_eq!(result, InputAction::SendToPane(vec![b'a']));
    }

    #[test]
    fn test_quit_binding() {
        let mut handler = InputHandler::new();

        let key = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::CONTROL);
        let result = handler.handle_key(key);

        assert_eq!(result, InputAction::Quit);
    }

    #[test]
    fn test_detach_command() {
        let mut handler = InputHandler::new();

        // Press prefix + d
        let prefix = KeyEvent::new(KeyCode::Char('b'), KeyModifiers::CONTROL);
        handler.handle_key(prefix);

        let d_key = KeyEvent::new(KeyCode::Char('d'), KeyModifiers::empty());
        let result = handler.handle_key(d_key);

        assert_eq!(result, InputAction::Detach);
    }

    #[test]
    fn test_enter_command_mode() {
        let mut handler = InputHandler::new();

        // Press prefix + :
        let prefix = KeyEvent::new(KeyCode::Char('b'), KeyModifiers::CONTROL);
        handler.handle_key(prefix);

        let colon_key = KeyEvent::new(KeyCode::Char(':'), KeyModifiers::empty());
        handler.handle_key(colon_key);

        assert_eq!(handler.mode(), InputMode::Command);
    }

    #[test]
    fn test_command_mode_input() {
        let mut handler = InputHandler::new();
        handler.mode = InputMode::Command;

        // Type "quit"
        for c in ['q', 'u', 'i', 't'] {
            let key = KeyEvent::new(KeyCode::Char(c), KeyModifiers::empty());
            handler.handle_key(key);
        }

        assert_eq!(handler.command_buffer(), "quit");
    }

    #[test]
    fn test_command_mode_escape() {
        let mut handler = InputHandler::new();
        handler.mode = InputMode::Command;
        handler.command_buffer = "test".to_string();

        let esc_key = KeyEvent::new(KeyCode::Esc, KeyModifiers::empty());
        handler.handle_key(esc_key);

        assert_eq!(handler.mode(), InputMode::Normal);
        assert!(handler.command_buffer().is_empty());
    }

    #[test]
    fn test_enter_copy_mode() {
        let mut handler = InputHandler::new();

        // Press prefix + [
        let prefix = KeyEvent::new(KeyCode::Char('b'), KeyModifiers::CONTROL);
        handler.handle_key(prefix);

        let bracket_key = KeyEvent::new(KeyCode::Char('['), KeyModifiers::empty());
        let result = handler.handle_key(bracket_key);

        assert_eq!(handler.mode(), InputMode::Copy);
        assert_eq!(result, InputAction::Command(ClientCommand::EnterCopyMode));
    }

    #[test]
    fn test_copy_mode_navigation() {
        let mut handler = InputHandler::new();
        handler.mode = InputMode::Copy;

        // Scroll up
        let up_key = KeyEvent::new(KeyCode::Up, KeyModifiers::empty());
        let result = handler.handle_key(up_key);

        assert_eq!(result, InputAction::ScrollUp { lines: 1 });
        assert_eq!(handler.scroll_offset(), 1);
    }

    #[test]
    fn test_exit_copy_mode() {
        let mut handler = InputHandler::new();
        handler.mode = InputMode::Copy;
        handler.scroll_offset = 10;

        let q_key = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::empty());
        let result = handler.handle_key(q_key);

        assert_eq!(handler.mode(), InputMode::Normal);
        assert_eq!(handler.scroll_offset(), 0);
        assert_eq!(result, InputAction::Command(ClientCommand::ExitCopyMode));
    }

    #[test]
    fn test_mouse_disabled() {
        let mut handler = InputHandler::new();
        handler.set_mouse_enabled(false);

        let mouse = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 10,
            row: 5,
            modifiers: KeyModifiers::empty(),
        };

        let result = handler.handle_mouse(mouse);
        assert_eq!(result, InputAction::None);
    }

    #[test]
    fn test_resize_event() {
        let mut handler = InputHandler::new();

        let event = Event::Resize(120, 40);
        let result = handler.handle_event(event);

        assert_eq!(result, InputAction::Resize { cols: 120, rows: 40 });
    }

    #[test]
    fn test_paste_event() {
        let mut handler = InputHandler::new();

        let event = Event::Paste("hello world".to_string());
        let result = handler.handle_event(event);

        assert_eq!(result, InputAction::SendToPane(b"hello world".to_vec()));
    }

    #[test]
    fn test_custom_prefix() {
        let custom_prefix = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::CONTROL);
        let mut handler = InputHandler::with_prefix(custom_prefix.clone());

        // Regular Ctrl+B should pass through
        let ctrlb = KeyEvent::new(KeyCode::Char('b'), KeyModifiers::CONTROL);
        let result = handler.handle_key(ctrlb);
        assert!(matches!(result, InputAction::SendToPane(_)));

        // Custom prefix should activate prefix mode
        let result = handler.handle_key(custom_prefix);
        assert_eq!(result, InputAction::None);
        assert_eq!(handler.mode(), InputMode::PrefixPending);
    }

    #[test]
    fn test_prefix_timeout() {
        let mut handler = InputHandler::new();
        handler.prefix_timeout = Duration::from_millis(1);

        // Press prefix
        let prefix = KeyEvent::new(KeyCode::Char('b'), KeyModifiers::CONTROL);
        handler.handle_key(prefix);
        assert_eq!(handler.mode(), InputMode::PrefixPending);

        // Wait for timeout
        std::thread::sleep(Duration::from_millis(10));

        // Next key should check timeout and reset to normal mode
        let key = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::empty());
        handler.handle_key(key);

        // Mode should have been reset to Normal before handling 'a'
        assert_eq!(handler.mode(), InputMode::Normal);
    }

    #[test]
    fn test_split_commands() {
        let mut handler = InputHandler::new();

        // Test vertical split (%)
        let prefix = KeyEvent::new(KeyCode::Char('b'), KeyModifiers::CONTROL);
        handler.handle_key(prefix);
        let percent = KeyEvent::new(KeyCode::Char('%'), KeyModifiers::empty());
        let result = handler.handle_key(percent);
        assert_eq!(result, InputAction::Command(ClientCommand::SplitVertical));

        // Test horizontal split (")
        let prefix = KeyEvent::new(KeyCode::Char('b'), KeyModifiers::CONTROL);
        handler.handle_key(prefix);
        let quote = KeyEvent::new(KeyCode::Char('"'), KeyModifiers::empty());
        let result = handler.handle_key(quote);
        assert_eq!(result, InputAction::Command(ClientCommand::SplitHorizontal));
    }

    #[test]
    fn test_vim_navigation() {
        let mut handler = InputHandler::new();

        // Test h/j/k/l navigation
        let prefix = KeyEvent::new(KeyCode::Char('b'), KeyModifiers::CONTROL);

        handler.handle_key(prefix.clone());
        let h_key = KeyEvent::new(KeyCode::Char('h'), KeyModifiers::empty());
        assert_eq!(handler.handle_key(h_key), InputAction::Command(ClientCommand::PaneLeft));

        handler.handle_key(prefix.clone());
        let j_key = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::empty());
        assert_eq!(handler.handle_key(j_key), InputAction::Command(ClientCommand::PaneDown));

        handler.handle_key(prefix.clone());
        let k_key = KeyEvent::new(KeyCode::Char('k'), KeyModifiers::empty());
        assert_eq!(handler.handle_key(k_key), InputAction::Command(ClientCommand::PaneUp));

        handler.handle_key(prefix);
        let l_key = KeyEvent::new(KeyCode::Char('l'), KeyModifiers::empty());
        assert_eq!(handler.handle_key(l_key), InputAction::Command(ClientCommand::PaneRight));
    }
}
