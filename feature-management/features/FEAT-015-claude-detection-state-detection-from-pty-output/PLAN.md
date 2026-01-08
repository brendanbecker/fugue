# Implementation Plan: FEAT-015

**Work Item**: [FEAT-015: Claude Detection - State Detection from PTY Output](PROMPT.md)
**Component**: ccmux-server
**Priority**: P1
**Created**: 2026-01-08
**Status**: Not Started

## Overview

Detect Claude Code state from PTY output (thinking, idle, tool use), capture session IDs, and emit state change events.

## Architecture Decisions

### ClaudeState Enum

```rust
pub enum ClaudeState {
    Idle,                    // Waiting for user input
    Thinking,                // Processing, spinner visible
    Coding,                  // Actively writing code
    ToolUse,                 // Executing a tool (Bash, Read, Write, etc.)
    AwaitingConfirmation,    // Waiting for user to approve an action
}
```

### ClaudeDetector Design

```rust
pub struct ClaudeDetector {
    pattern_registry: PatternRegistry,  // From FEAT-014
    current_state: ClaudeState,
    session_info: Option<ClaudeSessionInfo>,
    output_buffer: VecDeque<u8>,
    state_tx: broadcast::Sender<ClaudeStateChange>,
}
```

### State Detection Strategy

1. **Buffer Management**: Accumulate PTY output in a ring buffer
2. **Pattern Matching**: Apply patterns from registry against buffer
3. **State Transition**: Determine new state from matched patterns
4. **Event Emission**: Broadcast state changes to subscribers

### Pattern Matching Approach

- Use regex crate with compiled patterns for performance
- Strip ANSI sequences before matching (or account for them in patterns)
- Support multi-line pattern matching for complex markers
- Debounce rapid state changes (e.g., 100ms threshold)

## Affected Components

| Component | Type of Change | Risk Level |
|-----------|----------------|------------|
| ccmux-server/src/claude/detector.rs | New - State detection engine | High |
| ccmux-server/src/claude/state.rs | New - State types and events | Low |
| ccmux-server/src/claude/mod.rs | New - Module exports | Low |

## Dependencies

- FEAT-014: Provides PatternRegistry with Claude output patterns
- regex crate for pattern matching
- tokio broadcast for event distribution

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Pattern changes in Claude versions | Medium | High | Configurable patterns, version detection |
| False positive state detection | Medium | Medium | Strict pattern matching, validation |
| Performance overhead | Low | Medium | Compiled regex, efficient buffering |
| ANSI sequence interference | Medium | Low | Strip sequences or include in patterns |

## Implementation Phases

### Phase 1: State Types
- Define ClaudeState enum
- Define ClaudeSessionInfo struct
- Define ClaudeStateChange event type
- Add serialization with serde

### Phase 2: Pattern Matching Infrastructure
- Implement output buffer with bounded size
- Implement ANSI stripping utility
- Create pattern matcher wrapper
- Handle multi-line matching

### Phase 3: State Detection Engine
- Implement ClaudeDetector struct
- Implement feed() method for output processing
- Implement state transition logic
- Add debouncing for stability

### Phase 4: Event System
- Set up broadcast channel for state changes
- Implement subscribe() method
- Connect to session management layer

### Phase 5: Integration and Testing
- Wire detector into PTY output pipeline
- Add comprehensive test coverage
- Performance optimization

## Rollback Strategy

If implementation causes issues:
1. Revert commits associated with this work item
2. Remove claude module from ccmux-server
3. Document what went wrong in comments.md

## Testing Strategy

1. **Unit Tests**: Pattern matching, state transitions, buffer management
2. **Mock Tests**: Simulated Claude output sequences
3. **Integration Tests**: Full PTY to state detection pipeline
4. **Regression Tests**: Known Claude output samples

## Implementation Notes

<!-- Add notes during implementation -->

---
*This plan should be updated as implementation progresses.*
