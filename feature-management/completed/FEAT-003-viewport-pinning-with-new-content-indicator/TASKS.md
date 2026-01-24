# Task Breakdown: FEAT-003

**Work Item**: [FEAT-003: Viewport Pinning with New Content Indicator](PROMPT.md)
**Status**: Not Started
**Last Updated**: 2026-01-08

## Prerequisites

- [ ] Read and understand PROMPT.md
- [ ] Review PLAN.md and update if needed
- [ ] Verify no blocking dependencies
- [ ] Review current pane rendering code in fugue-client

## Design Tasks

- [ ] Review requirements and acceptance criteria
- [ ] Design ViewportState struct and transitions
- [ ] Design indicator widget layout and styling
- [ ] Identify affected components and interfaces
- [ ] Update PLAN.md with final approach
- [ ] Consider edge cases (resize, rapid output, empty pane)

## Implementation Tasks

### Protocol Extension
- [ ] Add viewport-related types to fugue-protocol
- [ ] Define ViewportState struct (if shared)
- [ ] Update protocol version if needed

### Client Viewport State
- [ ] Create ViewportState struct in client
- [ ] Add viewport state to per-pane UI state
- [ ] Implement offset calculation logic
- [ ] Implement pinning detection (scroll up = pin)
- [ ] Implement new line counting while pinned

### Content Rendering
- [ ] Modify pane renderer to use viewport offset
- [ ] Calculate visible line range from total content
- [ ] Handle scrollback buffer indexing
- [ ] Test rendering with various offsets

### Input Handling
- [ ] Add mouse wheel scroll event handling
- [ ] Implement scroll up/down with offset adjustment
- [ ] Add keyboard scroll handlers (Page Up/Down)
- [ ] Implement jump-to-bottom keybinding (G, Ctrl+End)

### Indicator Widget
- [ ] Create NewContentIndicator component
- [ ] Implement line count display ("▼ N new lines")
- [ ] Position indicator at bottom of pane
- [ ] Style indicator (color, background)
- [ ] Add click handler to indicator

### Scroll Behavior
- [ ] Implement instant jump to bottom
- [ ] Implement smooth scroll animation (optional)
- [ ] Add configuration option for scroll behavior
- [ ] Clear indicator on reaching bottom

### Configuration
- [ ] Add scroll_behavior option to config schema
- [ ] Add scrollback_lines limit option
- [ ] Wire config values to viewport behavior
- [ ] Document new configuration options

## Testing Tasks

- [ ] Unit tests for ViewportState
- [ ] Unit tests for offset calculations
- [ ] Unit tests for pinning logic
- [ ] Integration test: scroll up pins viewport
- [ ] Integration test: new content updates counter
- [ ] Integration test: jump to bottom works
- [ ] Integration test: click on indicator works
- [ ] Manual test with rapid output
- [ ] Manual test with pane resize
- [ ] Run full test suite

## Documentation Tasks

- [ ] Update user documentation (if any)
- [ ] Add code comments for viewport logic
- [ ] Document configuration options
- [ ] Update CHANGELOG if applicable

## Verification Tasks

- [ ] All acceptance criteria from PROMPT.md met
- [ ] Tests passing
- [ ] No performance regression with rapid output
- [ ] Update feature_request.json status
- [ ] Document completion in comments.md

## Completion Checklist

- [ ] All implementation tasks complete
- [ ] All tests passing
- [ ] Documentation updated
- [ ] PLAN.md reflects final implementation
- [ ] Ready for review/merge

---
*Check off tasks as you complete them. Update status field above.*
