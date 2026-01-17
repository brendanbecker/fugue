# BUG-038: MCP Handlers Return Wrong Response Types (Comprehensive)

**Priority**: P1
**Component**: ccmux-server (handlers/mcp_bridge.rs)
**Severity**: high
**Status**: new

## Problem Statement

Multiple MCP handlers return incorrect response enum variants. The handler logic executes correctly and produces valid data, but the response gets wrapped in the wrong type, causing deserialization failures.

This is a systemic issue affecting multiple operation types.

## Affected Operations

### List Operations
| Tool | Expected Response | Actual Response |
|------|-------------------|-----------------|
| `list_windows` | `WindowList` | `SessionList` |
| `list_panes` | `PaneList` | `WindowList` |

### Create Operations
| Tool | Expected Response | Actual Response |
|------|-------------------|-----------------|
| `create_pane` | `PaneCreated` | `SessionList` |

### Potentially Affected (untested)
- `create_window` - may return wrong type
- `create_session` - may return wrong type
- `split_pane` - may return wrong type
- Other handlers following similar dispatch pattern

## Steps to Reproduce

### Reproduce list_windows issue:
```
ccmux_list_windows(session: "session-name")
-> MCP error -32603: Unexpected response: SessionList { sessions: [...] }
```

### Reproduce list_panes issue:
```
ccmux_list_panes(session: "session-name")
-> MCP error -32603: Unexpected response: WindowList { windows: [...] }
```

### Reproduce create_pane issue:
```
ccmux_create_pane(direction: "vertical", name: "test")
-> MCP error -32603: Unexpected response: SessionList { sessions: [...] }
```

## Expected vs Actual Behavior

**Expected**: Each MCP tool returns its designated response type with the correct data structure.

**Actual**: Response types are shifted/mismatched. The pattern suggests an off-by-one error or enum variant index mismatch in the response dispatch logic.

## Evidence from QA Runs

### From BUG-035 QA run:
```
ccmux_list_windows(session: "session-0")
-> Error: Unexpected response: SessionList { sessions: [SessionInfo {...}] }

ccmux_list_panes(session: "session-0")
-> Error: Unexpected response: WindowList { windows: [WindowInfo {...}] }
```

### From BUG-038 QA run:
```
ccmux_create_pane(direction: "vertical", name: "demo-pane")
-> MCP error -32603: Unexpected response: SessionList { sessions: [
     SessionInfo { id: cebaa25e-..., name: "session-0", ... },
     SessionInfo { id: 7e4aca93-..., name: "dev-qa", ... }
   ] }
```

## Root Cause Hypothesis

The MCP response enum variant selection is broken. Likely locations:
1. Handler dispatch in `mcp_bridge.rs` - wrong variant being constructed
2. Response serialization - enum index mismatch
3. Match arm ordering doesn't align with enum definition

The consistent "shift" pattern (list_windows->SessionList, list_panes->WindowList, create_pane->SessionList) suggests the variant index is off by a fixed amount.

## Relationship to Other Bugs

- **Supersedes BUG-035**: This bug consolidates all response type issues
- **BUG-035** can be closed as duplicate once this is fixed

## Impact

- **Severity**: P1 - Multiple core MCP operations broken
- **User Impact**:
  - Cannot reliably list windows/panes with session filter
  - Cannot create panes in current session
  - Orchestration workflows fail intermittently
- **Workaround**: Some operations work without parameters (e.g., `list_panes()` without session filter worked during QA)

## Acceptance Criteria

- [ ] `list_sessions` returns `SessionList`
- [ ] `list_windows` returns `WindowList`
- [ ] `list_panes` returns `PaneList`
- [ ] `create_session` returns `SessionCreated`
- [ ] `create_window` returns `WindowCreated`
- [ ] `create_pane` returns `PaneCreated`
- [ ] `split_pane` returns appropriate split response
- [ ] All other MCP handlers return their correct response types
- [ ] Add tests to verify response type correctness for each handler

## Debugging Suggestions

1. Check enum definition order in response types
2. Verify match arm ordering in handler dispatch
3. Look for numeric enum variant usage vs named variants
4. Search for copy-paste errors in handler implementations
5. Add debug logging to trace which variant is being constructed vs returned
