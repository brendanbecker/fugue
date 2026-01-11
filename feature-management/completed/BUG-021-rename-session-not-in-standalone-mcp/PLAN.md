# Implementation Plan: BUG-021

**Work Item**: [BUG-021: ccmux_rename_session Not Handled in Standalone MCP Server](PROMPT.md)
**Component**: ccmux-server
**Priority**: P2
**Created**: 2026-01-10

## Overview

The `ccmux_rename_session` MCP tool works in bridge mode but is missing from standalone MCP server mode. This is a straightforward fix to add the missing handler code to match the bridge mode implementation.

## Architecture Decisions

- **Approach**: Mirror the bridge mode implementation pattern in standalone server
- **Trade-offs**: None significant - this is bringing standalone mode to parity with bridge mode
- **Consistency**: Use same parameter parsing and validation as bridge.rs

## Affected Components

| Component | Type of Change | Risk Level |
|-----------|----------------|------------|
| `ccmux-server/src/mcp/server.rs` | Add tool handling | Low |
| `ccmux-server/src/mcp/handlers.rs` | Add method to ToolContext | Low |

## Implementation Approach

### Phase 1: Add Tool Recognition

1. Add `ccmux_rename_session` to the `is_known_tool()` function
2. Add `RenameSession` variant to `ToolParams` enum

### Phase 2: Add Parameter Parsing

1. Add parsing case in `dispatch_tool()` match statement
2. Extract `session` (required string) and `name` (required string) parameters
3. Use same validation as bridge.rs (return `McpError::InvalidParams` for missing params)

### Phase 3: Add Execution Handler

1. Add execution case in the result match statement
2. Add `rename_session()` method to `ToolContext` in handlers.rs
3. Implement session lookup (by UUID or name) and rename logic

### Phase 4: Testing

1. Test in standalone MCP server mode
2. Verify parity with bridge mode behavior
3. Ensure no regression in other tools

## Dependencies

None - this is a self-contained fix within the MCP module.

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Regression in existing tools | Very Low | Medium | Test other MCP tools after change |
| Session lookup behavior differs from bridge | Low | Low | Match bridge.rs implementation |

## Rollback Strategy

If implementation causes issues:
1. Revert commits associated with this work item
2. Verify standalone MCP server returns to previous state
3. Document what went wrong in comments.md

## Implementation Notes

Reference implementation in bridge.rs:372-380:
```rust
"ccmux_rename_session" => {
    let session = arguments["session"]
        .as_str()
        .ok_or_else(|| McpError::InvalidParams("Missing 'session' parameter".into()))?;
    let name = arguments["name"]
        .as_str()
        .ok_or_else(|| McpError::InvalidParams("Missing 'name' parameter".into()))?;
    self.tool_rename_session(session, name).await
}
```

The standalone mode version should follow this same pattern but using `ToolContext` instead of the bridge's async methods.

---
*This plan should be updated as implementation progresses.*
