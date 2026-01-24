# FEAT-035: Configurable Tab/Pane Switching

**Priority**: P2
**Component**: fugue-client
**Type**: enhancement
**Estimated Effort**: medium
**Business Value**: high
**Status**: new

## Overview

Add keyboard navigation to switch between windows and panes using configurable keybindings. The default keybindings will use Ctrl+Tab for window cycling and Alt+Tab for pane cycling within the current window, with Shift variants for reverse cycling. All keybindings should be configurable in `~/.config/fugue/config.toml`.

## Problem Statement

Currently, fugue has limited keyboard navigation for switching between windows and panes:
- Prefix+n/p for next/previous pane
- Prefix+h/j/k/l for directional pane navigation
- No dedicated window switching keybindings without prefix

Users familiar with other terminal multiplexers (tmux) or tabbed interfaces expect:
- Quick window switching without prefix (like browser tab navigation)
- Pane cycling within the current window
- Reverse cycling with Shift modifier
- Configurable keybindings for custom workflows

## Technical Investigation: Left vs Right Control Keys

**Original Request**: Use Left Ctrl + Tab for windows, Right Ctrl + Tab for panes.

**Investigation Result**: This approach is **NOT feasible** with crossterm on most terminals.

### crossterm Modifier Key Handling

crossterm provides `ModifierKeyCode::LeftControl` and `ModifierKeyCode::RightControl`, but these are only available when a modifier key is pressed **alone** (generating `KeyCode::Modifier(...)` events). When a modifier is combined with another key (like Ctrl+Tab), the resulting `KeyEvent` only contains:

```rust
KeyEvent {
    code: KeyCode::Tab,
    modifiers: KeyModifiers::CONTROL,  // No left/right distinction
    kind: KeyEventKind::Press,
    state: KeyEventState::NONE,
}
```

The `KeyModifiers` flags (CONTROL, SHIFT, ALT, etc.) do not distinguish left from right variants.

### Terminal Protocol Limitations

The limitation is in the terminal protocol itself:
- Traditional ANSI escape sequences don't encode left/right modifier information
- CSI u (kitty keyboard protocol) can encode this but requires explicit terminal support
- Most terminals (including popular ones) don't send left/right control information in key combinations

### Recommended Alternative

Use different modifier combinations instead of left/right. Note: Alt+Tab and Ctrl+Tab are often captured by the OS (Windows grabs Alt+Tab, some terminals grab Ctrl+Tab), so we use PageUp/PageDown which are reliably passed through:

- **Ctrl+PageDown**: Cycle windows forward
- **Ctrl+PageUp**: Cycle windows backward
- **Ctrl+Shift+PageDown**: Cycle panes forward (within current window)
- **Ctrl+Shift+PageUp**: Cycle panes backward

This is familiar to users from:
- Web browsers (Ctrl+PageUp/PageDown for tabs in Firefox/Chrome)
- Many tabbed terminal emulators
- IDE tab navigation

## Solution

### Default Keybindings

| Action | Default Keybinding | Configurable Key |
|--------|-------------------|------------------|
| Next Window | Ctrl+PageDown | `keybindings.next_window_quick` |
| Previous Window | Ctrl+PageUp | `keybindings.prev_window_quick` |
| Next Pane | Ctrl+Shift+PageDown | `keybindings.next_pane_quick` |
| Previous Pane | Ctrl+Shift+PageUp | `keybindings.prev_pane_quick` |

These are "quick" bindings (no prefix required). Existing prefix-based bindings remain:
- Prefix+n: Next pane
- Prefix+p: Previous pane
- Prefix+0-9: Select window by index (future)

### Configuration Format

Add to `~/.config/fugue/config.toml`:

