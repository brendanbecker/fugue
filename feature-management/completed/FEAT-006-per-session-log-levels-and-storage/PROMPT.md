# FEAT-006: Per-Session Log Levels and Storage

**Priority**: P2
**Component**: logging
**Type**: new_feature
**Estimated Effort**: medium
**Business Value**: high

## Overview

Per-session logging with configurable levels stored in `.fugue/logs/{session_id}/`. This enables fine-grained control over what gets logged per session without affecting global logging configuration.

## Log Levels

| Level | Captures |
|-------|----------|
| spawns | Session lifecycle only |
| signals | Spawns + completions + errors |
| prompts | Signals + initial prompts |
| full | Complete transcripts |

## Current State

- Logging framework exists (LogOutput enum, LogConfig)
- Server logs to `~/.fugue/logs/fugue.log` (single file)
- Supports filter strings ("info", "debug", "fugue=debug,tokio=warn")
- FUGUE_LOG env var override
- No per-session logging, no rotation, no runtime changes

## Requirements

### Per-Session Log Directories
- Create `.fugue/logs/{session_id}/` for each session
- Session ID should be a unique identifier (UUID or timestamp-based)
- Directory structure supports multiple log files per session

### Configurable Log Level Per Session
- Log level can be set at session spawn time
- Supports: spawns, signals, prompts, full
- Default level configurable globally
- Override via session config or API

### Log Rotation Policy
- Size limits per log file (configurable, e.g., 10MB)
- Retention period (configurable, e.g., 7 days)
- Maximum total log size per session
- Automatic cleanup of old logs

### Structured Log Format
- JSON or structured text format for easy parsing
- Consistent schema across all log entries
- Timestamps in ISO 8601 format
- Session ID, event type, and payload in each entry

### Runtime Log Level Changes
- Change log level without restarting session
- API endpoint or command to modify level
- Changes take effect immediately
- No data loss during transition

### Audit Trail Separation
- Separate files for user actions vs system events
- User actions: input, commands, explicit actions
- System events: spawns, errors, internal state changes
- Cross-reference capability between files

## Affected Files

- `fugue-utils/src/logging.rs` - Core logging infrastructure
- `fugue-server/src/config/schema.rs` - Log level configuration schema
- `fugue-server/src/session/manager.rs` - Per-session log setup

## Implementation Tasks

### Section 1: Design
- [ ] Review current logging implementation in fugue-utils
- [ ] Design per-session log directory structure
- [ ] Define structured log format schema
- [ ] Design log rotation strategy
- [ ] Plan audit trail separation approach

### Section 2: Core Implementation
- [ ] Implement SessionLogLevel enum (spawns, signals, prompts, full)
- [ ] Create per-session log directory management
- [ ] Implement structured log writer
- [ ] Add session-specific log filtering
- [ ] Integrate with session lifecycle

### Section 3: Configuration
- [ ] Add log level to session config schema
- [ ] Implement default log level in global config
- [ ] Add runtime log level change API
- [ ] Support FUGUE_SESSION_LOG env override

### Section 4: Log Management
- [ ] Implement log rotation by size
- [ ] Implement log retention policy
- [ ] Add log cleanup on session termination (configurable)
- [ ] Implement audit trail file separation

### Section 5: Testing
- [ ] Unit tests for log level filtering
- [ ] Integration tests for per-session directories
- [ ] Test log rotation under load
- [ ] Test runtime level changes
- [ ] Test audit trail separation

### Section 6: Documentation
- [ ] Document log level meanings
- [ ] Document directory structure
- [ ] Document configuration options
- [ ] Add troubleshooting guide for logs

## Acceptance Criteria

- [ ] Each session creates its own log directory
- [ ] Log level controls what gets captured per session
- [ ] Logs rotate based on size limits
- [ ] Old logs are cleaned up per retention policy
- [ ] Log level can be changed at runtime
- [ ] User actions and system events are in separate files
- [ ] Structured format enables easy parsing/search
- [ ] No performance degradation at "spawns" level
- [ ] Full transcripts available when "full" level is set

## Dependencies

None - this is a standalone logging enhancement.

## Notes

- Consider using tracing spans for structured logging
- May want to integrate with external log aggregation systems in future
- Performance critical: logging should not block main event loop
- Consider compression for archived logs
