# Implementation Plan: FEAT-050

**Work Item**: [FEAT-050: Session Metadata Storage for Agent Identity](PROMPT.md)
**Component**: fugue-server (MCP)
**Priority**: P3
**Created**: 2026-01-10

## Overview

Allow storing arbitrary key-value metadata on sessions to track agent identity, role, and other workflow-specific information. This enables cleaner agent identity tracking and session queries for multi-agent orchestration workflows.

## Architecture Decisions

### Decision 1: Metadata Storage Location

**Choice**: Store metadata in the Session struct itself.

**Rationale**:
- Metadata is inherently tied to session lifecycle
- Simplifies access from session methods
- Natural serialization with session checkpoint
- No separate lookup table needed

**Alternatives Considered**:
- Separate metadata store - More complex, extra sync needed
- Store in SessionManager - Violates single responsibility
- Store only in MCP layer - Not available to other code paths

### Decision 2: Metadata Key-Value Types

**Choice**: `HashMap<String, String>` for metadata.

**Rationale**:
- String keys and values are simple and universal
- Easy to serialize/deserialize
- Sufficient for agent identity use cases
- No complex type handling needed

**Alternatives Considered**:
- `HashMap<String, serde_json::Value>` - Over-engineered for current needs
- Custom struct for known fields - Not flexible enough for arbitrary workflows
- Typed metadata enum - Too rigid, defeats extensibility

### Decision 3: Metadata in Protocol Messages

**Choice**: Include metadata in SessionInfo struct.

**Rationale**:
- SessionInfo is already used in list_sessions response
- Allows filtering/querying on client side
- Single source of truth for session properties

### Decision 4: Metadata Size Limits

**Choice**: Implement soft limits initially, hard limits later if needed.

**Rationale**:
- Most use cases need < 10 keys with short values
- Hard limits add complexity
- Can be added later if abuse occurs

**Suggested Limits** (for future):
- Max 100 keys per session
- Max 256 bytes per key
- Max 4KB per value

## Affected Components

| Component | Type of Change | Risk Level |
|-----------|----------------|------------|
| fugue-session/src/session.rs | Add metadata field | Low |
| fugue-server/src/mcp/tools.rs | Add tool definitions | Low |
| fugue-server/src/mcp/handlers.rs | Add handlers | Low |
| fugue-protocol/src/types.rs | Add metadata to SessionInfo | Low |
| fugue-persistence/src/checkpoint.rs | Serialize metadata | Low |

## Implementation Order

1. **Phase 1: Core Storage** (session.rs)
   - Add HashMap field
   - Add accessor methods
   - Update tests

2. **Phase 2: Protocol** (types.rs)
   - Add metadata to SessionInfo
   - Update serialization

3. **Phase 3: MCP Tools** (tools.rs, handlers.rs)
   - Add set_metadata tool
   - Add get_metadata tool
   - Update list_sessions to include metadata

4. **Phase 4: Persistence** (checkpoint.rs)
   - Update checkpoint format
   - Handle migration from old format

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Checkpoint format change | Low | Medium | Version checkpoint format, handle old versions |
| Memory bloat from large metadata | Low | Low | Document limits, add enforcement later |
| Metadata key collisions between workflows | Low | Low | Namespace conventions in docs |

## Rollback Strategy

If implementation causes issues:
1. Revert commits associated with this work item
2. Old checkpoints without metadata field will still work (HashMap defaults to empty)
3. Verify system returns to previous state
4. Document what went wrong in comments.md

## Testing Strategy

### Unit Tests
- Session metadata CRUD operations
- SessionInfo serialization with metadata
- MCP tool parameter validation

### Integration Tests
- Set metadata via MCP, retrieve via MCP
- List sessions shows metadata
- Checkpoint round-trip preserves metadata
- Restart preserves metadata

### Edge Cases
- Empty metadata
- Large number of keys
- Unicode keys/values
- Overwriting existing keys
- Getting non-existent key

## Implementation Notes

### session.rs Changes

```rust
use std::collections::HashMap;

pub struct Session {
    // existing fields...
    pub metadata: HashMap<String, String>,
}

impl Session {
    pub fn new(/* ... */) -> Self {
        Self {
            // ...
            metadata: HashMap::new(),
        }
    }

    pub fn get_metadata(&self, key: &str) -> Option<&str> {
        self.metadata.get(key).map(|s| s.as_str())
    }

    pub fn set_metadata(&mut self, key: String, value: String) {
        self.metadata.insert(key, value);
    }

    pub fn remove_metadata(&mut self, key: &str) -> Option<String> {
        self.metadata.remove(key)
    }

    pub fn all_metadata(&self) -> &HashMap<String, String> {
        &self.metadata
    }
}
```

### types.rs Changes

```rust
pub struct SessionInfo {
    // existing fields...
    pub metadata: HashMap<String, String>,
}
```

### MCP Tool Registration

```rust
// In tools.rs
Tool {
    name: "fugue_set_metadata".to_string(),
    description: Some("Set a metadata key-value pair on a session".to_string()),
    input_schema: json!({
        "type": "object",
        "properties": {
            "session": {"type": "string", "description": "Session name or ID"},
            "key": {"type": "string", "description": "Metadata key"},
            "value": {"type": "string", "description": "Metadata value"}
        },
        "required": ["session", "key", "value"]
    }),
},
Tool {
    name: "fugue_get_metadata".to_string(),
    description: Some("Get metadata from a session".to_string()),
    input_schema: json!({
        "type": "object",
        "properties": {
            "session": {"type": "string", "description": "Session name or ID"},
            "key": {"type": "string", "description": "Specific key (optional, returns all if omitted)"}
        },
        "required": ["session"]
    }),
}
```

---
*This plan should be updated as implementation progresses.*
