# Implementation Plan: FEAT-036

**Work Item**: [FEAT-036: Session-aware MCP Commands with Window/Pane Naming](PROMPT.md)
**Component**: fugue-server (MCP)
**Priority**: P1
**Created**: 2026-01-09

## Overview

MCP commands should intelligently default to the active session (one with attached clients) when no session is explicitly specified. Additionally, add the ability to name windows and panes for easier identification and management.

## Architecture Decisions

### Decision 1: Active Session Selection Strategy

**Choice**: Select session with most attached clients, with fallback to most recent creation time.

**Rationale**:
- Session with attached clients is clearly "active" from user perspective
- Most attached clients handles edge case of multiple active sessions
- Creation time fallback ensures predictable behavior when no clients attached
- Matches user mental model: "the session I'm looking at"

**Implementation**:
```rust
fn get_active_session(&self) -> Option<SessionId> {
    // 1. Find session(s) with most attached clients
    // 2. If tie or all zero, prefer most recently created/accessed
    // 3. Return None only if no sessions exist
}
```

**Trade-offs**:
- Slightly more complex than "first session"
- Requires tracking attached client count per session
- May behave unexpectedly if user has multiple clients attached to different sessions

### Decision 2: Client Attachment Tracking

**Choice**: Increment/decrement counter on client connect/disconnect messages.

**Rationale**:
- Simple atomic counter is sufficient
- No need for complex client identity tracking
- Works with existing message routing system
- Clean increment on attach, decrement on detach/disconnect

**Implementation Location**: `fugue-server/src/session/mod.rs`

**Data Structure**:
```rust
pub struct Session {
    // ... existing fields
    attached_clients: AtomicUsize,
    last_activity: Instant,
}
```

**Trade-offs**:
- Counter could drift if client disconnects uncleanly (needs cleanup logic)
- AtomicUsize avoids lock contention but requires careful ordering
- Alternative: track client IDs in HashSet (more accurate but heavier)

### Decision 3: Name Storage Location

**Choice**: Store names directly in Pane and Window structs.

**Rationale**:
- Names are intrinsic properties of panes/windows
- Simplest data model
- Names persist with session state
- Easy to serialize for persistence

**Implementation**:
```rust
pub struct Pane {
    // ... existing fields
    name: Option<String>,
}

pub struct Window {
    // ... existing fields
    name: String,  // Already exists, verify
}
```

**Alternatives Considered**:
- Separate naming map - adds complexity, no benefit
- Client-side only naming - doesn't persist, not visible to MCP

### Decision 4: Response Format Enhancement

**Choice**: All session-scoped tool responses include `session_id` and `session_name`.

**Rationale**:
- Users can verify correct session was targeted
- Helps debugging when unexpected behavior occurs
- Consistent response format across tools
- Small overhead for significant UX improvement

**Response Format**:
```json
{
  "content": [...],
  "session_context": {
    "session_id": "uuid",
    "session_name": "name"
  }
}
```

**Trade-offs**:
- Slightly larger response payloads
- Requires updating all session-scoped tool handlers

### Decision 5: New Tool Registration

**Choice**: Add `fugue_rename_pane` and `fugue_rename_window` as separate tools.

**Rationale**:
- Clear single-purpose tools
- Follows existing tool pattern in codebase
- Easy to understand and use
- Can be extended later (e.g., rename session)

**Alternatives Considered**:
- Generic `fugue_rename` with type parameter - more complex schema
- Update tools that accept name on creation only - misses rename use case
- Combine with other metadata updates - over-engineering

## Affected Components

| Component | Type of Change | Risk Level |
|-----------|----------------|------------|
| fugue-server/src/session/mod.rs | Add attached_clients tracking | Medium |
| fugue-server/src/session/pane.rs | Add name field if not present | Low |
| fugue-server/src/mcp/tools.rs | Add rename tools, name param | Medium |
| fugue-server/src/mcp/handlers.rs | Active session logic, rename handlers | High |
| fugue-server/src/mcp/server.rs | Route new tools | Low |

## Implementation Order

### Phase 1: Data Model Updates (Foundation)
1. Add `attached_clients: AtomicUsize` to Session
2. Add `name: Option<String>` to Pane (if not present)
3. Verify Window has `name` field
4. Add `get_active_session()` method to SessionManager
5. **Deliverable**: Data model ready for active session tracking

