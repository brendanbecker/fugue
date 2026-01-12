# FEAT-061: Add screen redraw command to fix display corruption

**Priority**: P2
**Component**: ccmux-client
**Type**: enhancement
**Estimated Effort**: small
**Business Value**: medium

## Overview

The ccmux TUI can become corrupted with overlapping/garbled content. This corruption manifests as multiple overlapping status lines, repeated tip messages scattered across the screen, and garbled pane contents. A dedicated keybinding to force a complete screen redraw would provide users a quick recovery mechanism.

## Problem Statement

Display corruption occurs in several scenarios:

1. **Rapid status line updates**: Claude status lines showing multiple "Crafting..." lines overlaid
2. **Terminal resize events**: Resize can leave artifacts or misaligned borders
3. **Session/window switching**: Switching between sessions or windows may not fully clear previous content
4. **Escape sequence handling**: Pane output containing escape sequences that are not fully processed can corrupt display state

**Current workaround**: Resize the terminal window, which triggers a redraw. This is clunky and not always convenient.

## Requested Feature

Add a keybinding (suggested: `Ctrl+B, r` or `Ctrl+L`) to force a complete screen redraw.

### Keybinding Options

| Option | Pros | Cons |
|--------|------|------|
| `Ctrl+B, r` | Consistent with tmux prefix pattern | Requires two keypresses |
| `Ctrl+L` | Single keypress, tmux-compatible | May conflict with shell clear |

**Recommendation**: Implement both:
- `Ctrl+B, r` as the primary documented keybind
- `Ctrl+L` as a direct shortcut (when not in a pane that would consume it)

### Redraw Behavior

The redraw operation should:

1. **Clear the entire terminal**: Call `terminal.clear()` in Ratatui
2. **Re-render all UI elements**:
   - Pane borders and dividers
   - Status bar(s)
   - Tab bar / session list
   - Active pane content from terminal buffer
3. **Reset terminal state**: Clear any corrupted style/attribute state
4. **Optionally notify panes**: Send SIGWINCH to child PTYs to trigger their own redraw

## Implementation Notes

### Ratatui Implementation

```rust
// In the main render loop or as a command handler
fn force_redraw(terminal: &mut Terminal<impl Backend>) -> Result<()> {
    // Clear the terminal completely
    terminal.clear()?;

    // Force a full redraw on next frame
    // (Ratatui tracks dirty regions; this ensures everything redraws)
    terminal.draw(|f| {
        // Normal render function - renders everything
        render_ui(f, &app_state);
    })?;

    Ok(())
}
```

### SIGWINCH to Child PTYs

To trigger child process redraw (useful for ncurses apps like vim, htop):

```rust
use nix::sys::signal::{kill, Signal};
use nix::unistd::Pid;

fn notify_panes_redraw(panes: &[Pane]) {
    for pane in panes {
        if let Some(pid) = pane.child_pid() {
            let _ = kill(Pid::from_raw(pid), Signal::SIGWINCH);
        }
    }
}
```

### Crossterm Backend

If using crossterm backend directly:

```rust
use crossterm::{execute, terminal::{Clear, ClearType}};

// Force clear
execute!(stdout(), Clear(ClearType::All))?;
execute!(stdout(), Clear(ClearType::Purge))?; // Clear scrollback too
```

## Location

Primary files to modify:

| File | Changes |
|------|---------|
| `ccmux-client/src/input/keys.rs` | Add `Redraw` action to key mapping |
| `ccmux-client/src/input/input_handler.rs` | Handle `Redraw` action |
| `ccmux-client/src/main.rs` | Implement `force_redraw()` function |
| `ccmux-client/src/ui/mod.rs` | May need to expose redraw capability |

## Dependencies

None - this is a standalone enhancement that uses existing Ratatui/crossterm capabilities.

## Acceptance Criteria

- [ ] `Ctrl+B, r` triggers full screen redraw
- [ ] Optional: `Ctrl+L` triggers redraw when appropriate
- [ ] After redraw, all UI elements are correctly positioned
- [ ] Pane contents are re-rendered from buffer (not lost)
- [ ] No visual artifacts remain after redraw
- [ ] Child processes receive SIGWINCH (configurable)
- [ ] Keybinding is documented in help/config

## Testing Approach

### Manual Testing

1. Cause display corruption (rapid resize, rapid output)
2. Press redraw keybinding
3. Verify display is restored correctly

### Automated Testing

- Unit test for key binding recognition
- Integration test for redraw command execution
- Verify terminal.clear() is called

## Notes

- tmux uses `Ctrl+L` for this functionality
- Some users may want to bind this to a custom key - consider making configurable
- May want to add a command mode command: `:redraw` or `:refresh`
