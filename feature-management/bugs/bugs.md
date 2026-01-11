# Bug Reports

**Project**: ccmux
**Last Updated**: 2026-01-11 (QA Demo Run)

## Summary Statistics
- Total Bugs: 37
- Open: 9
- Resolved: 27
- Deprecated: 1

## CRITICAL: Approach for Complex Bugs

**ULTRATHINK REQUIRED**: Bugs BUG-033, BUG-035, and BUG-036 have interconnected root causes and require deep analysis. When working on these bugs:

1. **ULTRATHINK** before making any code changes - trace the full message flow
2. **ULTRATHINK** about how previous fixes (BUG-028, BUG-029, BUG-030) may have introduced regressions
3. **ULTRATHINK** about the dual MCP implementations (standalone vs bridge) and state propagation
4. **ULTRATHINK** before assuming a simple fix - these bugs reveal systemic architectural issues

## Root Cause Clusters (from Retrospective Analysis)

| Cluster | Bugs | Risk | Key Insight |
|---------|------|------|-------------|
| **Protocol/Serialization** | BUG-033, BUG-035 | HIGH | JsonValue wrapper may break JSON access patterns |
| **Session State Management** | BUG-034, BUG-036 | HIGH | State doesn't propagate across MCP bridge boundary |
| **Broadcast Filtering** | BUG-036 | HIGH | BUG-029 fix may have over-filtered broadcasts |
| **Operation Reliability** | BUG-035, BUG-037 | MEDIUM | State accumulates/corrupts over long sessions |

## Paths Ruled Out

- **"Just add broadcasts"** - Doesn't work; broadcasts aren't reaching TUI
- **Fire-and-forget selection** - Selection tools need state confirmation
- **Simple JSON key checks** - JsonValue wrapper changes `.get()` behavior

## Active Bugs

| ID | Description | Priority | Status | Component | Link |
|----|-------------|----------|--------|-----------|------|
| BUG-036 | Selection tools don't switch TUI view | P0 | new | daemon/TUI | [Link](BUG-036-selection-tools-dont-switch-tui-view/) |
| BUG-035 | MCP handlers return wrong response types | P1 | new | daemon | [Link](BUG-035-mcp-handlers-return-wrong-response-types/) |
| BUG-033 | create_layout rejects all layout formats | P1 | new | daemon | [Link](BUG-033-create-layout-validation-rejects-all-formats/) |
| BUG-034 | create_window ignores selected session | P2 | new | daemon | [Link](BUG-034-create-window-ignores-selected-session/) |
| BUG-037 | close_pane returns AbortError | P2 | new | daemon | [Link](BUG-037-close-pane-aborts/) |
| BUG-032 | MCP handlers missing TUI broadcasts for pane/window/layout ops | P0 | new | ccmux-server | [Link](BUG-032-mcp-handlers-missing-tui-broadcasts/) |
| BUG-031 | Metadata not persisting across restarts | P1 | open | daemon | [Link](BUG-031-metadata-not-persisting-across-restarts/) |
| BUG-030 | Daemon unresponsive after create_window | P0 | fixed | daemon | [Link](BUG-030-daemon-unresponsive-after-create-window/) |
| BUG-029 | MCP response synchronization bug - responses lag by one call | P0 | fixed | daemon | [Link](BUG-029-create-window-unexpected-response/) |

## Priority Queue (Post-Retrospective)

| Priority | Bug | Risk | Effort | Approach |
|----------|-----|------|--------|----------|
| **P0** | BUG-036 | HIGH | Low | **ULTRATHINK**: Check `is_broadcast_message()` filter - may fix BUG-034 too |
| **P0** | BUG-033 | HIGH | Medium | **ULTRATHINK**: Verify JsonValue wrapper `.get()` compatibility |
| **P1** | BUG-034 | HIGH | Medium | Session state propagation audit |
| **P1** | BUG-035 | VERY HIGH | High | **ULTRATHINK**: State drift - needs stress testing to reproduce |
| **P2** | BUG-037 | Medium | Low | Timeout/abort handling |

## Files Needing Heavy Focus

| File | Issues | Notes |
|------|--------|-------|
| `ccmux-server/src/mcp/bridge.rs` | BUG-034, BUG-036 | **ULTRATHINK**: Session state + broadcast filtering (line 458-461) |
| `ccmux-server/src/handlers/mcp_bridge.rs` | BUG-033, BUG-035 | Layout validation + response types |
| `ccmux-protocol/src/types.rs` | BUG-033 | JsonValue wrapper - verify Deref impl |
| `ccmux-client/src/ui/app.rs` | BUG-036 | Selection broadcast handling (line 1541) |
| `ccmux-server/src/mcp/handlers.rs` | BUG-034 | Session state usage |

## Bug Details

### BUG-036: Selection Tools Don't Switch TUI View (P0) - ROOT CAUSE FOUND
**Status**: Ready to implement fix

