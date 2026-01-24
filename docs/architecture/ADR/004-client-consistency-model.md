# ADR-004: Client Consistency Model

## Status
Accepted

## Context

fugue uses a client-server architecture with a long-running daemon (server) and one or more UI clients (TUI) and automation clients (MCP bridge). Clients may disconnect/reconnect at any time (detach/attach), and the system must remain correct under:

- client reconnects
- server restarts and recovery from persisted state
- multiple concurrent writers (human via TUI + automation via MCP)
- lossy notification mechanisms (e.g., tokio broadcast lag/drop)

Historically, treating events as the sole source of truth leads to UI desynchronization when notifications are missed, reordered, or applied out of context.

## Decision

1. **Server state is authoritative.** Clients are projections.
2. **Clients synchronize via a Snapshot + Events model.**
   - On connect/reattach, the server sends a full `StateSnapshot` (authoritative).
   - After snapshot, the client applies `StateEvent`s incrementally.
3. **Events are advisory; state is authoritative.**
   - Events may be dropped or missed without correctness loss.
   - Clients must be able to request a resync at any time.
4. **Sequence gaps trigger resync.**
   - Clients detect missed events using a monotonically increasing server-issued `commit_seq`.
   - On gaps, clients request replay or full snapshot resync (see ADR-005).
5. **Idempotent commands.**
   - External commands (notably MCP) include a `command_id` to support safe retries.
6. **Human-control arbitration (MCP interrupt).**
   - The server supports a temporary “human control mode” which can block MCP session/window/pane mutations for a short interval during interactive use.
   - Blocked MCP commands fail explicitly with a structured error indicating “human control mode”.

## Consequences

### Positive
- UI correctness does not depend on event delivery.
- Reconnect is well-defined: snapshot restores canonical view.
- WAL replay and live execution share the same state machine model.
- Enables multiple clients with independent UI focus (per-client view state).

### Trade-offs
- Requires snapshot encoding and a resync RPC.
- Client must handle out-of-order or missing events (but this logic is straightforward).
- Some operations may need to be expressed as state transitions rather than UI diffs.

## Alternatives Considered

1. **Event-only projection (no snapshots)**
   - Rejected: missed events permanently desync UI without a convergence mechanism.

2. **Polling snapshots only (no events)**
   - Rejected: increases latency and load; reduces responsiveness; still needs sequencing to avoid regressions.

3. **Per-entity versions only**
   - Rejected as primary correctness mechanism: cross-entity invariants and layout updates require a total order to detect gaps reliably.
   - Allowed as an optional optimization/debug layer (see ADR-005).

## Notes

- Snapshots include full session/window/pane hierarchy, layout state, and relevant metadata required for rendering.
- Scrollback is included only as a bounded tail (configurable), and may be chunked/encoded for performance.

