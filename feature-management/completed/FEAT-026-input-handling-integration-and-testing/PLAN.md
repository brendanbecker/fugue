# Implementation Plan: FEAT-026

**Work Item**: [FEAT-026: Input Handling Integration and Testing](PROMPT.md)
**Component**: fugue-client
**Priority**: P1
**Created**: 2026-01-09

## Overview

Verify and test keyboard input forwarding from client to server PTY. This is primarily a verification and integration testing task since the input handling code already exists from FEAT-010.

## Architecture Decisions

### Existing Implementation Review

The input handling stack is already in place:

```
User Input (crossterm events)
       |
       v
keys.rs - Parse KeyEvent into internal representation
       |
       v
input_handler.rs - Convert to byte sequences
       |
       v
ClientMessage::Input - Send over Unix socket
       |
       v
Server - Route to correct pane's PTY
       |
       v
PTY - Process input, generate output
```

### Testing Strategy

1. **Unit Tests**: Verify each component in isolation
2. **Integration Tests**: Verify end-to-end flow
3. **Manual Tests**: Verify user experience

### Key Areas to Verify

| Area | File | What to Check |
|------|------|---------------|
| Key Parsing | `input/keys.rs` | All key types parsed correctly |
| Byte Conversion | `input_handler.rs` | Correct escape sequences generated |
| Message Sending | `main.rs` | ClientMessage::Input sent correctly |
| Server Handling | Server side | Input routed to correct pane |

## Affected Components

| Component | Type of Change | Risk Level |
|-----------|----------------|------------|
| fugue-client/src/input/keys.rs | Verify/Test | Low |
| fugue-client/src/input/input_handler.rs | Verify/Test | Low |
| fugue-client/src/input/mod.rs | May add tests | Low |
| fugue-client/tests/ | Add integration tests | Low |

## Dependencies

- **FEAT-022** (Message Routing): Server must correctly route Input messages to panes
- **FEAT-023** (PTY Output): Need to see output to verify input was processed

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Missing key combinations | Medium | Medium | Comprehensive test coverage |
| Incorrect escape sequences | Low | High | Compare against standard terminal behavior |
| Input lag | Low | Medium | Performance testing |
| Platform differences | Medium | Medium | Test on Linux, macOS |

## Implementation Phases

### Phase 1: Code Review (30 min)
- Review existing `keys.rs` implementation
- Review existing `input_handler.rs` implementation
- Identify any obvious gaps or issues
- Document findings

### Phase 2: Unit Tests (30 min)
- Add unit tests for key parsing
- Add unit tests for byte conversion
- Focus on edge cases (control chars, function keys)

### Phase 3: Integration Tests (30 min)
- Test client-to-server message flow
- Test server routing to PTY
- Verify round-trip with PTY output

### Phase 4: Manual Verification (30 min)
- Interactive testing with shell
- Test in vim/nano for key combinations
- Test Ctrl-C, Ctrl-D, Ctrl-Z
- Verify no input lag

## Test Cases

### Key Parsing Tests

| Key | Expected Parse |
|-----|----------------|
| 'a' | Char('a') |
| Ctrl-C | Ctrl + Char('c') |
| Alt-x | Alt + Char('x') |
| F1 | F(1) |
| Up Arrow | Up |
| Home | Home |

### Byte Conversion Tests

| Key | Expected Bytes |
|-----|----------------|
| 'a' | [0x61] |
| Enter | [0x0d] or [0x0a] |
| Ctrl-C | [0x03] |
| Ctrl-D | [0x04] |
| Escape | [0x1b] |
| Up Arrow | [0x1b, 0x5b, 0x41] |
| F1 | [0x1b, 0x4f, 0x50] or [0x1b, 0x5b, 0x31, 0x31, 0x7e] |

## Rollback Strategy

If issues are discovered:
1. Document findings as bugs
2. Revert any test-related changes if they cause issues
3. Original input handling code remains unchanged unless bugs found

## Implementation Notes

<!-- Add notes during implementation -->

---
*This plan should be updated as implementation progresses.*
