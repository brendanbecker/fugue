# ADR-001: The Dumb Pipe Strategy

## Status

Accepted

## Date

2026-01-16

## Context

ccmux began as a "Claude Code-aware terminal multiplexer" with ambitions to deeply integrate with AI agents. Over time, the codebase accumulated agent-specific features:

### Current Agent-Specific Code

**1. ClaudeState and ClaudeActivity (`ccmux-protocol/src/types.rs`)**
```rust
pub enum PaneState {
    Normal,
    Claude(ClaudeState),  // Claude-specific variant
    Exited { code: Option<i32> },
}

pub struct ClaudeState {
    pub session_id: Option<String>,
    pub activity: ClaudeActivity,
    pub model: Option<String>,
    pub tokens_used: Option<u64>,
}

pub enum ClaudeActivity {
    Idle,
    Thinking,
    Coding,
    ToolUse,
    AwaitingConfirmation,
}
```

**2. BeadsTask and BeadsStatus (`ccmux-protocol/src/types.rs`)**
```rust
pub struct BeadsTask {
    pub id: String,
    pub title: String,
    pub priority: i32,
    pub status: String,
    pub issue_type: String,
    pub assignee: Option<String>,
    pub labels: Vec<String>,
}

pub struct BeadsStatus {
    pub daemon_available: bool,
    pub ready_count: usize,
    pub ready_tasks: Vec<BeadsTask>,
    pub last_refresh: Option<u64>,
    pub error: Option<String>,
}
```

**3. Beads-Specific MCP Tools (`ccmux-server/src/mcp/bridge/handlers.rs`)**
- `ccmux_beads_assign` - Assign beads issue to pane
- `ccmux_beads_release` - Release issue from pane
- `ccmux_beads_find_pane` - Find pane working on issue
- `ccmux_beads_pane_history` - Get issue history for pane

### Problems with Tight Coupling

1. **Fragility**: When Claude Code's output format changes (e.g., new activity states, different progress indicators), the detection logic breaks. We've already seen this with status line parsing.

2. **Maintenance Burden**: Every agent integration requires protocol changes, server-side detection code, and client rendering logic. This creates an N x M explosion as we add more agents and more features.

3. **Limited Extensibility**: Other AI agents (Copilot, Cursor, custom agents) cannot benefit from ccmux's features without adding more hardcoded integrations.

4. **Architectural Pollution**: The core multiplexer's binary protocol carries tool-specific data structures (BeadsTask), coupling unrelated concerns.

5. **Testing Complexity**: Agent-specific code paths require mocking agent behavior, increasing test surface area.

### Observed Patterns

Experience has shown that:
- Agents are smarter than infrastructure—they can coordinate through simple message passing
- The most reliable features are the simplest ones (PTY multiplexing, input routing)
- Generic metadata storage is more useful than structured integration types
- Agents can opt into capabilities via sideband protocol rather than requiring detection

## Decision

ccmux will evolve toward being a **"dumb pipe"**—a minimal, reliable terminal multiplexer that agents communicate **through** rather than **with**.

### Core Principles

1. **Multiplex PTY streams reliably** - The primary job is connecting terminals to processes and routing I/O correctly.

2. **Provide generic metadata storage** - Sessions and panes have arbitrary key-value metadata that any client can use, not agent-specific structured types.

3. **Offer simple message passing** - The orchestration protocol delivers messages between sessions based on tags, not smart routing based on agent state.

4. **Expose capabilities via sideband protocol** - Agents opt into features (like health reporting) rather than ccmux detecting and inferring state.

### What Changes

| Before (Agent-Aware) | After (Dumb Pipe) |
|---------------------|-------------------|
| `PaneState::Claude(ClaudeState)` | `PaneState::Agent { name: String, metadata: HashMap }` |
| `BeadsTask` struct in protocol | Generic `Widget { type: String, data: JsonValue }` |
| `ClaudeActivity::Thinking` | `metadata.get("activity") == "thinking"` |
| Hardcoded beads MCP tools | Generic metadata read/write tools |
| Claude-specific detection logic | Optional agent self-reporting via sideband |

### What Stays the Same

- PTY management, input/output routing
- Session/window/pane hierarchy
- Orchestration message passing (tag-based routing)
- Sideband protocol (agents can still send structured commands)
- Generic metadata storage (already exists, just expand usage)

## Consequences

### Positive

- **Increased reliability**: Less code means fewer failure modes. The core multiplexer does one thing well.

- **Agent-agnostic**: Works with any AI agent (Claude, Copilot, custom agents, future agents) without code changes.

- **Easier maintenance**: No need to track agent API changes or update detection heuristics.

- **Cleaner architecture**: Separation of concerns—ccmux handles multiplexing, agents handle coordination.

- **Simpler testing**: Generic code paths are easier to test than agent-specific integrations.

- **Future-proof**: New agents and tools work automatically through generic interfaces.

### Negative

