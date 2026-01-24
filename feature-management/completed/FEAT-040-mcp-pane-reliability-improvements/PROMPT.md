# FEAT-040: MCP Pane Reliability Improvements

**Priority**: P2
**Component**: fugue-server
**Type**: feature
**Status**: completed

## Summary

Defensive improvements to MCP pane creation to ensure reliable behavior and proper PTY output handling.

## Changes

### 1. Deterministic Session Ordering

`SessionManager::list_sessions()` now returns sessions sorted by creation time (oldest first).

**Problem**: HashMap iteration order is non-deterministic. Code using `list_sessions().first()` could get different sessions across runs.

**Solution**: Sort by `created_at_millis()` timestamp to ensure consistent ordering.

**Files Changed**:
- `fugue-server/src/session/manager.rs:77-85`
- `fugue-server/src/session/session.rs:218-224` (added `created_at_millis()`)

### 2. Output Poller for MCP-Created Panes

MCP-created panes now start a PTY output poller, matching the behavior of regular pane creation.

**Problem**: `handle_create_pane_with_options()` spawned PTY but didn't start an output poller, so MCP-created panes would never receive PTY output.

**Solution**: Added `PtyOutputPoller::spawn_with_sideband()` call after PTY spawn.

**Files Changed**:
- `fugue-server/src/handlers/mcp_bridge.rs:342-365`

### 3. Integration Tests for MCP-to-TUI Broadcast

Added tests verifying the broadcast mechanism works correctly for MCP pane creation.

**Tests Added**:
- `registry::tests::test_mcp_to_tui_broadcast_except`
- `registry::tests::test_broadcast_except_unattached_client`
- `registry::tests::test_broadcast_to_different_session`
- `handlers::mcp_bridge::tests::test_mcp_pane_creation_broadcasts_to_tui`
- `handlers::mcp_bridge::tests::test_mcp_broadcast_fails_with_session_mismatch`
- `session::manager::tests::test_manager_list_sessions_ordered_by_creation`

## Context

These improvements were discovered while investigating BUG-010 (MCP pane broadcast not received). While they don't confirm the root cause of BUG-010, they are valuable defensive fixes and test coverage.

## Acceptance Criteria

- [x] `list_sessions()` returns sessions in deterministic order
- [x] MCP-created panes have output pollers
- [x] Integration tests cover MCP-to-TUI broadcast path
