# Task Breakdown: FEAT-035

**Work Item**: [FEAT-035: Configurable Tab/Pane Switching](PROMPT.md)
**Status**: Not Started
**Last Updated**: 2026-01-09

## Prerequisites

- [ ] Read and understand PROMPT.md
- [ ] Review PLAN.md and update if needed
- [ ] Verify FEAT-010 (Client Input) is complete
- [ ] Verify FEAT-017 (Configuration) is complete
- [ ] Identify current input handling code in fugue-client/src/input/

## Phase 1: Key Binding Parser

### 1.1 Create Error Types
- [ ] Create `KeyBindingError` enum in fugue-client/src/input/keys.rs
- [ ] Add variants: `Empty`, `UnknownModifier(String)`, `UnknownKey(String)`, `InvalidFKey`
- [ ] Implement `std::error::Error` and `Display`

### 1.2 Key Code Parser
- [ ] Add `parse_key_code(s: &str) -> Result<KeyCode, KeyBindingError>`
- [ ] Handle special keys: Tab, Enter, Space, Backspace, Delete, Home, End, etc.
- [ ] Handle arrow keys: Up, Down, Left, Right
- [ ] Handle function keys: F1-F12
- [ ] Handle single characters (case preserved)
- [ ] Handle PageUp, PageDown, Insert, Escape
- [ ] Return appropriate error for unknown keys

### 1.3 Key Binding Parser
- [ ] Add `parse_key_binding(s: &str) -> Result<Option<KeyBinding>, KeyBindingError>`
- [ ] Split on '-' delimiter
- [ ] Parse modifiers: Ctrl, Alt, Shift, Super (case insensitive)
- [ ] Handle modifier aliases: Control, Meta, Option, Cmd, Win
- [ ] Parse final part as key code
- [ ] Combine into `KeyBinding` struct
- [ ] Return `Ok(None)` for empty string (disabled binding)

### 1.4 KeyBinding Struct
- [ ] Create `KeyBinding` struct with `code: KeyCode`, `modifiers: KeyModifiers`
- [ ] Implement `matches(&self, event: &KeyEvent) -> bool`
- [ ] Implement `Display` for debug/logging
- [ ] Add `From<&str>` that panics (for tests/defaults)

### 1.5 Parser Unit Tests
- [ ] Test parsing "Tab" -> KeyCode::Tab, no modifiers
- [ ] Test parsing "Ctrl-PageDown" -> KeyCode::Tab, CONTROL modifier
- [ ] Test parsing "Ctrl-Shift-Tab" -> KeyCode::Tab, CONTROL|SHIFT
- [ ] Test parsing "Alt-a" -> KeyCode::Char('a'), ALT modifier
- [ ] Test parsing "F1" -> KeyCode::F(1)
- [ ] Test parsing "Shift-F12" -> KeyCode::F(12), SHIFT
- [ ] Test parsing "" -> Ok(None) (disabled)
- [ ] Test parsing "Foo-Tab" -> UnknownModifier error
- [ ] Test parsing "Ctrl-XYZ" -> UnknownKey error
- [ ] Test case insensitivity: "ctrl-tab" == "CTRL-TAB"

## Phase 2: Quick Bindings Structure

### 2.1 Create QuickBindings Struct
- [ ] Create `QuickBindings` struct in fugue-client/src/input/mod.rs
- [ ] Fields: `next_window`, `prev_window`, `next_pane`, `prev_pane`
- [ ] All fields are `Option<KeyBinding>` (None = disabled)

### 2.2 Default Implementation
- [ ] Implement `Default` for `QuickBindings`
- [ ] Default next_window: Ctrl-PageDown
- [ ] Default prev_window: Ctrl-Shift-Tab
- [ ] Default next_pane: Ctrl-Shift-PageDown
- [ ] Default prev_pane: Alt-Shift-Tab

### 2.3 Config Constructor
- [ ] Add `QuickBindings::from_config(config: &KeybindingConfig) -> Self`
- [ ] Parse each binding string from config
- [ ] Log warning on parse errors, fall back to default
- [ ] Handle missing fields with defaults

### 2.4 QuickBindings Unit Tests
- [ ] Test `Default::default()` produces expected bindings
- [ ] Test `from_config()` with all fields specified
- [ ] Test `from_config()` with partial config
- [ ] Test `from_config()` with invalid binding (falls back)
- [ ] Test `from_config()` with empty string (disabled)

## Phase 3: Input Handler Integration

### 3.1 Update InputHandler Struct
- [ ] Add `quick_bindings: QuickBindings` field
- [ ] Add `new_with_bindings(bindings: QuickBindings) -> Self` constructor
- [ ] Update `Default` to use default bindings

### 3.2 Add check_quick_bindings Method
- [ ] Add `check_quick_bindings(&self, key: &KeyEvent) -> Option<InputAction>`
- [ ] Check each binding in quick_bindings
- [ ] Return appropriate `InputAction::Command(ClientCommand::*)` on match
- [ ] Return `None` if no match

### 3.3 Integrate into Event Handling
- [ ] In `handle_normal_key()`, check quick bindings after quit check
- [ ] Check before prefix key detection
- [ ] Return early if quick binding matches
- [ ] Ensure prefix mode ignores quick bindings (prefix takes precedence)

