# BUG-072: fugue_kill_session hang regression

**Priority**: P1
**Component**: mcp, client, server
**Severity**: high
**Status**: resolved

## Problem

`fugue_kill_session` is causing client hangs again, despite the fix in BUG-058 (commit `9fd2481`). This is a regression of the original issue.

## Background

BUG-058 was fixed on 2026-01-18 by broadcasting `SessionEnded` to attached clients before detaching them in `handle_destroy_session`. The fix is confirmed to be in the currently running binary (built 2026-01-20 00:33).

Original fix commit: `9fd2481bae9e07ab0d2d366088a285b8936ef960`

## Reproduction Steps

1. Have multiple sessions running (orchestrator workflow with workers)
2. Orchestrator calls `fugue_kill_session` via MCP to kill a worker session
3. ~50% chance: Client hangs

## Expected Behavior

Session is killed and client continues operating normally.

## Actual Behavior

Client hangs after session is killed, similar to original BUG-058 symptoms.

### Detailed Observations (2026-01-23)

**Trigger context:**
- Using fugue as orchestrator
- Orchestrator kills worker sessions when work is complete
- ~50/50 chance of hang occurring per kill operation
- **Does not matter which session the user is viewing** - hang can occur regardless

**Symptoms:**
1. Screen lags/freezes - appears entirely frozen from user's viewpoint
2. No indication whether input is being processed
3. `Ctrl+b s` (session picker) **does work** - takes user to session management screen
4. **Cannot enter a session** from the session management screen
5. `n` for new session does not appear to work
6. Exiting client (`Ctrl+b d` or quit) and relaunching restores normal operation

**Key insight:** The hang affects the client globally, not just the session that was killed. This suggests the issue is in shared client state or event loop, not session-specific handling.

### Additional Observations (2026-01-24)

**New data point - daemon is fine, client render is broken:**
1. Screen froze, input appeared unresponsive
2. In session management screen: `n` (new session) and `r` (refresh) appeared to do nothing
3. After client restart: **new session existed** - the `n` command was actually processed!
4. Conclusion: Daemon received and processed commands, but client stopped updating UI

**This strongly suggests:** Client message receive loop or render loop is blocked/deadlocked. Commands are being sent successfully, daemon processes them, but client never receives/renders the responses.

## Hypotheses

Based on the 2026-01-23 observations:

### H1: Event Loop Blocked on Dead Channel
The client may be waiting on a channel/subscription related to the killed session. Since `Ctrl+b` keybindings work but normal operation doesn't, the event loop is running but something is blocking the render/update path.

### H2: Race Condition in Session Cleanup
The ~50% occurrence rate suggests a race condition. The `SessionEnded` broadcast may arrive before or after other cleanup events, causing inconsistent state.

### H3: Orphaned State in Client
The client may maintain state (subscriptions, pending requests) for the killed session that isn't properly cleaned up, causing subsequent operations to deadlock.

### H4: Session Picker Works but Attach Fails
Since `Ctrl+b s` works but entering a session doesn't, the issue may be in the session attach/select path rather than the render loop itself.

### H5: Client Receive Loop Deadlocked (Most Likely - 2026-01-24)
The 2026-01-24 observation proves:
- Commands are being sent (daemon created session)
- Daemon is processing correctly
- Client is NOT receiving/rendering responses

The client likely has separate tasks for:
1. Sending commands (working)
2. Receiving daemon messages (blocked/deadlocked)
3. Rendering UI (starved because no messages arriving)

The `Ctrl+b` keybindings may work because they're handled locally before hitting the message receive path. The deadlock is likely in the message receive task, possibly waiting on a channel related to the killed session.

## Investigation Steps

### Section 1: Verify Original Fix Still Present
- [ ] Confirm `SessionEnded` broadcast is still in `handle_destroy_session`
- [ ] Check if any recent commits modified session destruction logic
- [ ] Review commits since BUG-058 fix that touch session management

