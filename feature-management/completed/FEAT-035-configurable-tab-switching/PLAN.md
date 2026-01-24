# Implementation Plan: FEAT-035

**Work Item**: [FEAT-035: Configurable Tab/Pane Switching](PROMPT.md)
**Component**: fugue-client
**Priority**: P2
**Created**: 2026-01-09

## Overview

Add keyboard navigation to switch between windows and panes using configurable keybindings, with Ctrl+Tab for window cycling and Alt+Tab for pane cycling as defaults.

## Architecture Decisions

### Decision 1: Alternative to Left/Right Control Keys

**Choice**: Use different modifier combinations (Ctrl+Tab vs Alt+Tab) instead of left/right Control.

**Rationale**:
- crossterm's `KeyModifiers` does not distinguish left from right Control when combined with other keys
- Terminal protocols (ANSI, CSI u) generally don't encode left/right information in key combinations
- The original request is technically infeasible on most terminals
- Alt+Tab vs Ctrl+Tab is a familiar pattern from browsers and IDEs

**Trade-offs**:
- Alt+Tab may conflict with OS window managers (configurable as mitigation)
- Users expecting left/right Control will need to adjust expectations

### Decision 2: Key Binding Parser Design

**Choice**: Simple string parser with modifier-key format (e.g., "Ctrl-Shift-Tab").

**Rationale**:
- Familiar format from other applications (vim, tmux)
- Easy to read and edit in config files
- Flexible enough to support all keyboard combinations
- No complex grammar needed

**Format Specification**:
```
binding := modifier* key
modifier := "Ctrl" | "Alt" | "Shift" | "Super" (case insensitive)
key := single_char | special_key | f_key
special_key := "Tab" | "Enter" | "Space" | "Backspace" | "Delete" | ...
f_key := "F" number (1-12)
```

**Alternatives Considered**:
- JSON object format - Too verbose for simple bindings
- Vim-style `<C-Tab>` - Less familiar to general users
- crossterm literal format - Not user-friendly

### Decision 3: Quick Bindings vs Prefix Extension

**Choice**: Implement as "quick bindings" separate from prefix system.

**Rationale**:
- Quick bindings work immediately (no prefix needed)
- Cleaner separation of concerns in code
- Allows both systems to coexist
- Users can choose their preferred style

**Implementation**:
- Quick bindings checked before prefix key in `handle_normal_key()`
- Dedicated `QuickBindings` struct holds parsed bindings
- Falls through to prefix system if no quick binding matches

**Alternatives Considered**:
- Add to prefix binding system - Would require typing prefix first
- Replace prefix system - Breaking change, removes flexibility
- Double-tap for quick mode - Complicated timing logic

### Decision 4: Config Distribution

**Choice**: Client loads keybindings from config at startup.

**Rationale**:
- Client handles all input, so client needs the bindings
- Server config already exists and can be reused
- Simple one-time load at startup is sufficient
- Hot-reload can be added later if needed

**Implementation Path**:
- Client reads `~/.config/fugue/config.toml` directly (or from XDG path)
- Parse `[keybindings]` section into `QuickBindings`
- Pass to `InputHandler` during initialization

**Alternatives Considered**:
- Server sends bindings on connect - Adds protocol complexity
- Shared config crate - Good but not necessary for MVP
- Environment variables - Not user-friendly

### Decision 5: Window Cycling Strategy

**Choice**: Cycle through windows in creation order, focus first pane in target window.

**Rationale**:
- Simple and predictable behavior
- Matches tmux default behavior
- First pane is typically the "main" pane
- Can be enhanced later (remember last active pane per window)

**Alternatives Considered**:
- MRU (most recently used) order - More complex, needs state tracking
- Remember last active pane - Enhancement for future
- Show window selector UI - Different feature (window picker)

## Affected Components

| Component | Type of Change | Risk Level |
|-----------|----------------|------------|
| fugue-client/src/input/keys.rs | Add key binding parser | Low |
| fugue-client/src/input/mod.rs | Add quick bindings support | Medium |
| fugue-client/src/ui/app.rs | Add window cycling | Low |
| fugue-server/src/config/schema.rs | Add keybinding fields | Low |
| fugue-client/src/main.rs | Load config, pass bindings | Low |

## Implementation Order

### Phase 1: Key Binding Parser (Foundation)
- Implement key binding string parser
- Add `KeyBinding` struct with `matches()` method
- Comprehensive unit tests for parser
- **Deliverable**: Can parse and match any key binding string