### 3.4 InputHandler Unit Tests
- [ ] Test quick binding triggers NextWindow command
- [ ] Test quick binding triggers PreviousWindow command
- [ ] Test quick binding triggers NextPane command
- [ ] Test quick binding triggers PreviousPane command
- [ ] Test disabled binding (None) does not match
- [ ] Test non-matching key falls through to normal handling
- [ ] Test prefix mode ignores quick bindings

## Phase 4: Config Schema Updates

### 4.1 Update KeybindingConfig
- [ ] Add `next_window_quick: String` field
- [ ] Add `prev_window_quick: String` field
- [ ] Add `next_pane_quick: String` field
- [ ] Add `prev_pane_quick: String` field
- [ ] Add serde default annotations

### 4.2 Update Default Implementation
- [ ] Set next_window_quick default to "Ctrl-PageDown"
- [ ] Set prev_window_quick default to "Ctrl-Shift-Tab"
- [ ] Set next_pane_quick default to "Ctrl-Shift-PageDown"
- [ ] Set prev_pane_quick default to "Alt-Shift-Tab"

### 4.3 Config Parsing Tests
- [ ] Test TOML parsing with new fields
- [ ] Test parsing with missing fields (uses defaults)
- [ ] Test parsing with empty strings
- [ ] Test parsing with custom bindings

## Phase 5: Window Cycling

### 5.1 Add cycle_window Method
- [ ] Add `cycle_window(&mut self, offset: i32)` to App
- [ ] Get list of window IDs (sorted for consistency)
- [ ] Find current window from active pane
- [ ] Calculate new window index with wraparound
- [ ] Focus first pane in new window

### 5.2 Add Helper Methods
- [ ] Add `current_window_id(&self) -> Option<Uuid>`
- [ ] Add `first_pane_in_window(&self, window_id: Uuid) -> Option<Uuid>`
- [ ] Add `focus_pane(&mut self, pane_id: Uuid)` if not exists

### 5.3 Update Command Handler
- [ ] Handle `ClientCommand::NextWindow` -> cycle_window(1)
- [ ] Handle `ClientCommand::PreviousWindow` -> cycle_window(-1)
- [ ] Send `ClientMessage::SelectPane` for new active pane

### 5.4 Window Cycling Tests
- [ ] Test cycling forward through 3 windows
- [ ] Test cycling backward through 3 windows
- [ ] Test wraparound from last to first
- [ ] Test wraparound from first to last
- [ ] Test with single window (no change)
- [ ] Test empty windows (should not crash)

## Phase 6: Client Config Loading

### 6.1 Add Config Loading to Client
- [ ] Add config loading function in fugue-client
- [ ] Use same XDG paths as server
- [ ] Handle missing config file (use defaults)
- [ ] Handle parse errors gracefully (log warning, use defaults)

### 6.2 Wire Up Config to InputHandler
- [ ] Load config in main.rs before creating App
- [ ] Extract keybindings section
- [ ] Create QuickBindings from config
- [ ] Pass to InputHandler (add setter or constructor)

### 6.3 Config Loading Tests
- [ ] Test loading existing config
- [ ] Test missing config file
- [ ] Test malformed config file
- [ ] Test config with only keybindings section

## Phase 7: Testing and Polish

### 7.1 Integration Testing
- [ ] Test Ctrl-PageDown cycles windows (manual)
- [ ] Test Ctrl-Shift-PageDown cycles panes (manual)
- [ ] Test Shift variants for reverse (manual)
- [ ] Test with custom config file (manual)
- [ ] Test disabling a binding (manual)

### 7.2 Terminal Compatibility Testing
- [ ] Test in kitty
- [ ] Test in alacritty
- [ ] Test in wezterm
- [ ] Test in gnome-terminal
- [ ] Test in Windows Terminal (if WSL supported)
- [ ] Document any terminal-specific issues

### 7.3 Edge Case Testing
- [ ] Test rapid key presses
- [ ] Test while in prefix mode
- [ ] Test while in copy mode
- [ ] Test with session select UI
- [ ] Test with no active pane

## Phase 8: Documentation

### 8.1 Config File Documentation
- [ ] Document keybinding format in example config
- [ ] Document available modifiers
- [ ] Document available key names
- [ ] Document how to disable bindings

### 8.2 User Documentation
- [ ] Add keybinding section to README
- [ ] Document default keybindings
- [ ] Document terminal compatibility notes
- [ ] Document how to configure alternative bindings

### 8.3 Code Documentation
- [ ] Doc comments on KeyBinding struct
- [ ] Doc comments on parse functions
- [ ] Doc comments on QuickBindings
- [ ] Doc comments on cycle_window

## Completion Checklist

- [ ] All Phase 1 tasks complete (Key Binding Parser)
- [ ] All Phase 2 tasks complete (QuickBindings Structure)
- [ ] All Phase 3 tasks complete (Input Handler Integration)
- [ ] All Phase 4 tasks complete (Config Schema)
- [ ] All Phase 5 tasks complete (Window Cycling)
- [ ] All Phase 6 tasks complete (Client Config Loading)
- [ ] All Phase 7 tasks complete (Testing)
- [ ] All Phase 8 tasks complete (Documentation)
- [ ] All tests passing
- [ ] PLAN.md updated with final approach
- [ ] feature_request.json status updated
- [ ] Ready for review/merge

---
*Check off tasks as you complete them. Update status field above.*
