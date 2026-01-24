# Implementation Plan: BUG-020

**Work Item**: [BUG-020: Session Reattach from Session Manager Creates Client Without PTY](PROMPT.md)
**Component**: fugue-server
**Priority**: P1
**Created**: 2026-01-10

## Overview

When reattaching to a session via the session manager UI, the client connects but doesn't receive PTY output. The client is registered but not subscribed to the PTY output stream, leaving the user unable to interact with the terminal.

## Architecture Decisions

### Approach: To Be Determined

After investigation, the fix likely involves one of:

1. **Output Poller Registration**: Ensure new clients are added to the output poller's broadcast list
2. **Session Client Tracking**: Fix how session_clients tracks reattached clients
3. **Attach Handler Path**: Unify session manager attach with direct attach path
4. **PTY Reader Cloning**: Ensure PTY reader is shared correctly with new clients

### Trade-offs

| Option | Pros | Cons |
|--------|------|------|
| Fix Output Poller Registration | Direct fix | May not address root cause |
| Unify Attach Paths | Eliminates code duplication | Larger refactor |
| Add Client to Existing Stream | Simple | May have race conditions |
| Full State Sync on Attach | Ensures consistency | Performance cost |

**Decision**: TBD after investigation identifies exact failure point.

## Data Flow Analysis

### Normal Attach Flow (Working)

```
+--------+     +---------+     +-------------+     +-----------+
| Client | --> | Attach  | --> | Register in | --> | Subscribe |
| Attach |     | Handler |     | session_    |     | to PTY    |
|        |     |         |     | clients     |     | Output    |
+--------+     +---------+     +-------------+     +-----------+
```

### Session Manager Attach Flow (Broken)

```
+--------+     +---------+     +-------------+     +-----------+
| Select | --> | Session | --> | Register in | --> | ??? No    |
| Session|     | Manager |     | session_    |     | PTY Sub   |
|        |     | Handler |     | clients     |     |           |
+--------+     +---------+     +-------------+     +-----------+
```

### Questions to Answer

1. Is the client registered in `session_clients` after session manager attach?
2. Is the output poller aware of the new client?
3. Does the attach handler send the same messages for both paths?
4. Is there a race between client registration and output poller check?

## Affected Components

| Component | Type of Change | Risk Level |
|-----------|----------------|------------|
| `fugue-server/src/handlers/attach.rs` | Primary | Medium |
| `fugue-server/src/handlers/session.rs` | Primary | Medium |
| `fugue-server/src/pty/output.rs` | Secondary | Medium |
| `fugue-server/src/registry/` | Secondary | Low |
| `fugue-client/src/ui/` | Investigation | Low |

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Multiple attach code paths | High | Medium | Unify paths after fix |
| Race conditions in registration | Medium | High | Add proper synchronization |
| Breaking direct attach | Low | High | Test both paths |
| State inconsistency | Medium | Medium | Full state sync on attach |

## Rollback Strategy

If implementation causes issues:
1. Revert commits associated with this work item
2. Session manager attach remains broken (current state)
3. Direct attach continues to work
4. Consider alternative approach based on lessons learned

## Implementation Notes

<!-- Add notes during implementation -->

### Investigation Findings

*To be filled during investigation*

### Difference Between Attach Paths

*Compare session manager attach vs direct attach*

### Root Cause

*To be identified*

### Chosen Solution

*To be determined after investigation*

---
*This plan should be updated as implementation progresses.*
