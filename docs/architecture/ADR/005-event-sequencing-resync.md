# ADR-005: Event Sequencing and Resync

## Status
Accepted

## Context

fugue must provide a consistent view of server state to clients (TUI, MCP bridge) in the presence of:

- disconnected clients (detach/attach)
- event lag or drop (tokio broadcast semantics)
- multiple writers (human + MCP)
- crash recovery (checkpoint + WAL replay)

To converge reliably, clients need a way to detect missed updates and request the missing state.

## Decision

### 1) Use a global server-issued `commit_seq` as the primary ordering

- Every committed state mutation increments a global `commit_seq: u64`.
- `commit_seq` is assigned at the time the mutation is committed (i.e., after WAL append succeeds).

This `commit_seq` is the definitive “happens-before” order for client-visible state.

### 2) Optional per-entity versions for debugging/optimization

- Entities (session/window/pane/layout) may carry a `version: u64`.
- Per-entity versions are not used for gap detection; they are useful for:
  - debugging (“why did this pane change?”)
  - future selective diff snapshots
  - caches

### 3) Snapshot + Events convergence protocol

#### Snapshot
Server provides a `StateSnapshot`:
- `commit_seq` (the server’s current committed seq)
- full session/window/pane hierarchy + metadata/tags
- layout tree(s)
- focus state required for rendering (see note on per-client focus below)
- bounded scrollback tail per pane (configurable)

Snapshots are sent:
- on connect/reattach
- on explicit client request (resync)
- after detecting a sequence gap
- optionally on protocol version mismatch or decoding failure

#### Events
Server emits `StateEvent`:
- includes `commit_seq`
- includes the mutation kind and relevant identifiers
- intended to update client projections incrementally

### 4) Resync strategy: “Replay if possible, otherwise snapshot”

Clients maintain `last_applied_commit_seq`.

On receipt of an event with `commit_seq != last_applied + 1`, client enters `DESYNC` and initiates resync.

Resync flow:
1. Client requests `GetEventsSince(last_applied_commit_seq)`
2. If server can fulfill (bounded by retention), client replays events in order
3. If not fulfillable (events evicted, gap too large, server restarted), client requests `GetSnapshot()`

This supports both efficiency (replay) and simplicity/correctness (snapshot fallback).

### 5) WAL is the source of truth for sequencing

- `commit_seq` increments only for mutations that are durably recorded (WAL append success).
- On recovery, WAL replay reconstructs state and reconstitutes `commit_seq`.

This ensures:
- live execution order == replay order
- client convergence works across crashes

### 6) Multi-client focus is per-client view state

- Server state includes canonical hierarchy/layout.
- Focus/selection cursor is treated as per-client UI state (not global), to avoid “focus fights”.
- The snapshot includes enough info for a client to initialize (e.g., default focused pane), but clients may maintain their own focus thereafter.

### 7) Human-control arbitration policy (MCP interrupt)

The server may enter “human control mode” for a short interval when tmux-style interactive commands occur. During this interval:
- MCP commands that mutate session/window/pane/layout are rejected with a structured error
- Read-only MCP commands remain allowed
- The error includes the remaining block duration (optional)

## Consequences

### Positive
- Clients can always converge, regardless of missed notifications.
- Crash recovery and live behavior share a single commit log ordering.
- Efficient resync via replay, with snapshot fallback for correctness.

### Trade-offs
- Requires event retention for replay (bounded ring buffer or WAL-backed fetch).
- Requires explicit resync logic in clients.

## Implementation Notes

- Event retention can be an in-memory ring buffer keyed by `commit_seq`, sized conservatively.
- Snapshots should be encoded efficiently (bincode is fine initially; consider versioned framing).
- Scrollback persistence should be chunked in blocks (newline-oriented or byte-limit) to avoid WAL spam.

## Alternatives Considered

- **Per-entity sequencing only**
  - Rejected for gap detection; cross-entity invariants require total order.
- **Client polling snapshots**
  - Rejected as primary: less responsive, more load, still needs ordering to avoid regressions.

