# BUG-066: Mirror panes don't forward output across sessions

**Priority**: P2
**Component**: session/mirror
**Severity**: medium
**Status**: resolved

## Problem

Mirror panes created across sessions show no output. The mirror pane is created in the correct session (BUG-063 fix works), but output from the source pane in another session is not forwarded to the mirror.

## Reproduction Steps

1. Have an orchestrator in `session-0`
2. Create a worker session: `fugue_create_session(name: "worker")`
3. Get the worker's pane ID
4. From session-0, create a mirror: `fugue_mirror_pane(source_pane_id: "<worker-pane>", direction: "horizontal")`
5. Observe: Mirror pane appears in session-0 but is blank
6. Read worker pane: Has content (Claude thinking spinner, output, etc.)
7. Read mirror pane: Empty

## Expected Behavior

Mirror pane displays real-time output from the source pane, even when the source is in a different session.

## Actual Behavior

Mirror pane remains blank. Output forwarding only works for same-session mirrors (untested but implied by the implementation).

## Root Cause Analysis

BUG-063 fixed the *location* where mirror panes are created (caller's session instead of source's session). However, the output forwarding mechanism likely only subscribes to output events within the same session.

The mirror pane implementation needs to:
1. Subscribe to output from the source pane regardless of session
2. Forward that output to the mirror pane's buffer
3. Handle cross-session PTY output routing

## Relevant Code

- `fugue-server/src/handlers/pane.rs` - `handle_create_mirror`
- `fugue-server/src/session/` - Session and pane management
- `fugue-server/src/pty/` - PTY output handling

## Acceptance Criteria

- [ ] Mirror pane displays source pane output in real-time
- [ ] Works when source and mirror are in different sessions
- [ ] Output includes all content (text, escape sequences, colors)
- [ ] Mirror updates as source produces new output

## Impact

This bug significantly reduces the value of the mirror pane feature for "plate spinning" orchestration:
- Orchestrators cannot visually monitor worker agents in other sessions
- The entire cross-session mirror use case is broken
- Users must switch sessions manually to view agent output

## Workarounds

1. Use `fugue_read_pane` to periodically poll worker output
2. Switch to the worker session directly
3. Create mirror panes within the same session (limited use)

## Related

- BUG-063: Mirror pane created in wrong session (fixed - creation location)
- BUG-062: Mirror pane close timeout (fixed)
- BUG-059: Mirror pane AbortError (fixed)
- FEAT-062: Original mirror pane implementation

## Discovery Context

Found during Session 14 QA while attempting to monitor bug-065-worker agent via mirror pane. Mirror was created correctly in session-0 but showed no output from the worker session.

## Resolution

Fixed in Session 15 by implementing cross-session output forwarding in two places:

### 1. Output Forwarding (`fugue-server/src/pty/output.rs`)

Added logic to the `flush()` method of `PtyOutputPoller` to check if the source pane has any mirrors in other sessions. When forwarding output:
- Query the `MirrorRegistry` for all mirrors of the source pane
- For each mirror in a different session, create an `Output` message with the **mirror's pane_id**
- Broadcast to the mirror's session so the TUI routes it correctly

### 2. Initial Scrollback (`fugue-server/src/handlers/pane.rs`)

Added logic to `handle_create_mirror()` to copy existing scrollback content when a mirror is created:
- Read the source pane's scrollback lines
- Send as an initial `Output` message to the mirror's session
- Ensures mirror shows existing content, not just new output

### Tests Added

- `test_bug066_cross_session_mirror_output_forwarding`: Verifies output is forwarded to mirror panes in other sessions with correct pane_id
- `test_bug066_same_session_mirror_no_duplicate`: Ensures same-session mirrors don't receive duplicate forwarding
