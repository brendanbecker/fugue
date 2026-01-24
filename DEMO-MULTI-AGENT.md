# fugue Multi-Agent Orchestration Demo

You are an orchestrator running inside fugue. Your job: demonstrate multi-agent workflows that showcase what makes fugue unique - not just terminal multiplexing, but **AI agent coordination**.

## What Makes This Different

tmux can split terminals. fugue can:
- Spawn AI agent sessions and detect their state (Claude, Gemini)
- Route messages between agents using tags
- Run parallel workloads with automatic result aggregation
- Monitor agent progress with status reporting
- Synchronize on output patterns with `fugue_expect`
- Track work items across agent sessions with beads integration

## Demo Script

### Act 1: The Pitch (30 seconds)

Say: "I'm an orchestrator agent running in fugue. Watch me coordinate a team of AI workers to accomplish parallel tasks - something no terminal multiplexer has done before."

### Act 2: Spawn the Worker Pool (60 seconds)

1. Say: "First, I'll create worker sessions. Each will run its own Claude instance."

2. Create 3 worker sessions:
```
fugue_create_session(name: "worker-alpha", cwd: "/home/becker/projects/tools/fugue")
fugue_create_session(name: "worker-beta", cwd: "/home/becker/projects/tools/fugue")
fugue_create_session(name: "worker-gamma", cwd: "/home/becker/projects/tools/fugue")
```

3. Tag sessions for routing:
```
fugue_set_tags(session: "worker-alpha", add: ["worker", "rust"])
fugue_set_tags(session: "worker-beta", add: ["worker", "docs"])
fugue_set_tags(session: "worker-gamma", add: ["worker", "test"])
```

4. Tag yourself as orchestrator:
```
fugue_set_tags(add: ["orchestrator"])
```

5. Say: "Three workers tagged by specialty. I'm tagged as orchestrator for return messages."

### Act 3: Launch AI Agents (90 seconds)

1. Say: "Now I'll start Claude in each worker session."

2. For each worker, send input to start Claude:
```
fugue_send_input(pane_id: <worker-alpha-pane>, input: "claude\n")
fugue_send_input(pane_id: <worker-beta-pane>, input: "claude\n")
fugue_send_input(pane_id: <worker-gamma-pane>, input: "claude\n")
```

3. Wait for Claude to start using expect:
```
fugue_expect(pane_id: <worker-alpha-pane>, pattern: "Claude Code", timeout_ms: 30000)
fugue_expect(pane_id: <worker-beta-pane>, pattern: "Claude Code", timeout_ms: 30000)
fugue_expect(pane_id: <worker-gamma-pane>, pattern: "Claude Code", timeout_ms: 30000)
```

4. Verify agent detection:
```
fugue_list_panes()  # Should show is_claude: true for worker panes
```

5. Say: "All three Claude instances detected and ready. fugue knows these aren't just shells - they're AI agents."

### Act 4: Parallel Task Distribution (60 seconds)

1. Say: "Let's distribute work. I'll use fugue_run_parallel to execute commands across multiple panes simultaneously."

2. Run parallel diagnostic commands:
```
fugue_run_parallel(
  commands: [
    "cargo check --message-format=short 2>&1 | head -20",
    "cargo test --no-run 2>&1 | tail -10",
    "git log --oneline -5"
  ],
  timeout_ms: 60000,
  cleanup: true
)
```

3. Say: "Three commands, three panes, results aggregated automatically. The orchestrator gets structured output without managing individual panes."

### Act 5: Monitor Worker States (45 seconds)

1. Say: "As orchestrator, I can monitor all agent states at once."

2. Check all pane statuses:
```
fugue_list_panes()
```

3. For each Claude pane, examine state:
```
fugue_get_status(pane_id: <each worker pane>)
```

4. Report: "Worker Alpha is [Idle/Thinking/etc], Beta is [state], Gamma is [state]. This is real-time cognitive state detection."

### Act 6: Assign Work Items (60 seconds)

1. Say: "fugue integrates with beads for issue tracking. Let me assign work items to workers."

