# Implementation Plan: FEAT-029

**Work Item**: [FEAT-029: MCP Natural Language Terminal Control](PROMPT.md)
**Component**: fugue-server (MCP)
**Priority**: P1
**Created**: 2026-01-09

## Overview

Expand MCP tools to enable natural language terminal control by fixing the broken split direction bug and adding tools for session/window creation and listing.

## Architecture Decisions

### Decision 1: Follow Existing Handler Pattern

**Choice**: All new handlers follow the same pattern as `fugue_create_pane`.

**Rationale**:
- Consistency with existing codebase
- Proven pattern with error handling
- Uses existing `ToolContext` abstraction
- JSON output format already established

### Decision 2: Session/Window Creation Includes Default Pane with PTY

**Choice**: `create_session` and `create_window` always create a default pane with spawned PTY.

**Rationale**:
- Aligns with BUG-003 fix requirement
- Prevents empty sessions/windows that are unusable
- Users expect a working shell immediately
- Matches tmux behavior

**Alternative Considered**:
- Create empty session/window and let user add panes - Rejected because it leads to unusable state

### Decision 3: Split Direction Fix Scope

**Choice**: If current architecture doesn't support actual visual splitting, document as limitation and create follow-up work item.

**Rationale**:
- Split direction may require changes to window layout model
- Core tools (create, list) are more valuable than perfect splitting
- Don't block the feature on layout complexity
- Can iterate on split behavior later

**Implementation Options**:
1. If Window supports split positioning: Use `_direction` in layout calculation
2. If Window is flat list: Document limitation, all panes are "side by side" for now

### Decision 4: Tool Parameter Optionality

**Choice**: Make most parameters optional with sensible defaults.

**Rationale**:
- Natural language often omits details ("create a session" vs "create a session named X")
- Sensible defaults reduce friction
- Claude can infer missing values when needed
- Matches existing `fugue_create_pane` pattern

**Defaults**:
- session: First available session or auto-create
- name: Auto-generated
- command: User's $SHELL or /bin/sh

## Affected Components

| Component | Type of Change | Risk Level |
|-----------|----------------|------------|
| fugue-server/src/mcp/tools.rs | Add 4 tool definitions | Low |
| fugue-server/src/mcp/handlers.rs | Fix bug, add 4 handlers | Medium |
| fugue-server/src/mcp/server.rs | Add routing for new tools | Low |
| fugue-server/src/mcp/mod.rs | May need exports | Low |

## Implementation Order

1. **Phase 1: List Tools** (Lowest risk, read-only)
   - `fugue_list_sessions`
   - `fugue_list_windows`
   - These are read-only, lowest risk

2. **Phase 2: Create Tools** (Depends on BUG-003 pattern)
   - `fugue_create_session`
   - `fugue_create_window`
   - Follow BUG-003 fix pattern

3. **Phase 3: Fix Split Direction** (Scoped investigation)
   - Investigate current layout model
   - Implement if feasible, document limitation if not

4. **Phase 4: Tests and Cleanup**
   - Update existing tests
   - Add new tests
   - Verify all tools work end-to-end

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Split direction requires layout refactor | Medium | Low | Defer to follow-up if complex |
| Empty session bug interaction | Low | Medium | Verify BUG-003 is fixed first |
| Server.rs routing pattern unclear | Low | Low | Follow existing tool routing |
| Test coverage gaps | Medium | Low | Add comprehensive tests |

## Rollback Strategy

If implementation causes issues:
1. Revert commits associated with this work item
2. Verify existing 7 tools still work
3. Document what went wrong in comments.md

This is additive (new tools) so rollback is clean - old tools unaffected.

## Testing Strategy

### Unit Tests (handlers.rs)

Each new handler needs:
- `test_list_sessions_empty` - No sessions returns empty array
- `test_list_sessions_with_sessions` - Returns session info
- `test_list_windows_session_not_found` - Error case
- `test_list_windows_with_windows` - Returns window info
- `test_create_session_default` - Creates with defaults
- `test_create_session_named` - Creates with name
- `test_create_window_default` - Creates in first session
- `test_create_window_specific_session` - Creates in named session
- `test_create_window_session_not_found` - Error case

### Unit Tests (tools.rs)

- Update `test_expected_tools_present` to include new tools
- Verify all new tools have valid schemas

### Integration Tests

Manual MCP testing via stdio:
1. Start server with MCP enabled
2. Send tool calls and verify responses
3. Test natural language scenarios end-to-end

## Implementation Notes

### handlers.rs - New Handler Signatures

```rust
/// List all sessions
pub fn list_sessions(&self) -> Result<String, McpError> {
    // Return JSON array of session info
}

/// List windows in a session
pub fn list_windows(&self, session: Option<&str>) -> Result<String, McpError> {
    // Find session (by name or ID, or use first)
    // Return JSON array of window info
}

/// Create a new session
pub fn create_session(&mut self, name: Option<&str>) -> Result<String, McpError> {
    // Create session
    // Create default window
    // Create default pane with PTY (BUG-003 pattern)
    // Return JSON with all IDs
}

/// Create a new window
pub fn create_window(
    &mut self,
    session: Option<&str>,
    name: Option<&str>,
    command: Option<&str>,
) -> Result<String, McpError> {
    // Find or create session
    // Create window with optional name
    // Create default pane with PTY
    // Return JSON with window_id, pane_id
}
```

### server.rs - Routing Addition

Look for existing `handle_call_tool` or similar and add:

```rust
"fugue_list_sessions" => ctx.list_sessions(),
"fugue_list_windows" => {
    let session = args.get("session").and_then(|v| v.as_str());
    ctx.list_windows(session)
}
"fugue_create_session" => {
    let name = args.get("name").and_then(|v| v.as_str());
    ctx.create_session(name)
}
"fugue_create_window" => {
    let session = args.get("session").and_then(|v| v.as_str());
    let name = args.get("name").and_then(|v| v.as_str());
    let command = args.get("command").and_then(|v| v.as_str());
    ctx.create_window(session, name, command)
}
```

### Split Direction Investigation

Check in order:
1. Does `Window` have layout/split tracking? (`fugue-server/src/session/window.rs`)
2. Does `Pane` have position/dimension relative to siblings?
3. Is there a layout engine for calculating pane positions?

If yes to any: Implement proper split direction
If no to all: Document limitation, create follow-up FEAT for layout system

---
*This plan should be updated as implementation progresses.*
