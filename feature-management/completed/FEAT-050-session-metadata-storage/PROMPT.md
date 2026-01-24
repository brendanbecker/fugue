# FEAT-050: Session Metadata Storage for Agent Identity

**Priority**: P3
**Component**: fugue-server (MCP)
**Type**: new_feature
**Estimated Effort**: small
**Business Value**: low
**Status**: new

## Overview

Allow storing arbitrary key-value metadata on sessions to track agent identity, role, and other workflow-specific information. This enables cleaner agent identity tracking and session queries for multi-agent orchestration workflows like Gas Town.

## Problem Statement

Gas Town tracks agent identity via session naming convention (`gt-<rig>-<agent>`) and environment variables. Identity includes:

- **Role**: mayor, deacon, witness, refinery, crew, polecat
- **Rig name**: alpha, beta, gamma, etc.
- **Agent name**: Toast, Nux, Max, etc.

Currently, to determine a session's identity, callers must:
1. Parse the session name using naming conventions, OR
2. Query environment variables set in the session

This is fragile and workflow-specific. A generic metadata storage system would:
- Allow any workflow to store session attributes
- Enable querying sessions by metadata values
- Persist metadata across restarts via checkpoints

## Implementation Plan

### 1. Add Metadata to Session Struct

Add a `HashMap<String, String>` to store arbitrary key-value pairs:

```rust
// In fugue-session/src/session.rs
pub struct Session {
    // existing fields...
    pub metadata: HashMap<String, String>,
}
```

### 2. Add MCP Tool: `fugue_set_metadata`

```json
{
  "name": "fugue_set_metadata",
  "description": "Set a metadata key-value pair on a session",
  "inputSchema": {
    "type": "object",
    "properties": {
      "session": {
        "type": "string",
        "description": "Session name or ID"
      },
      "key": {
        "type": "string",
        "description": "Metadata key"
      },
      "value": {
        "type": "string",
        "description": "Metadata value"
      }
    },
    "required": ["session", "key", "value"]
  }
}
```

### 3. Add MCP Tool: `fugue_get_metadata`

```json
{
  "name": "fugue_get_metadata",
  "description": "Get metadata from a session",
  "inputSchema": {
    "type": "object",
    "properties": {
      "session": {
        "type": "string",
        "description": "Session name or ID"
      },
      "key": {
        "type": "string",
        "description": "Specific key to retrieve (optional, returns all if omitted)"
      }
    },
    "required": ["session"]
  }
}
```

### 4. Include Metadata in `fugue_list_sessions` Response

Extend the session list response to include metadata:

```json
{
  "sessions": [
    {
      "name": "gt-alpha-Toast",
      "id": "...",
      "metadata": {
        "role": "polecat",
        "rig": "alpha",
        "agent_name": "Toast"
      }
    }
  ]
}
```

### 5. Persist Metadata in Checkpoints

Update the checkpoint format to include session metadata. Ensure metadata is restored on server restart.

## Use Cases

```
# Set agent identity
fugue_set_metadata(session="gt-alpha-Toast", key="role", value="polecat")
fugue_set_metadata(session="gt-alpha-Toast", key="rig", value="alpha")
fugue_set_metadata(session="gt-alpha-Toast", key="agent_name", value="Toast")

# Query a specific key
fugue_get_metadata(session="gt-alpha-Toast", key="role")
# Returns: {"role": "polecat"}

# Get all metadata
fugue_get_metadata(session="gt-alpha-Toast")
# Returns: {"role": "polecat", "rig": "alpha", "agent_name": "Toast"}

# List sessions shows metadata
fugue_list_sessions()
# Returns sessions with their metadata for filtering
```

**Future Enhancement**: Add `fugue_find_sessions` tool to query by metadata:
```
fugue_find_sessions(metadata={"role": "polecat", "rig": "alpha"})
# Returns all polecat sessions in rig alpha
```

## Files to Modify

| File | Change |
|------|--------|
| `fugue-session/src/session.rs` | Add `metadata: HashMap<String, String>` to Session |
| `fugue-server/src/mcp/tools.rs` | Add `fugue_set_metadata` and `fugue_get_metadata` tools |
| `fugue-server/src/mcp/handlers.rs` | Implement tool handlers |
| `fugue-protocol/src/types.rs` | Add metadata to SessionInfo if needed |
| `fugue-persistence/src/checkpoint.rs` | Persist metadata in checkpoints |

## Implementation Tasks

### Section 1: Session Struct
- [ ] Add `metadata: HashMap<String, String>` to Session struct
- [ ] Update Session constructors
- [ ] Add `get_metadata()`, `set_metadata()`, `remove_metadata()` methods

### Section 2: MCP Tools
- [ ] Add `fugue_set_metadata` tool definition
- [ ] Add `fugue_get_metadata` tool definition
- [ ] Implement handlers for both tools
- [ ] Add metadata to `fugue_list_sessions` response

### Section 3: Persistence
- [ ] Update checkpoint format to include metadata
- [ ] Update checkpoint restore to load metadata
- [ ] Add migration for existing checkpoints (empty metadata)

### Section 4: Testing
- [ ] Unit tests for Session metadata methods
- [ ] MCP tool tests for set/get metadata
- [ ] Checkpoint round-trip test with metadata
- [ ] Integration test: set metadata, restart, verify preserved

## Acceptance Criteria

- [ ] Sessions can store arbitrary string key-value pairs
- [ ] MCP tool `fugue_set_metadata` works correctly
- [ ] MCP tool `fugue_get_metadata` works correctly
- [ ] Session list includes metadata
- [ ] Metadata persists across server restarts
- [ ] Existing sessions without metadata load without errors

## Notes

- This is a convenience feature; callers can use environment variables as a workaround
- Consider size limits on metadata to prevent abuse (e.g., 100 keys, 1KB per value)
- Future: `fugue_find_sessions` for querying by metadata could be a separate feature
- Complements FEAT-028 (tag-based routing) but serves a different purpose:
  - Tags are for message routing
  - Metadata is for arbitrary application data
