# INQ-006: Inter-Agent Communication Consolidation

## Core Question

How should fugue consolidate its multiple inter-agent communication mechanisms into a reliable, coherent system?

## Background

### Current State

fugue has accumulated multiple overlapping mechanisms for agent-to-agent communication:

| Mechanism | Purpose | Status |
|-----------|---------|--------|
| `fugue_send_orchestration` | Send messages by tag/session/broadcast | Unreliable (BUG-069) |
| `fugue_report_status` | Convenience wrapper for status to orchestrator | Depends on above |
| `fugue_request_help` | Convenience wrapper for help requests | Depends on above |
| `fugue_poll_messages` | Receive queued messages | May not receive sent messages |
| `fugue_broadcast` | Broadcast to all sessions | Unknown reliability |
| Tag-based routing | Route by session tags | Partial functionality |
| Native watchdog timer | Periodic check messages | Working but limited |

### The Problem

1. **No mechanism is end-to-end reliable** - Messages claim delivery but aren't received
2. **Overlapping abstractions** - Multiple ways to do the same thing
3. **Unclear mental model** - When to use which mechanism?
4. **Testing gaps** - No integration tests proving full workflows work
5. **Documentation drift** - AGENTS.md describes patterns that don't work

### Observed Failures

From BUG-069:
```
fugue_send_orchestration returns:
  {"delivered_count": 1, "success": true}

But fugue_poll_messages returns: nothing
```

The message is "delivered" but never receivable.

## Research Areas

### 1. Current Implementation Audit

- Trace `fugue_send_orchestration` from MCP handler to delivery
- Trace `fugue_poll_messages` from queue to response
- Identify where messages are lost
- Document the actual message flow vs intended flow

### 2. Message Queue Analysis

- How are messages queued per-session?
- Is there a single queue or multiple?
- Are messages being overwritten/cleared?
- Is `poll_messages` reading from the right queue?

### 3. Consolidation Options

**Option A: Fix existing mechanisms**
- Debug and fix current implementation
- Keep all existing tools
- Document when to use each

**Option B: Single unified mechanism**
- One `fugue_message` tool for all communication
- Deprecate convenience wrappers
- Simplify mental model

**Option C: Pub/Sub model**
- Sessions subscribe to topics/channels
- Publishers don't need to know receivers
- Built-in message persistence

**Option D: External integration**
- Integrate mcp-agent-mail or similar
- Delegate messaging to specialized system
- fugue focuses on terminal multiplexing

### 4. Reliability Requirements

- Message delivery guarantees (at-least-once, exactly-once?)
- Persistence across daemon restarts
- Ordering guarantees
- Timeout/retry semantics
- Dead letter handling

### 5. Testing Strategy

- Integration tests for full send→receive workflows
- Cross-session message delivery tests
- Load/stress testing for message throughput
- Chaos testing (daemon restart during message flow)

## Related Work Items

- **BUG-069**: Orchestration messages claim delivery but aren't received
- **BUG-060**: Orchestration tools session attachment
- **BUG-061**: send_orchestration target parsing
- **INQ-003**: Hierarchical orchestration messaging (scope overlap)

## Constraints

1. **Must work reliably** - No false "success" responses
2. **Must be testable** - Integration tests proving end-to-end flow
3. **Must be simple** - Clear mental model for when to use what
4. **Backward compatible** - Existing tools continue to work or have migration path
5. **Performance** - Low latency for real-time orchestration

## Research Agent Assignments

### Agent 1: Implementation Audit
- Trace message flow through codebase
- Identify where BUG-069 messages are lost
- Document actual vs expected behavior
- Files: `fugue-server/src/mcp/bridge/handlers.rs`, session management

### Agent 2: Architecture Options
- Evaluate consolidation options A-D
- Research mcp-agent-mail and alternatives
- Propose recommended architecture
- Consider migration path

### Agent 3: Testing & Reliability
- Design integration test suite
- Define reliability guarantees
- Propose monitoring/observability
- Create test plan for validation

## Success Criteria

- Root cause of BUG-069 identified
- Clear recommendation on consolidation approach
- Integration test suite specification
- Spawned FEAT(s) or BUG fixes with implementation specs
- Updated AGENTS.md with working patterns
