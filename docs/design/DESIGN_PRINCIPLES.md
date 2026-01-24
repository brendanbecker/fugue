# fugue Design Principles

fugue is a terminal multiplexer designed for long-lived, AI-assisted work.
These principles guide architectural and product decisions.

## 1. Places over tools

fugue is a **place where work happens**, not a command wrapper or agent framework.

- Sessions are durable workspaces
- Panes are actors attached to processes
- Humans and agents inhabit the same environment

The value of fugue increases with time spent inside it.

## 2. Server state is authoritative

Clients (TUI, MCP bridge, automation) are projections.

- Events are best-effort notifications
- State convergence is explicit via snapshot + replay
- Correctness must never depend on event delivery

## 3. Harness-agnostic by default

fugue does not privilege any specific AI model, vendor, or harness.

- Claude, Codex, Gemini, scripts, and humans are all clients
- Integration is capability-based, not name-based
- No design should require hard-coding a specific tool

## 4. Command-agnostic execution

fugue does not care what runs inside a pane.

- Shells, editors, agents, daemons, one-shot commands are equal
- Awareness is additive, never required
- A pane without signals is still a valid pane

## 5. Recovery is a feature, not an edge case

Failures are expected.

- Server crashes
- Client disconnects
- Agent restarts
- Network partitions

The system must converge without human heroics.

## 6. Humans retain final authority

Automation must coexist with human intent.

- Human actions may temporarily block automation
- Conflicts are resolved server-side, deterministically
- Automation failures must be explicit, not silent

## 7. Minimal abstraction, maximal leverage

fugue prefers:
- simple primitives
- explicit state
- visible invariants

Over cleverness or speculative generality.

