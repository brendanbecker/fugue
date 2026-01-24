# Implementation Plan: FEAT-010

**Work Item**: [FEAT-010: Client Input - Keyboard and Mouse Event Handling](PROMPT.md)
**Component**: fugue-client
**Priority**: P1
**Created**: 2026-01-08

## Overview

Keyboard and mouse event handling via crossterm, prefix key support (tmux-style), and input routing to active pane. This feature enables all user interaction with the fugue client.

## Architecture Decisions

### Event Handling Model

Use crossterm's async event stream with tokio for non-blocking input:

```rust
use crossterm::event::{Event, EventStream, KeyCode, KeyModifiers, MouseEvent};
use futures::StreamExt;

async fn input_loop(mut events: EventStream) {
    while let Some(Ok(event)) = events.next().await {
        match event {
            Event::Key(key) => handle_key(key),
            Event::Mouse(mouse) => handle_mouse(mouse),
            Event::Resize(w, h) => handle_resize(w, h),
            _ => {}
        }
    }
}
```

### Input State Machine

```
                    ┌─────────────────┐
                    │   Normal Mode   │
                    │ (input to pane) │
                    └────────┬────────┘
                             │ prefix key
                             ▼
                    ┌─────────────────┐
         timeout    │  Command Mode   │◄──┐
        ┌───────────│ (await command) │   │ invalid key
        │           └────────┬────────┘───┘
        │                    │ valid command
        ▼                    ▼
┌───────────────┐   ┌─────────────────┐
│ Normal Mode   │   │ Execute Action  │
└───────────────┘   └────────┬────────┘
                             │
                             ▼
                    ┌─────────────────┐
                    │   Normal Mode   │
                    └─────────────────┘
```

### Key Binding Configuration

TOML configuration format:

```toml
[prefix]
key = "C-b"  # Ctrl+B
timeout_ms = 500

[bindings.navigation]
"h" = "focus_left"
"j" = "focus_down"
"k" = "focus_up"
"l" = "focus_right"
"Left" = "focus_left"
"Down" = "focus_down"
"Up" = "focus_up"
"Right" = "focus_right"

[bindings.panes]
"|" = "split_vertical"
"-" = "split_horizontal"
"x" = "close_pane"
"z" = "toggle_zoom"

[bindings.windows]
"c" = "create_window"
"n" = "next_window"
"p" = "prev_window"
"0" = "select_window:0"
"1" = "select_window:1"

[bindings.session]
"d" = "detach"
"[" = "copy_mode"
":" = "command_prompt"
```

### Key Parsing

```rust
enum KeySpec {
    Simple(KeyCode),
    Modified { modifiers: KeyModifiers, key: KeyCode },
}

// Parse "C-b" -> Ctrl+B, "M-x" -> Alt+X, "S-Tab" -> Shift+Tab
fn parse_key_spec(s: &str) -> Result<KeySpec, ParseError>;
```

### Input Routing

```
┌──────────────────────────────────────────────────┐
│                  Input Handler                    │
├──────────────────────────────────────────────────┤
│  ┌────────────┐  ┌────────────┐  ┌────────────┐  │
│  │   Normal   │  │  Command   │  │   Mouse    │  │
│  │   Input    │  │   Mode     │  │  Handler   │  │
│  └─────┬──────┘  └─────┬──────┘  └─────┬──────┘  │
│        │               │               │          │
│        ▼               ▼               ▼          │
│  ┌─────────────────────────────────────────────┐ │
│  │              Action Dispatcher               │ │
│  └─────────────────────────────────────────────┘ │
│        │               │               │          │
│        ▼               ▼               ▼          │
│  ┌──────────┐   ┌──────────┐   ┌──────────┐      │
│  │ PTY Write│   │  Window  │   │   Pane   │      │
│  │          │   │  Mgmt    │   │  Select  │      │
│  └──────────┘   └──────────┘   └──────────┘      │
└──────────────────────────────────────────────────┘
```

## Affected Components

| Component | Type of Change | Risk Level |
|-----------|----------------|------------|
| fugue-client/src/input.rs | New - core input handling | Medium |
| fugue-client/src/keybindings.rs | New - binding config | Low |
| fugue-client/src/lib.rs | Medium - integrate input | Low |
| fugue-client/Cargo.toml | Minor - add crossterm | Low |

## Dependencies

- **FEAT-009**: Required for pane/window context to route input

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Input lag under load | Low | High | Async processing, buffering |
| Terminal compatibility issues | Medium | Medium | Test multiple emulators |
| Mouse capture conflicts | Medium | Low | Provide toggle, document |
| Modifier key detection issues | Low | Medium | Platform-specific testing |

## Implementation Phases

### Phase 1: Core Event Loop
- Set up crossterm raw mode
- Basic event polling
- Simple key echo for testing

### Phase 2: Command Mode
- Prefix key detection
- State machine implementation
- Timeout handling

### Phase 3: Key Bindings
- Configuration parsing
- Action dispatch
- Default bindings

### Phase 4: Input Routing
- PTY write integration
- Paste handling
- Bracketed paste

### Phase 5: Mouse Support
- Click handling
- Scroll wheel
- Pane selection

## Rollback Strategy

If implementation causes issues:
1. Revert commits associated with this work item
2. Input handling is isolated - no impact on server
3. Can fall back to simpler input handling if needed
4. Document what went wrong in comments.md

## Testing Strategy

1. **Unit Tests**: Key parsing, binding lookup, state machine
2. **Integration Tests**: Full input -> action flow
3. **Manual Tests**: Various terminal emulators
4. **Platform Tests**: Linux, macOS, Windows (CI matrix)

## Implementation Notes

<!-- Add notes during implementation -->

---
*This plan should be updated as implementation progresses.*
