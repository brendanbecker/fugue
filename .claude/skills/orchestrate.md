# Orchestration Skill

Multi-agent orchestration commands for managing worker agents with watchdog monitoring.

## Commands

### `/orchestrate spawn <task>`

Spawn a new worker agent for a task.

**Usage:**
```
/orchestrate spawn bug-066
/orchestrate spawn "implement feature X"
```

**What it does:**
1. Creates a new session named `<task>-worker`
2. Tags the session with `worker`
3. Launches Claude with the task in `--dangerously-skip-permissions` mode

**Implementation:**
```
1. fugue_create_session(name: "<task>-worker", cwd: $PWD)
2. fugue_set_tags(session: "<task>-worker", add: ["worker"])
3. Get pane_id from the new session
4. fugue_send_input(pane_id: <pane>, input: "claude --dangerously-skip-permissions '<task>'", submit: true)
```

---

### `/orchestrate status`

Show status of all worker agents.

**Usage:**
```
/orchestrate status
```

**What it does:**
1. Lists all sessions tagged as `worker` or matching `*-worker` pattern
2. Shows each worker's Claude state (working, idle, waiting for input)
3. Shows how long each worker has been running

**Implementation:**
```
1. fugue_list_panes to get all panes
2. Filter for sessions matching *-worker or having tag:worker
3. For each: fugue_get_status to get Claude state
4. Format and display summary table
```

---

### `/orchestrate monitor start`

Start the watchdog agent that monitors workers.

**Usage:**
```
/orchestrate monitor start
/orchestrate monitor start --interval 60   # Check every 60 seconds
```

**What it does:**
1. Creates a `__watchdog` session with `watchdog` tag
2. Launches a Haiku-powered Claude with monitoring prompt
3. Starts native fugue timer that sends "check" at intervals
4. Tags current session as `orchestrator` to receive alerts

**Implementation:**
```
1. Tag self as orchestrator: fugue_set_tags(add: ["orchestrator"])
2. Create watchdog session: fugue_create_session(name: "__watchdog")
3. Tag watchdog: fugue_set_tags(session: "__watchdog", add: ["watchdog"])
4. Get watchdog pane_id
5. Launch Claude: fugue_send_input(pane_id: <watchdog_pane>, input: "claude --model haiku ...", submit: true)
6. Start timer: fugue_watchdog_start(pane_id: <watchdog_pane>, interval_secs: 90)
```

**Watchdog Model:** Uses Claude Haiku for cost efficiency (monitoring is frequent, simple).

---

### `/orchestrate monitor stop`

Stop the watchdog agent and timer.

**Usage:**
```
/orchestrate monitor stop
```

**What it does:**
1. Stops the native watchdog timer
2. Kills the `__watchdog` session

**Implementation:**
```
1. fugue_watchdog_stop()
2. fugue_kill_session(session: "__watchdog")
```

---

### `/orchestrate kill <session>`

Kill a specific worker session.

**Usage:**
```
/orchestrate kill bug-066-worker
```

**What it does:**
1. Kills the specified worker session and all its panes
2. Cleans up resources

**Implementation:**
```
1. fugue_kill_session(session: "<session>")
```

---

### `/orchestrate collect [session]`

Collect work from completed workers.

**Usage:**
```
/orchestrate collect                    # Collect from all completed workers
/orchestrate collect bug-066-worker    # Collect from specific worker
```

**What it does:**
1. Reads the final output from the worker's pane
2. Optionally kills the worker session after collecting
3. Returns summary of work completed

**Implementation:**
```
1. fugue_list_panes to find completed workers (idle state)
2. For each (or specified): fugue_read_pane(pane_id, lines: 200)
3. Parse output for completion summary
4. Optionally: fugue_kill_session to clean up
```

---

## Watchdog Agent Prompt

The watchdog uses this specialized system prompt:

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

Classification rules:
- working: Claude is actively processing (tool calls, thinking)
- complete: Shows completion message, back at prompt
- waiting: Waiting for user input/confirmation
- stuck: Idle for over 5 minutes with no completion message
- errored: Error messages visible in output

Alert format (use fugue_send_orchestration):
{
  "target": {"tag": "orchestrator"},
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

---

## Orchestrator Integration

As the orchestrator, you should:

1. **Tag yourself** at startup:
   ```
   fugue_set_tags(add: ["orchestrator"])
   ```

2. **Poll for messages** periodically:
   ```
   fugue_poll_messages(worker_id: <your_session>)
   ```

3. **Act on alerts** from the watchdog:
   - `complete` → Run `/orchestrate collect <session>`
   - `waiting` → Check what input is needed, provide it via `fugue_send_input`
   - `stuck` → Investigate with `fugue_read_pane`, decide whether to restart
   - `errored` → Read error, decide whether to fix and retry

---

## Configuration

Environment variables (set via session metadata or environment):

| Variable | Default | Description |
|----------|---------|-------------|
| `WATCHDOG_INTERVAL` | 90 | Seconds between checks |
| `WATCHDOG_MODEL` | haiku | Model for watchdog agent |
| `WORKER_IDLE_THRESHOLD` | 300 | Seconds before worker considered stuck |

---

## Example Workflow

```
# 1. Start monitoring
/orchestrate monitor start

# 2. Spawn workers for tasks
/orchestrate spawn bug-066
/orchestrate spawn feat-103

# 3. Check status periodically
/orchestrate status

# 4. When watchdog alerts about completed workers
/orchestrate collect bug-066-worker

# 5. When done, stop monitoring
/orchestrate monitor stop
```
