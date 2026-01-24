# Bug Reports

**Project**: fugue
**Last Updated**: 2026-01-19

## Summary Statistics
- Total Bugs: 66
- Open: 0
- Resolved: 65
- Deprecated: 1

## Active Bugs

None! All bugs resolved.

## Priority Queue

Empty - all bugs fixed.

## Recent Activity

- 2026-01-19: Fixed BUG-066 - Mirror pane cross-session output forwarding (commit 5fa9ee7)
- 2026-01-19: Fixed BUG-065 - Parallel MCP request serialization (commit a358bf1) - verified after rebuild
- 2026-01-19: Filed BUG-066 - Mirror panes don't forward output across sessions
- 2026-01-19: Filed BUG-065 - Parallel MCP requests cause response mismatches (discovered during QA)
- 2026-01-19: Fixed BUG-064 - MCP response off-by-one (drain pending after timeout) (commit a6a3563)
- 2026-01-19: Fixed BUG-063 - Mirror pane cross-session (commit 93f5c87)
- 2026-01-19: Fixed BUG-062 - Mirror pane close timeout (commit 3b22ce0)
- 2026-01-19: Fixed BUG-047 - Compiler warnings cleanup (commit 1612e07)
- 2026-01-18: Fixed BUG-064 - MCP response off-by-one after timeout (stale responses in channel)
- 2026-01-18: Fixed BUG-061 - send_orchestration target parsing (commit b298b26)
- 2026-01-18: Fixed BUG-057 - Agent detection cross-contamination (commit 2ebec74)
- 2026-01-18: Fixed BUG-058, BUG-059, BUG-060 - Demo blockers resolved (Session 10)
- 2026-01-18: Fixed BUG-042 - Result nesting flattened in recv_from_daemon_with_timeout
- 2026-01-18: Filed BUG-058, BUG-059, BUG-060 - Issues from multi-agent orchestration demo
- 2026-01-18: Fixed BUG-054 - Add 50ms delay before Enter for TUI compatibility (commit 6abb547, Gemini)
- 2026-01-17: Filed BUG-057 - Agent detection cross-contamination discovered during QA
- 2026-01-17: Fixed BUG-053 - DSR [6n] cursor position handling (commit cb1839c)
- 2026-01-17: Resolved BUG-052 - tested and confirmed working (Gemini connects to fugue MCP)
- 2026-01-17: Archived BUG-051 - fixed direction mapping (commit e3d83f0)
- 2026-01-16: Fixed BUG-050 - cwd inheritance, merged to main, archived
- 2026-01-16: Verified BUG-038 already fixed (commit 99d024e), archived
- 2026-01-16: Fixed BUG-048, BUG-049, FEAT-093 - merged to main, archived

## Resolved Bugs (Recent)

| ID | Description | Resolution | Commit |
|----|-------------|------------|--------|
| BUG-066 | Mirror panes don't forward output across sessions | Fixed - forward output + copy scrollback | 5fa9ee7 |
| BUG-065 | Parallel MCP requests cause response mismatches | Fixed - request_lock mutex serialization | a358bf1 |
| BUG-064 | MCP response off-by-one after timeout | Fixed - drain pending messages after timeout | a6a3563 |
| BUG-063 | Mirror panes can't view other sessions | Fixed - create mirror in caller's attached session | 93f5c87 |
| BUG-062 | fugue_close_pane times out for mirror panes | Fixed - RespondWithBroadcast for mirror close | 3b22ce0 |
| BUG-061 | send_orchestration target parsing fails | Fixed - parse target as JSON string or object | b298b26 |
| BUG-057 | Agent detection cross-contamination | Fixed - reset state between agent checks | 2ebec74 |
| BUG-047 | Compiler warnings across crates | Fixed - addressed 51+ warnings | 1612e07 |
| BUG-042 | Excessive Result Nesting (Ok(Ok(...))) code smell | Fixed - flatten Result in recv_from_daemon_with_timeout | b6b93ff |
| BUG-054 | submit:true doesn't trigger Enter in TUI apps | Fixed - 50ms delay before Enter | 6abb547 |
| BUG-053 | Codex CLI cursor position error (DSR [6n]) | Fixed - handle DSR escape sequence in PTY | cb1839c |
| BUG-052 | Nested agents cannot connect to MCP | Verified working - no longer reproduces | N/A |
| BUG-051 | Split pane direction parameter has no effect | Fixed - direction mapping in handlers | e3d83f0 |
| BUG-050 | pane/session/window cwd inheritance | Fixed - pass cwd through MCP chain | ca1dcc9 |
| BUG-049 | send_input submit: true unreliable | Fixed - PTY write ordering | 4af3599 |
| BUG-048 | TUI flickers during spinner | Fixed - debounce state changes | 39ad9fc |
| BUG-046 | MCP select commands don't control TUI view | Fixed - notify TUI in target session | 1ccf693 |
| BUG-045 | Windows rendered as horizontal splits | Fixed - render only active window panes | fa137ab |
| BUG-044 | MCP bridge hangs, stops reading stdin | Fixed - async stdin | 07474c4 |
| BUG-043 | MCP handlers fail to unwrap Sequenced wrapper | Fixed - unwrap in recv_filtered | d995d55 |
| BUG-041 | Claude Code crashes on paste | Fixed - bracketed paste handling | 936aba7 |
| BUG-037 | close_pane returns AbortError | Fixed - unbounded channel | 4be5a93 |
| BUG-031 | Metadata not persisting | Fixed - log to persistence layer | 2286aab |

## Deprecated Bugs

| ID | Description | Reason |
|----|-------------|--------|
| BUG-012 | Text selection not working in TUI | Shift+click works (by design) |
