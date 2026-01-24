# Task Breakdown: FEAT-034

**Work Item**: [FEAT-034: Mouse Scroll Support](PROMPT.md)
**Status**: Not Started
**Last Updated**: 2026-01-09

## Prerequisites

- [ ] Read and understand PROMPT.md
- [ ] Review PLAN.md and update if needed
- [ ] Understand current mouse event flow through the codebase

## Investigation Tasks

- [ ] Review tui-term crate documentation for scroll offset rendering
- [ ] Check tui-term source code for `PseudoTerminal` scroll support
- [ ] Test vt100 Parser scrollback access methods
- [ ] Determine if manual scrollback rendering is needed
- [ ] Document findings in PLAN.md

## Implementation Tasks

### Phase 1: Core Scroll Action Handler

- [ ] Open `fugue-client/src/ui/app.rs`
- [ ] Locate `handle_input_action()` method
- [ ] Modify `InputAction::ScrollUp { lines }` handler:
  - [ ] Get mutable reference to active pane from pane_manager
  - [ ] Call `pane.scroll_up(lines)`
  - [ ] Update server notification to use pane's scroll_offset
- [ ] Modify `InputAction::ScrollDown { lines }` handler:
  - [ ] Get mutable reference to active pane from pane_manager
  - [ ] Call `pane.scroll_down(lines)`
  - [ ] Update server notification to use pane's scroll_offset
  - [ ] Handle JumpToBottom case when offset reaches 0

### Phase 2: Rendering Integration (if needed)

- [ ] Open `fugue-client/src/ui/pane.rs`
- [ ] Verify `render_pane()` uses `pane.scroll_offset`
- [ ] If tui-term supports offset:
  - [ ] Configure PseudoTerminal with scroll offset
- [ ] If tui-term doesn't support offset:
  - [ ] Implement manual scrollback row extraction
  - [ ] Render using raw Paragraph widgets
- [ ] Verify scroll indicator in `PaneWidget::create_block()` works

### Phase 3: State Cleanup (optional)

- [ ] Review `InputHandler::scroll_offset` usage
- [ ] Consider removing duplicate scroll state from InputHandler
- [ ] Or sync InputHandler.scroll_offset with Pane.scroll_offset
- [ ] Update copy mode to use unified scroll state

## Testing Tasks

- [ ] Build and run fugue client with changes
- [ ] Test: Generate terminal output (e.g., `ls -la /usr/bin`)
- [ ] Test: Scroll up with mouse wheel - verify older content shows
- [ ] Test: Scroll down with mouse wheel - verify return to live
- [ ] Test: Scroll to top boundary - verify stops at oldest content
- [ ] Test: Scroll to bottom boundary - verify stops at live position
- [ ] Test: Scroll indicator shows offset in pane title
- [ ] Test: New output arrives while scrolled - verify position maintained
- [ ] Test: Trackpad scroll gestures (if available)

## Edge Case Testing

- [ ] Test scroll with empty scrollback (new pane)
- [ ] Test scroll with full scrollback (1000+ lines)
- [ ] Test scroll during active output stream
- [ ] Test scroll across pane focus changes

## Documentation Tasks

- [ ] Update PLAN.md with investigation findings
- [ ] Document any API discoveries about tui-term
- [ ] Note any gotchas or limitations found

## Verification Tasks

- [ ] All scroll tests passing
- [ ] Scroll is responsive (no visible lag)
- [ ] Scroll indicator updates correctly
- [ ] No regressions in normal terminal rendering
- [ ] Copy mode still works (if scroll state unified)
- [ ] Update feature_request.json status when complete

## Completion Checklist

- [ ] All implementation tasks complete
- [ ] All tests passing
- [ ] PLAN.md updated with final approach
- [ ] No compiler warnings from changes
- [ ] Ready for review/merge

---
*Check off tasks as you complete them. Update status field above.*