### Phase 2: Quick Bindings Structure
- Create `QuickBindings` struct
- Add `from_config()` constructor
- Default bindings implementation
- **Deliverable**: Quick bindings data structure ready for integration

### Phase 3: Input Handler Integration
- Add quick binding checking to input handler
- Ensure proper ordering with prefix system
- Test that quick bindings trigger commands
- **Deliverable**: Quick bindings work in input handler

### Phase 4: Config Schema Updates
- Add fields to `KeybindingConfig`
- Update Default implementation
- Test TOML parsing
- **Deliverable**: Config can specify custom bindings

### Phase 5: Window Cycling
- Implement `cycle_window()` in App
- Wire up `NextWindow`/`PreviousWindow` commands
- Ensure pane cycling works across windows
- **Deliverable**: Can navigate windows and panes

### Phase 6: Client Config Loading
- Client loads config on startup
- Initialize InputHandler with bindings
- Handle missing config gracefully
- **Deliverable**: Full end-to-end working feature

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Alt+Tab captured by OS | High | Medium | Document, provide alternative bindings |
| Terminal doesn't send Ctrl+Tab | Medium | Medium | Test popular terminals, document compat |
| Config parsing errors crash client | Low | High | Robust error handling, fallback to defaults |
| Quick bindings conflict with apps | Medium | Low | User can disable/change bindings |
| Performance impact on every key | Low | Low | Binding check is O(1) hashmap lookup |

## Rollback Strategy

If implementation causes issues:
1. Feature is additive - disable quick bindings by setting empty strings
2. Revert commits if blocking issues
3. Prefix-based bindings remain as fallback
4. No server-side changes to roll back

## Testing Strategy

### Unit Tests
- Key binding parser with valid inputs
- Key binding parser with invalid inputs (error handling)
- `KeyBinding::matches()` with various KeyEvents
- `QuickBindings::from_config()` with partial configs
- Empty string disables binding

### Integration Tests
- Quick binding triggers `NextWindow` command
- Window cycling wraps around
- Pane cycling stays within window (unless window cycles)
- Config-specified bindings override defaults
- Missing config uses defaults

### Manual Testing
- Test in kitty, alacritty, wezterm, gnome-terminal
- Test Ctrl+Tab in each terminal
- Test Alt+Tab (watch for OS conflicts)
- Test Shift variants
- Test with custom config

## Implementation Notes

### Key Binding Matching

The `KeyBinding` struct should handle modifier order and normalization:

```rust
pub struct KeyBinding {
    code: KeyCode,
    modifiers: KeyModifiers,
}

impl KeyBinding {
    pub fn matches(&self, event: &KeyEvent) -> bool {
        event.code == self.code && event.modifiers == self.modifiers
    }

    /// Parse from string, returns None if disabled (empty string)
    pub fn parse(s: &str) -> Result<Option<Self>, KeyBindingError> {
        let s = s.trim();
        if s.is_empty() {
            return Ok(None); // Disabled
        }
        // ... parsing logic
    }
}
```

### Quick Bindings Struct

```rust
pub struct QuickBindings {
    pub next_window: Option<KeyBinding>,
    pub prev_window: Option<KeyBinding>,
    pub next_pane: Option<KeyBinding>,
    pub prev_pane: Option<KeyBinding>,
}

impl Default for QuickBindings {
    fn default() -> Self {
        Self {
            next_window: KeyBinding::parse("Ctrl-PageDown").unwrap(),
            prev_window: KeyBinding::parse("Ctrl-PageUp").unwrap(),
            next_pane: KeyBinding::parse("Ctrl-Shift-PageDown").unwrap(),
            prev_pane: KeyBinding::parse("Ctrl-Shift-PageUp").unwrap(),
        }
    }
}
```

### Config Integration Points

1. **Server Config** (`fugue-server/src/config/schema.rs`):
   - Add fields to `KeybindingConfig`
   - These are the canonical config definitions

2. **Client Config Loading** (new or in main.rs):
   - Client needs to read same config file
   - Or: add simple config reader just for keybindings
   - Parse into `QuickBindings`

3. **InputHandler Initialization**:
   ```rust
   let quick_bindings = QuickBindings::from_config(&config.keybindings);
   let input_handler = InputHandler::new_with_bindings(quick_bindings);
   ```

### Window Cycling Edge Cases

1. **Single window**: Cycling does nothing (or could create new window)
2. **No windows**: Should not happen in attached state
3. **Window with no panes**: Skip window or show empty state
4. **Active pane in multiple windows**: Follow the pane's window_id

---
*This plan should be updated as implementation progresses.*
