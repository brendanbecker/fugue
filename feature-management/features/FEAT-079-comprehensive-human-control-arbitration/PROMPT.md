# FEAT-079: Comprehensive Human-Control Arbitration

## Overview
Implement the full "Human Control Mode" arbitration logic defined in ADR-004/ADR-005. This expands the "User Priority Lockout" (FEAT-056) concept—which currently only blocks focus changes—to block *all* automated mutations that could interfere with active human work, including text input, pane resizing, process killing, and layout changes.

## Motivation
`control_plane.md` defines ccmux as an authoritative control plane where "Humans retain final authority" and "Arbitration rules are explicit and deterministic". Currently, we only prevent agents from stealing focus. If a user is typing in a pane, an agent can still interleave text, resize the pane, or kill the process. This breaks the "Control Plane" promise and causes user frustration.

## Requirements
- **Input Locking**: If a user is interacting with a pane (typing, mouse), block automated `send_input` to that pane.
- **Mutation Locking**: If "Human Control Mode" is active (via explicit toggle or activity), block automated layout changes (resize, split, kill) affecting the user's focus.
- **Arbitration Logic**: Centralize logic in `ccmux-server` to check permissions for *every* intent based on the source (Human vs MCP).
- **Feedback**: Reject blocked intents with specific errors (consumed by FEAT-077 UX).

## Design
- Extend `UserPriorityState` (from FEAT-056) to be a general `Arbitrator` component.
- The `Arbitrator` tracks "Human Activity" per pane/window.
- Intercept all relevant MCP calls (`send_input`, `resize_pane`, `kill_pane`, etc.) and validate against the `Arbitrator`.
- Define explicit rules:
  - **Focus**: Covered by FEAT-056.
  - **Input**: Block if user sent input to this pane < X ms ago.
  - **Structure**: Block if user modified layout < Y ms ago (or explicit lock).
- Integration with `FEAT-077`: The `Arbitrator` provides the "remaining block duration" for the error messages.

## Tasks
### Section 1: Server-side Arbitrator Refactor
- [ ] Rename/Refactor `UserPriorityState` to `Arbitrator`.
- [ ] Add tracking for Input and Layout events (timestamps per pane).
- [ ] Implement `check_access(resource, actor, action) -> Result<(), Delay>`.

### Section 2: Handler Integration
- [ ] Add `Arbitrator` checks to `ccmux_send_input` handler.
- [ ] Add `Arbitrator` checks to `ccmux_resize_pane` / `ccmux_split_pane`.
- [ ] Add `Arbitrator` checks to `ccmux_kill_pane` / `ccmux_kill_session`.

### Section 3: Protocol/Config
- [ ] Add configuration for "Input Lockout Duration" and "Layout Lockout Duration".
- [ ] Ensure `McpError` types support the detailed rejection info.

## Acceptance Criteria
- [ ] Agent cannot type into a pane while the user is typing (interleaved input prevented).
- [ ] Agent cannot resize/kill a pane the user is actively manipulating.
- [ ] "Human Control" applies to all destructive/disruptive actions, not just focus.
- [ ] Focus control (FEAT-056) remains preserved.

## Dependencies
- FEAT-056 (User Priority Lockout) - Base implementation to extend.
- FEAT-077 (UX Indicator) - Consumes the errors generated here.
