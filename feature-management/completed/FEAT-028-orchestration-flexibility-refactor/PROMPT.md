# FEAT-028: Orchestration Flexibility Refactor

**Priority**: P1
**Component**: fugue-protocol
**Type**: improvement
**Estimated Effort**: medium
**Business Value**: high
**Status**: new

## Overview

Replace methodology-specific orchestration concepts with generic primitives so fugue can support any workflow (Context Engineering, Gas Town, custom) rather than hardcoding a specific coordination pattern. Make fugue a "dumb pipe" that handles session lifecycle and message routing with arbitrary metadata, letting users define workflow semantics.

## Problem Statement

The current orchestration system (implemented in FEAT-004/FEAT-007) has methodology-specific concepts baked into the protocol:

### Protocol Issues (fugue-protocol/src/messages.rs)

1. **`WorkerStatus` enum** - Assumes task-based workflow with predefined states:
   ```rust
   pub enum WorkerStatus {
       Idle,
       Working,
       WaitingForInput,
       Blocked,
       Complete,
       Error,
   }
   ```

2. **`OrchestrationTarget::Orchestrator`** - Hardcodes "one orchestrator" concept:
   ```rust
   pub enum OrchestrationTarget {
       Orchestrator,  // <-- assumes single coordinator
       Session(Uuid),
       Broadcast,
       Worktree(String),
   }
   ```

3. **Workflow-specific message types**:
   ```rust
   pub enum OrchestrationMessage {
       StatusUpdate { ... status: WorkerStatus ... },
       TaskAssignment { task_id, description, files },  // <-- task-centric
       TaskComplete { task_id, success, summary },       // <-- task-centric
       HelpRequest { session_id, context },              // <-- workflow assumption
       Broadcast { ... },
       SyncRequest,
   }
   ```

### Type Issues (fugue-protocol/src/types.rs)

4. **`is_orchestrator: bool`** in SessionInfo - Binary role assumption:
   ```rust
   pub struct SessionInfo {
       // ...
       pub is_orchestrator: bool,  // <-- only one special role
   }
   ```

### Router Issues (fugue-server/src/orchestration/router.rs)

5. **`orchestrators: HashMap<String, Uuid>`** - One orchestrator per repo assumption:
   ```rust
   pub struct MessageRouter {
       // ...
       orchestrators: HashMap<String, Uuid>,  // <-- single coordinator
   }
   ```

## Solution: Generic Primitives

### 1. Generic OrchestrationMessage

Replace workflow-specific message variants with a generic payload:

```rust
/// Messages for cross-session orchestration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OrchestrationMessage {
    /// Application-defined message type (e.g., "status_update", "task_complete", "gas_town.auction")
    pub msg_type: String,
    /// Arbitrary JSON payload - schema defined by the workflow
    pub payload: serde_json::Value,
}
```

**Benefits**:
- Any workflow can define its own message types
- No protocol changes needed for new workflows
- Backward compatible via msg_type versioning

### 2. Tag-Based Session Roles

Replace `is_orchestrator: bool` with flexible tags:

```rust
pub struct SessionInfo {
    pub id: Uuid,
    pub name: String,
    pub created_at: u64,
    pub window_count: usize,
    pub attached_clients: usize,
    pub worktree: Option<WorktreeInfo>,
    /// Arbitrary tags for role identification (e.g., ["mayor"], ["worker"], ["coordinator", "merger"])
    pub tags: HashSet<String>,
}
```

**Benefits**:
- Sessions can have multiple roles
- Roles are workflow-defined, not protocol-defined
- Easy to add new roles without protocol changes

### 3. Tag-Based Routing Target

Replace `OrchestrationTarget::Orchestrator` with `Tagged`:

```rust
pub enum OrchestrationTarget {
    /// Send to sessions with specific tag
    Tagged(String),
    /// Send to specific session
    Session(Uuid),
    /// Broadcast to all sessions in same repo
    Broadcast,
    /// Send to sessions in specific worktree
    Worktree(String),
}
```

**Benefits**:
- Route to any role: `Tagged("mayor")`, `Tagged("merger")`, `Tagged("evaluator")`
- Multiple sessions can share a tag (e.g., all workers)
- Natural multicast when tag matches multiple sessions

### 4. Router Tag Management

Update router to manage tags instead of orchestrator flag:

```rust
pub struct MessageRouter {
    sessions: HashMap<Uuid, MessageSender>,
    session_repos: HashMap<Uuid, String>,
    session_worktrees: HashMap<Uuid, String>,
    /// Tags per session (replaces orchestrators map)
    session_tags: HashMap<Uuid, HashSet<String>>,
}

impl MessageRouter {
    pub fn register(
        &mut self,
        session_id: Uuid,
        repo_id: Option<String>,
        worktree_path: Option<String>,
        tags: HashSet<String>,  // <-- replaces is_orchestrator
    ) -> MessageReceiver;

    pub fn add_tag(&mut self, session_id: Uuid, tag: String);
    pub fn remove_tag(&mut self, session_id: Uuid, tag: &str);
    pub fn sessions_with_tag(&self, tag: &str) -> Vec<Uuid>;
}
```

## Files to Modify

### fugue-protocol/src/messages.rs (~200 lines affected)

