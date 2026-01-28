# FEAT-104: Watchdog Orchestration Skill

**Priority**: P1
**Component**: skill/orchestration
**Effort**: Medium
**Status**: done

## Summary

Create a Claude Code skill for multi-agent orchestration that includes a watchdog agent for monitoring spawned worker agents. The watchdog runs in a dedicated session and periodically checks on workers, alerting the orchestrator when workers need attention.

## Architecture

```
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│   Orchestrator  │     │    Watchdog     │     │    Workers      │
│   (session-0)   │◄────│   (__watchdog)  │────►│  (bug-066-*)    │
│                 │     │                 │     │  (feat-103-*)   │
│  tag:orchestrator│     │  tag:watchdog   │     │  tag:worker     │
└─────────────────┘     └─────────────────┘     └─────────────────┘
        ▲                       ▲
        │                       │
        │               ┌───────┴───────┐
        │               │ Background    │
        └───────────────│ Timer (90s)   │
          (orchestration│ "check"       │
           messages)    └───────────────┘
```

## Components

### 1. Orchestration Skill (`/orchestrate`)

Main skill that provides commands for multi-agent orchestration:

```bash
/orchestrate spawn <task>      # Spawn a worker agent for a task
/orchestrate status            # Show all workers and their status
/orchestrate monitor start     # Start the watchdog
/orchestrate monitor stop      # Stop the watchdog
/orchestrate kill <session>    # Kill a worker session
/orchestrate collect           # Collect completed work from workers
```

### 2. Watchdog Session

A dedicated Claude agent session that monitors workers:

- **Session name**: `__watchdog` (double underscore = system session)
- **Tag**: `watchdog`
- **Model**: Haiku (cheap, fast - just monitoring)
- **Prompt**: Specialized for worker monitoring

### 3. Background Timer

A background process that triggers the watchdog periodically:

```bash
# Runs in background, sends "check" to watchdog every N seconds
while true; do
  fugue_send_input --pane $WATCHDOG_PANE "check"
  sleep ${INTERVAL:-90}
done
```

### 4. Worker Conventions

Workers follow naming/tagging conventions for discovery:

- **Session naming**: `<type>-<id>-worker` (e.g., `bug-066-worker`, `feat-103-worker`)
- **Tag**: `worker`
- **Status reporting**: Workers use `fugue_report_status` to report state

## Watchdog Agent Behavior

When triggered with "check", the watchdog:

1. **Discover workers**: `fugue_list_panes` → filter by tag:worker or session pattern
2. **Assess each worker**:
   - `fugue_get_status` for state (Processing, Idle, Waiting for input)
   - `fugue_read_pane` for recent output (errors, completion messages)
3. **Classify status**:
   - `working` - Actively processing
   - `complete` - Finished successfully (prompt shows completion)
   - `waiting` - Waiting for user input/confirmation
   - `stuck` - Idle for too long or error state
   - `errored` - Error detected in output
4. **Notify orchestrator** (if needed):
   - Use `fugue_send_orchestration` to `tag:orchestrator`
   - Include summary and recommended action
5. **Be concise**: Don't interrupt unless necessary

### Watchdog System Prompt

```
You are a worker agent monitor. Your job is to periodically check on worker agents and alert the orchestrator when they need attention.

When you receive "check":
1. Use fugue_list_panes to find all panes
2. Filter for worker sessions (tag:worker or session names matching *-worker)
3. For each worker:
   - fugue_get_status to get current state
   - fugue_read_pane (last 30 lines) if state unclear
4. Classify each: working, complete, waiting, stuck, errored
5. If any workers need attention (complete, waiting, stuck, errored):
   - fugue_send_orchestration to tag:orchestrator with summary
6. If all workers healthy and working, respond briefly: "All N workers healthy"

Be concise. The orchestrator is busy - only interrupt when necessary.
Summarize, don't dump raw output.
```

## Orchestrator Integration

The orchestrator session:

1. **Tags itself**: `fugue_set_tags(add: ["orchestrator"])`
2. **Polls messages**: Periodically `fugue_poll_messages` for watchdog alerts
3. **Receives alerts**: Watchdog sends orchestration messages with worker summaries
4. **Takes action**: Reviews alerts, checks on workers, collects completed work

### Alert Message Format

