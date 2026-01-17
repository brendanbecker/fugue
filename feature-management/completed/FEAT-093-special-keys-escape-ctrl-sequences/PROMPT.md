# FEAT-093: Add support for sending special keys (Escape, Ctrl sequences, function keys)

**Priority**: P2
**Component**: ccmux-server
**Type**: new_feature
**Estimated Effort**: medium
**Business Value**: high

## Overview

Currently ccmux_send_input only sends literal text. There is no way to send special keys like Escape, Ctrl+C, Ctrl+U, function keys, or arrow keys. Escape sequences like `\x1b` are sent as literal text rather than interpreted as the actual key.

## Use Case

- Gemini CLI entered "shell mode" (triggered by `!` character)
- Needed to send Escape key to exit shell mode
- Tried sending `\x1b`, `Escape`, control characters - all appeared as literal text
- No way to recover without closing and recreating the pane

## Proposed Solutions

### Option 1: Add a `ccmux_press_key` tool (Recommended)

Similar to Playwright's `browser_press_key`, add a dedicated tool for sending special keys:

```json
{
  "name": "ccmux_press_key",
  "inputSchema": {
    "type": "object",
    "properties": {
      "pane_id": { "type": "string" },
      "key": {
        "type": "string",
        "description": "Key name: Escape, Enter, Tab, Backspace, Delete, Up, Down, Left, Right, Home, End, PageUp, PageDown, F1-F12, Ctrl+<key>, Alt+<key>"
      },
      "count": {
        "type": "integer",
        "default": 1,
        "description": "Number of times to press the key"
      }
    },
    "required": ["key"]
  }
}
```

### Option 2: Add `keys` parameter to send_input

Extend the existing `ccmux_send_input` tool:

```json
{
  "name": "ccmux_send_input",
  "inputSchema": {
    "type": "object",
    "properties": {
      "pane_id": { "type": "string" },
      "text": { "type": "string" },
      "keys": {
        "type": "array",
        "items": { "type": "string" },
        "description": "Special keys to send after text"
      }
    }
  }
}
```

## Key Mappings

| Key Name | Escape Sequence |
|----------|-----------------|
| Escape | `\x1b` |
| Enter | `\r` or `\n` |
| Tab | `\t` |
| Backspace | `\x7f` or `\x08` |
| Delete | `\x1b[3~` |
| Up | `\x1b[A` |
| Down | `\x1b[B` |
| Right | `\x1b[C` |
| Left | `\x1b[D` |
| Home | `\x1b[H` |
| End | `\x1b[F` |
| PageUp | `\x1b[5~` |
| PageDown | `\x1b[6~` |
| F1-F12 | `\x1bOP` through `\x1b[24~` |
| Ctrl+C | `\x03` |
| Ctrl+D | `\x04` |
| Ctrl+U | `\x15` |
| Ctrl+Z | `\x1a` |
| Ctrl+\\ | `\x1c` |

## Implementation Tasks

### Section 1: Design
- [ ] Decide between Option 1 (new tool) vs Option 2 (extend existing)
- [ ] Define complete key name vocabulary
- [ ] Document key-to-escape-sequence mapping

### Section 2: Protocol Changes
- [ ] Add new message type or extend SendInput message
- [ ] Update ccmux-protocol with key enum or key parsing

### Section 3: Server Implementation
- [ ] Implement key name to escape sequence translation
- [ ] Add handler for new tool/parameter
- [ ] Write bytes directly to PTY

### Section 4: MCP Tool Registration
- [ ] Register new tool or update existing tool schema
- [ ] Update tool descriptions with key name examples

### Section 5: Testing
- [ ] Unit tests for key name parsing
- [ ] Unit tests for escape sequence generation
- [ ] Integration test with interactive program (vim, less, etc.)

### Section 6: Documentation
- [ ] Document supported key names
- [ ] Add examples to tool documentation
- [ ] Update CHANGELOG

## Acceptance Criteria

- [ ] Can send Escape key to exit modes in terminal programs
- [ ] Can send Ctrl+C to interrupt running processes
- [ ] Can send arrow keys for navigation
- [ ] Can send function keys (F1-F12)
- [ ] Key names are case-insensitive
- [ ] Invalid key names return clear error message
- [ ] Documentation lists all supported key names

## Notes

- Consider whether to support key combinations like Ctrl+Shift+<key>
- May need to handle terminal application mode (cursor keys in application mode use different sequences)
- Should be consistent with crossterm's key handling since TUI already uses it
