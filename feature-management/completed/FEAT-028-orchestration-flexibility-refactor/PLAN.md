# Implementation Plan: FEAT-028

**Work Item**: [FEAT-028: Orchestration Flexibility Refactor](PROMPT.md)
**Component**: fugue-protocol
**Priority**: P1
**Created**: 2026-01-09

## Overview

Replace methodology-specific orchestration concepts with generic primitives so fugue can support any workflow (Context Engineering, Gas Town, custom) rather than hardcoding a specific coordination pattern.

## Architecture Decisions

### Decision 1: Generic Message Payload Format

**Choice**: Use `serde_json::Value` for the payload field.

**Rationale**:
- Already a dependency via serde ecosystem
- Allows any JSON-serializable structure
- Workflows can define their own schemas
- Easy debugging (human-readable when logged)
- Interoperable with external tools

**Alternatives Considered**:
- `Vec<u8>` raw bytes - Too opaque, harder to debug
- `Box<dyn Any>` - Not serializable across process boundary
- Custom trait object - Overly complex for the use case

### Decision 2: Tag Storage Structure

**Choice**: `HashMap<Uuid, HashSet<String>>` for session tags.

**Rationale**:
- O(1) lookup by session ID
- HashSet prevents duplicate tags
- String tags are flexible and human-readable
- Easy to query all tags for a session

**Alternatives Considered**:
- Inverted index (`HashMap<String, HashSet<Uuid>>`) - Better for tag->sessions queries but harder to clean up on unregister
- Both structures - More memory, sync complexity
- Tags on SessionInfo only - Requires passing full SessionInfo for routing decisions

### Decision 3: Keep Broadcast and Worktree Targets

**Choice**: Retain `OrchestrationTarget::Broadcast` and `OrchestrationTarget::Worktree` alongside the new `Tagged`.

**Rationale**:
- Broadcast is workflow-agnostic (useful for any pattern)
- Worktree routing is fugue-specific infrastructure concern
- Avoids forcing users to manage "all" tags
- Backward compatible conceptually

### Decision 4: Breaking Protocol Change Strategy

**Choice**: Make this a clean break rather than maintaining backward compatibility.

**Rationale**:
- The old types embed assumptions that can't be cleanly mapped
- fugue is pre-1.0, breaking changes are expected
- Simpler implementation without compatibility shims
- Cleaner documentation and examples

## Affected Components

| Component | Type of Change | Risk Level |
|-----------|----------------|------------|
| fugue-protocol/src/messages.rs | Major refactor (types) | Medium |
| fugue-protocol/src/types.rs | Modify SessionInfo | Low |
| fugue-server/src/orchestration/router.rs | Major refactor (logic) | Medium |
| fugue-server/src/orchestration/mod.rs | Update exports if needed | Low |
| Any code using old message types | Breaking change | High |

## Implementation Order

1. **Phase 1: Protocol Types** (messages.rs)
   - Add new types alongside old ones temporarily
   - This allows incremental testing

2. **Phase 2: SessionInfo** (types.rs)
   - Replace is_orchestrator with tags
   - Update tests

3. **Phase 3: Router** (router.rs)
   - Refactor to use tags
   - Must happen after SessionInfo change

4. **Phase 4: Cleanup**
   - Remove old message types
   - Remove old tests
   - Final test pass

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Breaking existing router tests | High | Medium | Update tests incrementally |
| Missing edge cases in tag routing | Medium | Medium | Comprehensive test coverage |
| Performance regression with HashSet | Low | Low | Benchmark if concerned |
| serde_json::Value serialization issues | Low | Medium | Test round-trip with various payloads |

## Rollback Strategy

If implementation causes issues:
1. Revert commits associated with this work item
2. Verify system returns to previous state
3. Document what went wrong in comments.md

Since this is a breaking change, rollback means all clients must also revert.

## Testing Strategy

### Unit Tests
- OrchestrationMessage serialization/deserialization with various payloads
- Tag operations (add, remove, query)
- OrchestrationTarget::Tagged routing logic
- SessionInfo with tags serialization

### Integration Tests
- End-to-end message routing with tags
- Multi-tag scenarios
- Empty tag scenarios
- Concurrent tag modifications

### Property-Based Tests (Optional)
- Arbitrary JSON payloads round-trip correctly
- Tag set operations maintain invariants

## Implementation Notes

### messages.rs Changes

```rust
// OLD
pub enum OrchestrationMessage {
    StatusUpdate { ... },
    TaskAssignment { ... },
    TaskComplete { ... },
    HelpRequest { ... },
    Broadcast { ... },
    SyncRequest,
}

// NEW
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OrchestrationMessage {
    pub msg_type: String,
    pub payload: serde_json::Value,
}

// Keep these for convenience
impl OrchestrationMessage {
    pub fn new(msg_type: impl Into<String>, payload: serde_json::Value) -> Self {
        Self {
            msg_type: msg_type.into(),
            payload,
        }
    }

    pub fn sync_request() -> Self {
        Self::new("sync_request", serde_json::Value::Null)
    }
}
```

### types.rs Changes

```rust
// OLD
pub struct SessionInfo {
    // ...
    pub is_orchestrator: bool,
}

// NEW
use std::collections::HashSet;

pub struct SessionInfo {
    // ...
    pub tags: HashSet<String>,
}
```

### router.rs Changes

```rust
// OLD
pub fn register(
    &mut self,
    session_id: Uuid,
    repo_id: Option<String>,
    worktree_path: Option<String>,
    is_orchestrator: bool,
) -> MessageReceiver;

// NEW
pub fn register(
    &mut self,
    session_id: Uuid,
    repo_id: Option<String>,
    worktree_path: Option<String>,
    tags: HashSet<String>,
) -> MessageReceiver;

// NEW methods
pub fn add_tag(&mut self, session_id: Uuid, tag: String) -> bool;
pub fn remove_tag(&mut self, session_id: Uuid, tag: &str) -> bool;
pub fn sessions_with_tag(&self, tag: &str) -> Vec<Uuid>;
pub fn get_tags(&self, session_id: Uuid) -> Option<&HashSet<String>>;
```

---
*This plan should be updated as implementation progresses.*
