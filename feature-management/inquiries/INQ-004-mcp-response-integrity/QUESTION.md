# INQ-004: MCP Response Integrity

## Core Question

Why does the MCP bridge return unexpected/mismatched responses, and what architectural changes are needed to guarantee response integrity?

## Background

### Current State

The MCP bridge uses a message-passing architecture between:
- Claude Code (MCP client) → MCP bridge → fugue daemon

BUG-064 and BUG-065 were filed and "fixed" to address response mixing:
- BUG-064: Added `drain_pending_messages()` after timeout
- BUG-065: Added `request_lock` mutex to serialize daemon requests

### The Problem

Despite these fixes, response integrity issues persist. Observed in Session 18:

**Symptom 1: Unexpected response types**
```
MCP error -32603: Unexpected response: PaneResized { pane_id: ... }
MCP error -32603: Unexpected response: SessionList { sessions: [...] }
MCP error -32603: Unexpected response: PaneContent { pane_id: ..., content: ... }
```

When calling `fugue_kill_session`, the bridge receives `PaneResized`, `SessionList`, or `PaneContent` messages instead of the expected `SessionKilled` response.

**Symptom 2: Session name mismatch**
```
Called: fugue_kill_session(session="midwestmtg-orchestrator")
Result: { session_name: "midwestmtg-claude-worker", success: true }
```

The kill command executed successfully but killed a different session than requested.

**Symptom 3: Parallel call interference**
Running multiple MCP calls in parallel (even with the request_lock) causes responses to be delivered to wrong callers.

### Root Cause Hypotheses

1. **Broadcast messages polluting response channel**: The daemon broadcasts events (PaneResized, etc.) to all listeners. These may be queued in the same channel as request/response messages.

2. **Request-response correlation missing**: No request ID to match responses to their originating requests.

3. **Session resolution race**: When multiple sessions match (or nearly match), resolution may be non-deterministic.

4. **Lock granularity insufficient**: The request_lock may not cover the full request-response cycle.

## Research Areas

### 1. Message Channel Architecture
- How are broadcast messages vs request/response messages handled?
- Is there channel separation, or do they share a path?
- Where does the mixing occur (daemon, bridge, or client)?

### 2. Request-Response Correlation
- Current: No correlation mechanism
- Options: Request IDs, tagged responses, separate channels
- What do other MCP implementations use?

### 3. Session Resolution
- How does `kill_session` resolve session names to IDs?
- Is resolution deterministic under concurrent access?
- What happens with partial name matches?

### 4. Lock and Synchronization Review
- Review the `request_lock` implementation
- Does it actually serialize the full round-trip?
- Are there async gaps where interleaving can occur?

### 5. Broadcast Filtering
- Should MCP clients filter out broadcast messages?
- Should broadcasts use a separate channel?
- Impact on event-driven features (status pane, mirrors)?

## Observed Evidence

**Session 18 Log:**
1. Called `fugue_list_sessions` - worked
2. Called `fugue_list_panes` - got `PaneResized` error
3. Retried `fugue_list_panes` - worked
4. Called `fugue_read_pane` - got `AllPanesList` error
5. Retried `fugue_read_pane` - worked
6. Called 3x `fugue_kill_session` in parallel - all got unexpected responses
7. Retried 3x sequentially - first got `PaneContent`, second got `PaneResized`, third got `SessionList`
8. Retried one at a time - worked but killed wrong session first two times

**Pattern:** Retrying eventually succeeds, suggesting stale messages draining over multiple attempts.

## Constraints

1. **Must not break working features**: Many orchestration tools work reliably
2. **Backward compatible**: Can't change MCP protocol arbitrarily
3. **Performance**: Solution can't add significant latency
4. **Simplicity**: Prefer simple fixes over architectural rewrites

## Success Criteria

- Identify definitive root cause with reproduction steps
- Propose fix that eliminates response mixing
- Verify fix handles parallel MCP calls correctly
- No regression in existing functionality
