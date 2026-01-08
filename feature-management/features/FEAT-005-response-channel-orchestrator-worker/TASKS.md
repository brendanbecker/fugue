# Task Breakdown: FEAT-005

**Work Item**: [FEAT-005: Response Channel for Orchestrator-Worker Communication](PROMPT.md)
**Status**: Not Started
**Last Updated**: 2026-01-08

## Prerequisites

- [ ] Read and understand PROMPT.md
- [ ] Review PLAN.md and update if needed
- [ ] Review ClaudeActivity enum in ccmux-protocol/src/types.rs
- [ ] Review sideband protocol design in docs/architecture/ADR/002-claude-communication.md
- [ ] Review existing pane.rs implementation

## Phase 1: Protocol Layer

- [ ] Add PaneTarget enum to ccmux-protocol/src/types.rs
  ```rust
  pub enum PaneTarget {
      ByName(String),
      ById(Uuid),
  }
  ```
- [ ] Add ReplyMessage struct to ccmux-protocol/src/messages.rs
- [ ] Add ReplyResponse struct for acknowledgment
- [ ] Implement Serialize/Deserialize for new types
- [ ] Update ccmux-protocol/src/lib.rs exports
- [ ] Add unit tests for serialization round-trip

## Phase 2: Server State Query

- [ ] Add `is_awaiting_input(&self) -> bool` method to Pane struct
- [ ] Add `get_claude_activity(&self) -> Option<ClaudeActivity>` method
- [ ] Ensure ClaudeState updates are propagated correctly
- [ ] Add tests for state query methods
- [ ] Handle edge case: non-Claude pane (raw shell)

## Phase 3: Server Message Routing

- [ ] Add ReplyMessage handler to session manager
- [ ] Implement PaneTarget resolution (name -> UUID lookup)
- [ ] Validate target pane exists
- [ ] Validate target pane is in AwaitingConfirmation state
- [ ] Get PTY writer for target pane
- [ ] Write message content to PTY stdin
- [ ] Optionally append newline
- [ ] Construct and return ReplyResponse
- [ ] Add error handling for write failures

## Phase 4: Client Command

- [ ] Add /reply command to command parser
- [ ] Parse pane target (name or UUID)
- [ ] Parse message content (quoted or unquoted)
- [ ] Construct ReplyMessage
- [ ] Send to server via existing connection
- [ ] Receive and display ReplyResponse
- [ ] Format success message for orchestrator pane
- [ ] Format error message with helpful context

## Phase 5: Testing

- [ ] Unit test: ReplyMessage serialization
- [ ] Unit test: PaneTarget serialization
- [ ] Unit test: ReplyResponse serialization
- [ ] Unit test: is_awaiting_input() true case
- [ ] Unit test: is_awaiting_input() false case
- [ ] Integration test: successful reply delivery
- [ ] Integration test: reply to non-waiting pane (expect error)
- [ ] Integration test: reply to non-existent pane (expect error)
- [ ] Integration test: reply by name vs by UUID

## Phase 6: Documentation

- [ ] Add /reply command to user documentation
- [ ] Document command syntax and examples
- [ ] Update ARCHITECTURE.md if needed
- [ ] Add troubleshooting section for common errors

## Optional: Reply Queueing (Defer)

- [ ] Design queue data structure
- [ ] Implement queue storage in session manager
- [ ] Add queue insertion when pane not waiting
- [ ] Add queue drain when pane enters AwaitingConfirmation
- [ ] Add timeout/expiration for queued messages
- [ ] Add /reply --queue flag or config option
- [ ] Document queue behavior

## Verification Tasks

- [ ] All acceptance criteria from PROMPT.md met
- [ ] All tests passing
- [ ] Update feature_request.json status
- [ ] Document completion in comments.md

## Completion Checklist

- [ ] All Phase 1-5 tasks complete
- [ ] All tests passing
- [ ] Documentation updated
- [ ] PLAN.md reflects final implementation
- [ ] Ready for review/merge

---
*Check off tasks as you complete them. Update status field above.*
