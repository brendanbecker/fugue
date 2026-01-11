# Task Breakdown: FEAT-059

**Work Item**: [FEAT-059: Beads Workflow Integration - Pane-Issue Correlation and Audit Trail](PROMPT.md)
**Status**: Not Started
**Last Updated**: 2026-01-11

## Prerequisites

- [ ] Read and understand PROMPT.md
- [ ] Review PLAN.md and update if needed
- [ ] Verify dependencies are met: FEAT-057, FEAT-058, FEAT-050
- [ ] Review existing pane metadata structure
- [ ] Review existing MCP tool patterns
- [ ] Review orchestration protocol messages

## Design Tasks

- [ ] Review requirements and acceptance criteria
- [ ] Confirm data structure for PaneWorkflowState
- [ ] Design issue history retention policy
- [ ] Determine command interception hook point
- [ ] Design crash event flow
- [ ] Update PLAN.md with findings

## Implementation Tasks

### Core Data Structures (ccmux-server/src/beads/workflow.rs)
- [ ] Create `beads/workflow.rs` module
- [ ] Define `PaneWorkflowState` struct
- [ ] Define `IssueHistoryEntry` struct
- [ ] Define `IssueOutcome` enum
- [ ] Implement `assign_issue()` method
- [ ] Implement `release_current()` method
- [ ] Implement `has_active_issue()` helper
- [ ] Add serde derives for serialization
- [ ] Add module to `beads/mod.rs`

### Pane Metadata Integration (ccmux-server/src/session/pane.rs)
- [ ] Add `workflow: PaneWorkflowState` field to Pane struct
- [ ] Initialize workflow state in Pane::new()
- [ ] Ensure workflow state serializes with pane
- [ ] Ensure workflow state deserializes on load
- [ ] Add workflow state to pane clone/copy operations

### MCP Tool Definitions (ccmux-server/src/mcp/tools.rs)
- [ ] Add `ccmux_beads_assign` tool definition
- [ ] Add `ccmux_beads_release` tool definition
- [ ] Add `ccmux_beads_find_pane` tool definition
- [ ] Add `ccmux_beads_pane_history` tool definition
- [ ] Register tools in tool list
- [ ] Add tools to documentation

### MCP Handlers (ccmux-server/src/mcp/handlers.rs)
- [ ] Define `BeadsAssignParams` struct
- [ ] Define `BeadsReleaseParams` struct
- [ ] Define `BeadsFindPaneParams` struct
- [ ] Define `BeadsPaneHistoryParams` struct
- [ ] Implement `handle_beads_assign()` handler
- [ ] Implement `handle_beads_release()` handler
- [ ] Implement `handle_beads_find_pane()` handler
- [ ] Implement `handle_beads_pane_history()` handler
- [ ] Add handlers to dispatch match
- [ ] Broadcast pane updates on assignment change

### Command Interception (ccmux-server/src/handlers/pty.rs)
- [ ] Create `CommandInterceptor` struct
- [ ] Implement `is_bd_close_command()` detection
- [ ] Implement `inject_session()` logic
- [ ] Add interception hook in PTY write path
- [ ] Make interception configurable
- [ ] Log intercepted/modified commands
- [ ] Handle edge cases (quotes, escapes)

### Crash Handling (ccmux-server/src/handlers/pane.rs)
- [ ] Define `CrashIssue` struct
- [ ] Create `PaneCrashHandler` struct
- [ ] Implement `handle_exit()` method
- [ ] Capture scrollback context on crash
- [ ] Mark issue as crashed if assigned
- [ ] Broadcast `PaneCrash` event
- [ ] Define `ServerEvent::PaneCrash` variant
- [ ] Connect handler to pane exit events

### Orchestration Protocol (ccmux-protocol/src/messages.rs)
- [ ] Add `current_issue_id: Option<String>` to StatusUpdate
- [ ] Add `issue_status: Option<String>` to StatusUpdate
- [ ] Update StatusUpdate serialization
- [ ] Update StatusUpdate deserialization
- [ ] Update status update builders to include issue
- [ ] Document protocol change

