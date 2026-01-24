# fugue as a Control Plane

fugue functions as a **control plane for interactive workspaces**.

This document clarifies what that means, what fugue is responsible for, and—just as importantly—what it is not.

---

## Definition

A control plane:
- owns authoritative state
- accepts intent from multiple actors
- arbitrates conflicts deterministically
- converges after failure
- coordinates, but does not perform, work

fugue satisfies these properties for **interactive, long-lived terminal workspaces** involving humans and automation.

---

## What fugue Controls

fugue is authoritative over:

- Session lifecycle (create, attach, detach, destroy)
- Window and pane hierarchy
- Layout structure and focus state
- Process attachment (PTY ownership)
- Command execution intent (spawns)
- Arbitration between human and automated actors
- Recovery and convergence after failure

All mutations of this state flow through the server and are recorded as committed state transitions.

---

## What fugue Does *Not* Control

fugue is **not** responsible for:

- Executing business logic
- Reasoning, planning, or decision-making
- Optimizing workflows or commands
- Understanding the semantics of processes it runs
- Managing infrastructure resources (CPU, memory, scheduling)

Processes inside panes—shells, editors, agents, CLIs—form the **data plane**.

fugue coordinates *where* and *how* they run, not *what* they do.

---

## Intent vs. Bytes

Traditional terminal multiplexers forward bytes.

fugue accepts **intent**:

- "Create a pane running this command"
- "Resize this layout"
- "Block automation temporarily"

These intents are:
- validated
- arbitrated
- recorded
- replayable

This is the defining distinction between a multiplexer and a control plane.

---

## Arbitration and Authority

fugue mediates between multiple actors:

- Human users (TUI / CLI)
- Automated clients (MCP, agents)
- Future programmatic controllers

Key principles:

- Humans retain final authority
- Conflicts are resolved server-side
- Arbitration rules are explicit and deterministic
- Automation failures are visible, not silent

Temporary "human control modes" may block automated mutations without disabling observation or read-only access.

---

## Failure and Convergence

Failure is expected.

fugue provides:

- WAL-backed durability
- Checkpoint-based recovery
- Snapshot + event convergence
- Client resync after disconnect or lag

After failure, the system converges to a known-good state without manual intervention.

This behavior is characteristic of a control plane, not a UI tool.

---

## Relationship to Other Control Planes

fugue does **not** replace infrastructure control planes such as Kubernetes.

Instead:

- Kubernetes controls *infrastructure state*
- fugue controls *interactive workspace state*

fugue may run on top of infrastructure managed by Kubernetes or other systems, but it occupies a distinct abstraction layer.

---

## Design Implications

Treating fugue as a control plane implies:

- APIs are contracts, not helpers
- Correctness and convergence matter more than UI polish
- Observability is a first-class concern
- Backward compatibility must be considered carefully

These constraints are intentional and enable safe coordination between humans and automation over time.

---

## Summary

fugue is a control plane for interactive work.

It provides:
- authoritative state
- intent handling
- arbitration
- recovery

It deliberately avoids owning the data plane.

This separation allows fugue to remain:
- harness-agnostic
- command-agnostic
- future-compatible with emerging agent systems

