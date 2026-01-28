# FEAT-114: Named/Multiple Watchdogs

**Priority**: P2
**Component**: watchdog
**Status**: done

## Problem

The current watchdog implementation only supports a single watchdog instance. This limits flexibility:
- Can't have multiple watchdogs monitoring different worker pools
- Can't have project-specific watchdogs (e.g., `__watchdog-featmgmt`, `__watchdog-beckerkube`)
- Can't easily identify which watchdog is which
- Starting a new watchdog automatically stops the previous one

## Solution

Add support for named watchdogs by:
1. Changing `WatchdogManager` from storing `Option<WatchdogState>` to `HashMap<String, WatchdogState>`
2. Adding optional `name` parameter to all watchdog MCP tools
3. Default name is "default" for backward compatibility

## Key Files

| File | Purpose |
|------|---------|
| `fugue-server/src/watchdog.rs` | WatchdogManager implementation |
| `fugue-protocol/src/messages.rs` | Protocol messages for watchdog |
| `fugue-server/src/mcp/tools.rs` | MCP tool schemas |
| `fugue-server/src/mcp/bridge/handlers.rs` | MCP tool implementations |
| `fugue-server/src/mcp/bridge/mod.rs` | MCP tool dispatch |
| `fugue-server/src/handlers/mod.rs` | Server-side handlers |

## Implementation Plan

### Section 1: Update WatchdogManager

- [x] Change state storage from `Option<WatchdogState>` to `HashMap<String, WatchdogState>`
- [x] Change cancel_tx from `Option<oneshot::Sender>` to `HashMap<String, oneshot::Sender>`
- [x] Update `start()` to accept optional name parameter (default: "default")
- [x] Update `stop()` to accept optional name parameter (None stops all)
- [x] Update `status()` to accept optional name parameter (None returns all)
- [x] Add `stop_all()` method for convenience

### Section 2: Update Protocol Messages

- [x] Add `name: Option<String>` to `WatchdogStart` message
- [x] Add `name: Option<String>` to `WatchdogStop` message
- [x] Add `name: Option<String>` to `WatchdogStatus` message
- [x] Add `name: String` to `WatchdogStarted` response
- [x] Add `name: Option<String>` to `WatchdogStopped` response
- [x] Update `WatchdogStatusResponse` to return list of watchdogs or single

### Section 3: Update MCP Tool Schemas

- [x] Add `name` parameter to `fugue_watchdog_start`
- [x] Add `name` parameter to `fugue_watchdog_stop`
- [x] Add `name` parameter to `fugue_watchdog_status`

### Section 4: Update MCP Bridge

- [x] Update `tool_watchdog_start()` to pass name
- [x] Update `tool_watchdog_stop()` to pass name
- [x] Update `tool_watchdog_status()` to pass name
- [x] Update dispatch in mod.rs to parse name parameter

### Section 5: Update Server Handlers

- [x] Update `handle_watchdog_start()` to use name
- [x] Update `handle_watchdog_stop()` to use name
- [x] Update `handle_watchdog_status()` to use name

### Section 6: Tests

- [x] Test starting multiple watchdogs with different names
- [x] Test stopping specific watchdog by name
- [x] Test stopping all watchdogs when name is None
- [x] Test getting status of specific watchdog
- [x] Test getting status of all watchdogs
- [x] Ensure backward compatibility (default name works)

## Acceptance Criteria

- [x] Can start multiple named watchdogs simultaneously
- [x] Each watchdog sends messages to its target pane independently
- [x] Can stop a specific watchdog by name
- [x] Can stop all watchdogs at once
- [x] Can query status of specific or all watchdogs
- [x] Backward compatible: existing code using no name continues to work
- [x] All existing tests pass
- [x] New tests cover multi-watchdog scenarios

## API Changes

### fugue_watchdog_start

```json
{
  "pane_id": "uuid",
  "interval_secs": 90,
  "message": "check",
  "name": "featmgmt"  // NEW: optional, defaults to "default"
}
```

### fugue_watchdog_stop

```json
{
  "name": "featmgmt"  // NEW: optional, if omitted stops all watchdogs
}
```

### fugue_watchdog_status

```json
{
  "name": "featmgmt"  // NEW: optional, if omitted returns all watchdogs
}
```

### Response Examples

**Single watchdog status:**
```json
{
  "is_running": true,
  "name": "featmgmt",
  "pane_id": "uuid",
  "interval_secs": 90,
  "message": "check"
}
```

**All watchdogs status:**
```json
{
  "watchdogs": [
    {"name": "default", "pane_id": "uuid1", "interval_secs": 90, "message": "check"},
    {"name": "featmgmt", "pane_id": "uuid2", "interval_secs": 30, "message": "ping"}
  ]
}
```

## Verification

```bash
# Build
cargo build

# Run tests
cargo test watchdog
cargo test -p fugue-protocol

# All tests should pass
```
