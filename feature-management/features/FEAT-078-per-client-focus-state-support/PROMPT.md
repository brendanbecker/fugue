# FEAT-078: Per-client focus state support

## Overview
Ensure that focus (active window/pane selection) is handled on a per-client basis rather than being global to the session. This prevents "focus fights" where multiple clients (e.g., a TUI user and an MCP agent) force each other to switch views.

## Motivation
When multiple clients are attached to the same session, they often have different needs. A human user in a TUI might want to watch one pane while an MCP agent is performing operations in another. If focus is global, the MCP agent's actions (which often involve selecting a pane) will disrupt the human user's view.

## Requirements
- Move focus/selection state from the session/window level to a per-client context in `ccmux-server`.
- Update the protocol to allow clients to send focus/selection commands that only affect their own view.
- When a client attaches, it should initialize its focus state from a reasonable default (e.g., the last active pane of the session) but subsequent changes must be independent.
- MCP tools that target "the selected pane" must use the focus state associated with that MCP client.
- Ensure TUI rendering uses the client-specific focus state to highlight the active pane.

## Design
Based on `ADR-005`.
- `ccmux-server` Registry should track `ClientState` which includes `focused_window_id` and `focused_pane_id` per connection.
- Protocol messages for selection (`SelectWindow`, `SelectPane`) should update the `ClientState`.
- `Broadcast` messages for state updates might need to be filtered or tailored if they include focus information, or focus should be moved entirely out of the shared `SessionState` that is broadcast.
- Snapshot initialization for new clients should provide a "starter" focus, but this is a one-time copy.

## Tasks
### Section 1: Server-side state refactor
- [ ] Modify `Registry` or `Client` session tracking to store per-client focus.
- [ ] Remove global focus state from `Session`/`Window` if applicable, or mark it as "default/last-human-focus".
- [ ] Update MCP tool handlers to use client-specific focus when resolving "current" targets.

### Section 2: Protocol updates
- [ ] Ensure `SelectWindow` and `SelectPane` messages are handled as per-client state updates.
- [ ] Update `SessionState` broadcast to exclude or separate focus state.

### Section 3: Client/TUI updates
- [ ] Update `ccmux-client` to maintain and send its own focus state.
- [ ] Ensure TUI rendering correctly reflects the local client's focus.

## Acceptance Criteria
- [ ] Human user in TUI can stay on Pane A while MCP agent operates on Pane B without the TUI view jumping to Pane B.
- [ ] Multiple TUI clients can be attached to the same session and view different windows/panes independently.
- [ ] MCP tools correctly target the pane last selected by that specific MCP client.

## Testing
- [ ] Integration test with two concurrent clients selecting different panes.
- [ ] Unit tests for `Registry` per-client state management.
- [ ] Verify MCP tool targeting logic with multiple agents.

## Dependencies
- FEAT-075 (Snapshot + replay resync API) - focus state should be part of the snapshot.
