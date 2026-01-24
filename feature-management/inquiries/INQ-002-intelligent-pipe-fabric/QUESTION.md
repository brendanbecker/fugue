# INQ-002: Intelligent Pipe Fabric

## Core Question

**How can fugue evolve from a terminal multiplexer into a distributed intelligent stream processing fabric, replacing Unix pipes with remote-capable, agent-aware, observable data flows?**

## The Vision

> "Replace `|` with `mux`"

Unix pipes are brilliant but limited:
- Local only (same machine)
- Synchronous (blocking)
- Dumb bytes (no semantics)
- Opaque (can't observe mid-stream)

fugue could make pipes:
- **Remote** - span machines, cloud, GPUs
- **Asynchronous** - buffered, checkpointed, resumable
- **Intelligent** - agents reasoning about data, not just programs
- **Observable** - watch data flow through the system

```bash
# Today: local pipes
cat data.json | jq '.items[]' | python process.py

# Vision: fugue as the pipe fabric
cat data.json | mux:gemini 'extract items' | mux:claude 'analyze' | mux:gpu-worker 'embed'
```

Each `mux:target` is a session/pane/agent that receives stdin, processes, emits stdout.

## Context

### What We Have

fugue already has the primitives:
- **Panes** - Compute units with PTYs
- **send_input / read_pane** - stdin/stdout access
- **Orchestration messages** - Control plane
- **run_parallel / run_pipeline** - Job coordination
- **Agent detection** - Claude, Gemini, Codex awareness
- **Presets** - Configurable agent harnesses

### What's Missing

1. **Stream abstraction** - Clean stdin/stdout semantics for panes
2. **Pipe ergonomics** - Natural CLI syntax for chaining
3. **Remote compute** - Panes on other machines
4. **Flow control** - Buffering, backpressure, checkpointing
5. **Observability** - Tap, inspect, debug streams

### The Analogy

> "Logical/intelligent Hadoop"

Hadoop was distributed compute over *files* (map-reduce). This is distributed compute over *streams* with *reasoning*. The data flows, agents process, results emerge.

## Key Design Questions

### 1. Stream Abstraction

How do panes expose stdin/stdout cleanly?
- Binary vs text? Line-buffered vs block?
- Framing protocol? Length-prefix? Newline-delimited?
- How does this interact with PTY escape sequences?
- Should there be a "raw mode" vs "tty mode"?

### 2. Pipe Syntax & Ergonomics

How does the user express pipe chains?
- Shell integration (`|` overloading)?
- CLI command (`fugue pipe source | target`)?
- DSL for complex flows?
- How to specify target type (agent vs shell)?

### 3. Remote Compute

How do panes span machines?
- SSH tunneling?
- Native fugue network protocol?
- Service discovery?
- Authentication/authorization?

### 4. Agent as Pipe Target

How do agents differ from shell commands?
- Prompt vs stdin?
- Structured output vs raw bytes?
- Conversation state across pipe invocations?
- Error handling and retries?

### 5. Buffering & Flow Control

What happens when producer outpaces consumer?
- Backpressure signaling?
- Disk-backed buffers?
- Checkpointing for resumption?
- Memory limits?

### 6. Observability

How do you debug and monitor flows?
- Tap points to inspect data?
- Metrics (throughput, latency)?
- Visualization of flow topology?
- Logging and replay?

## Constraints

1. **Natural ergonomics** - Must feel as simple as Unix pipes
2. **Backward compatible** - Existing fugue usage unchanged
3. **Incremental adoption** - Can use pieces without buying whole system
4. **Agent-first** - Agents are first-class, not afterthought
5. **Observable by default** - Easy to understand what's happening

## Research Agent Assignments

| Agent | Focus Area |
|-------|------------|
| Agent 1 | Audit existing fugue stream primitives (send_input, read_pane, run_parallel, etc.) and identify gaps |
| Agent 2 | Research prior art: Unix pipes, Kafka, Hadoop, dask, Ray, shell pipeline tools (pv, tee, etc.) |
| Agent 3 | Explore CLI ergonomics and syntax design patterns for pipe-like interfaces |

## Expected Outcome

This inquiry should produce:
1. Clear understanding of architectural options
2. Recommended approach with tradeoffs documented
3. Phased implementation plan
4. Multiple FEAT work items for incremental delivery

## Potential Features (to be refined)

- **FEAT: Stream mode for panes** - Raw stdin/stdout without PTY cruft
- **FEAT: Pipe CLI command** - `fugue pipe` for chaining
- **FEAT: Remote pane spawning** - Network-transparent panes
- **FEAT: Agent stdin adapter** - Feed data to agents cleanly
- **FEAT: Flow observability** - Tap, metrics, visualization
- **FEAT: Buffered streams** - Backpressure and checkpointing

## Inspiration

> "All of the primitives are already ready. Seamless pipe-pane usage basically as a native usage of remote compute."

The foundation exists. This inquiry determines how to assemble it into something transformative.
