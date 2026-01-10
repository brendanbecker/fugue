# BUG-011: Large Paste Input Crashes ccmux Session

**Priority**: P2 (Medium)
**Component**: ccmux-client / ccmux-server
**Status**: new
**Created**: 2026-01-10

## Summary

Pasting an extremely large amount of text into a ccmux terminal session causes the session to crash. There is no graceful handling or error message - the session simply dies.

## Symptoms

- Session crash on large paste
- No graceful handling or error message
- Requires session reattachment after crash
- Potentially loses unsaved work

## Data Flow to Investigate

The input flow in ccmux is:

```
TUI client -> Unix socket -> Server -> PTY
```

Key components in this path:

1. **TUI Client Input Handling** (`ccmux-client/src/input/`)
   - Receives paste events from terminal
   - Converts to `PtyInput` messages
   - Sends over Unix socket

2. **Unix Socket Protocol** (`ccmux-protocol/`)
   - Uses bincode serialization
   - May have message size limits
   - Framing/length-prefixed messages

3. **Server Message Handler** (`ccmux-server/src/handlers/`)
   - Receives `PtyInput` messages
   - Routes to appropriate pane

4. **PTY Write** (`ccmux-server/src/pty/`)
   - Writes input to PTY master
   - May have buffer limitations

## Likely Causes to Investigate

### 1. Buffer Overflow in Input Handling Path

The client may be creating an extremely large buffer when processing paste input without any size limits.

**Files to check:**
- `ccmux-client/src/input/mod.rs`
- `ccmux-client/src/input/keys.rs`

### 2. Message Size Limit Exceeded on Unix Socket Protocol

The protocol may have implicit or explicit message size limits. Bincode serialization of a huge payload could fail or the socket write could fail.

**Files to check:**
- `ccmux-protocol/src/lib.rs`
- Socket framing code (length-prefixed messages)

### 3. PTY Write Buffer Overwhelmed (No Chunking)

Writing a massive amount of data to the PTY master in a single write could overwhelm the kernel buffer or block indefinitely.

**Files to check:**
- `ccmux-server/src/pty/mod.rs`
- `ccmux-server/src/pty/output.rs`
- PTY write handling code

### 4. Bincode Serialization Failing on Huge Payloads

Bincode may have limitations on payload size or may allocate excessive memory trying to serialize/deserialize large messages.

**Files to check:**
- Serialization/deserialization code
- Message types in `ccmux-protocol/`

### 5. Memory Exhaustion from Allocating Large Input Buffer

The system may try to allocate a buffer for the entire paste content at once, causing memory exhaustion.

**Files to check:**
- All places where input is buffered

## Acceptance Criteria

- [ ] Root cause identified and documented
- [ ] Large pastes (>1MB) handled gracefully without crashing
- [ ] Either chunking implemented OR clear error message displayed
- [ ] User can continue working after a failed large paste
- [ ] Session remains stable and attached
- [ ] PTY/shell receives as much of the paste as is reasonable
- [ ] Add test case to prevent regression

## Implementation Tasks

### Section 1: Investigation

- [ ] Reproduce the crash with a large paste
- [ ] Identify exactly where the crash occurs (client, socket, server, or PTY)
- [ ] Add logging/tracing to pinpoint failure location
- [ ] Document root cause in PLAN.md

### Section 2: Fix Implementation

Based on root cause, implement appropriate fix:

#### If buffer overflow:
- [ ] Add size limits on input buffers
- [ ] Implement chunking for large inputs

#### If socket/protocol issue:
- [ ] Implement message size limits with proper error handling
- [ ] Add chunked message protocol for large payloads

#### If PTY write issue:
- [ ] Implement chunked PTY writes
- [ ] Add backpressure handling

#### If serialization issue:
- [ ] Add size checks before serialization
- [ ] Implement streaming serialization for large payloads

#### General:
- [ ] Add graceful error handling
- [ ] Display user-friendly error message when paste is too large

### Section 3: Testing

- [ ] Add unit test for large input handling
- [ ] Add integration test for large paste scenario
- [ ] Manual test with various paste sizes (1MB, 10MB, 100MB)
- [ ] Verify session remains stable after failed large paste
- [ ] Test that reasonable-sized pastes still work

### Section 4: Verification

- [ ] Confirm no crash on large paste
- [ ] Verify graceful degradation or chunked delivery
- [ ] All acceptance criteria met
- [ ] Update bug_report.json with resolution details

## Notes

This is a P2 bug because:
- It does not block core functionality
- It is an edge case (large pastes are not common)
- However, it causes a very poor user experience when it occurs
- Session crash can cause loss of work

The fix should prioritize graceful handling over perfect delivery of large content. It's acceptable to:
- Truncate extremely large pastes with a warning
- Chunk and slow-deliver large pastes
- Reject pastes over a configurable threshold with an error message

The key is that the session must not crash and the user must receive feedback about what happened.
