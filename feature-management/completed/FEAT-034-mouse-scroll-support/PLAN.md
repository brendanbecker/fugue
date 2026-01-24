# Implementation Plan: FEAT-034

**Work Item**: [FEAT-034: Mouse Scroll Support](PROMPT.md)
**Component**: fugue-client
**Priority**: P2
**Created**: 2026-01-09

## Overview

Enable mouse scroll (scrollwheel/trackpad) to navigate the terminal scrollback buffer in fugue panes.

## Architecture Decisions

### Approach: Client-Side Scroll with Server Sync

**Decision**: Update local pane scroll state immediately for responsive UX, then async notify server for state persistence.

**Rationale**:
- Immediate visual feedback is critical for scroll UX
- Server notification can happen async without blocking rendering
- Matches tmux behavior where scroll is client-side

**Trade-offs**:
- Pro: Responsive scrolling even with slow server connection
- Pro: Works immediately without round-trip
- Con: Slight complexity in maintaining two scroll states
- Con: Multiple clients viewing same pane may have different scroll positions (acceptable)

### Approach: Use Pane::scroll_offset as Source of Truth

**Decision**: Use `Pane::scroll_offset` in `fugue-client/src/ui/pane.rs` rather than `InputHandler::scroll_offset`.

**Rationale**:
- `Pane` owns the terminal rendering and scrollback
- Avoids dual state management
- `InputHandler::scroll_offset` was designed for copy mode keyboard navigation

**Trade-offs**:
- Pro: Single source of truth
- Pro: Pane already has scroll_up/scroll_down methods
- Con: May need to refactor copy mode to use Pane scroll

### Approach: tui-term PseudoTerminal for Rendering

**Decision**: Continue using `tui-term::PseudoTerminal` widget with vt100 Parser scrollback.

**Rationale**:
- Already integrated and working for live terminal output
- vt100 Parser manages scrollback buffer (1000 lines)
- Need to investigate if PseudoTerminal supports scroll offset rendering

**Alternative if tui-term doesn't support offset**:
- Manually extract rows from scrollback
- Render using raw Paragraph/Text widgets
- More work but full control over rendering

## Affected Components

| Component | Type of Change | Risk Level |
|-----------|----------------|------------|
| `fugue-client/src/ui/app.rs` | Modify `handle_input_action()` | Low |
| `fugue-client/src/ui/pane.rs` | Verify/update render_pane scroll support | Medium |
| `fugue-client/src/input/mod.rs` | Possibly remove duplicate scroll_offset | Low |

## Implementation Sequence

### Phase 1: Investigation (30 min)

1. Review tui-term source for scroll offset support
2. Test current vt100 Parser scrollback behavior
3. Determine rendering approach

### Phase 2: Core Fix (1 hour)

1. Update `handle_input_action()` for ScrollUp/ScrollDown
2. Call `pane.scroll_up()` / `pane.scroll_down()`
3. Test basic scroll functionality

### Phase 3: Rendering Integration (1-2 hours)

1. If tui-term supports offset: Configure PseudoTerminal
2. If not: Implement manual scrollback rendering
3. Verify scroll indicator displays correctly

### Phase 4: Polish (30 min)

1. Test edge cases (empty buffer, boundaries)
2. Verify scroll reset on new output
3. Test with actual terminal content

## Key Code Changes

### app.rs - handle_input_action()

```rust
// Before (current):
InputAction::ScrollUp { lines } => {
    if let Some(pane_id) = self.active_pane_id {
        let new_offset = self.input_handler.scroll_offset();
        self.connection
            .send(ClientMessage::SetViewportOffset { pane_id, offset: new_offset })
            .await?;
        let _ = lines;
    }
}

// After (proposed):
InputAction::ScrollUp { lines } => {
    if let Some(pane_id) = self.active_pane_id {
        // Update local UI pane immediately
        if let Some(pane) = self.pane_manager.get_mut(pane_id) {
            pane.scroll_up(lines);
        }
        // Async notify server for persistence
        if let Some(pane) = self.pane_manager.get(pane_id) {
            self.connection
                .send(ClientMessage::SetViewportOffset {
                    pane_id,
                    offset: pane.scroll_offset,
                })
                .await?;
        }
    }
}
```

### pane.rs - render_pane()

Investigation needed to determine if changes required. Current code:

```rust
let pseudo_term = PseudoTerminal::new(pane.screen())
    .style(Style::default().fg(Color::White).bg(Color::Black));

pseudo_term.render(inner, buf);
```

May need to add scroll offset parameter if tui-term supports it.

## Dependencies

| Dependency | Status | Notes |
|------------|--------|-------|
| FEAT-010 (Mouse Event Handling) | Complete (code exists) | Mouse events already captured |
| tui-term crate | External | Need to verify scroll offset API |
| vt100 Parser | External | Scrollback buffer working |

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| tui-term doesn't support scroll offset | Medium | High | Implement manual rendering fallback |
| Scroll state desync between client/server | Low | Low | Client is source of truth for its own view |
| Performance issues with large scrollback | Low | Medium | Already have 1000 line limit |
| Copy mode conflicts with scroll | Low | Medium | May need to unify scroll state |

## Rollback Strategy

If implementation causes issues:
1. Revert changes to app.rs handle_input_action()
2. Scroll will return to current behavior (server-only, no visual update)
3. No data loss risk - scroll state is transient

## Testing Strategy

### Manual Tests

1. **Basic scroll**: Generate output, scroll up, see history
2. **Boundary test**: Scroll past top (should stop), scroll past bottom (should stop)
3. **New output**: Scroll up, wait for output, verify scroll position maintained
4. **Scroll indicator**: Verify title bar shows scroll offset
5. **Return to live**: Scroll up, then scroll back down to bottom

### Automated Tests (if time permits)

1. Unit test for `pane.scroll_up()` / `pane.scroll_down()`
2. Test scroll boundaries with various scrollback sizes
3. Test scroll reset on `process_output()`

## Success Metrics

- Mouse scroll up shows older content
- Mouse scroll down shows newer content
- Scroll indicator visible when scrolled
- No perceptible lag during scrolling
- Scroll reset to bottom on new output (configurable in future)

## Implementation Notes

### tui-term Investigation Results

**TODO**: Document findings about tui-term scroll support after investigation.

Potential approaches:
1. `PseudoTerminal::scroll_offset()` method (if exists)
2. `Screen::rows_with_scrollback()` method (if exists)
3. Manual row extraction from Parser

### vt100 Parser Scrollback

The Parser is initialized with scrollback:
```rust
parser: Parser::new(rows, cols, 1000), // 1000 lines of scrollback
```

Access scrollback size:
```rust
let max_scroll = self.parser.screen().scrollback();
```

Need to find method to access actual scrollback content for rendering.

---
*This plan should be updated as implementation progresses.*
