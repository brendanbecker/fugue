# Implementation Plan: FEAT-014

**Work Item**: [FEAT-014: Terminal Parsing - ANSI/VT100 State Machine](PROMPT.md)
**Component**: fugue-server
**Priority**: P1
**Created**: 2026-01-08
**Status**: Not Started

## Overview

ANSI/VT100 terminal state parsing using vt100 crate, screen buffer management, and escape sequence handling.

## Architecture Decisions

### Terminal Parser Abstraction

The `TerminalParser` struct wraps vt100's `Parser`:

```rust
pub struct TerminalParser {
    parser: vt100::Parser,
    scrollback: ScrollbackBuffer,
    title: Option<String>,
    cwd: Option<PathBuf>,
}
```

### Screen Buffer Strategy

```
+------------------+
| Scrollback       |  <- Historical lines (configurable limit)
| Buffer           |
+------------------+
| Active Screen    |  <- Current visible area (rows x cols)
| (vt100::Parser)  |
+------------------+
```

### Diff-Based Updates

Use vt100's `contents_diff()` to track changes between frames:

```rust
// Get changes since last update
let diff = parser.screen().contents_diff(&previous_screen);

// Apply only changed cells to client
for cell_change in diff {
    // Send incremental update
}
```

### OSC Sequence Handling

| OSC Code | Purpose | Implementation |
|----------|---------|----------------|
| OSC 0 | Set icon name and window title | Extract and store title |
| OSC 1 | Set icon name | Ignore (use OSC 0/2) |
| OSC 2 | Set window title | Extract and store title |
| OSC 7 | Set working directory | Parse file:// URL, extract path |
| OSC 52 | Clipboard operations | Future enhancement |

## Affected Components

| Component | Type of Change | Risk Level |
|-----------|----------------|------------|
| fugue-server/src/pty/parser.rs | New - Terminal parser implementation | Medium |
| fugue-server/src/pty/buffer.rs | New - Scrollback buffer | Low |
| fugue-server/src/pty/mod.rs | Modify - Add exports | Low |

## Dependencies

- `vt100` crate for VT100 terminal emulation
- FEAT-013 for PTY handle integration

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Memory usage with large scrollback | Medium | Medium | Configurable scrollback limit, memory-efficient storage |
| Escape sequence edge cases | Medium | Low | Rely on vt100 crate's tested implementation |
| Performance with high output | Low | Medium | Efficient diff updates, batch processing |
| Unicode handling | Low | Low | vt100 handles unicode properly |

## Implementation Phases

### Phase 1: Core Parser Infrastructure
- TerminalParser struct wrapping vt100::Parser
- Basic process_output() method
- Screen state extraction

### Phase 2: Screen Buffer Management
- ScrollbackBuffer implementation
- Configurable scrollback limit
- Buffer resize handling

### Phase 3: Content Extraction and Diff
- Cell content and attribute extraction
- contents_diff() integration
- Efficient update generation

### Phase 4: OSC Parsing
- Title extraction (OSC 0/2)
- CWD detection (OSC 7)
- Event emission for title/CWD changes

### Phase 5: Integration
- Connect with PtyHandle from FEAT-013
- Process PTY output through parser
- Expose parsed state to session management

## Rollback Strategy

If implementation causes issues:
1. Revert commits associated with this work item
2. Keep parser.rs as stub
3. Document what went wrong in comments.md

## Testing Strategy

1. **Unit Tests**: Escape sequence parsing, cursor tracking, OSC parsing
2. **Integration Tests**: Full PTY output processing
3. **Fuzz Tests**: Random escape sequences (vt100 crate handles this)
4. **Performance Tests**: High-throughput output processing

## Implementation Notes

<!-- Add notes during implementation -->

---
*This plan should be updated as implementation progresses.*