```toml
[keybindings]
# Quick navigation (no prefix required)
next_window_quick = "Ctrl-PageDown"
prev_window_quick = "Ctrl-PageUp"
next_pane_quick = "Ctrl-Shift-PageDown"
prev_pane_quick = "Ctrl-Shift-PageUp"

# Alternative: Use F-keys if PageUp/PageDown don't work in your terminal
# next_window_quick = "F7"
# prev_window_quick = "Shift-F7"
# next_pane_quick = "F8"
# prev_pane_quick = "Shift-F8"

# Disable a binding by setting to empty string
# next_window_quick = ""
```

### Key String Format

Support parsing key strings in format:
- Single keys: `Tab`, `F1`, `Enter`, `Space`
- With modifiers: `Ctrl-Tab`, `Alt-a`, `Shift-F1`
- Multiple modifiers: `Ctrl-Shift-Tab`, `Ctrl-Alt-Delete`
- Case insensitive: `ctrl-tab`, `CTRL-TAB`, `Ctrl-Tab` all work

## Implementation Details

### Key String Parser

Add a function to parse key strings into crossterm `KeyEvent`:

```rust
/// Parse a key binding string like "Ctrl-Tab" or "Alt-Shift-a"
fn parse_key_binding(s: &str) -> Result<KeyEvent, KeyBindingError> {
    let parts: Vec<&str> = s.split('-').collect();
    let mut modifiers = KeyModifiers::empty();
    let key_part = parts.last().ok_or(KeyBindingError::Empty)?;

    for part in &parts[..parts.len()-1] {
        match part.to_lowercase().as_str() {
            "ctrl" | "control" => modifiers |= KeyModifiers::CONTROL,
            "alt" | "meta" | "option" => modifiers |= KeyModifiers::ALT,
            "shift" => modifiers |= KeyModifiers::SHIFT,
            "super" | "cmd" | "win" => modifiers |= KeyModifiers::SUPER,
            _ => return Err(KeyBindingError::UnknownModifier(part.to_string())),
        }
    }

    let code = parse_key_code(key_part)?;
    Ok(KeyEvent::new(code, modifiers))
}

fn parse_key_code(s: &str) -> Result<KeyCode, KeyBindingError> {
    match s.to_lowercase().as_str() {
        "tab" => Ok(KeyCode::Tab),
        "enter" | "return" => Ok(KeyCode::Enter),
        "space" => Ok(KeyCode::Char(' ')),
        "backspace" | "bs" => Ok(KeyCode::Backspace),
        "delete" | "del" => Ok(KeyCode::Delete),
        "insert" | "ins" => Ok(KeyCode::Insert),
        "home" => Ok(KeyCode::Home),
        "end" => Ok(KeyCode::End),
        "pageup" | "pgup" => Ok(KeyCode::PageUp),
        "pagedown" | "pgdn" => Ok(KeyCode::PageDown),
        "up" => Ok(KeyCode::Up),
        "down" => Ok(KeyCode::Down),
        "left" => Ok(KeyCode::Left),
        "right" => Ok(KeyCode::Right),
        "esc" | "escape" => Ok(KeyCode::Esc),
        s if s.starts_with('f') => {
            let num: u8 = s[1..].parse()?;
            Ok(KeyCode::F(num))
        }
        s if s.len() == 1 => Ok(KeyCode::Char(s.chars().next().unwrap())),
        _ => Err(KeyBindingError::UnknownKey(s.to_string())),
    }
}
```

### Quick Bindings Handler

Modify `InputHandler` to check for quick bindings before prefix handling:

