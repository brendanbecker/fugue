# fugue Invariants

This document lists correctness properties that fugue is designed to uphold. These invariants guide implementation, testing, and future refactors.

## State and Convergence

1. **Server state is authoritative.**
   - Clients (TUI, MCP bridge) are projections and must not be treated as sources of truth.

2. **Clients always converge.**
   - A client that reconnects or desynchronizes will converge to server state after applying either:
     - a contiguous sequence of events, or
     - a full snapshot resync.

3. **No correctness depends on broadcast delivery.**
   - Broadcast/event channels are best-effort notifications only.
   - Missing events must be recoverable via replay or snapshot.

## Persistence and Recovery

4. **WAL replay matches live execution.**
   - Applying the WAL and checkpoint during recovery reconstructs the same logical state transitions as live operation.

5. **Committed ordering is total and monotonic.**
   - All client-visible state mutations have a global committed order (`commit_seq`) and never move backward.

## Session / Window / Pane Integrity

6. **No orphaned entities.**
   - Every pane belongs to exactly one window and one session.
   - Layout references only existing pane IDs.
   - Closing a pane/window/session removes all dependent references.

7. **PTY lifecycle is tied to pane lifecycle.**
   - A pane owns its PTY/process group; termination paths do not leave zombie PTYs.

## Control Plane Safety

8. **Idempotent external commands.**
   - Commands issued via MCP are safe to retry and do not create duplicate resources when replayed.

9. **Human control mode is enforced server-side.**
   - When human control mode is active, blocked MCP mutations must be rejected deterministically with a structured error.

## Claude Awareness

10. **Claude state machine is well-formed.**
   - Claude state transitions are represented explicitly and are observable (source/reason).
   - Heuristic detection is best-effort and should never break core mux correctness.

## Practical Notes

- Not all invariants are equally “hard”. Items (1)-(7) are treated as strict correctness. Items (8)-(10) are strict for automation safety but may evolve as integration matures.

