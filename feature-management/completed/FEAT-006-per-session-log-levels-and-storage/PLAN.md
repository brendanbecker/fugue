# Implementation Plan: FEAT-006

**Work Item**: [FEAT-006: Per-Session Log Levels and Storage](PROMPT.md)
**Component**: logging
**Priority**: P2
**Created**: 2026-01-08

## Overview

Per-session logging with configurable levels (spawns, signals, prompts, full) stored in `.fugue/logs/{session_id}/`. Includes log rotation, structured format, runtime level changes, and audit trail separation.

## Architecture Decisions

### Log Level Hierarchy

The four log levels form a hierarchy where each level includes everything from the previous:

```
spawns < signals < prompts < full

spawns:  [session_start, session_end]
signals: spawns + [completion, error, signal_received]
prompts: signals + [initial_prompt, prompt_change]
full:    prompts + [all_output, input, state_changes]
```

### Directory Structure

```
~/.fugue/logs/
  {session_id}/
    session.log      # Main session log (level-filtered)
    audit_user.log   # User actions only
    audit_system.log # System events only
    metadata.json    # Session metadata and log config
```

### Structured Log Format

JSON Lines format for easy parsing:

```json
{"ts":"2026-01-08T12:34:56.789Z","level":"info","session":"abc123","event":"session_start","data":{...}}
{"ts":"2026-01-08T12:34:57.123Z","level":"debug","session":"abc123","event":"output","data":{"bytes":1024}}
```

### Integration Points

1. **Session Manager**: Creates log directory on session spawn, sets initial level
2. **SessionLogger**: New component handling per-session logging
3. **Config Schema**: Extends with per-session log level options
4. **Runtime API**: Endpoint to change log level mid-session

## Affected Components

| Component | Type of Change | Risk Level |
|-----------|----------------|------------|
| fugue-utils/src/logging.rs | Major - new SessionLogger | Medium |
| fugue-server/src/config/schema.rs | Minor - add log level config | Low |
| fugue-server/src/session/manager.rs | Medium - integrate SessionLogger | Medium |
| fugue-server/src/session/session.rs | Minor - log level accessors | Low |

## Dependencies

No external dependencies. Uses existing tracing infrastructure.

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Performance degradation at "full" level | Medium | Medium | Async writes, buffering, sampling option |
| Disk space exhaustion | Low | High | Rotation limits, monitoring, alerts |
| Log corruption on crash | Low | Medium | Atomic writes, fsync on important events |
| Complexity in audit trail separation | Medium | Low | Clear event categorization rules |

## Implementation Phases

### Phase 1: Core Infrastructure
- SessionLogLevel enum
- Per-session directory creation
- Basic structured log writer

### Phase 2: Integration
- Session manager integration
- Config schema updates
- Level filtering logic

### Phase 3: Management Features
- Log rotation
- Retention policy
- Audit trail separation

### Phase 4: Runtime Control
- Level change API
- Hot-reload support

## Rollback Strategy

If implementation causes issues:
1. Revert commits associated with this work item
2. Remove per-session log directories (data loss acceptable for logs)
3. Restore single-file logging behavior
4. Document what went wrong in comments.md

## Testing Strategy

1. **Unit Tests**: Log level filtering, format validation
2. **Integration Tests**: Directory creation, rotation triggers
3. **Load Tests**: Performance at "full" level with high throughput
4. **Recovery Tests**: Behavior after crash mid-write

## Implementation Notes

<!-- Add notes during implementation -->

---
*This plan should be updated as implementation progresses.*