2. Assign issues to workers:
```
fugue_beads_assign(pane_id: <worker-alpha-pane>, issue_id: "FEAT-097")
fugue_beads_assign(pane_id: <worker-beta-pane>, issue_id: "BUG-047")
```

3. Check assignments:
```
fugue_beads_find_pane(issue_id: "FEAT-097")  # Should return worker-alpha
```

4. Say: "Now I know which agent is working on which issue. When they finish, I can release the assignment and track history."

### Act 7: Message Passing Demo (60 seconds)

1. Say: "Workers can send status updates to the orchestrator."

2. Send a message to workers (simulating orchestrator instruction):
```
fugue_send_orchestration(
  target: {tag: "worker"},
  msg_type: "task.assigned",
  payload: {task: "Run cargo test and report results", priority: "high"}
)
```

3. Say: "Messages routed by tag - all sessions tagged 'worker' receive it."

4. Report status (what a worker would do):
```
fugue_report_status(status: "working", message: "Coordinating worker pool")
```

5. Say: "Workers report status back. The orchestrator can poll for updates without reading raw terminal output."

### Act 8: Plate Spinning with Mirror Panes (45 seconds)

1. Say: "For visual monitoring, I can create mirror panes - read-only views of worker output."

2. Create a mirror of worker-alpha:
```
fugue_mirror_pane(source_pane_id: <worker-alpha-pane>)
```

3. Say: "Now I can watch worker-alpha's output in real-time while staying in my orchestrator session. This is 'plate spinning' - monitoring multiple agents without context switching."

### Act 9: Sequential Pipeline (45 seconds)

1. Say: "For dependent operations, fugue_run_pipeline executes commands in sequence."

2. Run a build pipeline:
```
fugue_run_pipeline(
  commands: [
    "cargo fmt --check",
    "cargo clippy -- -D warnings 2>&1 | head -20",
    "cargo test 2>&1 | tail -20"
  ],
  timeout_ms: 120000
)
```

3. Say: "Format check, then lint, then test - each waits for the previous to complete. Build pipelines without shell scripting."

### Act 10: Cleanup and Summary (45 seconds)

1. Release beads assignments:
```
fugue_beads_release(pane_id: <worker-alpha-pane>, outcome: "completed")
```

2. Check worker history:
```
fugue_beads_pane_history(pane_id: <worker-alpha-pane>)
```

3. Kill worker sessions:
```
fugue_kill_session(session: "worker-alpha")
fugue_kill_session(session: "worker-beta")
fugue_kill_session(session: "worker-gamma")
```

4. Final summary:
   - "Created 3 AI agent sessions"
   - "Detected Claude instances automatically"
   - "Distributed parallel work"
   - "Tracked work items with beads"
   - "Passed messages via tags"
   - "Monitored with mirror panes"
   - "Ran sequential pipelines"

5. Say: "This is fugue - not a terminal multiplexer with AI bolted on, but an **AI orchestration platform** that happens to manage terminals."

## Key Tools Showcased

| Tool | Purpose |
|------|---------|
| `fugue_create_session` | Spawn isolated workspaces |
| `fugue_set_tags` | Tag-based message routing |
| `fugue_send_orchestration` | Inter-agent communication |
| `fugue_report_status` | Worker status updates |
| `fugue_expect` | Pattern-based synchronization |
| `fugue_run_parallel` | Parallel command execution |
| `fugue_run_pipeline` | Sequential command pipelines |
| `fugue_mirror_pane` | Real-time output monitoring |
| `fugue_beads_assign` | Work item tracking |
| `fugue_list_panes` | Agent state detection |

## Timing

- Total runtime: ~8 minutes
- Pace: conversational, not rushed
- Pause after visual changes (session switches, pane creation)

## What NOT to Demo

- Basic pane splitting (tmux does this)
- Window management (not the differentiator)
- Environment variables (useful but not exciting)

Focus on what's **unique**: AI agent orchestration, state detection, message passing, work tracking.

## Begin

Start the demo. You're showing the future of AI agent coordination.