```json
{
  "msg_type": "worker.alert",
  "payload": {
    "summary": "2 workers need attention",
    "workers": [
      {"session": "bug-066-worker", "status": "complete", "action": "collect work"},
      {"session": "feat-103-worker", "status": "waiting", "action": "provide input"}
    ]
  }
}
```

## Skill Commands Detail

### `/orchestrate spawn <task>`

```bash
# Creates worker session with proper tags and launches Claude
fugue_create_session(name: "<task>-worker", cwd: $PWD)
fugue_set_tags(session: "<task>-worker", add: ["worker"])
fugue_send_input(pane: <new_pane>, input: "claude --dangerously-skip-permissions '<task>'")
```

### `/orchestrate monitor start`

```bash
# Create watchdog session
fugue_create_session(name: "__watchdog")
fugue_set_tags(session: "__watchdog", add: ["watchdog"])

# Launch watchdog Claude with monitoring prompt
fugue_send_input(pane: <watchdog_pane>, input: "claude --system-prompt '<watchdog_prompt>'")

# Start background timer (in hidden pane or background shell)
fugue_create_pane(session: "__watchdog", command: "while true; do echo check; sleep 90; done")
```

### `/orchestrate monitor stop`

```bash
# Kill the watchdog session
fugue_kill_session(session: "__watchdog")
```

### `/orchestrate status`

```bash
# List all workers and their current status
fugue_list_panes | filter workers
for each worker:
  fugue_get_status
  format and display
```

### `/orchestrate collect <session>`

```bash
# Read worker's final output
fugue_read_pane(session: <session>, lines: 100)
# Optionally kill worker session
fugue_kill_session(session: <session>)
```

## Configuration

Configurable via skill arguments or environment:

| Setting | Default | Description |
|---------|---------|-------------|
| `WATCHDOG_INTERVAL` | 90 | Seconds between checks |
| `WATCHDOG_MODEL` | haiku | Model for watchdog agent |
| `WORKER_IDLE_THRESHOLD` | 300 | Seconds before worker considered stuck |
| `AUTO_COLLECT` | false | Auto-collect completed workers |

## Implementation Notes

### Using fugue MCP Tools

The skill leverages existing fugue MCP tools:

- `fugue_create_session` / `fugue_kill_session` - Session lifecycle
- `fugue_set_tags` / `fugue_get_tags` - Tag-based routing
- `fugue_send_orchestration` - Inter-agent messaging
- `fugue_get_status` / `fugue_read_pane` - Worker inspection
- `fugue_send_input` - Trigger watchdog and spawn workers
- `fugue_list_panes` - Worker discovery

### Haiku for Watchdog

The watchdog should use Claude Haiku:
- Cheap (monitoring is frequent)
- Fast (just checking status)
- Sufficient (no complex reasoning needed)

Use `fugue_create_pane` with `preset: "haiku-worker"` or configure model directly.

### Background Timer Implementation

Options:
1. **Separate pane**: Background shell in watchdog session
2. **fugue_run_parallel**: With infinite loop command
3. **External cron**: System-level scheduling

Recommend option 1 for simplicity and containment.

## Acceptance Criteria

- [ ] `/orchestrate spawn <task>` creates properly tagged worker session
- [ ] `/orchestrate monitor start` creates watchdog with timer
- [ ] `/orchestrate monitor stop` cleanly kills watchdog
- [ ] Watchdog correctly discovers workers by tag/name pattern
- [ ] Watchdog classifies worker status accurately
- [ ] Watchdog sends concise alerts to orchestrator
- [ ] Orchestrator receives and can act on alerts
- [ ] `/orchestrate status` shows all workers
- [ ] `/orchestrate collect` retrieves worker output
- [ ] Configurable interval and thresholds

## Future Enhancements

- **Auto-scaling**: Spawn more workers if backlog grows
- **Load balancing**: Distribute tasks across workers
- **Worker pools**: Pre-warmed worker sessions
- **Dashboard**: TUI showing worker grid status
- **Replay**: Re-run failed workers with different parameters

## Related

- FEAT-094: fugue_run_parallel (parallel command execution)
- FEAT-097: fugue_get_worker_status / fugue_poll_messages
- FEAT-102: Agent Status Pane
- BUG-065: Parallel MCP request serialization (now fixed)
- BUG-066: Mirror pane cross-session output
