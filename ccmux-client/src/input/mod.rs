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
pub use keys::{translate_key, KeyBinding};
pub use mouse::handle_mouse_event;
// QuickBindings is defined in this module and is public

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

/// Quick navigation bindings (no prefix required)
///
/// These bindings are checked before the prefix key, allowing
/// fast navigation without the prefix key delay.
#[derive(Debug, Clone)]
pub struct QuickBindings {
    /// Switch to next window (default: Ctrl+PageDown)
    pub next_window: Option<KeyBinding>,
    /// Switch to previous window (default: Ctrl+PageUp)
    pub prev_window: Option<KeyBinding>,
    /// Switch to next pane in current window (default: Ctrl+Shift+PageDown)
    pub next_pane: Option<KeyBinding>,
    /// Switch to previous pane (default: Ctrl+Shift+PageUp)
    pub prev_pane: Option<KeyBinding>,
}

impl Default for QuickBindings {
    fn default() -> Self {
        Self {
            next_window: KeyBinding::parse("Ctrl-PageDown").ok(),
            prev_window: KeyBinding::parse("Ctrl-PageUp").ok(),
            next_pane: KeyBinding::parse("Ctrl-Shift-PageDown").ok(),
            prev_pane: KeyBinding::parse("Ctrl-Shift-PageUp").ok(),
        }
    }
}

impl QuickBindings {
    /// Create empty quick bindings (all disabled)
    pub fn none() -> Self {
        Self {
            next_window: None,
            prev_window: None,
            next_pane: None,
            prev_pane: None,
        }
    }

    /// Create quick bindings from config strings
    ///
    /// Empty strings disable the binding.
    /// Invalid strings are logged and treated as disabled.
    pub fn from_config(
        next_window: &str,
        prev_window: &str,
        next_pane: &str,
        prev_pane: &str,
    ) -> Self {
        Self {
            next_window: Self::parse_optional(next_window, "next_window"),
            prev_window: Self::parse_optional(prev_window, "prev_window"),
            next_pane: Self::parse_optional(next_pane, "next_pane"),
            prev_pane: Self::parse_optional(prev_pane, "prev_pane"),
        }
    }

