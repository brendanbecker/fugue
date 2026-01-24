# FEAT-060: MCP Daemon Auto-Recovery

## Summary

Add automatic daemon reconnection and recovery capability to the MCP server layer, allowing Claude/agents to gracefully handle daemon crashes or disconnections without losing the entire session context.

## Motivation

During QA testing (BUG-028), a `fugue_create_layout` call crashed the daemon, leaving all MCP tools unusable with "Daemon connection lost" errors. The MCP server had no mechanism to:

1. Detect the daemon had crashed
2. Attempt reconnection
3. Recover session state
4. Notify the client of the situation

This creates a poor agent experience where a single bad API call can brick the entire MCP interface until manual intervention.

## Requirements

### Core Recovery

1. **Connection Health Monitoring**
   - Periodic heartbeat between MCP server and daemon
   - Detect connection loss within 2-3 seconds

2. **Automatic Reconnection**
   - On connection loss, attempt reconnection with exponential backoff
   - Max retry attempts configurable (default: 5)
   - Backoff: 100ms, 200ms, 400ms, 800ms, 1600ms

3. **Graceful Degradation**
   - If reconnection fails, return structured error to MCP client
   - Include `recoverable: false` flag and `reconnect_attempts` count
   - Suggest user action (restart daemon)

### State Recovery

4. **Session State Cache**
   - Cache last-known session/window/pane state in MCP server
   - On reconnect, diff cached state vs actual state
   - Report any lost sessions/panes to client

5. **Operation Replay** (stretch goal)
   - Queue failed operations during disconnect
   - Replay idempotent operations on reconnect
   - Skip non-idempotent operations with warning

### Error Handling

6. **Structured Error Response**
   ```json
   {
     "error": "daemon_connection_lost",
     "recoverable": true,
     "reconnect_status": "attempting",
     "reconnect_attempt": 3,
     "last_known_state": {
       "sessions": 2,
       "windows": 4,
       "panes": 8
     }
   }
   ```

7. **MCP Tool for Status**
   - Add `fugue_connection_status` tool
   - Returns: connected/disconnected/reconnecting
   - Includes uptime, reconnect history

## Implementation Notes

### Architecture

```
MCP Client (Claude)
       |
       v
MCP Server Layer
  - Connection monitor (heartbeat)
  - State cache
  - Reconnection logic
       |
       v
Unix Socket
       |
       v
fugue Daemon
```

### Key Components

1. **ConnectionMonitor** - Background task checking daemon health
2. **StateCache** - In-memory cache of session hierarchy
3. **ReconnectManager** - Handles backoff and retry logic
4. **ErrorTranslator** - Converts raw errors to structured MCP responses

### Affected Crates

- `fugue-server/src/mcp/` - Primary implementation
- `fugue-protocol/` - New heartbeat message type
- `fugue-client/` - May need to handle daemon restart notifications

## Success Criteria

- [ ] Daemon crash detected within 3 seconds
- [ ] Auto-reconnect succeeds when daemon restarts
- [ ] Structured error returned when reconnect fails
- [ ] `fugue_connection_status` tool works
- [ ] QA demo can continue after daemon restart without manual intervention

## Related

- **BUG-028**: Daemon crashes on `fugue_create_layout` (triggered this feature request)
- **FEAT-018**: MCP Server integration (base implementation)
- **FEAT-016**: Persistence (can leverage for state recovery)

## Priority

**P1** - Improves reliability for agent workflows. Currently a single crash requires full manual restart.
