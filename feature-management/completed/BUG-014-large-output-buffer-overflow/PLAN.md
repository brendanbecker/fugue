# Implementation Plan: BUG-014

**Work Item**: [BUG-014: Large Output Buffer Overflow](PROMPT.md)
**Component**: terminal-buffer
**Priority**: P1
**Created**: 2026-01-10

## Overview

Large output from Claude (e.g., extensive README diffs) causes the viewport/buffer to overflow, making input unresponsive. The session continues running and detach works, but reattaching shows the same unresponsive state. This indicates a persistent buffer capacity issue on the server or in the terminal state.

## Architecture Decisions

### Approach: To Be Determined

After investigation, choose from:

1. **Scrollback Limit with Ring Buffer**: Implement a configurable max scrollback that evicts old lines
2. **Output Throttling**: Rate-limit output delivery to prevent flooding
3. **Backpressure Protocol**: Add flow control between server and client
4. **Event Loop Fairness**: Ensure input events are processed between output batches
5. **Combination**: Multiple mechanisms working together

### Trade-offs

| Option | Pros | Cons |
|--------|------|------|
| Scrollback Limit | Simple, bounded memory | Loses history |
| Output Throttling | Predictable, maintains responsiveness | Delays visible output |
| Backpressure | Robust flow control | Protocol complexity |
| Event Loop Fairness | Addresses symptom directly | May not fix root cause |
| Incremental State | Efficient updates | Major refactor |

**Decision**: TBD after investigation identifies root cause.

## Data Flow Analysis

```
+-----+     +----------------+     +-----------+     +-------------+     +----------+
| PTY | --> | OutputPoller   | --> | Broadcast | --> | Unix Socket | --> | TUI      |
|     |     | (accumulates)  |     | (queue)   |     | (serialize) |     | (render) |
+-----+     +----------------+     +-----------+     +-------------+     +----------+
                                                                               |
                                                                               v
                                                                          +----------+
                                                                          | Screen   |
                                                                          | Buffer   |
                                                                          | (scroll) |
                                                                          +----------+
```

### Potential Failure Points

| Location | Failure Mode | Symptom |
|----------|--------------|---------|
| OutputPoller | Accumulates too much before flush | Memory growth |
| Broadcast Queue | Queue grows unboundedly | Memory exhaustion |
| Unix Socket | Write buffer backs up | Blocking writes |
| TUI Render | Can't keep up with output | Event loop starvation |
| Screen Buffer | Unbounded scrollback growth | OOM or slowdown |
| Terminal State | Server keeps full history | Memory exhaustion |

### Why Detach/Reattach Doesn't Help

This is a key observation. Possible explanations:

1. **Server-side state persists**: The terminal state/scrollback is stored on the server and the same bloated state is sent on reattach
2. **Buffer in server remains full**: Output messages are queued on the server and flood the client immediately on reattach
3. **State sync is the problem**: The initial state sync on attach is too large to process

## Files to Investigate

| File | Purpose | Risk Level |
|------|---------|------------|
| `ccmux-server/src/pty/output.rs` | PTY output polling | High |
| `ccmux-server/src/handlers/` | Broadcast handling | High |
| `ccmux-client/src/ui/app.rs` | Main TUI loop | High |
| `ccmux-client/src/terminal/` | Terminal emulator (if exists) | High |
| Screen buffer implementation | Scrollback storage | Critical |
| Attach handler | State sync on attach | High |

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Root cause hard to identify | Medium | High | Systematic investigation with metrics |
| Fix causes output delay | Medium | Medium | Configurable limits |
| Loss of scrollback history | High | Low | Make limit configurable, document |
| Performance regression | Low | Medium | Benchmark before/after |
| Incomplete fix (multiple causes) | Medium | High | Address each cause found |

## Rollback Strategy

If implementation causes issues:
1. Revert commits associated with this work item
2. The unresponsive behavior was the baseline - reverting returns to that
3. Consider alternative approach based on lessons learned

## Implementation Notes

<!-- Add notes during implementation -->

### Investigation Findings

*To be filled during investigation*

### Root Cause

*To be identified*

### Chosen Solution

*To be determined after investigation*

---
*This plan should be updated as implementation progresses.*
