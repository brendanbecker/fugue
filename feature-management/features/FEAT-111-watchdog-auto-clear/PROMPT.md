# FEAT-111: Watchdog Auto-Clear Cycle

**Priority**: P1
**Component**: skill/orchestration
**Effort**: Small
**Status**: new

## Summary

Integrate `/clear` into the watchdog monitoring cycle to keep watchdog context minimal. After completing each monitoring cycle and sending any notifications, the watchdog automatically clears its conversation context. This eliminates the need for compaction (which creates summaries unsuitable for monitoring) and ensures fresh context each cycle.

## Related Features

- **FEAT-110**: Watchdog Monitor Agent (companion feature - defines the core monitoring cycle)
- **FEAT-104**: Watchdog Orchestration Skill (original watchdog concept)

## Problem Statement

Watchdog agents run indefinitely, periodically checking worker status. Without context management:

1. **Context bloat**: Each monitoring cycle accumulates context (tool calls, responses, reasoning)
2. **Compaction overhead**: Auto-compaction creates summaries that aren't useful for monitoring
3. **Stale state**: Old worker statuses linger in context, causing confusion
4. **Cost accumulation**: Larger context = higher API costs per cycle

The watchdog doesn't need history - it only needs current worker status each cycle.

## Solution: Auto-Clear After Each Cycle

```
┌─────────────────────────────────────────────────────────────┐
│                    Watchdog Monitor Cycle                   │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│   ┌──────────┐    ┌──────────┐    ┌──────────┐    ┌──────┐ │
│   │   Poll   │───►│  Detect  │───►│  Notify  │───►│ Clear│ │
│   │ Workers  │    │  Issues  │    │ (if any) │    │      │ │
│   └──────────┘    └──────────┘    └──────────┘    └──────┘ │
│                                                      │      │
│                                                      │      │
│   ◄──────────────────────────────────────────────────┘      │
│                        (repeat)                             │
└─────────────────────────────────────────────────────────────┘
```

## Core Behavior

### When to Clear

Clear **after EVERY monitoring cycle** once the following conditions are met:

1. Worker polling complete
2. Status assessment complete
3. All notifications (if any) sent successfully
4. No pending retries

### Clear Preconditions

Before clearing, verify:

```
[ ] All fugue_send_orchestration calls completed successfully
[ ] All fugue_report_status calls completed successfully
[ ] No notification failures requiring retry
```

