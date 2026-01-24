# INQ-003: Hierarchical Orchestration Messaging

## Core Question

How should fugue handle multi-level orchestration hierarchies (overseer → orchestrators → workers) with proper message routing, and should we integrate with or learn from external solutions like mcp-agent-mail?

## Background

### Current State

fugue has orchestration messaging primitives:
- `fugue_report_status` - Workers report status to sessions tagged "orchestrator"
- `fugue_send_orchestration` - Send messages to targets by tag, session, or broadcast
- `fugue_poll_messages` - Check inbox for incoming messages
- Tag-based routing via `fugue_set_tags`

### The Problem

The current model is **flat**. When a worker calls `fugue_report_status`, it goes to ALL sessions tagged "orchestrator". This works for single-orchestrator scenarios but breaks with hierarchies:

```
overseer (manages multiple projects)
├── fugue-orchestrator (tagged: orchestrator)
│   ├── feat-105-worker (tagged: worker)
│   └── feat-106-worker (tagged: worker)
├── midwestmtg-orchestrator (tagged: orchestrator)
│   ├── midwestmtg-claude-worker (tagged: worker)
│   └── midwestmtg-gemini-worker (tagged: worker)
```

**Issues observed:**
1. midwestmtg workers report to fugue-orchestrator (wrong!)
2. Overseer gets flooded with low-level worker status spam
3. No parent-child relationship tracked
4. Can't distinguish overseer from project orchestrators

### External Context: mcp-agent-mail

Jeffrey Emanuel's `mcp-agent-mail` is an MCP server for agent-to-agent messaging. We should research:
- What patterns does it use?
- Could fugue integrate with it?
- Should we adopt similar concepts?
- Is there an emerging standard for MCP agent messaging?

GitHub: https://github.com/jwemanuel/mcp-agent-mail (verify URL)

## Research Areas

### 1. Parent-Child Tracking
- Should sessions track their spawning parent?
- Automatic vs explicit parent assignment?
- How to handle orphaned sessions?

### 2. Message Routing Patterns
- Direct (session-to-session)
- Hierarchical (up to parent, down to children)
- Broadcast (current flat model)
- Topic/channel based

### 3. Role Differentiation
- Current: just tags (orchestrator, worker)
- Proposed: overseer > orchestrator > worker hierarchy
- Or: flexible parent/child without fixed roles?

### 4. External Integration
- mcp-agent-mail analysis
- Other MCP messaging projects
- Interoperability considerations

### 5. Backward Compatibility
- How to preserve flat orchestration for simple cases
- Migration path for existing setups

## Constraints

1. **Must support 3+ levels** - overseer → orchestrator → worker minimum
2. **Scoped reporting** - Workers report to their parent, not all orchestrators
3. **Upward reporting** - Orchestrators can report to overseer
4. **Backward compatible** - Flat orchestration still works
5. **Interoperable** - Consider external MCP messaging ecosystem

## Research Agent Assignments

### Agent 1: Internal Architecture
- Analyze current fugue messaging implementation
- Propose parent-child tracking mechanisms
- Design hierarchical routing changes
- Consider performance implications

### Agent 2: External Solutions
- Research mcp-agent-mail thoroughly
- Find other MCP agent messaging projects
- Identify patterns and standards
- Evaluate integration vs inspiration

### Agent 3: Use Cases & UX
- Document orchestration use cases (2-level, 3-level, N-level)
- Design API/UX for hierarchical messaging
- Consider failure modes (orphaned workers, dead orchestrators)
- Propose migration path

## Success Criteria

- Clear recommendation on architecture approach
- Decision on mcp-agent-mail integration vs native solution
- Spawned FEAT(s) with implementation specs
- Backward compatibility plan
