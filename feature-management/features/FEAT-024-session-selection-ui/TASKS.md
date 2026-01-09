# Task Breakdown: FEAT-024

**Work Item**: [FEAT-024: Session Selection UI](PROMPT.md)
**Status**: In Progress
**Last Updated**: 2026-01-09

## Prerequisites

- [x] Read and understand PROMPT.md
- [x] Review PLAN.md and update if needed
- [x] Review current app.rs implementation (lines 478-517, 667-711)
- [ ] Verify FEAT-021 (Server Socket) is available

## Review Tasks

- [x] Review `handle_session_select_input()` implementation
- [x] Review `draw_session_select()` implementation
- [x] Compare implementation against requirements
- [x] Document any gaps found

## Implementation Tasks

### UI Rendering
- [x] Verify session list renders correctly
- [x] Verify selection highlight is visible
- [x] Verify empty state displays helpful message
- [x] Consider if List widget would improve display (upgraded from Paragraph to List)

### Navigation
- [x] Test up arrow navigation
- [x] Test down arrow navigation
- [x] Test 'k' for up navigation
- [x] Test 'j' for down navigation
- [x] Test bounds checking (can't go below 0 or above list length)

### Session Actions
- [x] Test Enter key attaches to selected session
- [x] Test 'n' key creates new session
- [x] Test 'r' key refreshes session list
- [x] Verify auto-attach after session creation

### Metadata Display
- [x] Verify session name displays
- [x] Verify window count displays
- [x] Verify attached client count displays

## Testing Tasks

- [ ] Manual test: Empty session list state
- [ ] Manual test: Single session in list
- [ ] Manual test: Multiple sessions in list
- [ ] Manual test: Navigate to first/last session
- [ ] Manual test: Attach to existing session
- [ ] Manual test: Create and attach to new session
- [ ] Manual test: Refresh session list

## Documentation Tasks

- [x] Verify help text shows correct keybindings
- [ ] Update CHANGELOG if needed

## Verification Tasks

- [ ] All acceptance criteria from PROMPT.md met
- [x] No regressions in existing functionality
- [ ] Update feature_request.json status
- [ ] Document completion in comments.md

## Completion Checklist

- [x] All implementation tasks complete
- [x] All tests passing
- [ ] PLAN.md updated with final notes
- [ ] Ready for review/merge

---
*Check off tasks as you complete them. Update status field above.*
