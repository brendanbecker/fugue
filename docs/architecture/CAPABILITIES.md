# Capability Signaling in fugue

fugue supports optional awareness of processes running inside panes.
This awareness is **capability-based**, not command-based.

## Motivation

Hard-coding support for specific tools does not scale.
Instead, fugue discovers what a pane can do based on explicit signals.

This allows fugue to remain:

* harness-agnostic
* command-agnostic
* forward-compatible with new agents and tools

## Core Idea

A pane may advertise **capabilities** via sideband signaling or control channels.

If no capabilities are advertised:

* The pane is treated as a generic process
* No behavior breaks
* No special casing occurs

Awareness is **additive**, never required.

## Capability Categories (Examples)

Capabilities describe *what a pane can do*, not *what it is*.

### `state_signal`

Indicates the pane can emit structured state transitions.

Examples:

* Idle
* Thinking
* Waiting
* Streaming
* Complete

### `control_plane`

Indicates the pane can accept structured control commands (e.g. RPC, MCP-style interactions).

### `identity`

Indicates the pane can identify itself with stable metadata:

* agent name
* run ID
* task ID

### `resume`

Indicates the pane supports resumable execution across restarts.

## Discovery Mechanism (Example)

A process may emit a sideband capability announcement:

```
<fugue:capabilities>{
  "state": "sideband-v1",
  "control": "mcp",
  "identity": true,
  "resume": true
}</fugue:capabilities>
```

fugue records these capabilities as pane metadata.

## Preference Order

fugue prefers capability information in the following order:

1. **Explicit signaling**

   * Sideband protocol
   * RPC / MCP-style control

2. **Adapter-based integration**

   * Thin shims that translate external signals into fugue capabilities

3. **Heuristic detection**

   * Pattern matching or inference
   * Used only as a fallback

Command-name matching must **never** be the sole mechanism.

## Design Constraints

* Capability signaling is optional
* Absence of signals must not degrade core mux behavior
* Capabilities must be forward-compatible
* New capability types must not break older clients

## Non-Goals

This system does **not**:

* Require processes to implement any protocol
* Privilege specific AI vendors or tools
* Encode agent logic or behavior
* Enforce capability correctness

Capabilities are declarations of intent, not guarantees.

