# FEAT-015: Claude Detection - State Detection from PTY Output

**Priority**: P1
**Component**: fugue-server
**Type**: new_feature
**Estimated Effort**: large
**Business Value**: high
**Status**: complete

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

- `fugue-server/src/claude/detector.rs`
- `fugue-server/src/claude/state.rs`
- `fugue-server/src/claude/mod.rs`

## Implementation Tasks

### Section 1: Design
- [x] Analyze Claude Code output patterns for state markers
- [x] Design ClaudeState struct with all state variants
- [x] Design ClaudeDetector interface
- [x] Plan event emission architecture
- [x] Document pattern matching strategy

### Section 2: State Types
- [x] Implement ClaudeState enum (Idle, Thinking, Coding, ToolUse, AwaitingConfirmation)
- [x] Implement ClaudeSessionInfo struct (session ID, model, token usage)
- [x] Implement ClaudeStateChange event type
- [x] Add serialization support for state types

### Section 3: Pattern Matching
- [x] Implement output buffer for pattern accumulation
- [x] Implement regex patterns for state detection
- [x] Implement session ID extraction pattern
- [x] Implement token usage parsing (not available in PTY output)
- [x] Implement model identification parsing
- [x] Handle ANSI escape sequences in output

### Section 4: State Detection Engine
- [x] Implement ClaudeDetector struct
- [x] Implement state transition logic
- [x] Implement event emission on state change
- [x] Handle edge cases (rapid state changes, partial output)
- [x] Implement debouncing for state transitions

### Section 5: Integration
- [x] Integrate detector with PTY output stream
- [x] Connect state events to session management
- [x] Add logging for state transitions
- [x] Handle detector errors gracefully

### Section 6: Testing
- [x] Unit tests for pattern matching
- [x] Unit tests for state transitions
- [x] Integration tests with mock PTY output
- [x] Test edge cases (malformed output, rapid changes)
- [x] Performance tests for pattern matching

## Acceptance Criteria

- [x] All Claude Code states correctly detected
- [x] Session ID extracted when available
- [x] Token usage tracked accurately (N/A - not visible in PTY output)
- [x] Model identified from output
- [x] State change events emitted reliably
- [x] Pattern matching handles ANSI sequences
- [x] No false positives in state detection
- [x] All tests passing (66 claude-specific tests, 782 total)

## Dependencies

- FEAT-014 (Claude Detection - Output Pattern Registry)

## Notes

- Claude Code output patterns may vary between versions
- Pattern registry (FEAT-014) provides pattern definitions
- State detection should be efficient for real-time processing
- Consider buffering strategy for multi-line patterns