### Section 2: Identify Regression Cause
- [ ] Run with RUST_LOG=debug and capture logs during hang
- [ ] Compare current behavior with BUG-058 fix expectations
- [ ] Check if there are new code paths that bypass the fix
- [ ] Look for race conditions in session cleanup

### Section 3: Test Scenarios
- [ ] Test killing current session vs non-current session
- [ ] Test killing session with attached clients vs detached
- [ ] Test killing session via MCP vs via TUI (Ctrl+D)
- [ ] Test with single client vs multiple clients

### Section 4: New Test Cases (2026-01-23)
- [ ] Test killing session user is NOT viewing - does hang still occur?
- [ ] Capture client state when hung - what is the client waiting on?
- [ ] Check if session picker actions (select, new) send messages that never complete
- [ ] Monitor client-daemon communication during and after kill
- [ ] Test rapid sequential kills vs single kills

## Acceptance Criteria

- [ ] `fugue_kill_session` completes without hanging the client
- [ ] Root cause of regression identified and documented
- [ ] Fix addresses regression without breaking original fix
- [ ] Client remains fully operational after session kill (can select sessions, create new ones)
- [ ] No hang regardless of which session user is viewing when kill occurs
- [ ] Add regression test if possible

## Related Files

- `fugue-server/src/session/manager.rs` - Session destruction logic
- `fugue-server/src/mcp/bridge/handlers.rs` - kill_session MCP handler
- `fugue-client/src/ui/app.rs` - Client state management
- `fugue-protocol/src/messages.rs` - SessionEnded message

## Root Cause Analysis (2026-01-24)

**H5 was correct: Client Receive Loop Deadlocked**

The deadlock occurs due to a circular dependency between two bounded channels in the client's `Connection` module:

1. **`outgoing` channel** (100 messages): Main loop → connection_task → socket
2. **`incoming` channel** (100 messages): Socket → connection_task → main loop

In `connection_task`:
```rust
tokio::select! {
    Some(msg) = outgoing.recv() => { ... }
    result = framed.next() => {
        if incoming.send(msg).await.is_err() {  // <-- Can BLOCK here
            break;
        }
    }
}
```

When `incoming.send(msg).await` blocks (channel full), the task can't poll `outgoing.recv()`. Meanwhile, the main loop's message handlers call `connection.send()` which blocks if `outgoing` is full.

**Deadlock scenario:**
1. Server sends burst of messages (e.g., during session kill, many PaneClosed/Output messages)
2. `incoming` channel fills up (100 messages)
3. `connection_task` blocks on `incoming.send()`
4. Main loop processes a message (e.g., `SessionEnded`), calls `connection.send(ListSessions)`
5. If `outgoing` accumulates enough pending sends, `connection.send()` blocks
6. Main loop can't drain `incoming` because it's blocked on `connection.send()`
7. DEADLOCK

The ~50% occurrence rate matches a race condition - sometimes the timing allows normal flow, sometimes it triggers the deadlock.

## Resolution

Changed the `incoming` channel from bounded (100 messages) to unbounded. This ensures `incoming.send()` never blocks, breaking the deadlock potential.

**File changed:** `fugue-client/src/connection/client.rs`
- Changed `mpsc::channel::<ServerMessage>(100)` to `mpsc::unbounded_channel::<ServerMessage>()`
- Changed `mpsc::Sender<ServerMessage>` to `mpsc::UnboundedSender<ServerMessage>`
- Changed `incoming.send(msg).await` to `incoming.send(msg)` (unbounded send is synchronous)

**Trade-off:** Unbounded channels can grow indefinitely under memory pressure. However:
- The daemon rate-limits output
- Tick-based processing (50 messages/tick at 100ms) keeps up with normal traffic
- Memory pressure from messages is unlikely in practice

## Related Issues

- BUG-058: Original fix for kill_session client hang
