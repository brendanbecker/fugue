# FEAT-015: Claude Detection - State Detection from PTY Output

**Priority**: P1
**Component**: ccmux-server
**Type**: new_feature
**Estimated Effort**: large
**Business Value**: high
**Status**: new

## Overview

Detect Claude Code state from PTY output (thinking, idle, tool use), capture session IDs, and emit state change events.

## Requirements

- Pattern matching for Claude Code output markers
- State detection: Idle, Thinking, Coding, ToolUse, AwaitingConfirmation
- Session ID extraction from Claude output
- Token usage tracking
- Model identification
- State change event emission
- ClaudeState struct population

## Affected Files

- `ccmux-server/src/claude/detector.rs`
- `ccmux-server/src/claude/state.rs`
- `ccmux-server/src/claude/mod.rs`

## Implementation Tasks

### Section 1: Design
- [ ] Analyze Claude Code output patterns for state markers
- [ ] Design ClaudeState struct with all state variants
- [ ] Design ClaudeDetector interface
- [ ] Plan event emission architecture
- [ ] Document pattern matching strategy

### Section 2: State Types
- [ ] Implement ClaudeState enum (Idle, Thinking, Coding, ToolUse, AwaitingConfirmation)
- [ ] Implement ClaudeSessionInfo struct (session ID, model, token usage)
- [ ] Implement ClaudeStateChange event type
- [ ] Add serialization support for state types

### Section 3: Pattern Matching
- [ ] Implement output buffer for pattern accumulation
- [ ] Implement regex patterns for state detection
- [ ] Implement session ID extraction pattern
- [ ] Implement token usage parsing
- [ ] Implement model identification parsing
- [ ] Handle ANSI escape sequences in output

### Section 4: State Detection Engine
- [ ] Implement ClaudeDetector struct
- [ ] Implement state transition logic
- [ ] Implement event emission on state change
- [ ] Handle edge cases (rapid state changes, partial output)
- [ ] Implement debouncing for state transitions

### Section 5: Integration
- [ ] Integrate detector with PTY output stream
- [ ] Connect state events to session management
- [ ] Add logging for state transitions
- [ ] Handle detector errors gracefully

### Section 6: Testing
- [ ] Unit tests for pattern matching
- [ ] Unit tests for state transitions
- [ ] Integration tests with mock PTY output
- [ ] Test edge cases (malformed output, rapid changes)
- [ ] Performance tests for pattern matching

## Acceptance Criteria

- [ ] All Claude Code states correctly detected
- [ ] Session ID extracted when available
- [ ] Token usage tracked accurately
- [ ] Model identified from output
- [ ] State change events emitted reliably
- [ ] Pattern matching handles ANSI sequences
- [ ] No false positives in state detection
- [ ] All tests passing

## Dependencies

- FEAT-014 (Claude Detection - Output Pattern Registry)

## Notes

- Claude Code output patterns may vary between versions
- Pattern registry (FEAT-014) provides pattern definitions
- State detection should be efficient for real-time processing
- Consider buffering strategy for multi-line patterns