**DELETE**:
- `WorkerStatus` enum (lines 46-54)
- `OrchestrationMessage::StatusUpdate` variant
- `OrchestrationMessage::TaskAssignment` variant
- `OrchestrationMessage::TaskComplete` variant
- `OrchestrationMessage::HelpRequest` variant
- `OrchestrationTarget::Orchestrator` variant
- Related tests (~200 lines)

**ADD**:
- New `OrchestrationMessage` struct with `msg_type` and `payload`
- `OrchestrationTarget::Tagged(String)` variant
- New tests for generic messages

### fugue-protocol/src/types.rs (~50 lines affected)

**MODIFY**:
- `SessionInfo`: Replace `is_orchestrator: bool` with `tags: HashSet<String>`
- Update all tests that use `is_orchestrator`

### fugue-server/src/orchestration/router.rs (~100 lines affected)

**MODIFY**:
- Replace `orchestrators: HashMap<String, Uuid>` with `session_tags: HashMap<Uuid, HashSet<String>>`
- Change `register()` signature: `is_orchestrator: bool` -> `tags: HashSet<String>`
- Update `route()` to handle `OrchestrationTarget::Tagged`
- Remove `get_orchestrator()` method

**ADD**:
- `add_tag(session_id, tag)` method
- `remove_tag(session_id, tag)` method
- `sessions_with_tag(tag)` method
- `get_tags(session_id)` method

## Implementation Tasks

### Section 1: Protocol Types
- [ ] Define new `OrchestrationMessage` struct with msg_type and payload
- [ ] Add `OrchestrationTarget::Tagged(String)` variant
- [ ] Remove `OrchestrationTarget::Orchestrator` variant
- [ ] Delete `WorkerStatus` enum
- [ ] Delete old `OrchestrationMessage` enum variants

### Section 2: SessionInfo Changes
- [ ] Replace `is_orchestrator: bool` with `tags: HashSet<String>` in SessionInfo
- [ ] Update SessionInfo constructors/tests
- [ ] Add serde support for HashSet<String>

### Section 3: Router Refactor
- [ ] Replace `orchestrators` map with `session_tags` map
- [ ] Update `register()` to accept `tags: HashSet<String>`
- [ ] Implement `add_tag()` method
- [ ] Implement `remove_tag()` method
- [ ] Implement `sessions_with_tag()` method
- [ ] Update `route()` to handle `Tagged` target
- [ ] Remove `get_orchestrator()` method

### Section 4: Test Updates
- [ ] Update/replace WorkerStatus tests
- [ ] Update/replace orchestration message tests
- [ ] Update SessionInfo tests for tags
- [ ] Update router tests for tag-based routing
- [ ] Add tests for multi-tag scenarios

### Section 5: Documentation
- [ ] Update doc comments for new types
- [ ] Add examples of workflow-specific message payloads
- [ ] Document migration path for existing code

## Acceptance Criteria

- [ ] Sessions can register with arbitrary tags (e.g., "mayor", "worker", "coordinator")
- [ ] Messages can be routed to sessions by tag using `OrchestrationTarget::Tagged(String)`
- [ ] Generic message payload allows any workflow to define its own message types
- [ ] All existing tests updated or replaced with tag-based equivalents
- [ ] No methodology-specific types remain in the protocol
- [ ] `serde_json::Value` payload allows arbitrary JSON structures
- [ ] Tag operations (add/remove) work dynamically after registration

## Example Usage

### Context Engineering Workflow
```rust
// Orchestrator registration
router.register(session_id, Some(repo), None, hashset!["orchestrator"]);

// Worker registration
router.register(worker_id, Some(repo), Some(worktree), hashset!["worker"]);

// Worker sends status
let msg = OrchestrationMessage {
    msg_type: "status_update".to_string(),
    payload: json!({
        "session_id": worker_id,
        "status": "working",
        "task_id": task_id,
    }),
};
client.send(ClientMessage::SendOrchestration {
    target: OrchestrationTarget::Tagged("orchestrator".to_string()),
    message: msg,
});
```

### Gas Town Workflow
```rust
// Mayor registration
router.register(mayor_id, Some(repo), None, hashset!["mayor"]);

// Citizen registration
router.register(citizen_id, Some(repo), None, hashset!["citizen", "builder"]);

// Mayor broadcasts auction
let msg = OrchestrationMessage {
    msg_type: "gas_town.auction".to_string(),
    payload: json!({
        "item": "implement feature X",
        "starting_bid": 100,
    }),
};
client.send(ClientMessage::SendOrchestration {
    target: OrchestrationTarget::Broadcast,
    message: msg,
});

// Citizen sends bid to mayor
let bid = OrchestrationMessage {
    msg_type: "gas_town.bid".to_string(),
    payload: json!({
        "auction_id": auction_id,
        "amount": 75,
        "capabilities": ["rust", "async"],
    }),
};
client.send(ClientMessage::SendOrchestration {
    target: OrchestrationTarget::Tagged("mayor".to_string()),
    message: bid,
});
```

## Dependencies

- FEAT-007 (Protocol Layer) - Must be complete for this refactor

## Estimated Scope

- ~500-800 lines changed (mostly deletions and test updates)
- Breaking change to protocol - version bump required
- No runtime dependencies added (serde_json already in use)

## Notes

- This is a breaking protocol change - coordinate with any existing clients
- Consider adding protocol version negotiation if not already present
- The `SyncRequest` variant could be kept as it's generic, or replaced with a msg_type
- `Broadcast` target remains useful as a convenience over tagging
