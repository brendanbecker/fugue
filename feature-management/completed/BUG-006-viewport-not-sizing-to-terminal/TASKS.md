# Task Breakdown: BUG-006 - Viewport not sizing to terminal dimensions

**Status**: Ready for Implementation
**Priority**: P2
**Estimated Effort**: Small (1-2 hours)

## Prerequisites

- [ ] Read and understand PROMPT.md root cause analysis
- [ ] Review PLAN.md implementation approach
- [ ] Understand the `handle_server_message()` flow in `app.rs`

## Implementation Tasks

### Phase 1: Fix UI Pane Sizing

- [ ] Locate `ServerMessage::Attached` handler in `fugue-client/src/ui/app.rs`
- [ ] Add terminal size calculation before pane creation loop
  ```rust
  let (term_cols, term_rows) = self.terminal_size;
  let pane_rows = term_rows.saturating_sub(3);
  let pane_cols = term_cols.saturating_sub(2);
  ```
- [ ] Modify `add_pane()` calls to use calculated dimensions instead of `pane_info.rows/cols`

### Phase 2: Send Resize to Server

- [ ] After pane creation loop, add resize message sending
- [ ] Send resize for each pane in the session
- [ ] Ensure async/await is properly handled

### Phase 3: Testing

- [ ] Manual test: Large terminal -> create session -> verify correct size
- [ ] Manual test: Small terminal -> attach to existing session -> verify resize
- [ ] Manual test: Verify `stty size` shows correct dimensions in PTY
- [ ] Manual test: Multiple panes scenario
- [ ] Run existing test suite to check for regressions

## Verification Checklist

- [ ] Viewport fills terminal on attach
- [ ] `stty size` reports correct dimensions
- [ ] Dynamic resize still works after attach
- [ ] Detach and reattach works correctly
- [ ] No regression in session creation flow
- [ ] No regression in pane creation flow

## Code Review Points

- [ ] No unwraps or panics introduced
- [ ] Error handling for resize send failures
- [ ] Comments explaining the terminal size calculation
- [ ] Edge case handling (tiny/zero terminal size)

## Definition of Done

- [ ] Bug no longer reproduces
- [ ] All manual tests pass
- [ ] Existing test suite passes
- [ ] Code is reviewed and merged
