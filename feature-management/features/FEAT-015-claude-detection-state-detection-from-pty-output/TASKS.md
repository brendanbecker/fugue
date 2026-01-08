# Task Breakdown: FEAT-015

**Work Item**: [FEAT-015: Claude Detection - State Detection from PTY Output](PROMPT.md)
**Status**: Not Started
**Last Updated**: 2026-01-08

## Prerequisites

- [ ] Read and understand PROMPT.md
- [ ] Review PLAN.md and update if needed
- [ ] Verify FEAT-014 (Pattern Registry) is complete
- [ ] Study Claude Code output patterns
- [ ] Review regex crate documentation

## Design Tasks

- [ ] Analyze Claude Code terminal output for state markers
- [ ] Document state transition diagram
- [ ] Design ClaudeState enum with all variants
- [ ] Design ClaudeSessionInfo struct
- [ ] Design ClaudeDetector API
- [ ] Plan event emission architecture
- [ ] Update PLAN.md with final design

## Implementation Tasks

### State Types (state.rs)
- [ ] Create ClaudeState enum (Idle, Thinking, Coding, ToolUse, AwaitingConfirmation)
- [ ] Create ClaudeSessionInfo struct (session_id, model, token_usage)
- [ ] Create ClaudeStateChange event struct
- [ ] Implement Display for ClaudeState
- [ ] Add serde Serialize/Deserialize derives
- [ ] Add Default impl for ClaudeState (Idle)

### Pattern Matching Infrastructure
- [ ] Implement bounded output buffer (VecDeque)
- [ ] Implement ANSI escape sequence stripping
- [ ] Create PatternMatcher wrapper for regex
- [ ] Implement multi-line pattern support
- [ ] Add pattern match result types

### State Detection Engine (detector.rs)
- [ ] Create ClaudeDetector struct
- [ ] Implement new() constructor with PatternRegistry
- [ ] Implement feed(bytes) method
- [ ] Implement state detection logic
- [ ] Implement debouncing (configurable threshold)
- [ ] Implement state transition validation
- [ ] Handle partial pattern matches

### Session Info Extraction
- [ ] Implement session ID extraction
- [ ] Implement model identification
- [ ] Implement token usage parsing
- [ ] Handle missing/incomplete info gracefully

### Event System
- [ ] Set up tokio::sync::broadcast channel
- [ ] Implement subscribe() -> Receiver
- [ ] Emit ClaudeStateChange on transitions
- [ ] Include previous and new state in events
- [ ] Add timestamp to events

### Module Organization (mod.rs)
- [ ] Create claude module
- [ ] Export public types
- [ ] Re-export PatternRegistry from FEAT-014

## Testing Tasks

- [ ] Unit test: ClaudeState serialization
- [ ] Unit test: ANSI stripping
- [ ] Unit test: Pattern matching for each state
- [ ] Unit test: State transitions
- [ ] Unit test: Debouncing behavior
- [ ] Unit test: Session ID extraction
- [ ] Unit test: Token usage parsing
- [ ] Integration test: Full output processing
- [ ] Integration test: Event emission
- [ ] Test: Rapid state changes
- [ ] Test: Malformed output handling
- [ ] Test: Buffer overflow handling

## Documentation Tasks

- [ ] Document ClaudeState variants
- [ ] Document ClaudeDetector API
- [ ] Document event subscription pattern
- [ ] Add usage examples
- [ ] Document pattern matching behavior

## Verification Tasks

- [ ] All acceptance criteria from PROMPT.md met
- [ ] Tests passing
- [ ] Update feature_request.json status
- [ ] Document completion in PLAN.md

## Completion Checklist

- [ ] All implementation tasks complete
- [ ] All tests passing
- [ ] Documentation updated
- [ ] PLAN.md reflects final implementation
- [ ] Ready for review/merge

---
*Check off tasks as you complete them. Update status field above.*
