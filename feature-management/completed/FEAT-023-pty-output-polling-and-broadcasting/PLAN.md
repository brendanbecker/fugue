# Implementation Plan: FEAT-023

**Work Item**: [FEAT-023: PTY Output Polling and Broadcasting](PROMPT.md)
**Component**: fugue-server
**Priority**: P0 (Critical)
**Created**: 2026-01-09

## Overview

Poll PTY output in background tasks and broadcast to connected clients. Spawn background tokio tasks per PTY handle, read from PtyHandle.reader() non-blocking, collect bytes into buffers, and send ServerMessage::Output to clients attached to the pane's session.

## Architecture Decisions

### Task-per-PTY Model

Each PTY handle gets its own dedicated tokio task:

```
PTY Handle 1 --> PtyOutputPoller Task 1 --> Broadcast Channel
PTY Handle 2 --> PtyOutputPoller Task 2 --> Broadcast Channel
PTY Handle N --> PtyOutputPoller Task N --> Broadcast Channel
                                                   |
                                                   v
                                         Client Router (FEAT-022)
                                                   |
                                    +------+-------+-------+
                                    |      |       |       |
                                    v      v       v       v
                                Client1 Client2 Client3 Client4
```

### Output Buffer Strategy

**Two flush triggers:**
1. **Newline flush**: When `\n` detected, flush complete lines immediately
2. **Timeout flush**: After configurable timeout (~50ms), flush partial buffer

```rust
struct OutputBuffer {
    data: Vec<u8>,
    max_size: usize,        // Default 64KB
    flush_timeout: Duration, // Default 50ms
    last_flush: Instant,
}
```

### Message Format

```rust
// In fugue-protocol/src/messages.rs
pub enum ServerMessage {
    // ... existing variants ...
    Output {
        pane_id: PaneId,
        data: Vec<u8>,      // Raw bytes, may contain ANSI
    },
    PtyEof {
        pane_id: PaneId,
    },
}
```

### Task Lifecycle

```
Pane Created
     |
     v
PTY Spawned (FEAT-013)
     |
     v
spawn_output_poller(pty_handle, pane_id, broadcast_tx)
     |
     +---> PtyOutputPoller task runs
     |          |
     |          +---> Read from PTY
     |          |
     |          +---> Buffer bytes
     |          |
     |          +---> Flush on newline/timeout
     |          |
     |          +---> Send to broadcast channel
     |          |
     |          +---> (repeat)
     |
     v
PTY EOF detected
     |
     v
Send PtyEof message
     |
     v
Task terminates
```

## Affected Components

| Component | Type of Change | Risk Level |
|-----------|----------------|------------|
| fugue-server/src/pty/output.rs | New - Output polling module | Medium |
| fugue-server/src/pty/mod.rs | Modify - Export output module | Low |
| fugue-server/src/server/mod.rs | Modify - Integrate polling | Medium |
| fugue-protocol/src/messages.rs | Modify - Add Output message | Low |

## Dependencies

- **FEAT-021**: Server Socket - Client connection handling
- **FEAT-022**: Message Routing - Client session tracking for output routing

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Output loss on high throughput | Medium | High | Batch reads, adequate buffer size |
| Memory growth with slow clients | Medium | Medium | Max buffer size, backpressure |
| Task leaks on error | Low | Medium | Proper cleanup with Drop |
| Race condition on pane close | Low | Medium | Cancellation token coordination |
| Output corruption | Low | High | No mutation of data, pass-through |

## Implementation Phases

### Phase 1: Core Polling Infrastructure

Create `fugue-server/src/pty/output.rs`:

```rust
use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;

pub struct PtyOutputPoller {
    pane_id: PaneId,
    reader: PtyReader,
    broadcast_tx: broadcast::Sender<ServerMessage>,
    cancel_token: CancellationToken,
    buffer: OutputBuffer,
}

impl PtyOutputPoller {
    pub fn spawn(
        pane_id: PaneId,
        reader: PtyReader,
        broadcast_tx: broadcast::Sender<ServerMessage>,
    ) -> (JoinHandle<()>, CancellationToken) {
        let cancel_token = CancellationToken::new();
        let poller = Self { /* ... */ };

        let handle = tokio::spawn(poller.run());
        (handle, cancel_token)
    }

    async fn run(mut self) {
        loop {
            tokio::select! {
                _ = self.cancel_token.cancelled() => break,
                result = self.read_with_timeout() => {
                    match result {
                        Ok(Some(bytes)) => self.handle_output(bytes),
                        Ok(None) => break, // EOF
                        Err(e) => { /* log and continue or break */ }
                    }
                }
            }
        }
        self.flush_final();
        self.send_eof();
    }
}
```

### Phase 2: Buffer Management

Implement intelligent buffering:

```rust
impl OutputBuffer {
    fn push(&mut self, data: &[u8]) -> Option<Vec<u8>> {
        self.data.extend_from_slice(data);

        // Check for newline flush
        if let Some(pos) = self.data.iter().rposition(|&b| b == b'\n') {
            let to_flush = self.data.drain(..=pos).collect();
            self.last_flush = Instant::now();
            return Some(to_flush);
        }

        // Check for size limit
        if self.data.len() >= self.max_size {
            return Some(std::mem::take(&mut self.data));
        }

        None
    }

    fn should_timeout_flush(&self) -> bool {
        !self.data.is_empty()
            && self.last_flush.elapsed() >= self.flush_timeout
    }

    fn flush(&mut self) -> Vec<u8> {
        self.last_flush = Instant::now();
        std::mem::take(&mut self.data)
    }
}
```

### Phase 3: Message Broadcasting

Integrate with server broadcast channel:

```rust
impl PtyOutputPoller {
    fn send_output(&self, data: Vec<u8>) {
        let msg = ServerMessage::Output {
            pane_id: self.pane_id,
            data,
        };
        // Ignore send errors (no receivers is ok)
        let _ = self.broadcast_tx.send(msg);
    }

    fn send_eof(&self) {
        let msg = ServerMessage::PtyEof {
            pane_id: self.pane_id,
        };
        let _ = self.broadcast_tx.send(msg);
    }
}
```

### Phase 4: Server Integration

Wire up in server:

```rust
// When creating a pane with PTY
let (reader, writer) = pty_handle.split();
let (poller_handle, cancel_token) = PtyOutputPoller::spawn(
    pane_id,
    reader,
    broadcast_tx.clone(),
);

// Store cancel_token for cleanup
pane.output_poller_cancel = Some(cancel_token);
```

### Phase 5: Testing

- Unit tests for OutputBuffer behavior
- Unit tests for PtyOutputPoller with mock PTY
- Integration tests with actual PTY
- Stress tests for high-throughput scenarios

## Rollback Strategy

If implementation causes issues:
1. Revert commits associated with this work item
2. Remove output.rs module from pty/
3. Remove ServerMessage::Output variant
4. Verify PTY spawning still works without output polling
5. Document what went wrong in comments.md

## Implementation Notes

<!-- Add notes during implementation -->

---
*This plan should be updated as implementation progresses.*
