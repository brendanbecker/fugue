# fugue Observability

fugue is an interactive system with a correctness model (snapshot + events) and a persistence model (checkpoint + WAL). Observability focuses on:

- correctness signals (desync + resync)
- latency in interactive command paths
- persistence overhead
- Claude-state transitions (for “Claude-aware” value)

## Logging

fugue uses structured logging (tracing) and supports a human-readable format for development.

### Recommended structured fields
- `client_id`
- `session_id`, `window_id`, `pane_id`
- `command_id` (for MCP / external control)
- `commit_seq`
- `event_kind`
- `resync_reason` (gap, lagged, reconnect, decode_error, manual)
- `latency_ms` for request/ack paths

### Log events to emit
- client connect / disconnect
- snapshot sent (size, commit_seq)
- event emitted (commit_seq, kind)
- desync detected (expected_seq, got_seq)
- resync started/completed (replay vs snapshot)
- WAL append timing
- checkpoint start/end timing
- human control mode entered/exited
- Claude state transitions (old -> new, source, reason)

## Metrics (first tranche)

### Correctness / Convergence
- `fugue_client_resync_total{reason, mode}`  
  - `mode`: replay|snapshot
- `fugue_client_desync_total{reason}`
- `fugue_events_replay_requested_total`
- `fugue_events_replay_failed_total{reason}` (evicted, restart, unsupported)

### Latency
- `fugue_command_latency_ms{origin}`  
  - `origin`: tui|mcp|internal
- `fugue_snapshot_encode_ms`
- `fugue_event_dispatch_ms`

### Persistence
- `fugue_wal_append_ms`
- `fugue_checkpoint_duration_ms`
- `fugue_wal_bytes_written_total`
- `fugue_checkpoint_bytes_written_total`

### Claude Awareness
- `fugue_claude_state_transitions_total{source}`
- `fugue_claude_state_duration_ms{state, source}`
- `fugue_claude_state_flap_total` (rapid toggling heuristic)

## Tracing Spans

Recommended span boundaries:
- `rpc.request` (client request handling)
- `state.apply` (apply command -> mutate state)
- `wal.append`
- `checkpoint.write`
- `snapshot.encode`
- `event.emit`
- `client.resync`

## Debug Endpoints / Commands (optional)

- `fugue_get_status` should report:
  - server `commit_seq`
  - number of connected clients
  - replay buffer range (min_seq..max_seq)
  - WAL + checkpoint health
  - human control mode active/inactive

## Principles

- Prefer correctness signals over “channel health” signals.
- Broadcast lag metrics are optional; the design should not rely on broadcast delivery for correctness.

