# Task Breakdown: FEAT-006

**Work Item**: [FEAT-006: Per-Session Log Levels and Storage](PROMPT.md)
**Status**: Not Started
**Last Updated**: 2026-01-08

## Prerequisites

- [ ] Read and understand PROMPT.md
- [ ] Review PLAN.md and update if needed
- [ ] Review current logging in ccmux-utils/src/logging.rs
- [ ] Review session management in ccmux-server/src/session/

## Design Tasks

- [ ] Define SessionLogLevel enum variants (spawns, signals, prompts, full)
- [ ] Design structured log entry schema (JSON Lines)
- [ ] Design per-session directory structure
- [ ] Define event categorization for audit trails
- [ ] Design log rotation triggers and limits
- [ ] Document configuration schema additions

## Implementation Tasks

### Core Logging Infrastructure
- [ ] Create SessionLogLevel enum in ccmux-utils
- [ ] Implement level hierarchy (spawns < signals < prompts < full)
- [ ] Create StructuredLogEntry struct
- [ ] Implement JSON Lines log writer
- [ ] Add timestamp formatting (ISO 8601)

### Per-Session Directory Management
- [ ] Implement session log directory creation
- [ ] Create metadata.json writer
- [ ] Implement directory cleanup on session end (configurable)
- [ ] Add session ID to all log entries

### Session Logger Component
- [ ] Create SessionLogger struct
- [ ] Implement level-filtered logging methods
- [ ] Add async write support (non-blocking)
- [ ] Implement log buffering for performance

### Configuration
- [ ] Add session_log_level to config schema
- [ ] Add default_session_log_level global setting
- [ ] Add log_rotation config (max_size, retention_days)
- [ ] Support CCMUX_SESSION_LOG env override

### Session Manager Integration
- [ ] Create SessionLogger on session spawn
- [ ] Pass log level from config/spawn params
- [ ] Log session lifecycle events
- [ ] Clean up logger on session termination

### Log Rotation
- [ ] Implement size-based rotation
- [ ] Implement retention-based cleanup
- [ ] Add rotation metadata tracking
- [ ] Test rotation under continuous write

### Audit Trail Separation
- [ ] Define user action events list
- [ ] Define system event list
- [ ] Implement dual-file writing
- [ ] Add cross-reference session ID

### Runtime Level Changes
- [ ] Add set_log_level method to SessionLogger
- [ ] Expose via session API
- [ ] Ensure atomic level switch
- [ ] Log level change events

## Testing Tasks

- [ ] Unit test: SessionLogLevel ordering
- [ ] Unit test: Log entry serialization
- [ ] Unit test: Level filtering logic
- [ ] Integration test: Directory creation
- [ ] Integration test: Log rotation
- [ ] Integration test: Runtime level change
- [ ] Load test: "full" level performance
- [ ] Test: Audit trail separation

## Documentation Tasks

- [ ] Document log levels in user docs
- [ ] Document directory structure
- [ ] Document configuration options
- [ ] Add examples for log parsing
- [ ] Update CHANGELOG

## Verification Tasks

- [ ] All acceptance criteria from PROMPT.md met
- [ ] Tests passing
- [ ] No performance regression at "spawns" level
- [ ] Update feature_request.json status
- [ ] Document completion in comments.md

## Completion Checklist

- [ ] All implementation tasks complete
- [ ] All tests passing
- [ ] Documentation updated
- [ ] PLAN.md reflects final implementation
- [ ] Ready for review/merge

---
*Check off tasks as you complete them. Update status field above.*
