# BUG-069: Orchestration messages claim delivery but orchestrator never receives them

**Priority**: P2
**Component**: mcp/orchestration
**Severity**: medium
**Status**: fixed

## Problem

Orchestration messages sent via `fugue_send_orchestration` (and by extension `fugue_report_status`) report successful delivery but the target orchestrator session never receives the messages via `fugue_poll_messages`.

## Root Cause (FOUND)

**This was NOT a code bug but a user-facing design issue.**

The original issue was:
1. Messages sent to tag "orchestrator" are delivered to sessions with that tag
2. The user was polling "session-0" thinking it was the orchestrator
3. But "session-0" did NOT have the "orchestrator" tag
4. The actual orchestrator-tagged session received the messages
5. Polling "session-0" returned empty because messages went elsewhere

The `fugue_poll_messages` tool required the user to explicitly specify which session to poll, which was error-prone because users didn't always know which session was tagged.

## Fix Applied

1. **Made `worker_id` parameter optional** in `fugue_poll_messages`
   - If omitted, polls the caller's attached session automatically
   - This makes it impossible to poll the "wrong" session

2. **Improved tool documentation**
   - Clarified that `worker_id` specifies "which session's inbox to poll"
   - Added tip about ensuring the polled session has the correct tag

3. **Added comprehensive tests** verifying:
   - Messages go to the correct tagged session
   - Polling the wrong session returns empty (expected behavior)
   - Polling with `None` uses the attached session
   - Polling with `None` when not attached returns error

## Files Changed

- `fugue-protocol/src/messages.rs` - Made `PollMessages.worker_id` optional
- `fugue-server/src/handlers/orchestration.rs` - Updated handler to support `None`
- `fugue-server/src/mcp/bridge/mod.rs` - Updated tool call routing
- `fugue-server/src/mcp/bridge/handlers.rs` - Updated MCP handler signature
- `fugue-server/src/mcp/tools.rs` - Updated tool schema and description

## Observed Behavior (from __watchdog session)

The watchdog successfully called `fugue_send_orchestration`:
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

However, the orchestrator (session-0) never received this message via `fugue_poll_messages`.

## Acceptance Criteria

- [x] `fugue_report_status` successfully delivers messages to `orchestrator`-tagged sessions
- [x] `fugue_poll_messages` returns status updates sent via `fugue_report_status`
- [x] Status updates include the status enum and message text
- [x] Works in the watchdog -> orchestrator communication pattern
- [x] `fugue_poll_messages` with no `worker_id` polls the attached session

## How to Use (After Fix)

**Before (error-prone):**
```json
// User had to know the exact session name
{"tool": "fugue_poll_messages", "input": {"worker_id": "session-0"}}
```

**After (recommended):**
```json
// Just poll your attached session - no need to know the name
{"tool": "fugue_poll_messages", "input": {}}
```

**Or explicitly if needed:**
```json
// Still works if you need to poll a specific session
{"tool": "fugue_poll_messages", "input": {"worker_id": "orchestrator-session"}}
```

## Impact

This fix restores the core watchdog monitoring pattern documented in AGENTS.md and CLAUDE.md:
- Watchdogs can now reliably alert orchestrators when workers complete
- Orchestrators can be notified of worker status changes
- The automated monitoring workflow is functional

## Related

- BUG-060: Orchestration tools session attachment
- BUG-061: send_orchestration target parsing
- FEAT-005: Response channel orchestrator-worker
