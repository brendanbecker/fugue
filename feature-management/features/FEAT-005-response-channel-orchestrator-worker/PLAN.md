# Implementation Plan: FEAT-005

**Work Item**: [FEAT-005: Response Channel for Orchestrator-Worker Communication](PROMPT.md)
**Component**: orchestration
**Priority**: P1
**Created**: 2026-01-08

## Overview

Orchestrator can respond to worker input prompts without switching panes using `/reply` command. Only delivers to sessions in input-wait state (AwaitingConfirmation). Not an interrupt mechanism.

## Architecture Decisions

### Protocol Message Design

The ReplyMessage should be minimal and focused:

```rust
pub struct ReplyMessage {
    pub target: PaneTarget,  // Name or UUID
    pub content: String,
    pub newline: bool,       // Whether to append newline (default: true)
}

pub enum PaneTarget {
    ByName(String),
    ById(Uuid),
}
```

### State Query Approach

Two options for checking input-wait state:

**Option A: Query-then-send (chosen)**
- Client requests pane state before sending
- Clear error messages
- Slight race condition window

**Option B: Atomic send-if-waiting**
- Server checks state and sends atomically
- No race condition
- Less informative errors

**Decision**: Start with Option A for clarity, migrate to B if race conditions are problematic.

### Command Syntax

```
/reply <pane-id> <message>
/reply worker-3 "use async, we need non-blocking"
/reply worker-3 use async approach    # Quotes optional for simple messages
```

Pane identification:
- By name: `worker-3`, `main`, `orchestrator`
- By short UUID: `a1b2c3d4` (first 8 chars)
- By full UUID: `a1b2c3d4-e5f6-...`

## Affected Components

| Component | Type of Change | Risk Level |
|-----------|----------------|------------|
| ccmux-protocol/src/messages.rs | Add new message type | Low |
| ccmux-protocol/src/types.rs | Add PaneTarget enum | Low |
| ccmux-protocol/src/lib.rs | Export new types | Low |
| ccmux-client/src/commands.rs | Add /reply command parser | Medium |
| ccmux-server/src/session/pane.rs | Add input-wait detection | Medium |
| ccmux-server/src/session/manager.rs | Route reply messages | Medium |

## Implementation Sequence

### Phase 1: Protocol Layer (Low Risk)

1. Add `PaneTarget` enum to types.rs
2. Add `ReplyMessage` struct to messages.rs
3. Add `ReplyResponse` for server acknowledgment
4. Update lib.rs exports
5. Add serialization tests

### Phase 2: Server State Query (Medium Risk)

1. Add `is_awaiting_input(&self) -> bool` to Pane
2. Add `get_claude_activity(&self) -> Option<ClaudeActivity>` to Pane
3. Ensure ClaudeState is kept up-to-date during pane operation
4. Add query endpoint for pane state

### Phase 3: Server Message Routing (Medium Risk)

1. Handle ReplyMessage in session manager
2. Resolve PaneTarget to actual pane
3. Validate pane is in AwaitingConfirmation state
4. Write to pane's stdin via PTY handle
5. Return ReplyResponse to client

### Phase 4: Client Command (Medium Risk)

1. Parse `/reply` command in command handler
2. Extract pane target and message content
3. Send ReplyMessage to server
4. Display response (success or error)

### Phase 5: Testing & Polish

1. Unit tests for all new types
2. Integration test: successful reply flow
3. Integration test: reply to non-waiting pane (error)
4. Integration test: reply to non-existent pane (error)
5. Documentation updates

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Race condition: pane exits wait state between check and send | Low | Low | Atomic check-and-send in Phase 2 if needed |
| PTY write failure | Low | Medium | Return clear error, don't crash |
| Pane name collision | Medium | Low | Support UUID fallback |
| Message too long | Low | Low | Truncate with warning |

## Rollback Strategy

If implementation causes issues:
1. Revert commits associated with this work item
2. Protocol changes are additive, won't break existing clients
3. Verify system returns to previous state
4. Document what went wrong in comments.md

## Open Questions

1. Should `/reply` support multiline messages?
   - Proposal: Yes, via heredoc syntax or quoted strings

2. Should we support reply queueing?
   - Proposal: Defer to optional enhancement phase

3. What happens if stdin write is partial?
   - Proposal: Retry once, then fail with error

## Implementation Notes

<!-- Add notes during implementation -->

---
*This plan should be updated as implementation progresses.*