- `select_session`, `select_window`, `focus_pane` return success but TUI never switches
- User stayed on session-0 the whole QA run despite multiple selection calls
- Only auto-focus on pane creation works

**ROOT CAUSE CONFIRMED**: TUI handlers at `app.rs:1521-1548` only update local state for items in current session. They do NOT switch sessions.

**FIX**: When `SessionFocused` arrives with different `session_id`, send `AttachSession` to switch.

**Location**: `ccmux-client/src/ui/app.rs` lines 1521-1548

### BUG-035: MCP Handlers Return Wrong Response Types (P1) - VERY TRICKY
**ULTRATHINK MULTIPLE TIMES** - State drift only appears after many operations.

- `list_windows` returned `SessionList` instead of `WindowList`
- `list_panes` returned `WindowList` instead of `PaneList`
- Data inside is correct, wrong wrapper type
- Only appeared later in session after many operations
- **Hypothesis**: Response queue gets out of sync, type tag corruption
- **Approach**: Stress test with 100+ operations, trace response types

### BUG-033: create_layout Rejects All Layout Formats (P1) - INVESTIGATION COMPLETE
**Status**: Debug logging needed to confirm hypothesis

- All layout JSON specs rejected with "must contain 'pane' or 'splits'"
- Even simplest `{"pane": {}}` fails
- Daemon doesn't crash (BUG-028 fix worked), but validation too strict

**INVESTIGATION FINDINGS**:
- Tests that bypass MCP bridge pass (call `handle_create_layout` directly)
- The MCP bridge at `bridge.rs:763` does `arguments["layout"].clone()`
- If layout passed as string (not object), `layout.get("pane")` returns None

**NEXT STEP**: Add debug logging at `bridge.rs:763` to see actual value type

**Location**: `ccmux-server/src/mcp/bridge.rs` lines 760-764

### BUG-034: create_window Ignores Selected Session (P2)
- After `select_session` to dev-qa, `create_window` still created in session-0
- Tool docs say "Uses active session if omitted" but it doesn't
- Created stray "logs" window in orchestrator's session
- **Workaround**: Always pass explicit `session` parameter
- **Related**: May share root cause with BUG-036

### BUG-037: close_pane Returns AbortError (P2)
- `MCP error -32001: AbortError: The operation was aborted`
- Pane doesn't close, user had to do it manually
- **Workaround**: User manually closes via keyboard

## Bug Status

See `feature-management/completed/` for resolved work items.

### Resolved Bugs

| ID | Description | Priority | Resolution |
|----|-------------|----------|------------|
| BUG-030 | Daemon unresponsive after create_window | P0 | Fixed - wrap serde_json::Value for bincode compatibility |
| BUG-029 | MCP response synchronization bug | P0 | Fixed - filter broadcast messages in recv_response_from_daemon |
| BUG-028 | Daemon crashes on `ccmux_create_layout` with nested layout | P0 | Fixed - two-phase pane creation to avoid lock contention |
| BUG-027 | MCP response routing swapped between handlers | P0 | Fixed - filter broadcast messages in recv_response_from_daemon |
| BUG-026 | Focus management broken (auto-focus, focus_pane, select_window) | P1 | Fixed - broadcast focus changes to TUI |
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

## Recent Activity

| Date | Bug ID | Action | Description |
|------|--------|--------|-------------|
| 2026-01-11 | BUG-037 | Created | close_pane returns AbortError |
| 2026-01-11 | BUG-036 | Created | Selection tools don't switch TUI view (P0) |
| 2026-01-11 | BUG-035 | Created | MCP handlers return wrong response types |
| 2026-01-11 | BUG-034 | Created | create_window ignores selected session |
| 2026-01-11 | BUG-033 | Created | create_layout rejects all layout formats |
| 2026-01-11 | BUG-030 | Resolved | Fixed: wrap serde_json::Value for bincode compatibility |
| 2026-01-11 | BUG-032 | Created | MCP handlers missing TUI broadcasts for split_pane, create_window, create_layout, resize_pane |
| 2026-01-11 | BUG-031 | Created | Metadata not persisting across daemon restarts |
| 2026-01-11 | BUG-030 | Created | Daemon unresponsive after create_window operations |
| 2026-01-11 | BUG-029 | Resolved | Fixed: filter broadcast messages in recv_response_from_daemon |
| 2026-01-11 | BUG-029 | Created | MCP response synchronization - responses lag by one call |
| 2026-01-11 | BUG-028 | Resolved | Fixed: two-phase pane creation avoids lock contention |
| 2026-01-11 | BUG-028 | Created | Daemon crashes on ccmux_create_layout with nested layout spec |
| 2026-01-11 | BUG-027 | Resolved | Fixed: filter broadcast messages in recv_response_from_daemon |
| 2026-01-11 | BUG-026 | Resolved | Fixed: broadcast focus changes to TUI clients |
| 2026-01-11 | BUG-025 | Resolved | Fixed: return user's requested direction in response |
