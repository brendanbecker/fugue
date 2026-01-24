# Implementation Plan: BUG-043

**Work Item**: [BUG-043: MCP tool handlers fail to unwrap Sequenced message wrapper](PROMPT.md)
**Component**: mcp-bridge
**Priority**: P1
**Created**: 2026-01-16

## Overview

The daemon wraps responses in `ServerMessage::Sequenced` for WAL persistence tracking (FEAT-075), but the MCP bridge's `recv_response_from_daemon()` function returns these wrapped messages directly to tool handlers. Handlers expect unwrapped message types and fail with "Unexpected response: Sequenced { ... }" errors.

## Architecture Decisions

### Approach: Unwrap at recv_response_from_daemon()

**Decision**: Unwrap Sequenced messages at the `recv_response_from_daemon()` layer, not in individual handlers.

**Rationale**:
1. Single point of change - affects all handlers uniformly
2. Handlers remain clean and focused on their specific response types
3. Maintains Sequenced wrapper in daemon and persistence layer
4. Minimal code change with maximum impact

**Alternative Considered**: Update each handler to handle Sequenced variant
- Rejected: Would require changes to 10+ handlers and create repetitive code

### Trade-offs

| Aspect | Impact |
|--------|--------|
| Code Changes | Minimal - single function modification |
| Handler Changes | None required |
| Persistence | Unaffected - Sequenced still used for WAL |
| Testing | Medium - need to verify all affected tools |

## Affected Components

| Component | Type of Change | Risk Level |
|-----------|----------------|------------|
| `connection.rs:recv_response_from_daemon()` | Primary fix | Low |
| `connection.rs:is_broadcast_message()` | May need update | Low |
| `handlers.rs` | No changes | None |
| Persistence layer | No changes | None |

## Implementation Steps

### Step 1: Locate and Understand Current Implementation

```
fugue-server/src/mcp/bridge/connection.rs
- Line ~390-427: is_broadcast_message()
- recv_response_from_daemon() - find exact location
```

### Step 2: Implement Fix

Modify `recv_response_from_daemon()`:

```rust
pub async fn recv_response_from_daemon(&mut self) -> Result<ServerMessage, McpError> {
    let msg = self.recv_filtered(|msg| !Self::is_broadcast_message(msg)).await?;
    // Unwrap Sequenced messages to get the actual response
    match msg {
        ServerMessage::Sequenced { inner, .. } => Ok(*inner),
        other => Ok(other),
    }
}
```

### Step 3: Handle Edge Cases

Consider:
- Nested Sequenced (unlikely but possible): Add recursive unwrapping if needed
- Broadcast messages that might be Sequenced: Verify is_broadcast_message handles this

### Step 4: Testing Strategy

1. Unit test for unwrapping logic
2. Integration tests for affected MCP tools
3. Verify working tools still work
4. Test persistence/WAL replay

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Break working tools | Low | High | Test all tools before/after |
| Break persistence | Low | High | Verify WAL replay works |
| Incomplete fix | Medium | Medium | Test all affected tools |
| Performance impact | Very Low | Low | Unwrapping is O(1) |

## Rollback Strategy

If implementation causes issues:
1. Revert the single commit modifying recv_response_from_daemon()
2. Handlers will fail with Sequenced wrapper again (known state)
3. Consider alternative fix approaches

## Success Metrics

1. All affected tools (kill_session, set_tags, broadcast, etc.) work correctly
2. Working tools (create_pane, list_panes, etc.) continue to work
3. Persistence tracking and WAL replay unaffected
4. No new "Unexpected response" errors in MCP logs

## Implementation Notes

<!-- Add notes during implementation -->

---
*This plan should be updated as implementation progresses.*
