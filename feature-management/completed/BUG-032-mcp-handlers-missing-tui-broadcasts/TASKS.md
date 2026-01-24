# Task Breakdown: BUG-032

**Work Item**: [BUG-032: MCP Handlers Missing TUI Broadcasts](PROMPT.md)
**Status**: Not Started
**Last Updated**: 2026-01-11

## Prerequisites

- [ ] Read and understand PROMPT.md
- [ ] Review PLAN.md and update if needed
- [ ] Understand `ResponseWithBroadcast` pattern in `handle_create_pane_with_options`
- [ ] Check existing broadcast message types in `fugue-protocol/src/messages.rs`

## Investigation Tasks

- [ ] Confirm affected line numbers match current code
- [ ] Verify `PaneCreated`, `PaneResized` messages exist and TUI handles them
- [ ] Check if `WindowCreated` message exists or needs to be added
- [ ] Identify how `handle_create_layout` should broadcast multiple panes

## Implementation Tasks

### Handler 1: handle_resize_pane_delta (simplest)

- [ ] Capture `session_id` early in the handler (from `find_pane` result)
- [ ] Change return type from `Response` to `ResponseWithBroadcast`
- [ ] Set `broadcast` to same `PaneResized` message
- [ ] Test: resize via MCP, verify TUI updates

### Handler 2: handle_split_pane

- [ ] Re-acquire session_manager lock after PTY spawn to get `pane_info`
- [ ] Call `pane.to_info()` to get `PaneInfo` struct
- [ ] Change return type from `Response` to `ResponseWithBroadcast`
- [ ] Set `broadcast` to `PaneCreated { pane: pane_info, direction }`
- [ ] Test: split via MCP, verify TUI shows both panes

### Handler 3: handle_create_window_with_options

- [ ] Decide: broadcast `PaneCreated` for default pane OR add `WindowCreated` message
- [ ] Collect `pane_info` for the default pane
- [ ] Change return type from `Response` to `ResponseWithBroadcast`
- [ ] Set appropriate broadcast message
- [ ] If adding `WindowCreated`: update protocol and TUI handler
- [ ] Test: create window via MCP, verify TUI awareness

### Handler 4: handle_create_layout (most complex)

- [ ] Collect `PaneInfo` for each pane during layout creation
- [ ] Decide broadcast strategy (see PLAN.md options)
- [ ] Option 1: Broadcast `LayoutCreated` with pane list
- [ ] Option 2: Broadcast first pane, TUI requests full state
- [ ] Implement chosen approach
- [ ] If TUI changes needed: add `LayoutCreated` broadcast handler
- [ ] Test: create layout via MCP, verify all panes render

## Testing Tasks

- [ ] Add test: `test_split_pane_broadcasts_to_tui`
- [ ] Add test: `test_create_window_broadcasts_to_tui`
- [ ] Add test: `test_create_layout_broadcasts_to_tui`
- [ ] Add test: `test_resize_pane_broadcasts_to_tui`
- [ ] Run full test suite: `cargo test`
- [ ] Manual test: all 4 operations with TUI visible

## Verification Tasks

- [ ] Confirm TUI updates immediately for all 4 operations
- [ ] Confirm `Ctrl+B o` cycles through MCP-created panes
- [ ] Confirm no regressions in MCP response handling
- [ ] Update bug_report.json status to resolved
- [ ] Document resolution in comments.md if needed

## Completion Checklist

- [ ] All 4 handlers return `ResponseWithBroadcast`
- [ ] All new tests passing
- [ ] All existing tests passing
- [ ] PLAN.md updated with final approach
- [ ] Manual verification complete
- [ ] Ready for review/merge

---
*Check off tasks as you complete them. Update status field above.*
