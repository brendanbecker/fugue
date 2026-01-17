# Bug Reports

**Project**: ccmux
**Last Updated**: 2026-01-17

## Summary Statistics
- Total Bugs: 52
- Open: 4
- Resolved: 47
- Deprecated: 1

## Active Bugs

| ID | Description | Priority | Status | Component |
|----|-------------|----------|--------|-----------|
| BUG-052 | Agents inside ccmux panes cannot connect to ccmux MCP server | P1 | new | mcp-bridge |
| BUG-051 | Split pane direction parameter has no effect - always creates horizontal panes | P1 | new | mcp-handlers |
| BUG-047 | Clean up compiler warnings across ccmux crates | P3 | partial | build |
| BUG-042 | Excessive Result Nesting (Ok(Ok(...))) code smell | P3 | new | mcp-bridge |

## Priority Queue

| Priority | Bug | Risk | Effort | Notes |
|----------|-----|------|--------|-------|
| **P1** | BUG-052 | High | High | Blocks multi-agent orchestration use case |
| **P1** | BUG-051 | Medium | Medium | Core feature broken - direction parameter ignored |
| **P3** | BUG-047 | Low | Low | Code quality cleanup |
| **P3** | BUG-042 | Low | Medium | Refactor Result nesting |

## Recent Activity

- 2026-01-17: Created BUG-052 - nested agents cannot connect to ccmux MCP server
- 2026-01-17: Created BUG-051 - split pane direction parameter has no effect
- 2026-01-16: Fixed BUG-050 - cwd inheritance, merged to main, archived
- 2026-01-16: Verified BUG-038 already fixed (commit 99d024e), archived
- 2026-01-16: Fixed BUG-048, BUG-049, FEAT-093 - merged to main, archived
- 2026-01-16: Created BUG-050 - pane/session/window cwd inheritance
- 2026-01-16: Archived BUG-043, BUG-044, BUG-045, BUG-046 (all fixed)

## Resolved Bugs (Recent)

| ID | Description | Resolution | Commit |
|----|-------------|------------|--------|
| BUG-050 | pane/session/window cwd inheritance | Fixed - pass cwd through MCP chain | ca1dcc9 |
| BUG-038 | create_pane returns wrong response type | Fixed - SessionsChanged broadcast type | 99d024e |
| BUG-049 | send_input submit: true unreliable | Fixed - PTY write ordering | 937339a |
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