- **Loss of "magic" features**: Users won't see Claude-specific activity indicators (Thinking, Coding, ToolUse) without agent-side changes.

- **Agent-side burden**: Agents must explicitly report their state via sideband protocol rather than relying on detection.

- **Less "batteries included"**: Initial setup may require more configuration for advanced workflows.

- **Migration effort**: Existing code using ClaudeState/BeadsTask needs refactoring.

### Mitigation for Negatives

- Provide clear documentation and examples for agent-side integration patterns
- Offer a "Claude compatibility layer" skill or wrapper that translates between the old and new patterns
- Gradual deprecation with clear migration path

## Implementation Path

### Phase 1: Generic Widget System (FEAT-083)

Replace `BeadsTask` and similar hardcoded types with a generic widget protocol:

```rust
// Before
pub struct BeadsTask {
    pub id: String,
    pub title: String,
    pub priority: i32,
    // ... specific fields
}

// After
pub struct Widget {
    pub widget_type: String,       // e.g., "beads-task", "claude-progress"
    pub data: JsonValue,           // Arbitrary JSON payload
    pub priority: Option<i32>,     // Optional ordering hint
    pub expires_at: Option<u64>,   // Optional TTL
}
```

Clients render widgets based on `widget_type`, with unknown types showing a fallback.

### Phase 2: Abstract Agent State (FEAT-084)

Generalize `ClaudeState` to work with any agent:

```rust
// Before
pub enum PaneState {
    Normal,
    Claude(ClaudeState),
    Exited { code: Option<i32> },
}

// After
pub enum PaneState {
    Normal,
    Agent(AgentState),    // Generic agent state
    Exited { code: Option<i32> },
}

pub struct AgentState {
    pub agent_type: String,              // e.g., "claude", "copilot", "custom"
    pub status: AgentStatus,             // Busy, Idle, Error, AwaitingInput
    pub metadata: HashMap<String, String>, // Agent-specific details
}

pub enum AgentStatus {
    Idle,
    Busy,
    Error,
    AwaitingInput,
}
```

### Phase 3: Deprecate Agent-Specific Sideband Commands

- Mark beads-specific tools as deprecated
- Provide migration documentation
- Eventually remove in a future major version

### Phase 4: Document Agent-Side Patterns

Create comprehensive documentation for:
- How agents report their state via sideband
- How to create custom widgets
- Patterns for agent coordination through generic metadata
- Example integrations for common agents

## Examples: Before and After

### Example 1: Displaying Agent Activity

**Before** (hardcoded ClaudeActivity):
```rust
// Server detects Claude and parses status line
match pane.state {
    PaneState::Claude(cs) => {
        match cs.activity {
            ClaudeActivity::Thinking => render_spinner("Thinking..."),
            ClaudeActivity::Coding => render_progress("Writing code"),
            // ... handle each variant
        }
    }
}
```

**After** (generic metadata):
```rust
// Agent reports state via sideband: {"activity": "thinking", "message": "Analyzing code..."}
match pane.state {
    PaneState::Agent(state) => {
        let activity = state.metadata.get("activity").unwrap_or("idle");
        let message = state.metadata.get("message").unwrap_or("");
        render_status(activity, message);
    }
}
```

### Example 2: Task Tracking Widget

**Before** (BeadsTask in protocol):
```rust
// Protocol includes BeadsTask, server queries beads daemon
let beads_status = query_beads_daemon(pane.cwd);
for task in beads_status.ready_tasks {
    render_task_widget(task.id, task.title, task.priority);
}
```

**After** (generic Widget):
```rust
// Any tool can publish widgets via sideband
// Sideband: {"type": "widget.publish", "widget_type": "task", "data": {...}}
for widget in pane.widgets {
    match widget.widget_type.as_str() {
        "task" => render_task_widget(&widget.data),
        "progress" => render_progress_widget(&widget.data),
        _ => render_generic_widget(&widget),
    }
}
```

### Example 3: Agent Coordination

**Before** (Claude-specific orchestration):
```rust
// Orchestrator checks Claude state directly
if worker.claude_state.activity == ClaudeActivity::Idle {
    assign_task_to_worker(worker);
}
```

**After** (generic status check):
```rust
// Workers report status via orchestration messages
// Worker sends: {"type": "status.update", "status": "idle"}
// Orchestrator receives and routes tasks based on metadata
if worker.agent_state.status == AgentStatus::Idle {
    assign_task_to_worker(worker);
}
```

## References

- FEAT-083: Protocol Generalization: Generic Widget System
- FEAT-084: Protocol Generalization: Abstract Agent State
- `ccmux-protocol/src/types.rs` - Current ClaudeState, BeadsTask definitions
- `ccmux-server/src/mcp/bridge/handlers.rs` - Current beads-specific tools

## Decision Makers

- Architecture Team (implicit via PR review)

## Related Decisions

- This ADR establishes the strategic direction; FEAT-083 and FEAT-084 implement it.