```rust
impl InputHandler {
    fn handle_normal_key(&mut self, key: KeyEvent) -> InputAction {
        // 1. Check for quit binding (Ctrl+Q)
        if self.is_quit_key(&key) {
            return InputAction::Quit;
        }

        // 2. Check for quick navigation bindings (NEW)
        if let Some(action) = self.check_quick_bindings(&key) {
            return action;
        }

        // 3. Check for prefix key
        if self.is_prefix_key(&key) {
            self.mode = InputMode::PrefixPending;
            self.prefix_time = Some(Instant::now());
            return InputAction::None;
        }

        // 4. Translate key to bytes and send to pane
        // ...existing code...
    }

    fn check_quick_bindings(&self, key: &KeyEvent) -> Option<InputAction> {
        if self.quick_bindings.next_window.matches(key) {
            return Some(InputAction::Command(ClientCommand::NextWindow));
        }
        if self.quick_bindings.prev_window.matches(key) {
            return Some(InputAction::Command(ClientCommand::PreviousWindow));
        }
        if self.quick_bindings.next_pane.matches(key) {
            return Some(InputAction::Command(ClientCommand::NextPane));
        }
        if self.quick_bindings.prev_pane.matches(key) {
            return Some(InputAction::Command(ClientCommand::PreviousPane));
        }
        None
    }
}
```

### Config Schema Updates

Add to `KeybindingConfig` in `fugue-server/src/config/schema.rs`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct KeybindingConfig {
    // Existing prefix-based bindings
    pub split_horizontal: String,
    pub split_vertical: String,
    pub focus_left: String,
    // ... existing fields ...

    // NEW: Quick navigation (no prefix)
    pub next_window_quick: String,
    pub prev_window_quick: String,
    pub next_pane_quick: String,
    pub prev_pane_quick: String,
}

impl Default for KeybindingConfig {
    fn default() -> Self {
        Self {
            // ... existing defaults ...

            // Quick navigation defaults
            next_window_quick: "Ctrl-PageDown".into(),
            prev_window_quick: "Ctrl-PageUp".into(),
            next_pane_quick: "Ctrl-Shift-PageDown".into(),
            prev_pane_quick: "Ctrl-Shift-PageUp".into(),
        }
    }
}
```

### Window Cycling Implementation

Add window cycling logic to `App`:

```rust
impl App {
    /// Cycle through windows by offset (positive = forward, negative = backward)
    fn cycle_window(&mut self, offset: i32) {
        if self.windows.is_empty() {
            return;
        }

        // Get current window from active pane
        let current_window_id = self.active_pane_id
            .and_then(|pid| self.panes.get(&pid))
            .map(|p| p.window_id);

        let window_ids: Vec<Uuid> = self.windows.keys().copied().collect();
        let current_index = current_window_id
            .and_then(|id| window_ids.iter().position(|&w| w == id))
            .unwrap_or(0);

        let new_index = if offset > 0 {
            (current_index + offset as usize) % window_ids.len()
        } else {
            let abs_offset = (-offset) as usize;
            (current_index + window_ids.len() - (abs_offset % window_ids.len())) % window_ids.len()
        };

        let new_window_id = window_ids[new_index];
        // Focus first pane in new window
        if let Some(pane_id) = self.first_pane_in_window(new_window_id) {
            self.focus_pane(pane_id);
        }
    }

