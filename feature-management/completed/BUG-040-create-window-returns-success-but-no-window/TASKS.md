# Task Breakdown: BUG-040

**Work Item**: [BUG-040: ccmux_create_window returns success but doesn't actually create windows](PROMPT.md)
**Status**: Not Started
**Last Updated**: 2026-01-11

## Prerequisites

- [ ] Read and understand PROMPT.md
- [ ] Review PLAN.md and update if needed
- [ ] Review BUG-034 fix commit (3e14861) to understand recent changes

## Investigation Tasks

- [ ] Review `ccmux-server/src/mcp/handlers.rs` create_window handler
- [ ] Review `ccmux-server/src/session/manager.rs` window creation methods
- [ ] Trace code path from create_window call to response
- [ ] Identify where window is created but not persisted
- [ ] Compare with working window creation paths (e.g., initial session setup)
- [ ] Document root cause in PLAN.md

## Implementation Tasks

- [ ] Fix window persistence in identified location
- [ ] Ensure window is added to session's window collection
- [ ] Verify window_count is updated correctly
- [ ] Ensure list_windows query includes new windows
- [ ] Self-review changes

## Testing Tasks

- [ ] Add test: create_window returns window that appears in list_windows
- [ ] Add test: window_count increments after create_window
- [ ] Add test: BUG-034 scenario still works (session parameter respected)
- [ ] Run existing MCP handler tests
- [ ] Manual testing of create_window workflow

## Verification Tasks

- [ ] Confirm create_window creates persistent windows
- [ ] Confirm list_sessions shows updated window_count
- [ ] Confirm list_windows includes new windows
- [ ] Verify BUG-034 fix still works
- [ ] Update bug_report.json status
- [ ] Document resolution in comments.md

## Completion Checklist

- [ ] All implementation tasks complete
- [ ] All tests passing
- [ ] PLAN.md updated with final approach
- [ ] Ready for review/merge

---
*Check off tasks as you complete them. Update status field above.*
