# Implementation Plan: FEAT-049

**Work Item**: [FEAT-049: Add tmux-compatible CLI wrapper (fugue-compat)](PROMPT.md)
**Component**: fugue-compat
**Priority**: P2
**Created**: 2026-01-10

## Overview

Create a CLI binary that accepts tmux command syntax and translates to fugue MCP calls, enabling drop-in replacement for existing tools like Gas Town.

## Architecture Decisions

### Approach: New Binary Crate

Create a new crate `fugue-compat/` in the workspace that:
1. Uses clap for tmux-compatible argument parsing
2. Connects to fugue daemon via existing Unix socket protocol
3. Translates tmux commands to MCP tool calls
4. Formats output to match tmux exactly

### Trade-offs

| Decision | Pros | Cons |
|----------|------|------|
| New binary crate | Clean separation, can be installed independently | Additional build artifact |
| Reuse fugue-protocol | Consistent with existing codebase | Tight coupling to internal protocol |
| clap for CLI parsing | Robust, well-tested | Additional dependency (though already in workspace) |

### Alternative Considered: Shell Script Wrapper

A shell script translating args to `fugue mcp` calls was considered but rejected because:
- Harder to maintain
- Less portable
- More difficult to match exact tmux exit codes and output format

## Affected Components

| Component | Type of Change | Risk Level |
|-----------|----------------|------------|
| fugue-compat/ (new) | Primary - new crate | Low |
| Cargo.toml (workspace) | Add new member | Low |
| fugue-protocol | Reuse existing | None |
| fugue-utils | Reuse existing | None |

## Command Mapping Details

### new-session

```
tmux: new-session -d -s NAME [-c DIR] [CMD]
fugue: fugue_create_session(name=NAME, working_dir=DIR)
       + fugue_send_input(session=NAME, input=CMD) if CMD provided
```

### send-keys

```
tmux: send-keys -t TARGET [-l] TEXT [TEXT...]
fugue: fugue_send_input(target=TARGET, input=TEXT, literal=true if -l)
```

Note: tmux `send-keys` without `-l` interprets special keys like `Enter`, `C-c`, etc.

### kill-session

```
tmux: kill-session -t NAME
fugue: fugue_kill_session(name=NAME)
```

### has-session

```
tmux: has-session -t =NAME
fugue: fugue_list_sessions() | filter by exact name match
       exit 0 if found, exit 1 if not
```

### list-sessions

```
tmux: list-sessions [-F FORMAT]
fugue: fugue_list_sessions() | format according to FORMAT string
```

Default format: `#{session_name}: #{session_windows} windows (created #{session_created})`

### capture-pane

```
tmux: capture-pane -p -t TARGET [-S start] [-E end]
fugue: fugue_read_pane(target=TARGET, start_line=start, end_line=end)
```

## Target Syntax Support

### Session Targeting (-t)

tmux target syntax: `session:window.pane`
- `session` - session name
- `session:window` - window in session
- `session:window.pane` - pane in window
- `=session` - exact match (for has-session)

### Format Strings (-F)

Common format variables to support:
- `#{session_name}` - session name
- `#{session_windows}` - window count
- `#{session_created}` - creation timestamp
- `#{session_attached}` - attached client count

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| tmux flag incompatibility | Medium | Medium | Start with Gas Town subset, expand as needed |
| Output format mismatch | Medium | High | Extensive testing against real tmux output |
| Exit code differences | Low | Medium | Document and match tmux exit codes |
| Socket connection issues | Low | Low | Reuse proven fugue-client connection code |

## Testing Strategy

1. **Unit Tests**: Command parsing, format string handling
2. **Integration Tests**: Compare fugue-compat vs tmux output for each command
3. **End-to-End**: Run Gas Town's tmux wrapper with fugue-compat binary

## Rollback Strategy

If implementation causes issues:
1. The crate is independent - simply don't deploy it
2. Users can continue using tmux directly
3. No impact on core fugue functionality

## Implementation Notes

### Phase 1 Priority

Focus on commands actually used by Gas Town (from internal/tmux/tmux.go):
1. `new-session` - session creation
2. `send-keys` - sending commands
3. `kill-session` - cleanup
4. `has-session` - existence check
5. `capture-pane` - output capture

### Deferred to Phase 2

- `split-window` / `split-pane`
- `select-window` / `select-pane`
- `resize-pane`
- Complex format strings
- Configuration file support

---
*This plan should be updated as implementation progresses.*
