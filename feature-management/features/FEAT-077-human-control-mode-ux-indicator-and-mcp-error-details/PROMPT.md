# FEAT-077: Human-control mode UX indicator and MCP error details

## Overview
Surface human-control mode in the TUI with a clear indicator and countdown timer, and return structured MCP errors that include remaining block duration when human-control arbitration rejects agent actions.

## Motivation
ADR-004/ADR-005 define human-control arbitration, but the current UX does not make the mode obvious and the MCP error payload lacks machine-readable timing. Operators need immediate visibility into when control is locked and agents need structured data to react or retry appropriately.

## Requirements
- Display a persistent human-control indicator in the TUI with remaining block time.
- Update the indicator in real time as the timer counts down or the block expires.
- MCP responses for human-control rejection include structured fields with remaining block duration.
- Align message shapes and semantics with ADR-004/ADR-005.

## Design
- Client: add a status-line or header badge that includes a countdown timer and state label.
- Server: extend MCP error payloads for arbitration rejection to include remaining block duration (e.g., seconds and/or absolute expiry).
- Protocol: if needed, version or extend error types to include structured timing without breaking existing clients.

## Tasks
### Section 1: Client UX indicator
- [ ] Identify TUI location and layout rules for the human-control indicator.
- [ ] Add indicator rendering and countdown display.
- [ ] Wire timer updates to state changes and expiry.

### Section 2: MCP error details
- [ ] Extend error struct to include remaining block duration.
- [ ] Populate remaining duration for human-control rejection errors.
- [ ] Ensure clients can parse and display the duration consistently.

### Section 3: Protocol alignment
- [ ] Verify alignment with ADR-004/ADR-005 semantics.
- [ ] Update any related documentation or metadata fields if required.

## Acceptance Criteria
- [ ] Human-control mode is clearly indicated in the TUI with a visible countdown.
- [ ] MCP human-control rejection errors include structured remaining-block fields.
- [ ] Existing clients remain compatible (or changes are documented/guarded).

## Testing
- [ ] Unit tests for error payload shaping and duration serialization.
- [ ] Integration test for TUI indicator state changes on arbitration updates.

## Dependencies
- ADR-004/ADR-005 definitions (reference for arbitration semantics)
