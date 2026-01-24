# FEAT-005: Response Channel for Orchestrator-Worker Communication

**Priority**: P1
**Component**: orchestration
**Type**: new_feature
**Estimated Effort**: medium
**Business Value**: high

## Overview

Orchestrator can respond to worker input prompts without switching panes:

```
/reply worker-3 "use async, we need non-blocking"
```

Only delivers to sessions in input-wait state. Not an interrupt mechanism.

## Benefits

- Enables seamless orchestrator-worker communication without context switching
- Improves multi-agent workflow efficiency
- Allows orchestrator to remain focused while providing input to workers
- Reduces cognitive overhead in complex multi-pane sessions

## Requirements

1. **Detect input-wait state in Claude sessions** (AwaitingConfirmation in ClaudeActivity)
2. **`/reply <pane-id> <message>` command syntax**
3. **Route message to target pane's stdin**
4. **Error if target not in input-wait state**
5. **Show confirmation in orchestrator pane**
6. **Optional**: queue replies for panes not yet waiting

## Current State

- Sideband protocol designed (`<fugue:input target="...">`) but not implemented
- ClaudeActivity enum includes AwaitingConfirmation state (in `fugue-protocol/src/types.rs`)
- Parent-child tracking documented (parent_pane field)
- Parsing code is pseudocode/stubs only

## Affected Files

| File | Change Type | Description |
|------|-------------|-------------|
| `fugue-client/src/commands.rs` | Add | /reply command handler |
| `fugue-server/src/session/pane.rs` | Modify | Input-wait detection logic |
| `fugue-protocol/src/lib.rs` | Add | Reply message type |

## Implementation Tasks

### Section 1: Protocol Layer
- [ ] Add ReplyMessage type to fugue-protocol
- [ ] Define message format: target pane ID + message content
- [ ] Add serialization/deserialization support
- [ ] Update protocol exports in lib.rs

### Section 2: Server-Side Input-Wait Detection
- [ ] Implement input-wait state query in pane.rs
- [ ] Expose ClaudeActivity state via pane info
- [ ] Add method to check if pane is in AwaitingConfirmation state
- [ ] Handle edge cases (pane not found, not a Claude session)

### Section 3: Client-Side Command
- [ ] Parse `/reply <pane-id> <message>` command syntax
- [ ] Validate pane-id format and existence
- [ ] Send ReplyMessage to server
- [ ] Display confirmation or error in orchestrator pane

### Section 4: Server-Side Message Routing
- [ ] Receive ReplyMessage from client
- [ ] Validate target pane is in input-wait state
- [ ] Write message to target pane's stdin
- [ ] Return success/failure status to client

### Section 5: Testing
- [ ] Unit tests for ReplyMessage serialization
- [ ] Unit tests for input-wait detection
- [ ] Integration tests for /reply command flow
- [ ] Error case tests (invalid pane, not waiting)

### Section 6: Optional Enhancement
- [ ] Design queue mechanism for pending replies
- [ ] Implement reply queuing when pane not yet waiting
- [ ] Add timeout/expiration for queued replies
- [ ] Document queue behavior

## Acceptance Criteria

- [ ] `/reply worker-3 "message"` sends message to worker-3's stdin
- [ ] Command fails with clear error if target pane not in input-wait state
- [ ] Orchestrator sees confirmation of successful delivery
- [ ] Works with pane names and pane IDs
- [ ] No message delivery to panes not awaiting input
- [ ] Tests cover happy path and error cases
- [ ] Documentation updated with /reply command usage

## Technical Notes

### ClaudeActivity Enum (from fugue-protocol/src/types.rs)

```rust
pub enum ClaudeActivity {
    Idle,
    Thinking,
    Coding,
    ToolUse,
    AwaitingConfirmation,
}
```

### Sideband Protocol Reference (from ADR-002)

The `<fugue:input>` tag was designed for Claude-initiated input:

```xml
<fugue:input to="worker-1">
use async approach
</fugue:input>
```

This feature implements the complementary direction: user/orchestrator to worker.

### Parent-Child Tracking

The `parent_pane: Option<Uuid>` field in pane metadata enables:
- Identifying which pane spawned a worker
- Notifying parent when child completes
- Routing replies back through the hierarchy

## Dependencies

None - this is a foundational orchestration feature.

## Notes

- Consider rate limiting to prevent reply spam
- May want to add `/reply-all` for broadcasting to all waiting panes
- Future: integrate with MCP for structured reply format
