# Tasks: FEAT-079 - Comprehensive Human-Control Arbitration

## Section 1: Server-side Arbitrator Refactor
- [x] Read `fugue-server/src/user_priority.rs` to understand current implementation.
- [x] Rename/Refactor `UserPriorityState` to `Arbitrator`.
- [x] Add tracking for Input and Layout events (timestamps per pane).
- [x] Implement `check_access(resource, actor, action) -> Result<(), Delay>`.

## Section 2: Handler Integration
- [x] Add `Arbitrator` checks to `fugue_send_input` handler.
- [x] Add `Arbitrator` checks to `fugue_resize_pane` / `fugue_split_pane`.
- [x] Add `Arbitrator` checks to `fugue_kill_pane` / `fugue_kill_session`.

## Section 3: Protocol/Config
- [x] Add configuration for "Input Lockout Duration" and "Layout Lockout Duration".
- [x] Ensure `McpError` types support the detailed rejection info.