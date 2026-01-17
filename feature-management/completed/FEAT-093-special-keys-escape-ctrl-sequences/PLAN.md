# Implementation Plan: FEAT-093

**Work Item**: [FEAT-093: Add support for sending special keys](PROMPT.md)
**Component**: ccmux-server
**Priority**: P2
**Created**: 2026-01-16

## Overview

Add capability to send special keys (Escape, Ctrl sequences, function keys, arrow keys) through the MCP interface, enabling full control of interactive terminal applications.

## Architecture Decisions

### Recommendation: Option 1 - New `ccmux_press_key` Tool

**Rationale**:
1. **Separation of concerns**: Text input and key presses are semantically different operations
2. **Cleaner API**: No confusion about whether `\x1b` in text is literal or interpreted
3. **Consistency**: Matches Playwright's model which users may be familiar with
4. **Extensibility**: Easier to add features like key repeat, modifiers, sequences

### Key Name Vocabulary

Use crossterm-compatible key names for consistency with existing TUI code:
- Single character: `a`, `A`, `1`, `!`
- Named keys: `Escape`, `Enter`, `Tab`, `Backspace`, `Delete`, `Insert`
- Navigation: `Up`, `Down`, `Left`, `Right`, `Home`, `End`, `PageUp`, `PageDown`
- Function keys: `F1` through `F12`
- Modifiers: `Ctrl+<key>`, `Alt+<key>`, `Shift+<key>`

### Implementation Location

| Component | Changes |
|-----------|---------|
| `ccmux-protocol/src/types.rs` | Add `PressKey` message variant |
| `ccmux-server/src/handlers/mcp.rs` | Add `ccmux_press_key` tool handler |
| `ccmux-server/src/keys.rs` (new) | Key name parsing and escape sequence generation |

## Affected Components

| Component | Type of Change | Risk Level |
|-----------|----------------|------------|
| ccmux-protocol | Add message type | Low |
| ccmux-server/handlers/mcp.rs | Add tool handler | Low |
| ccmux-server (new keys.rs) | New module | Low |

## Dependencies

None - this is a standalone feature.

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Incorrect escape sequences | Medium | Medium | Reference crossterm source, test with multiple terminals |
| Application mode vs normal mode | Low | Low | Document limitation, consider future enhancement |
| Key name ambiguity | Low | Low | Use well-defined vocabulary, case-insensitive |

## Rollback Strategy

If implementation causes issues:
1. Revert commits associated with this work item
2. Verify system returns to previous state
3. Document what went wrong in comments.md

## Implementation Notes

### Escape Sequence Reference

From ANSI/VT100 standards:
- Control characters: Ctrl+A = 0x01, Ctrl+B = 0x02, ..., Ctrl+Z = 0x1A
- Escape: 0x1B
- Arrow keys (normal mode): ESC [ A/B/C/D
- Arrow keys (application mode): ESC O A/B/C/D
- Function keys vary by terminal emulator

### crossterm Integration

Consider using crossterm's `KeyCode` enum for parsing, then generating raw escape sequences. This ensures consistency with the TUI's key handling.

```rust
use crossterm::event::KeyCode;

fn key_name_to_bytes(name: &str) -> Result<Vec<u8>, KeyError> {
    // Parse "Ctrl+C" -> KeyCode::Char('c') with Ctrl modifier
    // Then generate appropriate escape sequence
}
```

---
*This plan should be updated as implementation progresses.*
