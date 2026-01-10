# Implementation Plan: BUG-011

**Work Item**: [BUG-011: Large Paste Input Crashes Session](PROMPT.md)
**Component**: ccmux-client / ccmux-server
**Priority**: P2
**Created**: 2026-01-10

## Overview

Large paste input causes ccmux session to crash. The input path flows from TUI client through Unix socket to server and finally to PTY. The crash could occur at any point in this pipeline when the input size exceeds some limit or overwhelms a buffer.

## Architecture Decisions

### Approach: To Be Determined

After investigation, choose from:

1. **Input Chunking**: Break large pastes into smaller chunks before sending
2. **Size Limits with Error Handling**: Reject pastes over a threshold with user feedback
3. **Streaming Protocol**: Implement streaming for large payloads
4. **Combination**: Size limit + chunking for allowed range

### Trade-offs

| Option | Pros | Cons |
|--------|------|------|
| Chunking | Full paste delivered, no user action needed | Complex implementation, may be slow |
| Size Limits | Simple to implement, predictable behavior | Limits functionality, user frustration |
| Streaming | Most robust, handles any size | Major protocol changes |
| Truncation | Simple, immediate | Loses data, user confusion |

**Decision**: TBD after investigation identifies root cause.

## Data Flow Analysis

```
+-------------+     +-----------------+     +-------------+     +-----+
| TUI Client  | --> | Unix Socket     | --> | Server      | --> | PTY |
| (crossterm) |     | (bincode msgs)  |     | (handlers)  |     |     |
+-------------+     +-----------------+     +-------------+     +-----+
     |                    |                      |                 |
     v                    v                      v                 v
  Paste event       Serialize &           Deserialize &      Write to
  from terminal     frame message         route to pane      PTY master
```

### Potential Failure Points

| Location | Failure Mode | Symptom |
|----------|--------------|---------|
| Client | OOM allocating paste buffer | Client crash |
| Client | Stack overflow processing | Client crash |
| Socket | Message too large for framing | Write error/crash |
| Socket | Bincode size limit exceeded | Serialization panic |
| Server | OOM deserializing | Server crash |
| Server | Handler overwhelmed | Server crash |
| PTY | Write buffer full | Block/timeout |
| PTY | Kernel buffer exceeded | EAGAIN or crash |

## Files to Investigate

| File | Purpose | Risk Level |
|------|---------|------------|
| `ccmux-client/src/input/mod.rs` | Client input handling | High |
| `ccmux-client/src/input/keys.rs` | Keystroke processing | Medium |
| `ccmux-client/src/socket.rs` | Client socket communication | High |
| `ccmux-protocol/src/lib.rs` | Message definitions | High |
| `ccmux-protocol/src/frame.rs` | Message framing (if exists) | High |
| `ccmux-server/src/handlers/mod.rs` | Server message handlers | High |
| `ccmux-server/src/pty/mod.rs` | PTY management | High |

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Root cause hard to identify | Medium | High | Add tracing/logging |
| Fix introduces new bugs | Low | High | Comprehensive testing |
| Fix causes performance regression | Medium | Medium | Benchmark before/after |
| Chunking complicates protocol | Medium | Medium | Keep changes minimal |

## Rollback Strategy

If implementation causes issues:
1. Revert commits associated with this work item
2. The crash behavior was the baseline - reverting returns to that
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
