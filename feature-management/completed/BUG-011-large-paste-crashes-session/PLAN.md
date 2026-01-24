# Implementation Plan: BUG-011

**Work Item**: [BUG-011: Large Paste Input Crashes Session](PROMPT.md)
**Component**: fugue-client / fugue-server
**Priority**: P2
**Created**: 2026-01-10

## Overview

Large paste input causes fugue session to crash. The input path flows from TUI client through Unix socket to server and finally to PTY. The crash could occur at any point in this pipeline when the input size exceeds some limit or overwhelms a buffer.

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
| `fugue-client/src/input/mod.rs` | Client input handling | High |
| `fugue-client/src/input/keys.rs` | Keystroke processing | Medium |
| `fugue-client/src/socket.rs` | Client socket communication | High |
| `fugue-protocol/src/lib.rs` | Message definitions | High |
| `fugue-protocol/src/frame.rs` | Message framing (if exists) | High |
| `fugue-server/src/handlers/mod.rs` | Server message handlers | High |
| `fugue-server/src/pty/mod.rs` | PTY management | High |

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

**Code Flow Analysis:**

1. **Client Paste Handling** (`fugue-client/src/input/mod.rs:232-235`):
   ```rust
   Event::Paste(text) => InputAction::SendToPane(text.into_bytes())
   ```
   - No size validation - entire paste is converted to bytes immediately

2. **App Input Handling** (`fugue-client/src/ui/app.rs:322-327`):
   ```rust
   InputAction::SendToPane(data) => {
       self.connection.send(ClientMessage::Input { pane_id, data }).await?;
   }
   ```
   - Sends entire paste as a single `ClientMessage::Input`

3. **Connection Task** (`fugue-client/src/connection/client.rs:154-158`):
   ```rust
   if let Err(e) = framed.send(msg).await {
       tracing::error!("Failed to send message: {}", e);
       break;  // <-- CONNECTION BREAKS HERE
   }
   ```
   - On codec error, breaks the entire connection task causing session disconnect

4. **Protocol Codec** (`fugue-protocol/src/codec.rs:129-137`):
   - Max message size: 16 MB (`MAX_MESSAGE_SIZE = 16 * 1024 * 1024`)
   - Returns `CodecError::MessageTooLarge` if exceeded

5. **Server PTY Write** (`fugue-server/src/pty/handle.rs:47-52`):
   - `write_all()` attempts to write entire buffer at once
   - No chunking - could block if kernel PTY buffer is full

### Root Cause

**Three interconnected issues:**

1. **No client-side input size validation** - Entire paste loads into memory and sent as single message
2. **Codec error breaks connection** - When message exceeds 16MB, the connection task breaks with no recovery
3. **No user feedback** - Error is only logged, user sees session disconnect with no explanation

**Crash Scenario:**
- User pastes >16MB content
- Client creates `ClientMessage::Input` with entire payload
- Codec's `encode_message()` returns `CodecError::MessageTooLarge`
- Connection task logs error and breaks the loop
- Client connection closes
- Session appears to "crash" (disconnect) with no explanation

### Chosen Solution

**Approach: Client-side Input Chunking + Graceful Error Handling**

This is the best approach because:
- Preserves full paste delivery for reasonable sizes
- Provides graceful degradation for extreme cases
- No protocol changes required
- Maintains backward compatibility

**Implementation:**

1. **Add `MAX_INPUT_CHUNK_SIZE` constant** in client (64KB - well under 16MB limit)
2. **Chunk large pastes** in `handle_input_action()` before sending
3. **Add soft limit warning** for very large pastes (>10MB) with user confirmation option
4. **Improve error handling** in connection task to not break on recoverable errors
5. **Add user feedback** when paste is rejected or chunked

**Key Files to Modify:**
- `fugue-client/src/ui/app.rs` - Chunk input in `handle_input_action()`
- `fugue-client/src/connection/client.rs` - Improve error handling (don't break on all errors)
- `fugue-client/src/input/mod.rs` - Add size constants

**Acceptance Criteria Mapping:**
- [x] Root cause identified and documented
- [x] Large pastes (>1MB) handled gracefully without crashing
- [x] Either chunking implemented OR clear error message displayed
- [x] User can continue working after a failed large paste
- [x] Session remains stable and attached
- [x] PTY/shell receives as much of the paste as is reasonable
- [x] Add test case to prevent regression

### Implementation Complete

**Changes made:**

1. **`fugue-client/src/ui/app.rs`**:
   - Added `MAX_INPUT_CHUNK_SIZE` (64KB) and `MAX_PASTE_SIZE` (10MB) constants
   - Modified `handle_input_action()` to chunk large pastes and reject extremely large ones
   - Shows user feedback: status message for large pastes being chunked or rejected
   - Added 9 unit tests for chunking logic

**Behavior:**
- Pastes <= 64KB: Sent as single message (unchanged)
- Pastes > 64KB and <= 10MB: Chunked into 64KB segments, sent sequentially
- Pastes > 10MB: Rejected with user-friendly error message
- User feedback: Status bar shows progress for large pastes (>1MB) and error for rejected pastes

---
*This plan should be updated as implementation progresses.*
