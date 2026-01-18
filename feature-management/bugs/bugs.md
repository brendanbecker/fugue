# Bug Reports

**Project**: ccmux
**Last Updated**: 2026-01-17

## Summary Statistics
- Total Bugs: 56
- Open: 2
- Resolved: 53
- Deprecated: 1

## Active Bugs

| ID | Description | Priority | Status | Component |
|----|-------------|----------|--------|-----------|
| BUG-047 | Clean up compiler warnings across ccmux crates | P3 | partial | build |
| BUG-042 | Excessive Result Nesting (Ok(Ok(...))) code smell | P3 | new | mcp-bridge |

## Priority Queue

| Priority | Bug | Risk | Effort | Notes |
|----------|-----|------|--------|-------|
| **P3** | BUG-047 | Low | Low | Code quality cleanup |
| **P3** | BUG-042 | Low | Medium | Refactor Result nesting |

## Recent Activity

- 2026-01-17: Fixed BUG-053 - DSR [6n] cursor position handling (commit cb1839c)
- 2026-01-17: Fixed BUG-054 - TUI Enter handling fix (commit 3ce77dc)
- 2026-01-17: Resolved BUG-052 - tested and confirmed working (Gemini connects to ccmux MCP)
- 2026-01-17: Archived BUG-051 - fixed direction mapping (commit e3d83f0)
- 2026-01-16: Fixed BUG-050 - cwd inheritance, merged to main, archived
- 2026-01-16: Verified BUG-038 already fixed (commit 99d024e), archived
- 2026-01-16: Fixed BUG-048, BUG-049, FEAT-093 - merged to main, archived

## Resolved Bugs (Recent)

| ID | Description | Resolution | Commit |
|----|-------------|------------|--------|
| BUG-054 | send_input submit:true doesn't trigger Enter in TUI | Fixed - send input and enter separately | 3ce77dc |
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