If a notification fails:
1. Retry the notification (up to 3 attempts)
2. If retry succeeds, proceed to clear
3. If retry exhausted, log error and clear anyway (don't get stuck)

### What Gets Preserved (Not in Context)

The watchdog is self-contained - everything it needs comes from external sources:

| Data | Source | Not Context |
|------|--------|-------------|
| Worker list | `fugue_list_panes` | Dynamic discovery |
| Worker status | `fugue_get_status` | Real-time query |
| Orchestrator target | Session tags (`orchestrator`) | Tag-based routing |
| Detection thresholds | Config file / session metadata | External config |
| Check interval | Watchdog timer | External trigger |

## Implementation Options

### Option 1: Send `/clear` via stdin (Recommended)

After cycle completion, use `fugue_send_input` to send `/clear`:

```json
{
  "tool": "fugue_send_input",
  "input": {
    "pane_id": "<watchdog_pane>",
    "input": "/clear",
    "submit": true
  }
}
```

**Pros**:
- Uses existing infrastructure
- No new MCP tools needed
- Claude handles the clear command natively

**Cons**:
- Watchdog must send this to itself (possible via known pane ID)
- Slight timing complexity

### Option 2: New `fugue_watchdog_clear` MCP Tool

Add dedicated tool for watchdog self-clear:

```json
{
  "name": "fugue_watchdog_clear",
  "description": "Clear the calling session's conversation context. For watchdog agents to reset after each monitoring cycle.",
  "parameters": {}
}
```

Implementation would:
1. Identify the calling session (from MCP request context)
2. Send `/clear` to that session's active Claude pane
3. Return success/failure

**Pros**:
- Clean API
- Self-targeting (no pane ID needed)

**Cons**:
- New tool to implement and maintain
- MCP bridge needs to track calling session

### Option 3: Build into Watchdog Skill/Prompt

The watchdog prompt explicitly instructs the agent to clear:

```
After completing your check and sending any alerts:
1. Confirm all notifications were sent
2. Send "/clear" to reset for next cycle

Do not skip the clear step. Fresh context each cycle.
```

**Pros**:
- No code changes
- Prompt-driven behavior

**Cons**:
- Relies on agent compliance
- Agent might forget or skip

### Recommendation

**Option 1 + Option 3 combined**:
- Build clear instruction into watchdog prompt (Option 3)
- Watchdog uses stdin approach (Option 1) to send `/clear`
- No new MCP tools needed
- Explicit prompt guidance ensures compliance

## Integration with FEAT-110 (Watchdog Monitor Agent)

FEAT-110 defines the core monitoring cycle. FEAT-111 adds the cleanup phase:

```
FEAT-110 Cycle:
  poll → detect → notify

FEAT-111 Addition:
  poll → detect → notify → CLEAR → repeat
```

### Watchdog Prompt Addition (for FEAT-110)

Add to watchdog system prompt:

```
## Context Management

After completing each monitoring cycle:

1. Verify all notifications sent successfully
2. If any notification failed, retry up to 3 times
3. Once all notifications confirmed (or retries exhausted):
   - Type "/clear" to reset your context
   - This keeps your context minimal and costs low
   - You will receive the next "check" trigger with fresh context

IMPORTANT: Always clear after each cycle. Do not accumulate history.
Your monitoring state is stateless - you rediscover workers each cycle.
```

## Watchdog State After Clear

After `/clear`, the watchdog has:

- **System prompt**: Monitoring instructions (preserved)
- **MCP tools**: All fugue tools available (preserved)
- **Session identity**: Same session, same tags (preserved)
- **Conversation history**: Empty (cleared)

The watchdog then waits for the next timer trigger (`check` message).

## Configuration

Add to watchdog config (session metadata or config file):

| Setting | Default | Description |
|---------|---------|-------------|
| `auto_clear` | `true` | Enable auto-clear after cycles |
| `notification_retries` | `3` | Max notification retry attempts |
| `clear_on_error` | `true` | Clear even if notifications failed |

## Error Handling

### Notification Failure

```
Notification fails → Retry (up to N times)
                  → If success: proceed to clear
                  → If exhausted: log error, clear anyway
```

### Clear Failure

If `/clear` itself fails (rare):
1. Log the failure
2. Continue to next cycle (will accumulate context)
3. Subsequent cycles should retry clear

## Acceptance Criteria

- [ ] Watchdog clears context after each monitoring cycle
- [ ] Clear happens only after all notifications confirmed
- [ ] Failed notifications trigger retry before clear
- [ ] Watchdog prompt includes clear instructions
- [ ] No conversation history accumulates across cycles
- [ ] Watchdog remains functional after clear (tools, prompt preserved)
- [ ] Config option to disable auto-clear (for debugging)
- [ ] Watchdog can discover workers from scratch each cycle (stateless)

## Testing

### Unit Tests

- Verify clear instruction in watchdog prompt
- Verify notification retry logic
- Verify clear timing (after notifications, not before)

### Integration Tests

1. Run watchdog for multiple cycles
2. Verify context resets between cycles
3. Verify notifications still reach orchestrator
4. Verify worker discovery works after clear

### Manual Testing

```bash
# Start orchestrator and workers
/orchestrate spawn task-1
/orchestrate spawn task-2
/orchestrate monitor start

# Observe watchdog cycles
# - Check: discovers workers
# - Notify: sends alerts (if needed)
# - Clear: context resets

# Verify with fugue_read_pane that watchdog context stays minimal
```

## Future Enhancements

- **Selective clear**: Clear conversation but preserve specific memories
- **Clear metrics**: Track clear frequency, context size at clear time
- **Adaptive clearing**: Clear only when context exceeds threshold
- **Clear notification**: Inform orchestrator when watchdog clears (debugging)