    /// Parse an optional binding (empty = disabled)
    fn parse_optional(s: &str, name: &str) -> Option<KeyBinding> {
        let s = s.trim();
        if s.is_empty() {
            return None;
        }
        match KeyBinding::parse(s) {
            Ok(binding) => Some(binding),
            Err(e) => {
                tracing::warn!("Invalid quick binding for {}: {} ({})", name, s, e);
                None
            }
        }
    }
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
    /// Quick navigation bindings (no prefix required)
    quick_bindings: QuickBindings,
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
            quick_bindings: QuickBindings::default(),
        }
    }

    /// Create with custom prefix key
    pub fn with_prefix(prefix: KeyEvent) -> Self {
        let mut handler = Self::new();
        handler.prefix = prefix;
        handler
    }

    /// Set quick navigation bindings
    pub fn set_quick_bindings(&mut self, bindings: QuickBindings) {
        self.quick_bindings = bindings;
    }

    /// Get quick bindings (for testing/inspection)
    pub fn quick_bindings(&self) -> &QuickBindings {
        &self.quick_bindings
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

        // Check for quick navigation bindings (no prefix required)
        if let Some(action) = self.check_quick_bindings(&key) {
            return action;
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

    /// Check if key matches any quick navigation binding
    fn check_quick_bindings(&self, key: &KeyEvent) -> Option<InputAction> {
        if let Some(ref binding) = self.quick_bindings.next_window {
            if binding.matches(key) {
                return Some(InputAction::Command(ClientCommand::NextWindow));
            }
        }
        if let Some(ref binding) = self.quick_bindings.prev_window {
            if binding.matches(key) {
                return Some(InputAction::Command(ClientCommand::PreviousWindow));
            }
        }
        if let Some(ref binding) = self.quick_bindings.next_pane {
            if binding.matches(key) {
                return Some(InputAction::Command(ClientCommand::NextPane));
            }
        }
        if let Some(ref binding) = self.quick_bindings.prev_pane {
            if binding.matches(key) {
                return Some(InputAction::Command(ClientCommand::PreviousPane));
            }
        }
        None
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

        // Handle command key (tmux-compatible bindings)
        match key.code {
            // Window management (tmux defaults)
            KeyCode::Char('c') => InputAction::Command(ClientCommand::CreateWindow),
            KeyCode::Char('&') => InputAction::Command(ClientCommand::CloseWindow),
            KeyCode::Char('n') => InputAction::Command(ClientCommand::NextWindow),
            KeyCode::Char('p') => InputAction::Command(ClientCommand::PreviousWindow),
            KeyCode::Char('w') => InputAction::Command(ClientCommand::ListWindows),
            KeyCode::Char('0') => InputAction::Command(ClientCommand::SelectWindow(0)),
            KeyCode::Char('1') => InputAction::Command(ClientCommand::SelectWindow(1)),
            KeyCode::Char('2') => InputAction::Command(ClientCommand::SelectWindow(2)),
            KeyCode::Char('3') => InputAction::Command(ClientCommand::SelectWindow(3)),
            KeyCode::Char('4') => InputAction::Command(ClientCommand::SelectWindow(4)),
            KeyCode::Char('5') => InputAction::Command(ClientCommand::SelectWindow(5)),
            KeyCode::Char('6') => InputAction::Command(ClientCommand::SelectWindow(6)),
            KeyCode::Char('7') => InputAction::Command(ClientCommand::SelectWindow(7)),
            KeyCode::Char('8') => InputAction::Command(ClientCommand::SelectWindow(8)),
            KeyCode::Char('9') => InputAction::Command(ClientCommand::SelectWindow(9)),

            // Pane management (tmux defaults)
            KeyCode::Char('x') => InputAction::Command(ClientCommand::ClosePane),
            KeyCode::Char('%') => InputAction::Command(ClientCommand::SplitVertical),
            KeyCode::Char('"') => InputAction::Command(ClientCommand::SplitHorizontal),

            // Pane navigation (tmux defaults)
            KeyCode::Char('o') => InputAction::Command(ClientCommand::NextPane),
            KeyCode::Char(';') => InputAction::Command(ClientCommand::PreviousPane),
            KeyCode::Left => InputAction::Command(ClientCommand::PaneLeft),
            KeyCode::Down => InputAction::Command(ClientCommand::PaneDown),
            KeyCode::Up => InputAction::Command(ClientCommand::PaneUp),
            KeyCode::Right => InputAction::Command(ClientCommand::PaneRight),

            // Pane navigation (vim-style extension, common in tmux configs)
            KeyCode::Char('h') => InputAction::Command(ClientCommand::PaneLeft),
            KeyCode::Char('j') => InputAction::Command(ClientCommand::PaneDown),
            KeyCode::Char('k') => InputAction::Command(ClientCommand::PaneUp),
            KeyCode::Char('l') => InputAction::Command(ClientCommand::PaneRight),

            // Zoom/fullscreen pane
            KeyCode::Char('z') => InputAction::Command(ClientCommand::ToggleZoom),

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
            KeyCode::Esc => {
                self.mode = InputMode::Normal;
                self.scroll_offset = 0;
                InputAction::Command(ClientCommand::ExitCopyMode)
            }

            // Cancel selection / exit (q)
            KeyCode::Char('q') => {
                // If in visual mode, first cancel selection
                // If not, exit copy mode
                self.mode = InputMode::Normal;
                self.scroll_offset = 0;
                InputAction::Command(ClientCommand::ExitCopyMode)
            }

            // Start visual mode (character-wise)
            KeyCode::Char('v') => {
                InputAction::Command(ClientCommand::StartVisualMode)
            }

            // Start visual line mode
            KeyCode::Char('V') => {
                InputAction::Command(ClientCommand::StartVisualLineMode)
            }

            // Yank selection
            KeyCode::Char('y') | KeyCode::Enter | KeyCode::Char(' ') => {
                // Yank will be handled by app - it will yank and exit copy mode
                let action = InputAction::Command(ClientCommand::YankSelection);
                self.mode = InputMode::Normal;
                self.scroll_offset = 0;
                action
            }

            // Vertical navigation (cursor up/down)
            KeyCode::Up | KeyCode::Char('k') => {
                InputAction::Command(ClientCommand::MoveCopyCursor {
                    row_delta: -1,
                    col_delta: 0,
                })
            }
            KeyCode::Down | KeyCode::Char('j') => {
                InputAction::Command(ClientCommand::MoveCopyCursor {
                    row_delta: 1,
                    col_delta: 0,
                })
            }

            // Horizontal navigation (cursor left/right)
            KeyCode::Left | KeyCode::Char('h') => {
                InputAction::Command(ClientCommand::MoveCopyCursor {
                    row_delta: 0,
                    col_delta: -1,
                })
            }
            KeyCode::Right | KeyCode::Char('l') => {
                InputAction::Command(ClientCommand::MoveCopyCursor {
                    row_delta: 0,
                    col_delta: 1,
                })
            }

            // Beginning/end of line
            KeyCode::Char('0') | KeyCode::Home => {
                InputAction::Command(ClientCommand::MoveCopyCursor {
                    row_delta: 0,
                    col_delta: -1000, // Move to beginning (will be clamped)
                })
            }
            KeyCode::Char('$') | KeyCode::End => {
                InputAction::Command(ClientCommand::MoveCopyCursor {
                    row_delta: 0,
                    col_delta: 1000, // Move to end (will be clamped)
                })
            }

            // Page navigation
            KeyCode::PageUp | KeyCode::Char('b') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.scroll_offset = self.scroll_offset.saturating_add(24);
                InputAction::ScrollUp { lines: 24 }
            }
            KeyCode::PageDown | KeyCode::Char('f') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.scroll_offset = self.scroll_offset.saturating_sub(24);
                InputAction::ScrollDown { lines: 24 }
            }

            // Go to top/bottom
            KeyCode::Char('g') if !key.modifiers.contains(KeyModifiers::SHIFT) => {
                self.scroll_offset = usize::MAX;
                InputAction::ScrollUp { lines: usize::MAX }
            }
            KeyCode::Char('G') => {
                self.scroll_offset = 0;
                InputAction::ScrollDown { lines: usize::MAX }
            }

            // Word movement (simplified - move by 5 chars)
            KeyCode::Char('w') => {
                InputAction::Command(ClientCommand::MoveCopyCursor {
                    row_delta: 0,
                    col_delta: 5,
                })
            }
            KeyCode::Char('b') if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                InputAction::Command(ClientCommand::MoveCopyCursor {
                    row_delta: 0,
                    col_delta: -5,
                })
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

        // Press 'c' for create window (tmux default)
        let c_key = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::empty());
        let result = handler.handle_key(c_key);

        assert_eq!(result, InputAction::Command(ClientCommand::CreateWindow));
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

        // Move cursor up (changed from scroll to cursor movement)
        let up_key = KeyEvent::new(KeyCode::Up, KeyModifiers::empty());
        let result = handler.handle_key(up_key);

        assert_eq!(
            result,
            InputAction::Command(ClientCommand::MoveCopyCursor {
                row_delta: -1,
                col_delta: 0
            })
        );
    }

    #[test]
    fn test_copy_mode_visual_mode() {
        let mut handler = InputHandler::new();
        handler.mode = InputMode::Copy;

        // Press 'v' to enter visual mode
        let v_key = KeyEvent::new(KeyCode::Char('v'), KeyModifiers::empty());
        let result = handler.handle_key(v_key);

        assert_eq!(result, InputAction::Command(ClientCommand::StartVisualMode));
    }

    #[test]
    fn test_copy_mode_visual_line_mode() {
        let mut handler = InputHandler::new();
        handler.mode = InputMode::Copy;

        // Press 'V' to enter visual line mode
        let v_key = KeyEvent::new(KeyCode::Char('V'), KeyModifiers::empty());
        let result = handler.handle_key(v_key);

        assert_eq!(result, InputAction::Command(ClientCommand::StartVisualLineMode));
    }

    #[test]
    fn test_copy_mode_yank() {
        let mut handler = InputHandler::new();
        handler.mode = InputMode::Copy;

        // Press 'y' to yank
        let y_key = KeyEvent::new(KeyCode::Char('y'), KeyModifiers::empty());
        let result = handler.handle_key(y_key);

        assert_eq!(result, InputAction::Command(ClientCommand::YankSelection));
        assert_eq!(handler.mode(), InputMode::Normal); // Should exit copy mode
    }

    #[test]
    fn test_copy_mode_horizontal_movement() {
        let mut handler = InputHandler::new();
        handler.mode = InputMode::Copy;

        // Move left with 'h'
        let h_key = KeyEvent::new(KeyCode::Char('h'), KeyModifiers::empty());
        let result = handler.handle_key(h_key);

        assert_eq!(
            result,
            InputAction::Command(ClientCommand::MoveCopyCursor {
                row_delta: 0,
                col_delta: -1
            })
        );

        // Move right with 'l'
        let l_key = KeyEvent::new(KeyCode::Char('l'), KeyModifiers::empty());
        let result = handler.handle_key(l_key);

        assert_eq!(
            result,
            InputAction::Command(ClientCommand::MoveCopyCursor {
                row_delta: 0,
                col_delta: 1
            })
        );
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

    // ==================== Quick Bindings Tests ====================

    #[test]
    fn test_quick_bindings_default() {
        let bindings = QuickBindings::default();
        assert!(bindings.next_window.is_some());
        assert!(bindings.prev_window.is_some());
        assert!(bindings.next_pane.is_some());
        assert!(bindings.prev_pane.is_some());
    }

    #[test]
    fn test_quick_bindings_none() {
        let bindings = QuickBindings::none();
        assert!(bindings.next_window.is_none());
        assert!(bindings.prev_window.is_none());
        assert!(bindings.next_pane.is_none());
        assert!(bindings.prev_pane.is_none());
    }

    #[test]
    fn test_quick_bindings_from_config() {
        let bindings = QuickBindings::from_config(
            "Ctrl-Tab",
            "Ctrl-Shift-Tab",
            "Alt-n",
            "Alt-p",
        );
        assert!(bindings.next_window.is_some());
        assert!(bindings.prev_window.is_some());
        assert!(bindings.next_pane.is_some());
        assert!(bindings.prev_pane.is_some());
    }

    #[test]
    fn test_quick_bindings_from_config_empty_disabled() {
        let bindings = QuickBindings::from_config("", "", "", "");
        assert!(bindings.next_window.is_none());
        assert!(bindings.prev_window.is_none());
        assert!(bindings.next_pane.is_none());
        assert!(bindings.prev_pane.is_none());
    }

    #[test]
    fn test_quick_bindings_from_config_invalid_disabled() {
        let bindings = QuickBindings::from_config(
            "Invalid-Binding",
            "Ctrl-PageUp",
            "Notakey",
            "Ctrl-Shift-PageUp",
        );
        // Invalid ones should be None
        assert!(bindings.next_window.is_none());
        assert!(bindings.next_pane.is_none());
        // Valid ones should work
        assert!(bindings.prev_window.is_some());
        assert!(bindings.prev_pane.is_some());
    }

    #[test]
    fn test_quick_binding_next_window() {
        let mut handler = InputHandler::new();

        // Default: Ctrl+PageDown triggers NextWindow
        let key = KeyEvent::new(KeyCode::PageDown, KeyModifiers::CONTROL);
        let result = handler.handle_key(key);
        assert_eq!(result, InputAction::Command(ClientCommand::NextWindow));
    }

    #[test]
    fn test_quick_binding_prev_window() {
        let mut handler = InputHandler::new();

        // Default: Ctrl+PageUp triggers PreviousWindow
        let key = KeyEvent::new(KeyCode::PageUp, KeyModifiers::CONTROL);
        let result = handler.handle_key(key);
        assert_eq!(result, InputAction::Command(ClientCommand::PreviousWindow));
    }

    #[test]
    fn test_quick_binding_next_pane() {
        let mut handler = InputHandler::new();

        // Default: Ctrl+Shift+PageDown triggers NextPane
        let key = KeyEvent::new(KeyCode::PageDown, KeyModifiers::CONTROL | KeyModifiers::SHIFT);
        let result = handler.handle_key(key);
        assert_eq!(result, InputAction::Command(ClientCommand::NextPane));
    }

    #[test]
    fn test_quick_binding_prev_pane() {
        let mut handler = InputHandler::new();

        // Default: Ctrl+Shift+PageUp triggers PreviousPane
        let key = KeyEvent::new(KeyCode::PageUp, KeyModifiers::CONTROL | KeyModifiers::SHIFT);
        let result = handler.handle_key(key);
        assert_eq!(result, InputAction::Command(ClientCommand::PreviousPane));
    }

    #[test]
    fn test_quick_bindings_disabled() {
        let mut handler = InputHandler::new();
        handler.set_quick_bindings(QuickBindings::none());

        // Ctrl+PageDown should now pass through to pane (not trigger quick binding)
        let key = KeyEvent::new(KeyCode::PageDown, KeyModifiers::CONTROL);
        let result = handler.handle_key(key);
        // Should be SendToPane with the key translated to bytes
        assert!(matches!(result, InputAction::SendToPane(_)));
    }

    #[test]
    fn test_quick_bindings_custom() {
        let mut handler = InputHandler::new();
        handler.set_quick_bindings(QuickBindings::from_config(
            "F7",      // next window
            "Shift-F7", // prev window
            "F8",      // next pane
            "Shift-F8", // prev pane
        ));

        // F7 should now trigger NextWindow
        let f7 = KeyEvent::new(KeyCode::F(7), KeyModifiers::empty());
        let result = handler.handle_key(f7);
        assert_eq!(result, InputAction::Command(ClientCommand::NextWindow));

        // Shift+F7 should trigger PreviousWindow
        let shift_f7 = KeyEvent::new(KeyCode::F(7), KeyModifiers::SHIFT);
        let result = handler.handle_key(shift_f7);
        assert_eq!(result, InputAction::Command(ClientCommand::PreviousWindow));
    }

    #[test]
    fn test_quick_bindings_dont_interfere_with_quit() {
        let mut handler = InputHandler::new();

        // Even with quick bindings, Ctrl+Q should still quit
        let quit_key = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::CONTROL);
        let result = handler.handle_key(quit_key);
        assert_eq!(result, InputAction::Quit);
    }

    #[test]
    fn test_quick_bindings_dont_interfere_with_prefix() {
        let mut handler = InputHandler::new();

        // Prefix key (Ctrl+B) should still work
        let prefix = KeyEvent::new(KeyCode::Char('b'), KeyModifiers::CONTROL);
        let result = handler.handle_key(prefix);
        assert_eq!(result, InputAction::None);
        assert_eq!(handler.mode(), InputMode::PrefixPending);
    }
}
