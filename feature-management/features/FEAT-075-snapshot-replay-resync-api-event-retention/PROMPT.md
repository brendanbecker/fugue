# FEAT-075: Snapshot + replay resync API (event retention)

## Overview
Implement snapshot + events convergence with a GetEventsSince / replay buffer API and snapshot fallback to ensure clients can resync after gaps or reconnects.

## Motivation
Clients can currently fall behind or disconnect and miss events, which risks divergence between client state and server state. A resync flow with snapshots, replay buffers, and event retention ensures eventual convergence and predictable recovery semantics.

## Requirements
- Add GetEventsSince API in ccmux-protocol for event replay.
- Implement replay buffer + retention policies on the server.
- Add snapshot fallback and commit_seq handling for resync.
- Implement client resync flow to request snapshots/events after gaps or reconnects.
- Define convergence rules per ADR-004/ADR-005.

## Design
High-level flow:
- Server retains events up to a retention window; each event has a commit_seq.
- Client tracks last_seen_commit_seq and uses GetEventsSince to replay when gaps are detected.
- If events are no longer retained or gaps exceed retention, server returns snapshot + current commit_seq.
- Client applies snapshot, then replays events (if any) to converge.

## Tasks
### Section 1: Protocol changes
- [ ] Add GetEventsSince request/response messages to ccmux-protocol.
- [ ] Add snapshot response payload and commit_seq metadata.
- [ ] Update ADR-004/ADR-005 references in protocol docs if needed.

### Section 2: Server retention and replay
- [ ] Add commit_seq tracking to event publication pipeline.
- [ ] Implement replay buffer with retention window and pruning.
- [ ] Add snapshot generation and fallback path.

### Section 3: Client resync flow
- [ ] Track last_seen_commit_seq in client state.
- [ ] Implement gap detection and GetEventsSince call.
- [ ] Apply snapshot + replay to converge client state.

## Acceptance Criteria
- [ ] Client recovers correctly after disconnects or missed events.
- [ ] Replay buffer handles retention and eviction without crashes.
- [ ] Snapshot fallback converges state when replay is insufficient.
- [ ] commit_seq is monotonically increasing and used for resync.

## Testing
- [ ] Unit tests for retention window and pruning.
- [ ] Integration tests for disconnect/reconnect resync.

## Dependencies
- ADR-004/ADR-005 definitions for convergence rules.
