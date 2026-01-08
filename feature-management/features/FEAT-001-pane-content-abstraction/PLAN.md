# Implementation Plan: FEAT-001

**Work Item**: [FEAT-001: Pane Content Abstraction (Terminal vs Canvas)](PROMPT.md)
**Component**: session/pane
**Priority**: P1
**Created**: 2026-01-08

## Overview

Refactor the pane system to support multiple content types beyond terminals. This enables rich interactive widgets (canvases) to be displayed alongside traditional PTY-based terminal content.

## Architecture Decisions

### 1. PaneContent vs PaneState Separation

**Decision**: Keep `PaneContent` (what is displayed) separate from `PaneState` (operational mode).

**Rationale**:
- Orthogonal concerns should have orthogonal representations
- A canvas could be in "Claude" mode if spawned by Claude
- A terminal could exit (Exited state) while canvas doesn't have that concept
- Cleaner code and easier reasoning

### 2. Trait Object for Canvas

**Decision**: Use `Box<dyn Canvas>` for runtime polymorphism.

**Rationale**:
- Allows adding new canvas types without modifying PaneContent enum
- Enables external crates to define canvas types
- Slight runtime overhead acceptable for interactive widgets

**Trade-offs**:
- Dynamic dispatch overhead (~nanoseconds per call)
- Cannot be `Copy` or easily cloned
- Requires careful lifetime management

### 3. Canvas in Protocol Crate

**Decision**: Define `Canvas` trait in `ccmux-protocol` crate.

**Rationale**:
- Protocol crate is shared between client and server
- Allows client to understand canvas messages
- Keeps type definitions centralized

### 4. Sideband Command Format

**Decision**: XML-style tags consistent with existing `<ccmux:spawn>` pattern.

**Format**:
```xml
<ccmux:canvas type="diff" path="src/main.rs"/>
<ccmux:canvas type="test_results">{"passed": 5, "failed": 1}</ccmux:canvas>
```

## Affected Components

| Component | Type of Change | Risk Level |
|-----------|----------------|------------|
| `ccmux-protocol/src/lib.rs` | Add PaneContent, Canvas trait | Medium |
| `ccmux-server/src/session/pane.rs` | Major refactor | High |
| `ccmux-server/src/canvas/` (new) | New module | Low |
| `ccmux-server/src/session/mod.rs` | Export changes | Low |

## Dependencies

None - this is a foundational feature that other features may depend on.

## Implementation Phases

### Phase 1: Core Type Definitions (ccmux-protocol)

1. Define `Canvas` trait:
   ```rust
   pub trait Canvas: Send + Sync {
       fn render(&self, area: Rect, buf: &mut Buffer);
       fn handle_input(&mut self, event: Event) -> Option<CanvasAction>;
       fn title(&self) -> &str;
   }
   ```

2. Define `CanvasAction` enum:
   ```rust
   pub enum CanvasAction {
       Close,
       Scroll(i32),
       Custom(String),
   }
   ```

3. Define `PaneContent` enum:
   ```rust
   pub enum PaneContent {
       Terminal(TerminalState),
       Canvas(Box<dyn Canvas>),
   }
   ```

### Phase 2: Pane Refactor (ccmux-server)

1. Update `Pane` struct to use `PaneContent`
2. Implement content-type dispatch in render path
3. Implement content-type dispatch in input handling
4. Update pane factory methods

### Phase 3: Canvas Implementations

1. Create `canvas/` module structure
2. Implement `DiffCanvas`:
   - Parse unified diff format
   - Render with syntax highlighting
   - Support scroll, line navigation
3. Implement `TestResultsCanvas`:
   - Parse test result JSON
   - Render pass/fail summary
   - Expandable test details

### Phase 4: Sideband Integration

1. Add canvas command parsing to sideband parser
2. Implement canvas spawning in session manager
3. Handle canvas updates and lifecycle

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Breaking existing terminal functionality | Medium | High | Comprehensive testing, feature flag |
| Canvas rendering performance | Low | Medium | Dirty tracking, profile early |
| Complex input handling | Medium | Medium | Clear state machine, good tests |
| Sideband parsing edge cases | Low | Low | Fuzzing, strict parsing |

## Rollback Strategy

If implementation causes issues:

1. Revert commits associated with FEAT-001
2. Verify terminal-only functionality works
3. Document what went wrong in comments.md
4. Consider phased approach with feature flag

## Testing Strategy

### Unit Tests
- `PaneContent` dispatch logic
- Each `Canvas` implementation render output
- Input handling state transitions

### Integration Tests
- Sideband command parsing and canvas creation
- Canvas lifecycle (create, render, close)
- Mixed terminal/canvas sessions

### Manual Testing
- Visual verification of diff rendering
- Test results display with real test output
- Input responsiveness in canvases

## Implementation Notes

<!-- Add notes during implementation -->

---
*This plan should be updated as implementation progresses.*