### Phase 2: Client Attachment Tracking
1. Increment counter when client attaches to session
2. Decrement counter when client detaches or disconnects
3. Handle unclean disconnects (timeout cleanup)
4. **Deliverable**: Accurate client count per session

### Phase 3: Active Session Integration
1. Update `fugue_list_windows` to use active session
2. Update `fugue_create_window` to use active session
3. Update `fugue_create_pane` to use active session
4. Update `fugue_list_panes` to use active session when no filter
5. Update tool descriptions
6. **Deliverable**: All session-scoped tools use active session

### Phase 4: Naming Features
1. Add `name` parameter to `fugue_create_pane` schema
2. Implement pane name assignment on creation
3. Implement `fugue_rename_pane` tool
4. Implement `fugue_rename_window` tool
5. Update list responses to include names
6. **Deliverable**: Full naming support

### Phase 5: Response Enhancement
1. Add `session_context` to all session-scoped responses
2. Include `session_id` and `session_name`
3. Update response formatting in handlers
4. **Deliverable**: Consistent session context in responses

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Client count drift on unclean disconnect | Medium | Low | Periodic cleanup, heartbeat timeout |
| Breaking existing tool consumers | Medium | Medium | Maintain backward-compatible response structure |
| Active session wrong when multiple clients | Low | Medium | Document behavior, user can specify explicitly |
| Name collisions | Low | Low | Names are display-only, IDs remain authoritative |
| Performance impact of tracking | Low | Low | AtomicUsize is lock-free |

## Rollback Strategy

If implementation causes issues:
1. Active session logic can be disabled by always returning first session
2. Naming is additive - can be removed without breaking existing functionality
3. Response format additions are backward compatible
4. Session attachment tracking can be disabled independently

## Testing Strategy

### Unit Tests
- `get_active_session()` with various session/client combinations
- Client count increment/decrement
- Pane naming on creation
- Pane/window rename

### Integration Tests
- Create pane with 2 sessions, verify active session selection
- Attach/detach clients, verify count updates
- Create named pane, verify in list output
- Rename pane/window, verify change persists

### Manual Testing
- Multiple sessions in real fugue setup
- Attach client, create pane via MCP
- Verify pane appears in correct session
- Test naming workflow end-to-end

## Implementation Notes

### Get Active Session Logic

```rust
impl SessionManager {
    pub fn get_active_session(&self) -> Option<SessionId> {
        let sessions = self.sessions.read().unwrap();

        // Find session with most attached clients
        let mut best: Option<(SessionId, usize, Instant)> = None;

        for (id, session) in sessions.iter() {
            let clients = session.attached_clients.load(Ordering::SeqCst);
            let activity = session.last_activity;

            match &best {
                None => best = Some((*id, clients, activity)),
                Some((_, best_clients, best_activity)) => {
                    if clients > *best_clients
                       || (clients == *best_clients && activity > *best_activity) {
                        best = Some((*id, clients, activity));
                    }
                }
            }
        }

        best.map(|(id, _, _)| id)
    }
}
```

### Updating Tool Handler Pattern

Each session-scoped tool handler should follow this pattern:

```rust
async fn handle_create_pane(&self, args: CreatePaneArgs) -> ToolResult {
    // Determine session
    let session_id = args.session_id
        .or_else(|| self.session_manager.get_active_session())
        .ok_or_else(|| ToolError::NoSession)?;

    // Get session info for response
    let session = self.session_manager.get_session(session_id)?;
    let session_name = session.name.clone();

    // ... perform operation ...

    // Return with session context
    ToolResult::success(json!({
        "pane_id": pane_id,
        "session_context": {
            "session_id": session_id,
            "session_name": session_name
        }
    }))
}
```

### Client Attachment Events

Client attachment should be tracked via explicit messages or inferred from behavior:

```rust
// Option A: Explicit attach message
ClientMessage::AttachSession { session_id } => {
    session_manager.increment_clients(session_id);
}

// Option B: Infer from pane selection
ClientMessage::SelectPane { pane_id } => {
    let session_id = get_session_for_pane(pane_id);
    session_manager.mark_client_active(client_id, session_id);
}
```

Option B is more implicit but tracks actual usage better.

---
*This plan should be updated as implementation progresses.*
