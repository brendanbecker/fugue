# FEAT-074: Observability instrumentation (metrics, tracing, status)

## Overview
Add structured logging fields, tracing spans, and metrics across fugue-server, plus expand status reporting for operational visibility.

## Motivation
We need consistent observability for correctness, latency, persistence, and Claude state to diagnose issues quickly and support ongoing reliability work. The detailed instrumentation targets are outlined in `docs/scratch/chatgpt/OBSERVABILITY.md`.

## Requirements
- Implement structured logging fields as specified in `docs/scratch/chatgpt/OBSERVABILITY.md`.
- Add tracing spans for the critical server paths defined in `docs/scratch/chatgpt/OBSERVABILITY.md`.
- Add metrics for correctness, latency, persistence, and Claude state per `docs/scratch/chatgpt/OBSERVABILITY.md`.
- Extend `fugue_get_status` to expose:
  - `commit_seq`
  - `clients`
  - replay buffer range
  - WAL/checkpoint health

## Design
- Follow the observability taxonomy in `docs/scratch/chatgpt/OBSERVABILITY.md`.
- Keep structured logs and metrics keyed consistently so status responses align with telemetry.

## Tasks
### Section 1: Structured logging
- [ ] Inventory and add required structured fields across server subsystems.
- [ ] Validate log field names match `docs/scratch/chatgpt/OBSERVABILITY.md`.

### Section 2: Tracing spans
- [ ] Add spans around critical request/response paths and persistence flows.
- [ ] Propagate relevant context through nested operations.

### Section 3: Metrics
- [ ] Add correctness, latency, persistence, and Claude state metrics.
- [ ] Confirm metric units, labels, and cardinality constraints.

### Section 4: Status API
- [ ] Extend `fugue_get_status` output to include commit sequence, client details, replay buffer range, and WAL/checkpoint health.

## Acceptance Criteria
- [ ] Logs include the structured fields specified in `docs/scratch/chatgpt/OBSERVABILITY.md`.
- [ ] Tracing spans exist for the required operations and include relevant context.
- [ ] Metrics for correctness/latency/persistence/Claude state are emitted with expected labels.
- [ ] `fugue_get_status` returns commit sequence, client count/details, replay buffer range, and WAL/checkpoint health.

## Testing
- [ ] Unit tests (where applicable)
- [ ] Integration tests (where applicable)

## Dependencies
- `docs/scratch/chatgpt/OBSERVABILITY.md`
