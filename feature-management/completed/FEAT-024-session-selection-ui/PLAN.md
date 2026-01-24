# Implementation Plan: FEAT-024

**Work Item**: [FEAT-024: Session Selection UI](PROMPT.md)
**Component**: fugue-client
**Priority**: P1
**Created**: 2026-01-09

## Overview

Implement the session selection UI that appears when the client connects, allowing users to select from available sessions or create new ones. A basic implementation already exists and may only need enhancement.

## Architecture Decisions

### Current Implementation Assessment

The session selection UI already has a functional implementation:

```rust
// app.rs line ~478
async fn handle_session_select_input(&mut self, key: KeyEvent) -> Result<()> {
    match key.code {
        KeyCode::Up | KeyCode::Char('k') => { /* move up */ }
        KeyCode::Down | KeyCode::Char('j') => { /* move down */ }
        KeyCode::Enter => { /* attach to session */ }
        KeyCode::Char('n') => { /* create new session */ }
        KeyCode::Char('r') => { /* refresh list */ }
        _ => {}
    }
}

// app.rs line ~667
fn draw_session_select(&self, frame: &mut Frame, area: Rect) {
    // Renders session list with selection highlight
}
```

### Widget Choice

**Current**: Uses `Paragraph` widget with manually formatted lines
**Consideration**: Could use `List` widget for better semantics

Recommendation: Keep current Paragraph approach - it's working and List widget doesn't provide significant advantages for this use case.

### Session Metadata Display

Currently displays:
- Session name
- Window count
- Attached client count

Could add:
- Pane count (requires protocol extension or client-side tracking)
- Creation timestamp (requires protocol extension)

Recommendation: Keep current metadata for now; enhancements can be separate features.

## Affected Components

| Component | Type of Change | Risk Level |
|-----------|----------------|------------|
| fugue-client/src/ui/app.rs | Enhancement | Low |

## Dependencies

- **FEAT-021**: Server Socket - provides session list

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Regression in existing functionality | Low | Medium | Keep changes minimal |
| UI rendering issues | Low | Low | Test on multiple terminals |

## Implementation Approach

Given that the functionality is already substantially implemented, the approach is:

1. **Review** current implementation against requirements
2. **Identify** any gaps or improvements needed
3. **Implement** missing functionality if any
4. **Test** all session selection flows
5. **Document** and close feature

### Gap Analysis Checklist

| Requirement | Status | Notes |
|-------------|--------|-------|
| Render session list | Done | Uses Paragraph widget |
| Highlight selection | Done | Uses cyan color and "> " prefix |
| Up/down navigation | Done | Arrow keys and j/k |
| Enter to attach | Done | Sends AttachSession |
| Create new session | Done | 'n' key |
| Session metadata | Done | Name, windows, clients |

## Rollback Strategy

If changes cause issues:
1. Revert changes to app.rs
2. Original functionality will be restored
3. Document issues in comments.md

## Testing Strategy

1. **Manual Testing**: Navigate session list, attach, create new
2. **Edge Cases**: Empty list, single session, many sessions
3. **Keyboard**: Verify all key bindings work

## Implementation Notes

The feature appears to be substantially complete. This work item may primarily involve:
- Verification testing
- Minor UI polish if desired
- Documentation updates

---
*This plan should be updated as implementation progresses.*
