# BUG-069: Orchestration messages claim delivery but orchestrator never receives them

**Priority**: P2
**Component**: mcp/orchestration
**Severity**: medium
**Status**: new

## Problem

Orchestration messages sent via `ccmux_send_orchestration` (and by extension `ccmux_report_status`) report successful delivery but the target orchestrator session never receives the messages via `ccmux_poll_messages`.

## Observed Behavior (from __watchdog session)

The watchdog successfully called `ccmux_send_orchestration`:
```
target: {"tag": "orchestrator"}
msg_type: "worker.complete"
payload: {
  "worker_session": "feat-016-worker",
  "worker_id": "15425a37-19dc-47ca-bd57-f77059f80b7b",
  "task": "FEAT-016",
  "branch": "feat/016-ats-scoring",
  "commit": "c8ee2e3",
  "summary": "ATS scoring module implemented and committed. Worker is now idle at prompt. Ready for merge/cleanup."
}
```

The response indicated success:
```json
{
  "delivered_count": 1,
  "success": true
}
```

However, the orchestrator (session-0) never received this message via `ccmux_poll_messages`.

## Reproduction Steps

1. Create orchestrator session tagged with `orchestrator`
2. Create watchdog session tagged with `watchdog`
3. From watchdog, call `ccmux_send_orchestration` targeting `{"tag": "orchestrator"}`
4. Observe: Response shows `delivered_count: 1, success: true`
5. In orchestrator, call `ccmux_poll_messages`
6. Observe: No messages received despite "successful" delivery

## Expected Behavior

When `ccmux_send_orchestration` returns `delivered_count: 1`, the target session should actually receive the message via `ccmux_poll_messages`.

## Actual Behavior

- Send returns success with `delivered_count: 1`
- Target session's `ccmux_poll_messages` returns no messages
- Message is lost somewhere between "delivery" and poll queue

## Root Cause Analysis

Possible causes:
1. **Delivery vs queue mismatch**: Message is "delivered" to session but not added to poll queue
2. **Wrong session targeted**: `delivered_count: 1` may be counting wrong session
3. **Poll queue clearing**: Messages may be cleared before orchestrator polls
4. **Session attachment issue**: Orchestrator may need explicit attachment to receive messages
5. **Worker ID mismatch**: `ccmux_poll_messages(worker_id)` may expect different ID format

## Relevant Code

- `ccmux-server/src/mcp/bridge/handlers.rs` - `handle_report_status` implementation
- `ccmux-server/src/mcp/bridge/handlers.rs` - `handle_poll_messages` implementation
- `ccmux-server/src/session/` - Tag-based routing logic
- Orchestration message infrastructure

## Acceptance Criteria

- [ ] `ccmux_report_status` successfully delivers messages to `orchestrator`-tagged sessions
- [ ] `ccmux_poll_messages` returns status updates sent via `ccmux_report_status`
- [ ] Status updates include the status enum and message text
- [ ] Works in the watchdog -> orchestrator communication pattern

## Impact

This bug breaks the core watchdog monitoring pattern documented in AGENTS.md and CLAUDE.md:
- Watchdogs cannot alert orchestrators when workers complete
- Orchestrators cannot be notified of worker status changes
- The automated monitoring workflow is non-functional

## Workarounds

1. Use `ccmux_send_orchestration` directly with explicit target
2. Use `ccmux_read_pane` to poll worker output manually
3. Orchestrator manually checks worker sessions periodically

## Related

- BUG-060: Orchestration tools session attachment
- BUG-061: send_orchestration target parsing
- FEAT-005: Response channel orchestrator-worker
