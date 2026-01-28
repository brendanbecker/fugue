# Bug Reports

**Project**: fugue
**Last Updated**: 2026-01-28

## Summary Statistics
- Total Bugs: 74
- Open: 2
- Resolved: 71
- Deprecated: 1

## Active Bugs

| ID | Description | Priority | Severity | Status |
|----|-------------|----------|----------|--------|
| BUG-067 | Mirror pane splits in wrong direction | P3 | Low | new |
| BUG-068 | fugue_focus_pane returns AbortError | P2 | Medium | new |

## Priority Queue

| Priority | ID | Description |
|----------|----|-------------|
| P2 | BUG-068 | fugue_focus_pane returns AbortError |
| P3 | BUG-067 | Mirror pane splits in wrong direction |

## Recent Activity

- 2026-01-28: Fixed BUG-073 - get_tags now requires explicit session param (commit 975952a)
- 2026-01-28: Fixed BUG-074 - verified create_session returns pane_id (commit f6ec2ba)
- 2026-01-28: Fixed BUG-069 - Orchestration delivery mismatch (made worker_id optional) (commit bae0bb8)
- 2026-01-28: Fixed BUG-071 - Watchdog timer submit (increased delay to 200ms) (commit 5a4b989)
- 2026-01-28: Filed BUG-074 - fugue_create_session should return pane ID
- 2026-01-24: Filed BUG-073 - fugue_get_tags returns wrong session's tags
- 2026-01-24: Fixed BUG-072 - kill_session hang regression (unbounded channel) (commit 868535a)
- 2026-01-19: Fixed BUG-066 - Mirror pane cross-session output forwarding (commit 5fa9ee7)
- 2026-01-19: Fixed BUG-065 - Parallel MCP request serialization (commit a358bf1)
- 2026-01-19: Fixed BUG-064 - MCP response off-by-one (commit a6a3563)
- 2026-01-19: Fixed BUG-063 - Mirror pane cross-session (commit 36bd961)
- 2026-01-19: Fixed BUG-062 - Mirror pane close timeout (commit 798dbdc)
- 2026-01-19: Fixed BUG-061 - send_orchestration target parsing (commit b298b26)
- 2026-01-19: Fixed BUG-047 - Compiler warnings cleanup (commit bbd5276)
- 2026-01-18: Fixed BUG-057 - Agent detection cross-contamination (commit 2ebec74)
- 2026-01-18: Fixed BUG-059 - Mirror pane AbortError (commit 578ace5)
- 2026-01-18: Fixed BUG-042 - Result nesting flattened (commit b6b93ff)
- 2026-01-18: Fixed BUG-054 - submit:true Enter delay (commit b0ca2d5)
- 2026-01-17: Fixed BUG-053 - Codex CLI cursor position DSR handling (commit 37d3e33)

## Resolved Bugs (Recent)

| ID | Description | Resolution | Commit |
|----|-------------|------------|--------|
| BUG-074 | create_session pane_id verified | Fixed - already returns pane_id properly | f6ec2ba |
| BUG-073 | get_tags wrong session | Fixed - require explicit session param | 975952a |
| BUG-072 | kill_session hang regression | Fixed - unbounded incoming channel | 868535a |
| BUG-071 | Watchdog timer submit not working | Fixed - 200ms delay for TUI | 5a4b989 |
| BUG-070 | Session switch rendering corruption | Fixed | 6f447c9 |
| BUG-069 | Orchestration messages not delivered | Fixed - optional worker_id in poll | bae0bb8 |
| BUG-066 | Mirror panes don't forward output | Fixed - forward output + scrollback | 5fa9ee7 |
| BUG-065 | Parallel MCP response mismatches | Fixed - request_lock serialization | a358bf1 |
| BUG-064 | MCP response off-by-one | Fixed - drain pending after timeout | a6a3563 |
| BUG-063 | Mirror panes can't view other sessions | Fixed - create in attached session | 36bd961 |
| BUG-062 | close_pane timeout for mirrors | Fixed - don't filter PaneClosed | 798dbdc |
| BUG-061 | send_orchestration target parsing | Fixed - parse JSON string/object | b298b26 |
| BUG-060 | Orchestration tools need attachment | Fixed | completed |
| BUG-059 | Mirror pane AbortError | Fixed | 578ace5 |
| BUG-058 | kill_session client hang | Fixed | completed |
| BUG-057 | Agent detection cross-contamination | Fixed - reset between checks | 2ebec74 |
| BUG-054 | submit:true not triggering Enter | Fixed - 200ms delay all paths | b0ca2d5 |
| BUG-053 | Codex CLI cursor position error | Fixed - DSR [6n] handling | 37d3e33 |
| BUG-047 | Compiler warnings | Fixed - 51+ warnings addressed | bbd5276 |
| BUG-042 | Excessive Result nesting | Fixed - flatten in recv | b6b93ff |

## Deprecated Bugs

| ID | Description | Reason |
|----|-------------|--------|
| BUG-012 | Text selection not working in TUI | Shift+click works (by design) |
