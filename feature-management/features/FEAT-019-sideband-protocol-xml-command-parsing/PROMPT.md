# FEAT-019: Sideband Protocol - XML Command Parsing from Claude Output

**Priority**: P2
**Component**: ccmux-server
**Type**: new_feature
**Estimated Effort**: medium
**Business Value**: medium
**Status**: new

## Overview

Parse XML-style commands from Claude output (`<ccmux:spawn>`, `<ccmux:input>`) for lightweight Claude-ccmux communication.

## Requirements

- XML-style command parsing from PTY output stream
- Supported commands:
  - `<ccmux:spawn cmd="..." cwd="..."/>` - Spawn new pane
  - `<ccmux:input target="pane-id">text</ccmux:input>` - Send input to pane
  - `<ccmux:control action="focus" target="pane-id"/>` - Control commands
  - `<ccmux:canvas type="diff" path="..."/>` - Spawn canvas widget
- Command validation and error handling
- Strip sideband commands from rendered output
- Event emission for parsed commands

## Affected Files

- `ccmux-server/src/sideband/parser.rs`
- `ccmux-server/src/sideband/commands.rs`
- `ccmux-server/src/sideband/mod.rs`

## Implementation Tasks

### Section 1: Design
- [ ] Design XML command grammar and parsing strategy
- [ ] Design command type hierarchy (spawn, input, control, canvas)
- [ ] Design event emission system for parsed commands
- [ ] Plan integration with PTY output stream
- [ ] Document error handling strategy

### Section 2: Command Types
- [ ] Define SidebandCommand enum with all command variants
- [ ] Implement SpawnCommand with cmd, cwd, and optional attributes
- [ ] Implement InputCommand with target and text content
- [ ] Implement ControlCommand with action and target
- [ ] Implement CanvasCommand with type and path
- [ ] Add validation for all command fields

### Section 3: XML Parser
- [ ] Implement streaming XML tag detection in output
- [ ] Parse `<ccmux:*>` opening tags with attributes
- [ ] Handle self-closing tags (`/>`)
- [ ] Parse closing tags and extract content
- [ ] Handle malformed XML gracefully (log warning, pass through)
- [ ] Support nested content (e.g., text with special chars)

### Section 4: Output Filtering
- [ ] Strip recognized sideband commands from rendered output
- [ ] Preserve non-sideband content exactly
- [ ] Handle partial commands across buffer boundaries
- [ ] Buffer management for incomplete tags

### Section 5: Event Emission
- [ ] Define SidebandEvent type for parsed commands
- [ ] Implement event channel for command notifications
- [ ] Connect parser to session event loop
- [ ] Handle command execution errors

### Section 6: Testing
- [ ] Unit tests for each command type parsing
- [ ] Unit tests for attribute extraction
- [ ] Test malformed XML handling
- [ ] Test partial command buffering
- [ ] Test output stripping
- [ ] Integration test with PTY output stream

## Acceptance Criteria

- [ ] All four command types can be parsed correctly
- [ ] Sideband commands are stripped from rendered output
- [ ] Malformed commands produce warnings but don't crash
- [ ] Partial commands across buffers are handled correctly
- [ ] Events are emitted for parsed commands
- [ ] All tests passing

## Dependencies

- FEAT-014 - Sideband protocol foundation (if applicable)

## Notes

- XML-style syntax chosen for visibility in terminal output during debugging
- Commands should be designed to be ignored by other terminal emulators
- Consider rate limiting to prevent command flooding
- Canvas command enables future rich content rendering
