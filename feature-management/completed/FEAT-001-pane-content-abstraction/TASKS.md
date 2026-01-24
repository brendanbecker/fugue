# Task Breakdown: FEAT-001

**Work Item**: [FEAT-001: Pane Content Abstraction (Terminal vs Canvas)](PROMPT.md)
**Status**: Not Started
**Last Updated**: 2026-01-08

## Prerequisites

- [ ] Read and understand PROMPT.md
- [ ] Review PLAN.md and update if needed
- [ ] Review current pane.rs implementation
- [ ] Review fugue-protocol/src/lib.rs structure

## Phase 1: Core Type Definitions

### Protocol Types (fugue-protocol)
- [ ] Define `CanvasAction` enum (Close, Scroll, Custom)
- [ ] Define `Canvas` trait with render(), handle_input(), title()
- [ ] Define `TerminalState` wrapper type (if needed)
- [ ] Define `PaneContent` enum (Terminal, Canvas variants)
- [ ] Add necessary ratatui re-exports (Rect, Buffer)
- [ ] Add crossterm Event re-export

### Verify Phase 1
- [ ] Protocol crate compiles
- [ ] Types are properly exported
- [ ] Documentation added for public types

## Phase 2: Pane Refactor

### Update Pane Struct (fugue-server)
- [ ] Add `content: PaneContent` field to `Pane`
- [ ] Update `Pane::new()` to accept content type parameter
- [ ] Add `Pane::new_terminal()` convenience constructor
- [ ] Add `Pane::new_canvas()` convenience constructor

### Render Path Updates
- [ ] Implement content-type dispatch in pane rendering
- [ ] Terminal rendering uses existing terminal logic
- [ ] Canvas rendering calls Canvas::render()
- [ ] Handle title display for both content types

### Input Handling Updates
- [ ] Implement content-type dispatch in input handling
- [ ] Terminal input uses existing PTY write path
- [ ] Canvas input calls Canvas::handle_input()
- [ ] Handle CanvasAction results (Close, etc.)

### Verify Phase 2
- [ ] Server crate compiles
- [ ] Existing terminal tests pass
- [ ] Manual test: terminal panes work as before

## Phase 3: Canvas Implementations

### Canvas Module Setup
- [ ] Create `fugue-server/src/canvas/mod.rs`
- [ ] Create `fugue-server/src/canvas/diff.rs`
- [ ] Create `fugue-server/src/canvas/test_results.rs`
- [ ] Export canvas module from session/mod.rs

### DiffCanvas Implementation
- [ ] Define `DiffCanvas` struct (diff data, scroll position)
- [ ] Implement diff parsing (unified format)
- [ ] Implement Canvas::render() for diff display
- [ ] Implement Canvas::handle_input() (scroll, close)
- [ ] Add syntax highlighting for diff lines (+/-)
- [ ] Implement Canvas::title() ("Diff: filename")

### TestResultsCanvas Implementation
- [ ] Define `TestResultsCanvas` struct
- [ ] Define test result data structures
- [ ] Implement Canvas::render() for test display
- [ ] Implement Canvas::handle_input() (expand/collapse, scroll)
- [ ] Show pass/fail counts, timing
- [ ] Implement Canvas::title() ("Test Results")

### Verify Phase 3
- [ ] Canvas module compiles
- [ ] Unit tests for DiffCanvas rendering
- [ ] Unit tests for TestResultsCanvas rendering
- [ ] Manual test: render sample diff
- [ ] Manual test: render sample test results

## Phase 4: Sideband Integration

### Parser Updates
- [ ] Add `<fugue:canvas>` command to sideband parser
- [ ] Parse `type` attribute (diff, test_results)
- [ ] Parse content/data (path or inline JSON)
- [ ] Handle malformed canvas commands gracefully

### Session Manager Integration
- [ ] Add canvas spawn handler to session manager
- [ ] Create appropriate canvas based on type
- [ ] Add canvas pane to session
- [ ] Handle canvas close action

### Verify Phase 4
- [ ] Sideband parser tests pass
- [ ] Integration test: spawn diff canvas via sideband
- [ ] Integration test: spawn test_results canvas via sideband
- [ ] Manual test: end-to-end canvas spawning

## Phase 5: Testing & Documentation

### Unit Tests
- [ ] PaneContent dispatch tests
- [ ] Canvas trait object behavior tests
- [ ] DiffCanvas parsing edge cases
- [ ] TestResultsCanvas data handling

### Integration Tests
- [ ] Terminal pane creation and rendering
- [ ] Canvas pane creation and rendering
- [ ] Mixed session with terminals and canvases
- [ ] Sideband canvas command processing

### Documentation
- [ ] Update ARCHITECTURE.md with PaneContent design
- [ ] Document Canvas trait for implementers
- [ ] Document sideband canvas command format
- [ ] Add code comments for complex logic

### Verify Phase 5
- [ ] All unit tests pass
- [ ] All integration tests pass
- [ ] Documentation reviewed
- [ ] No regression in existing functionality

## Completion Checklist

- [ ] All implementation tasks complete
- [ ] All tests passing
- [ ] PLAN.md updated with final approach
- [ ] feature_request.json status updated
- [ ] Ready for review/merge

---
*Check off tasks as you complete them. Update status field above.*
