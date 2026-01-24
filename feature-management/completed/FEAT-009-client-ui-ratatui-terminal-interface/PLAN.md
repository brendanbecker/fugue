# Implementation Plan: FEAT-009

**Work Item**: [FEAT-009: Client UI - Ratatui Terminal Interface](PROMPT.md)
**Component**: fugue-client
**Priority**: P1
**Created**: 2026-01-08

## Overview

Ratatui-based terminal UI with pane rendering using tui-term, status bar, borders, and Claude state indicators. This is the primary visual interface for fugue.

## Architecture Decisions

### Component Hierarchy

```
App
  |-- Terminal (crossterm backend)
  |-- State
  |     |-- ConnectionState
  |     |-- LayoutState
  |     |-- PaneStates (HashMap<PaneId, PaneState>)
  |     |-- FocusedPane
  |
  |-- Widgets
        |-- RootLayout
        |     |-- PaneContainer (recursive splits)
        |     |     |-- PaneWidget (tui-term)
        |     |     |-- BorderWidget
        |     |
        |     |-- StatusBar
        |
        |-- ClaudeIndicator (per pane)
```

### Event Flow

```
Terminal Events (crossterm) ----+
                                |
Server Events (Unix socket) ----+---> Event Loop ---> State Update ---> Render
                                |
Tick Events (for animations) ---+
```

### State Model

```rust
struct AppState {
    connection: ConnectionState,  // Connected, Disconnected, Reconnecting
    layout: LayoutNode,           // Tree of splits and panes
    panes: HashMap<PaneId, PaneState>,
    focused: Option<PaneId>,
    status_message: Option<String>,
}

struct PaneState {
    id: PaneId,
    title: String,
    terminal: PseudoTerminal,  // tui-term
    claude_state: ClaudeState,
    scrollback_offset: usize,
    has_new_content: bool,
}

enum ClaudeState {
    Idle,
    Thinking,
    ToolUse(String),  // tool name
    Error(String),
    Complete,
}
```

### Layout System

Tree-based layout with constraints:

```rust
enum LayoutNode {
    Split {
        direction: Direction,  // Horizontal or Vertical
        ratio: f32,            // 0.0 to 1.0, position of split
        first: Box<LayoutNode>,
        second: Box<LayoutNode>,
    },
    Pane(PaneId),
}
```

### tui-term Integration

tui-term provides `PseudoTerminal` widget that:
- Accepts raw bytes from PTY
- Handles ANSI escape sequences
- Renders to Ratatui buffer
- Supports scrollback

## Affected Components

| Component | Type of Change | Risk Level |
|-----------|----------------|------------|
| fugue-client/src/ui.rs | Major - complete rewrite | High |
| fugue-client/src/ui/layout.rs | New file | Medium |
| fugue-client/src/ui/pane.rs | New file | Medium |
| fugue-client/src/ui/status.rs | New file | Low |
| fugue-client/src/ui/borders.rs | New file | Low |
| fugue-client/src/ui/indicators.rs | New file | Low |
| fugue-client/src/main.rs | Medium - integrate UI | Medium |
| fugue-client/Cargo.toml | Minor - add deps | Low |

## Dependencies

- **FEAT-007**: Client-server protocol for receiving pane updates
- **FEAT-011**: Claude state detection for indicator updates

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Performance with many panes | Medium | Medium | Lazy rendering, damage tracking |
| tui-term compatibility issues | Low | High | Fallback to raw text rendering |
| Complex layout edge cases | Medium | Low | Comprehensive layout tests |
| Terminal compatibility | Low | Medium | Test on multiple terminals |
| Flickering during rapid updates | Medium | Medium | Double buffering, batched renders |

## Implementation Phases

### Phase 1: Foundation
- Add dependencies to Cargo.toml
- Basic terminal setup/teardown
- Simple single-pane rendering
- Event loop skeleton

### Phase 2: Layout System
- Layout tree implementation
- Split/resize logic
- Constraint calculations
- Border rendering

### Phase 3: Pane Widget
- tui-term integration
- Pane focus handling
- Scrollback support
- Pane titles

### Phase 4: Status and Indicators
- Status bar implementation
- Claude state indicators
- Animation support
- Dynamic updates

### Phase 5: Polish
- Error handling
- Edge cases
- Performance optimization
- Documentation

## Rollback Strategy

If implementation causes issues:
1. Revert commits associated with this work item
2. Restore stub ui.rs
3. Client falls back to headless/logging mode
4. Document what went wrong in comments.md

## Testing Strategy

1. **Unit Tests**: Layout calculations, state transitions
2. **Widget Tests**: Render to test buffer, verify output
3. **Integration Tests**: Mock terminal, simulate events
4. **Manual Tests**: Visual verification on real terminals
5. **Cross-platform**: Test on Linux, macOS, Windows (WSL)

## Key Dependencies (Crates)

```toml
[dependencies]
ratatui = "0.28"           # TUI framework
tui-term = "0.2"           # Terminal emulation widget
crossterm = "0.28"         # Terminal backend
```

## Implementation Notes

<!-- Add notes during implementation -->

---
*This plan should be updated as implementation progresses.*