    fn first_pane_in_window(&self, window_id: Uuid) -> Option<Uuid> {
        self.panes.values()
            .filter(|p| p.window_id == window_id)
            .min_by_key(|p| p.index)
            .map(|p| p.id)
    }
}
```

## Files to Modify

### fugue-client/src/input/mod.rs
- Add `QuickBindings` struct to hold parsed bindings
- Add `check_quick_bindings()` method to `InputHandler`
- Modify `handle_normal_key()` to check quick bindings first
- Add `set_quick_bindings()` method for configuration

### fugue-client/src/input/keys.rs (NEW or modify)
- Add `parse_key_binding()` function
- Add `parse_key_code()` function
- Add `KeyBindingError` type
- Add `KeyBinding` wrapper type with `matches()` method

### fugue-server/src/config/schema.rs
- Add `next_window_quick`, `prev_window_quick`, `next_pane_quick`, `prev_pane_quick` to `KeybindingConfig`
- Update `Default` impl with new defaults

### fugue-client/src/ui/app.rs
- Add `cycle_window()` method
- Update `handle_client_command()` to handle `NextWindow`/`PreviousWindow`
- Wire up quick bindings from config (needs config loading in client)

### fugue-client/src/main.rs
- Load config and pass keybindings to `InputHandler`

## Implementation Tasks

### Section 1: Key Binding Parser
- [ ] Create `KeyBindingError` enum for parser errors
- [ ] Implement `parse_key_code()` for individual keys
- [ ] Implement `parse_key_binding()` for full binding strings
- [ ] Create `KeyBinding` struct with `matches(&KeyEvent)` method
- [ ] Add unit tests for parser

### Section 2: Quick Bindings Structure
- [ ] Create `QuickBindings` struct with fields for each quick action
- [ ] Add `from_config()` constructor that parses config strings
- [ ] Add `Default` impl with hardcoded fallbacks
- [ ] Update `InputHandler` to hold `QuickBindings`
- [ ] Add `set_quick_bindings()` method to `InputHandler`

### Section 3: Input Handler Integration
- [ ] Add `check_quick_bindings()` method
- [ ] Modify `handle_normal_key()` to check quick bindings early
- [ ] Ensure quick bindings don't conflict with prefix key
- [ ] Add tests for quick binding triggering

### Section 4: Config Schema
- [ ] Add new fields to `KeybindingConfig`
- [ ] Update `Default` implementation
- [ ] Test TOML parsing with new fields
- [ ] Handle empty string = disabled binding

### Section 5: Window Cycling
- [ ] Add `cycle_window()` method to `App`
- [ ] Add `first_pane_in_window()` helper
- [ ] Update `handle_client_command()` for `NextWindow`/`PreviousWindow`
- [ ] Send server message when window changes (if needed)

### Section 6: Client Config Loading
- [ ] Add config loading to client (or receive from server)
- [ ] Parse keybindings from config
- [ ] Initialize `InputHandler` with configured bindings
- [ ] Handle config reload (if hot-reload is implemented)

### Section 7: Testing
- [ ] Unit tests for key binding parser
- [ ] Unit tests for `QuickBindings::from_config()`
- [ ] Integration tests for quick binding actions
- [ ] Manual testing with different terminals

### Section 8: Documentation
- [ ] Document keybinding format in config file
- [ ] Add example configurations
- [ ] Document terminal compatibility notes
- [ ] Update README with keybinding info

## Acceptance Criteria

- [ ] Ctrl+PageDown cycles to next window (default)
- [ ] Ctrl+PageUp cycles to previous window (default)
- [ ] Ctrl+Shift+PageDown cycles to next pane in current window (default)
- [ ] Ctrl+Shift+PageUp cycles to previous pane (default)
- [ ] All quick bindings are configurable via config.toml
- [ ] Empty string disables a binding
- [ ] Parser handles various key string formats
- [ ] Clear error messages for invalid key binding strings
- [ ] Existing prefix-based bindings continue to work
- [ ] Quick bindings don't interfere with normal terminal input
- [ ] Works with default config (no config file needed)

## Dependencies

- **FEAT-010** (Client Input - Keyboard and Mouse Event Handling) - Base input handling
- **FEAT-017** (Configuration - TOML Config with Hot Reload) - Config infrastructure

## Terminal Compatibility Notes

The default bindings use Ctrl+PageUp/PageDown which are well-supported:
- **Ctrl+PageUp/PageDown**: Works in virtually all terminals
- **Ctrl+Shift+PageUp/PageDown**: Well supported in modern terminals
- **Ctrl+Tab/Alt+Tab**: Often captured by OS (Windows grabs Alt+Tab, some terminals grab Ctrl+Tab)

**Fallback Options**:
- Users can configure F-key bindings if needed
- Users can rely on prefix-based bindings as backup
- Document which terminals are tested and compatible

## Notes

- The original request for left/right Control distinction is not feasible due to terminal protocol limitations
- Consider adding visual feedback when switching windows (brief highlight or status message)
- May want to add window index display in status bar for reference
- Could add `Prefix+0-9` for direct window selection in the future
