# Implementation Plan: FEAT-019

**Work Item**: [FEAT-019: Sideband Protocol - XML Command Parsing from Claude Output](PROMPT.md)
**Component**: fugue-server
**Priority**: P2
**Created**: 2026-01-08

## Overview

Parse XML-style commands from Claude output (`<fugue:spawn>`, `<fugue:input>`) for lightweight Claude-fugue communication.

## Architecture Decisions

### Command Type Hierarchy

```rust
pub enum SidebandCommand {
    Spawn(SpawnCommand),
    Input(InputCommand),
    Control(ControlCommand),
    Canvas(CanvasCommand),
}

pub struct SpawnCommand {
    pub cmd: String,
    pub cwd: Option<PathBuf>,
    pub env: HashMap<String, String>,
}

pub struct InputCommand {
    pub target: PaneId,
    pub text: String,
}

pub struct ControlCommand {
    pub action: ControlAction,
    pub target: PaneId,
}

pub struct CanvasCommand {
    pub canvas_type: CanvasType,
    pub path: Option<PathBuf>,
}
```

### Parsing Strategy

Two-phase parsing approach:
1. **Tag Detection**: Scan output for `<fugue:` prefix
2. **Command Parsing**: Extract tag name, attributes, and content

State machine for handling partial commands:

```
Passthrough -> TagStart -> TagName -> Attributes -> Content -> TagEnd
     ^                                                           |
     +-----------------------------------------------------------+
```

### Output Filtering

The parser maintains a buffer for incomplete tags:
- Output before `<fugue:` is passed through immediately
- Content between `<fugue:` and `/>` or `</fugue:*>` is buffered
- Complete commands are parsed and stripped
- Incomplete commands at buffer end are held for next read

### Event Emission

```rust
pub enum SidebandEvent {
    CommandParsed(SidebandCommand),
    ParseError { raw: String, error: String },
}
```

Events are sent via tokio::sync::mpsc channel to the session manager.

## Affected Components

| Component | Type of Change | Risk Level |
|-----------|----------------|------------|
| fugue-server/src/sideband/parser.rs | New - XML parser implementation | Medium |
| fugue-server/src/sideband/commands.rs | New - Command type definitions | Low |
| fugue-server/src/sideband/mod.rs | New - Module exports | Low |

## Dependencies

- FEAT-014 - Sideband protocol foundation (for event integration)
- PTY I/O system (FEAT-013) - for output stream access

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Parser vulnerability (injection) | Low | High | Strict validation, no shell expansion |
| Performance impact on output | Medium | Medium | Efficient scanning, minimal buffering |
| Incomplete command handling | Medium | Low | Timeout-based buffer flush |
| Command flooding | Low | Medium | Rate limiting on command execution |

## Implementation Phases

### Phase 1: Command Type Definitions
- Define all command structs and enums
- Implement validation methods
- Add serialization for debugging

### Phase 2: Parser Implementation
- Implement tag detection state machine
- Parse attributes into command structs
- Handle content extraction for input commands

### Phase 3: Output Filtering
- Integrate parser into PTY output path
- Strip recognized commands from output
- Handle buffer boundaries

### Phase 4: Event Integration
- Connect parser to event channel
- Integrate with session manager
- Add error event handling

## Rollback Strategy

If implementation causes issues:
1. Revert commits associated with this work item
2. Remove sideband module from fugue-server
3. Disable sideband parsing in PTY output path
4. Document what went wrong in comments.md

## Testing Strategy

1. **Unit Tests**: Each command type parsing, attribute extraction
2. **Edge Cases**: Malformed XML, partial tags, special characters
3. **Integration Tests**: Full PTY output pipeline with sideband commands
4. **Security Tests**: Injection attempts, oversized commands

## Implementation Notes

<!-- Add notes during implementation -->

---
*This plan should be updated as implementation progresses.*
