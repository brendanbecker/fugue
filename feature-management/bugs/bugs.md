# Bug Reports

**Project**: ccmux
**Last Updated**: 2026-01-10

## Summary Statistics
- Total Bugs: 21
- Open: 0
- Resolved: 20
- Deprecated: 1

## Bug Status

All bugs have been resolved or deprecated. See `feature-management/completed/` for detailed work items.

### Resolved Bugs

| ID | Description | Priority | Resolution |
|----|-------------|----------|------------|
| BUG-001 | Client input not captured | P0 | Fixed |
| BUG-002 | Flaky test (shared temp dir) | P2 | Fixed - used tempfile::TempDir |
| BUG-003 | Session missing default pane | P0 | Fixed |
| BUG-004 | Zombie panes hang client on reattach | P1 | Fixed - auto-cleanup on PTY exit |
| BUG-005 | Sideband parsing not integrated | P0 | Fixed - integrated into PTY output flow |
| BUG-006 | Viewport not sizing to terminal | P1 | Fixed - client uses terminal size on attach |
| BUG-007 | Shift+Tab not passed through | P1 | Fixed - added BackTab handler |
| BUG-008 | Pane/window creation no PTY | P0 | Fixed - spawn PTY on creation |
| BUG-009 | Flaky persistence tests | P2 | Fixed - test isolation |
| BUG-010 | MCP pane broadcast not received by TUI | P1 | Fixed |
| BUG-011 | Large paste crashes session | P2 | Fixed - graceful chunking |
| BUG-013 | Mouse scroll wheel not working | P2 | Fixed |
| BUG-014 | Large output buffer overflow | P2 | Fixed (stream-c) |
| BUG-015 | Layout not recalculated on pane close | P2 | Fixed (stream-b) |
| BUG-016 | PTY output not routed to pane state | P1 | Fixed - enables Claude detection + MCP read_pane |
| BUG-017 | MCP send_input doesn't handle Enter key | P1 | Fixed |
| BUG-018 | TUI pane interaction failure | P1 | Fixed (stream-a) |
| BUG-019 | Claude detector UTF-8 panic causes TUI hang | P1 | Fixed - char boundary check |
| BUG-020 | Session reattach creates client without PTY | P1 | Fixed (stream-a) - send scrollback on reattach |
| BUG-021 | ccmux_rename_session missing from standalone MCP | P1 | Fixed |

### Deprecated Bugs

| ID | Description | Reason |
|----|-------------|--------|
| BUG-012 | Text selection not working in TUI | Shift+click works (by design) |

## Recent Activity

| Date | Bug ID | Action | Description |
|------|--------|--------|-------------|
| 2026-01-10 | BUG-021 | Resolved | ccmux_rename_session added to standalone MCP server |
| 2026-01-10 | BUG-020 | Resolved | Fixed in commit 8f53895 - send scrollback on reattach |
| 2026-01-10 | BUG-019 | Resolved | UTF-8 char boundary fix in ClaudeDetector |
| 2026-01-10 | BUG-018 | Resolved | Session reattach fixed (stream-a) |
| 2026-01-10 | BUG-017 | Resolved | MCP send_input Enter key handling |
| 2026-01-10 | BUG-016 | Resolved | PTY output routing to pane state |
| 2026-01-10 | BUG-015 | Resolved | Layout recalculation on pane close (stream-b) |
| 2026-01-10 | BUG-014 | Resolved | Buffer overflow handling (stream-c) |
| 2026-01-10 | BUG-013 | Resolved | Mouse scroll wheel fixed |
| 2026-01-10 | BUG-012 | Deprecated | Shift+click works for native selection |
| 2026-01-10 | BUG-011 | Resolved | Large paste graceful handling |
| 2026-01-10 | BUG-010 | Resolved | MCP broadcast fixed |
| 2026-01-10 | BUG-009 | Resolved | Persistence test isolation |
| 2026-01-09 | BUG-008 | Resolved | PTY spawn on pane/window creation |
| 2026-01-09 | BUG-007 | Resolved | BackTab keycode handler |
| 2026-01-09 | BUG-006 | Resolved | Viewport terminal sizing |
| 2026-01-09 | BUG-005 | Resolved | Sideband parsing integration |
| 2026-01-09 | BUG-004 | Resolved | Zombie pane cleanup |
| 2026-01-09 | BUG-003 | Resolved | Default pane on session creation |
| 2026-01-09 | BUG-002 | Resolved | Flaky test tempdir fix |
| 2026-01-09 | BUG-001 | Resolved | Client input capture |
