# FEAT-076: Capability Signaling Protocol (Sideband Metadata)

## Overview
Add explicit capability signaling via a sideband (or control channel) so panes can advertise state/control/identity/resume capabilities. Capabilities are stored as per-pane metadata and should be sourced from explicit signals rather than heuristics when available.

## Motivation
Current capability detection relies on heuristics and inferred state. A clear signaling protocol improves reliability, enables richer integrations, and lets panes publish identity and resume support in a consistent, machine-readable way.

## Requirements
- Define a sideband/control-channel capability signaling message format per `docs/scratch/chatgpt/CAPABILITIES.md`.
- Support advertising at least: state, control, identity, and resume capabilities.
- Store capabilities in per-pane metadata and expose them via existing metadata APIs.
- Prefer explicit signals over heuristic detection when both are present.
- Ensure capability updates are idempotent and version-tolerant.

## Design
- Extend the sideband protocol with a capability message (e.g., `<ccmux:capabilities ...>` or equivalent control frame) that can update per-pane capability metadata.
- Update server-side capability resolution to combine explicit signals with heuristic detection, favoring explicit values.
- Provide a small capability schema (keys + values) and version tag to allow forward compatibility.

## Tasks
### Section 1: Protocol Spec
- [ ] Define capability signaling message and schema in the protocol docs.
- [ ] Add codec support in `ccmux-protocol` for capability messages.

### Section 2: Server Handling
- [ ] Parse capability messages from sideband/control channel.
- [ ] Persist capability data into pane metadata store.
- [ ] Resolve conflicts by preferring explicit signals over heuristics.

### Section 3: Client/UX Integration
- [ ] Surface capability metadata in relevant views or tools (as needed).
- [ ] Ensure sideband capability updates can be consumed by clients.

### Section 4: Documentation & Tests
- [ ] Document capability signaling usage and examples.
- [ ] Add unit/integration tests for capability parsing and storage.

## Acceptance Criteria
- [ ] Panes can advertise capabilities via sideband/control channel.
- [ ] Capability data is persisted in pane metadata and available to clients.
- [ ] Explicit signals override heuristic detection when present.
- [ ] Tests cover parsing and storage of capability signals.

## Testing
- [ ] Unit tests
- [ ] Integration tests

## Dependencies
- FEAT-050 (Session metadata storage)
