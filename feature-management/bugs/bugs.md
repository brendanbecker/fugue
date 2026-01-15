# Bug Reports

**Project**: ccmux
**Last Updated**: 2026-01-14

## Summary Statistics
- Total Bugs: 41
- Open: 6
- Resolved: 33
- Deprecated: 1

## CRITICAL: Approach for Complex Bugs

**ULTRATHINK REQUIRED**: Bugs BUG-033, BUG-035, and BUG-036 have interconnected root causes and require deep analysis. When working on these bugs:

1. **ULTRATHINK** before making any code changes - trace the full message flow
2. **ULTRATHINK** about how previous fixes (BUG-028, BUG-029, BUG-030) may have introduced regressions
3. **ULTRATHINK** about the dual MCP implementations (standalone vs bridge) and state propagation
4. **ULTRATHINK** before assuming a simple fix - these bugs reveal systemic architectural issues

## Active Bugs

| ID | Description | Priority | Status | Component | Link |
|----|-------------|----------|--------|-----------|------|
| BUG-041 | Claude Code crashes on paste inside ccmux | P1 | new | pty/client | [Link](BUG-041-claude-code-crashes-on-paste-inside-ccmux/) |
| BUG-038 | create_pane returns wrong response type | P1 | new | mcp | [Link](BUG-038-create-pane-returns-wrong-response-type/) |
| BUG-037 | close_pane returns AbortError | P2 | new | daemon | [Link](BUG-037-close-pane-aborts/) |
| BUG-032 | MCP handlers missing TUI broadcasts for pane/window/layout ops | P0 | new | ccmux-server | [Link](BUG-032-mcp-handlers-missing-tui-broadcasts/) |
| BUG-031 | Metadata not persisting across restarts | P1 | open | daemon | [Link](BUG-031-metadata-not-persisting-across-restarts/) |

## Priority Queue (Post-Retrospective)

| Priority | Bug | Risk | Effort | Approach |
|----------|-----|------|--------|----------|
| **P1** | BUG-041 | HIGH | Medium | Investigate PTY bracketed paste mode and termios settings |
| **P2** | BUG-037 | Medium | Low | Timeout/abort handling |

## Bug Status

### Resolved Bugs

| ID | Description | Priority | Resolution |
|----|-------------|----------|------------|
| BUG-040 | create_window returns success but doesn't create windows | P1 | Fixed - use active_session_id() instead of first session |
| BUG-039 | MCP tools hang intermittently through Claude Code | P1 | Fixed - connection recovery on timeout and infinite loop fixes |
| BUG-036 | Selection tools don't switch TUI view | P0 | Fixed - use global broadcasts for focus changes |
| BUG-035 | MCP handlers return wrong response types | P1 | Fixed - strict broadcast filtering and connection recovery |
| BUG-034 | create_window ignores selected session | P2 | Fixed - use active_session_id() instead of first session |
| BUG-033 | create_layout rejects all layout formats | P1 | Fixed - parse layout strings in bridge handler |
| BUG-030 | Daemon unresponsive after create_window | P0 | Fixed - wrap serde_json::Value for bincode compatibility |
| BUG-029 | MCP response synchronization bug | P0 | Fixed - filter broadcast messages in recv_response_from_daemon |
| BUG-028 | Daemon crashes on `ccmux_create_layout` with nested layout | P0 | Fixed - two-phase pane creation to avoid lock contention |
| BUG-027 | MCP response routing swapped between handlers | P0 | Fixed - filter broadcast messages in recv_response_from_daemon |
| BUG-026 | Focus management broken (auto-focus, focus_pane, select_window) | P1 | Fixed - broadcast focus changes to TUI clients |
| BUG-025 | create_pane direction response mismatch | P2 | Fixed - return user's requested direction |
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
| BUG-022 | Viewport stuck above bottom after subagent | P2 | Fixed - reset scroll position on resize (b567daf) |
| BUG-023 | ccmux_create_session doesn't spawn shell | P1 | Fixed - added command param + output poller (1ac7e60) |
| BUG-024 | Runaway pane spawning via sideband | P0 | Fixed - OSC escape sequence format (5da1697) |

### Deprecated Bugs

| ID | Description | Reason |
|----|-------------|--------|
| BUG-012 | Text selection not working in TUI | Shift+click works (by design) |