# BUG-063: Mirror panes cannot view other sessions (defeats entire purpose)

**Priority**: P1
**Component**: session
**Severity**: critical
**Status**: fixed

## Problem

The mirror pane feature (FEAT-062) creates mirrors in the **same session** as the source pane, not in the calling session. This completely defeats the intended purpose of mirror panes.

The whole point of mirror panes is "plate spinning" - an orchestrator in session A should be able to create mirror panes to watch agents running in sessions B, C, D, etc. Currently, if you call `fugue_mirror_pane` from session A targeting a pane in session B, the mirror is created in session B - which is useless.

## Reproduction Steps

1. Have orchestrator in `session-0`
2. Have worker agent in `worker-session` with pane `worker-pane-id`
3. From orchestrator, call `fugue_mirror_pane(source_pane_id: "worker-pane-id")`
4. Observe: Mirror created in `worker-session`, NOT in `session-0`
5. Orchestrator still can't see the worker's output

## Expected Behavior

Mirror pane should be created in the **calling client's session** (or a specified target session), displaying output from the source pane in another session.

## Actual Behavior

Mirror is created in the same session as the source pane, making it invisible to the orchestrator who requested it.

## Root Cause

Looking at `fugue-server/src/handlers/pane.rs` `handle_create_mirror`:

```rust
// Get the source pane information
let (session_id, window_id, session_name) = {
    let session_manager = self.session_manager.read().await;
    let pane_info = match session_manager.find_pane(source_pane_id) {
        // ... finds session from SOURCE pane
    };
};

// Create the mirror pane
let session = match session_manager.get_session_mut(session_id) {  // Uses SOURCE session!
    // ...
};
```

The handler uses the source pane's session ID to determine where to create the mirror. It should use the requesting client's attached session instead.

## Fix

The `CreateMirror` message handler needs to:

1. Accept an optional `target_session_id` parameter (defaults to caller's attached session)
2. Create the mirror pane in the target session, not the source session
3. Set up cross-session output forwarding from source pane to mirror pane

The MCP tool `fugue_mirror_pane` should:
1. Default to creating the mirror in the MCP client's attached session
2. Optionally accept a `target_session` parameter for explicit control

## Acceptance Criteria

- [ ] `fugue_mirror_pane` creates mirror in the caller's session by default
- [ ] Mirror displays real-time output from source pane in different session
- [ ] Optional `target_session` parameter for explicit control
- [ ] Orchestrator can watch multiple worker sessions via mirrors

## Impact

This bug makes the entire mirror pane feature unusable for its primary use case:
- Cannot do "plate spinning" multi-agent monitoring
- DEMO-MULTI-AGENT.md Act 8 (mirror panes) doesn't work as intended
- Orchestrators have no visibility into worker sessions

## Related

- FEAT-062: Original mirror pane implementation
- BUG-059: Mirror pane AbortError (fixed response handling, but cross-session was never implemented)
- BUG-062: Mirror pane close timeout (separate issue)
