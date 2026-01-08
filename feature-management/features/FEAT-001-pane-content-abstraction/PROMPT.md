# FEAT-001: Pane Content Abstraction (Terminal vs Canvas)

**Priority**: P1
**Component**: session/pane
**Type**: new_feature
**Estimated Effort**: large
**Business Value**: high

## Overview

Panes should not be tightly coupled to PTY sessions. Design for multiple content types using a PaneContent enum that supports Terminal (PTY + vt100 parser + buffer) and Canvas (structured interactive widget) variants.

This enables spawning rich interactive views like diff viewers and test results viewers via sideband commands, allowing Claude Code to present structured information in a more readable format than raw terminal output.

## Technical Design

### PaneContent Enum

```rust
enum PaneContent {
    Terminal(TerminalSession),  // PTY + vt100 parser + buffer
    Canvas(Box<dyn Canvas>),    // Structured interactive widget
}

trait Canvas {
    fn render(&self, area: Rect, buf: &mut Buffer);
    fn handle_input(&mut self, event: Event) -> Option<Action>;
}
```

### Integration with Existing PaneState

The existing `PaneState` enum (Normal, Claude, Exited) tracks operational state and should remain separate from content type. The relationship is:

- `PaneContent` - WHAT the pane displays (Terminal or Canvas)
- `PaneState` - HOW the pane behaves (Normal, Claude mode, Exited)

A pane can have any combination: a Terminal in Claude mode, a Canvas in Normal mode, etc.

### Sideband Command for Canvas Spawning

Support spawning canvases via sideband protocol:

```xml
<ccmux:canvas type="diff" path="src/main.rs"/>
<ccmux:canvas type="test_results" data="..."/>
```

## Requirements

1. **Replace current implicit terminal-only model with explicit PaneContent enum**
   - Define `PaneContent` enum in `ccmux-protocol/src/lib.rs`
   - Update `Pane` struct to use `PaneContent` instead of implicit terminal assumption

2. **Define Canvas trait for structured interactive widgets**
   - Create `Canvas` trait with `render()` and `handle_input()` methods
   - Support ratatui's `Rect` and `Buffer` types for rendering

3. **MVP canvas types: Diff viewer, Test results viewer**
   - Implement `DiffCanvas` for side-by-side or unified diff display
   - Implement `TestResultsCanvas` for structured test output

4. **Support spawning canvases via sideband**
   - Parse `<ccmux:canvas>` sideband commands
   - Create and manage canvas lifecycle

5. **Integrate with existing PaneState**
   - Ensure PaneContent and PaneState remain orthogonal
   - Canvas panes should support all relevant states

## Current State

- `PaneState` enum exists but tracks state, not content type
- Parser is a stub ("Terminal parsing - to be implemented")
- No content renderer abstraction
- Panes implicitly assume terminal content

## Affected Files

| File | Type of Change |
|------|----------------|
| `ccmux-server/src/session/pane.rs` | Major refactor - add PaneContent |
| `ccmux-protocol/src/lib.rs` | New types - PaneContent, Canvas trait |
| `ccmux-server/src/canvas/` (new) | New module - Canvas implementations |
| `ccmux-server/src/session/mod.rs` | Update to export canvas module |

## Implementation Tasks

### Section 1: Core Abstractions
- [ ] Define `PaneContent` enum in ccmux-protocol
- [ ] Define `Canvas` trait in ccmux-protocol
- [ ] Define `Action` enum for canvas input handling results

### Section 2: Pane Refactor
- [ ] Update `Pane` struct to use `PaneContent`
- [ ] Implement `TerminalSession` wrapper type
- [ ] Update pane creation to specify content type
- [ ] Update pane rendering to dispatch by content type

### Section 3: Canvas Module
- [ ] Create `ccmux-server/src/canvas/mod.rs`
- [ ] Implement `DiffCanvas` for diff viewing
- [ ] Implement `TestResultsCanvas` for test output
- [ ] Add canvas-specific input handling

### Section 4: Sideband Integration
- [ ] Parse `<ccmux:canvas>` commands
- [ ] Create canvas panes from sideband commands
- [ ] Handle canvas lifecycle (creation, updates, close)

### Section 5: Testing
- [ ] Unit tests for PaneContent dispatch
- [ ] Unit tests for Canvas trait implementations
- [ ] Integration tests for sideband canvas spawning

## Acceptance Criteria

- [ ] PaneContent enum properly distinguishes Terminal and Canvas content
- [ ] Canvas trait enables custom widget implementations
- [ ] DiffCanvas renders diff output in structured format
- [ ] TestResultsCanvas renders test results in structured format
- [ ] Sideband `<ccmux:canvas>` command spawns canvas panes
- [ ] Existing terminal functionality unchanged
- [ ] PaneState remains orthogonal to PaneContent
- [ ] All tests passing
- [ ] Documentation updated

## Notes

### Design Considerations

1. **Trait Object vs Enum**: Using `Box<dyn Canvas>` allows runtime extensibility but has dynamic dispatch overhead. Could consider an enum-based approach if canvas types are known at compile time.

2. **Rendering Performance**: Canvas rendering should be efficient - consider dirty tracking to avoid unnecessary redraws.

3. **Input Handling**: Canvas input handling returns `Option<Action>` to allow canvases to either handle input internally or bubble up actions to the parent.

### Future Extensions

- Additional canvas types: Log viewer, JSON explorer, Image viewer (sixel)
- Canvas composition (multiple canvases in splits)
- Canvas serialization for session persistence
