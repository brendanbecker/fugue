# Implementation Plan: FEAT-061

**Work Item**: [FEAT-061: Add screen redraw command to fix display corruption](PROMPT.md)
**Component**: ccmux-client
**Priority**: P2
**Created**: 2026-01-11

## Overview

Add a keybinding to force a complete screen redraw when the TUI becomes corrupted. This is a common feature in terminal multiplexers (tmux uses Ctrl+L) and provides a quick recovery mechanism for display issues.

## Architecture Decisions

### Keybinding Strategy

**Decision**: Implement `Ctrl+B, r` as the primary keybinding.

**Rationale**:
- Consistent with existing ccmux keybinding pattern (prefix + action)
- Avoids conflicts with shell `Ctrl+L` (clear screen)
- Users can still use `Ctrl+L` in their shell within panes
- tmux uses both `prefix + r` (source config) and `Ctrl+L` (redraw), so `prefix + r` for redraw is acceptable

**Alternative considered**: Also binding `Ctrl+L` at the client level, but this would break expected shell behavior.

### Redraw Implementation

**Approach**: Two-phase redraw

1. **Phase 1: Terminal Clear**
   - Use `terminal.clear()` to clear Ratatui's internal buffer
   - Use crossterm `Clear(ClearType::All)` to clear actual terminal

2. **Phase 2: Force Full Render**
   - Mark entire terminal as dirty
   - Call normal render function
   - All widgets re-render from current state

### SIGWINCH Handling

**Decision**: Send SIGWINCH to active pane only by default.

**Rationale**:
- Reduces unnecessary work for background panes
- Active pane is most likely to have corruption visible
- Can add option to refresh all panes if needed later

## Affected Components

| Component | Type of Change | Risk Level |
|-----------|----------------|------------|
| ccmux-client/src/input/keys.rs | Add key mapping | Low |
| ccmux-client/src/input/input_handler.rs | Add action handler | Low |
| ccmux-client/src/main.rs | Add redraw function | Low |
| ccmux-client/src/ui/mod.rs | May expose redraw | Low |

## Dependencies

None - uses existing Ratatui and crossterm capabilities.

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Redraw causes flicker | Medium | Low | Use terminal.clear() before render |
| State not fully reset | Low | Medium | Test with various corruption scenarios |
| SIGWINCH causes issues | Low | Medium | Make SIGWINCH optional/configurable |

## Implementation Phases

### Phase 1: Add Keybinding (15 min)

1. Add `Redraw` variant to `InputAction` enum
2. Map `Ctrl+B, r` sequence to `Redraw` action
3. Update key handling documentation

### Phase 2: Implement Redraw Function (30 min)

1. Create `force_redraw()` function in main.rs or ui module
2. Clear terminal buffer and screen
3. Force full render pass
4. Handle any error conditions

### Phase 3: SIGWINCH Integration (15 min)

1. Add function to send SIGWINCH to pane process
2. Call after redraw if pane has child process
3. Make this behavior configurable

### Phase 4: Testing (30 min)

1. Manual testing with various corruption scenarios
2. Add unit test for key binding
3. Document testing results

## Code Snippets

### InputAction Addition

```rust
// In input/keys.rs or similar
pub enum InputAction {
    // ... existing variants
    Redraw,
}
```

### Key Mapping

```rust
// In command mode handler (after prefix key)
match key {
    KeyCode::Char('r') => Some(InputAction::Redraw),
    // ... other mappings
}
```

### Redraw Function

```rust
fn force_redraw<B: Backend>(terminal: &mut Terminal<B>) -> Result<()> {
    // Clear everything
    terminal.clear()?;

    // The next draw() call will render everything fresh
    Ok(())
}
```

## Rollback Strategy

If implementation causes issues:
1. Remove keybinding mapping
2. Remove redraw function
3. Revert any UI module changes

Changes are isolated and easily reversible.

## Implementation Notes

<!-- Add notes during implementation -->

---
*This plan should be updated as implementation progresses.*