### Configuration (ccmux-server/src/config.rs)
- [ ] Define `BeadsWorkflowConfig` struct
- [ ] Add `enabled: bool` field (default: true)
- [ ] Add `auto_inject_session: bool` field (default: true)
- [ ] Add `issue_on_crash: bool` field (default: false)
- [ ] Add `crash_context_lines: usize` field (default: 50)
- [ ] Add `show_issue_in_status: bool` field (default: true)
- [ ] Add `auto_detect_issue: bool` field (default: false)
- [ ] Add `[beads.workflow]` section to BeadsConfig
- [ ] Add serde deserialization with defaults
- [ ] Document configuration options

### UI Integration (ccmux-client/src/ui/pane.rs)
- [ ] Add issue display to pane status rendering
- [ ] Format issue ID for display (truncate if long)
- [ ] Respect `show_issue_in_status` config
- [ ] Style issue display (color, formatting)
- [ ] Handle no-issue case gracefully

## Testing Tasks

### Unit Tests - Core Structures
- [ ] Test PaneWorkflowState::assign_issue() creates entry
- [ ] Test PaneWorkflowState::assign_issue() releases previous
- [ ] Test PaneWorkflowState::release_current() updates history
- [ ] Test IssueHistoryEntry serialization
- [ ] Test IssueOutcome serialization
- [ ] Test workflow state with no issue
- [ ] Test multiple assignment/release cycles

### Unit Tests - Command Interception
- [ ] Test is_bd_close_command() with "bd close"
- [ ] Test is_bd_close_command() with "bd c"
- [ ] Test is_bd_close_command() with non-bd commands
- [ ] Test inject_session() adds parameter
- [ ] Test inject_session() skips if --session present
- [ ] Test interception when disabled

### MCP Tool Tests
- [ ] Test ccmux_beads_assign with valid issue
- [ ] Test ccmux_beads_assign with pane_id
- [ ] Test ccmux_beads_assign with focused pane default
- [ ] Test ccmux_beads_assign with invalid pane
- [ ] Test ccmux_beads_release with outcome
- [ ] Test ccmux_beads_release without outcome
- [ ] Test ccmux_beads_find_pane found case
- [ ] Test ccmux_beads_find_pane not found case
- [ ] Test ccmux_beads_pane_history with history
- [ ] Test ccmux_beads_pane_history empty history

### Integration Tests
- [ ] Test full assign -> work -> release flow
- [ ] Test assignment persists across restart
- [ ] Test history accumulates correctly
- [ ] Test crash handler broadcasts event
- [ ] Test command injection end-to-end
- [ ] Test orchestration messages include issue

### Edge Case Tests
- [ ] Test rapid assign/release cycles
- [ ] Test assign same issue twice
- [ ] Test release with no active issue
- [ ] Test very long issue IDs
- [ ] Test special characters in issue ID
- [ ] Test concurrent assignments to different panes

### Manual Testing
- [ ] Assign issue via MCP, verify status display
- [ ] Run `bd close`, inspect command for --session
- [ ] Kill pane with assigned issue, check crash event
- [ ] Query pane by issue with ccmux_beads_find_pane
- [ ] View history with ccmux_beads_pane_history
- [ ] Restart server, verify workflow state restored

## Documentation Tasks

- [ ] Document new MCP tools in tool reference
- [ ] Document configuration options
- [ ] Document workflow tracking behavior
- [ ] Document crash issue creation flow
- [ ] Add code comments to PaneWorkflowState
- [ ] Add code comments to command interception
- [ ] Update CHANGELOG

## Verification Tasks

- [ ] All acceptance criteria from PROMPT.md met
- [ ] Tests passing
- [ ] Update feature_request.json status
- [ ] Document any issues in comments.md

## Completion Checklist

- [ ] All implementation tasks complete
- [ ] All tests passing
- [ ] Documentation updated
- [ ] PLAN.md reflects final implementation
- [ ] No regressions in existing functionality
- [ ] Dependencies confirmed working
- [ ] Ready for review/merge

---
*Check off tasks as you complete them. Update status field above.*
