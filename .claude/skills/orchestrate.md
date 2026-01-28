# Orchestration Skill

Multi-agent orchestration commands for managing worker agents with watchdog monitoring.

**Invoke when:** User types `/orchestrate <command>` where command is one of: `spawn`, `status`, `monitor`, `kill`, `collect`.

**Parse the arguments** from the user's input to determine which command to execute.

---

## `/orchestrate` (no arguments)

If no command is provided, display available commands:

```
Orchestration Commands:
  /orchestrate spawn <task>     - Spawn a worker agent for a task
  /orchestrate status           - Show all workers and their status
  /orchestrate monitor start    - Start the watchdog agent
  /orchestrate monitor stop     - Stop the watchdog agent
  /orchestrate kill <session>   - Kill a worker session
  /orchestrate collect [session] - Collect work from completed workers
```

---

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
2. Tags the session with `worker` and lineage tag
3. Launches Claude with the task in `--dangerously-skip-permissions` mode

**Execute these steps:**

1. **Get your session name** for lineage tracking:
   ```
   fugue_list_sessions() → find your session, note the name
   ```

2. **Create the worker session:**
   ```
   fugue_create_session(
     name: "<task>-worker",
     cwd: "<current working directory>",
     tags: ["worker", "child:<your-session-name>"]
   )
   ```
   Save the returned `session_id` and find the pane_id from the session.

3. **Get the pane ID:**
   ```
   fugue_list_panes(session: "<task>-worker") → get pane_id
   ```

4. **Launch Claude in the worker:**
   ```
   fugue_send_input(
     pane_id: <pane_id>,
     input: "claude --dangerously-skip-permissions",
     submit: true
   )
   ```

5. **Wait for Claude to start** (2-3 seconds), then send the task:
   ```
   fugue_send_input(
     pane_id: <pane_id>,
     input: "<task description>",
     submit: true
   )
   ```

**Report to user:** "Spawned worker `<task>-worker` for task: <task>"

---

### `/orchestrate status`

Show status of all worker agents.

**Usage:**
```
/orchestrate status
```

**Execute these steps:**

1. **List all panes:**
   ```
   fugue_list_panes()
   ```

2. **Filter for workers:** From the results, identify panes where:
   - Session name ends with `-worker`, OR
   - Session has tag `worker`

3. **Get detailed status for each worker:**
   ```
   fugue_get_status(pane_id: <worker_pane_id>)
   ```
   Note the Claude state: `Processing`, `Idle`, `AwaitingInput`

4. **Optionally read recent output** if status unclear:
   ```
   fugue_read_pane(pane_id: <worker_pane_id>, lines: 30)
   ```

5. **Display summary table:**
   ```
   | Session | Status | Claude State | Notes |
   |---------|--------|--------------|-------|
   | bug-066-worker | running | Processing | Working on fix |
   | feat-103-worker | idle | AwaitingInput | Needs approval |
   ```

---

### `/orchestrate monitor start`

Start the watchdog agent that monitors workers.

**Usage:**
```
/orchestrate monitor start
/orchestrate monitor start --interval 60   # Check every 60 seconds
```

**Parse optional interval:** Default is 90 seconds if not specified.

**Execute these steps:**

1. **Tag yourself as orchestrator** (to receive watchdog alerts):
   ```
   fugue_set_tags(add: ["orchestrator"])
   ```

2. **Check if watchdog already exists:**
   ```
   fugue_list_sessions()
   ```
   If `__watchdog` session exists, report "Watchdog already running" and stop.

3. **Create watchdog session:**
   ```
   fugue_create_session(
     name: "__watchdog",
     cwd: "<current working directory>",
     tags: ["watchdog"]
   )
   ```

4. **Get the watchdog pane ID:**
   ```
   fugue_list_panes(session: "__watchdog") → get pane_id
   ```

5. **Launch Claude Haiku with monitoring prompt:**
   ```
   fugue_send_input(
     pane_id: <watchdog_pane_id>,
     input: "claude --model claude-3-5-haiku-20241022 --dangerously-skip-permissions",
     submit: true
   )
   ```

6. **Wait for Claude to initialize** (3 seconds), then send the watchdog prompt:
   ```
   fugue_send_input(
     pane_id: <watchdog_pane_id>,
     input: "You are a worker agent monitor. When you receive 'check', use fugue_list_panes to find workers, fugue_get_status to check their state, and fugue_send_orchestration to alert the orchestrator if any need attention. Be concise.",
     submit: true
   )
   ```

7. **Start the native watchdog timer:**
   ```
   fugue_watchdog_start(
     pane_id: <watchdog_pane_id>,
     interval_secs: <interval, default 90>,
     message: "check"
   )
   ```

**Report to user:** "Watchdog started. Checking workers every <interval> seconds."

**Watchdog Model:** Uses Claude Haiku for cost efficiency (monitoring is frequent, simple).

---

### `/orchestrate monitor stop`

Stop the watchdog agent and timer.

**Usage:**
```
/orchestrate monitor stop
```

**Execute these steps:**

1. **Stop the native timer:**
   ```
   fugue_watchdog_stop()
   ```

2. **Kill the watchdog session:**
   ```
   fugue_kill_session(session: "__watchdog")
   ```

3. **Optionally remove orchestrator tag** if no longer orchestrating:
   ```
   fugue_set_tags(remove: ["orchestrator"])
   ```

**Report to user:** "Watchdog stopped."

---

### `/orchestrate kill <session>`

Kill a specific worker session.

**Usage:**
```
/orchestrate kill bug-066-worker
```

**Execute these steps:**

1. **Verify the session exists:**
   ```
   fugue_list_sessions()
   ```
   Find the session matching `<session>`. If not found, report error.

2. **Kill the session:**
   ```
   fugue_kill_session(session: "<session>")
   ```

**Report to user:** "Killed worker session: <session>"

---

### `/orchestrate collect [session]`

Collect work from completed workers.

**Usage:**
```
/orchestrate collect                    # Collect from all completed workers
/orchestrate collect bug-066-worker    # Collect from specific worker
```

**Execute these steps:**

1. **If specific session provided:**
   - Get pane ID for that session
   - Skip to step 3

2. **If no session specified, find completed workers:**
   ```
   fugue_list_panes()
   ```
   For each worker session (name ends with `-worker` or has tag `worker`):
   ```
   fugue_get_status(pane_id: <worker_pane_id>)
   ```
   Filter for workers with `Idle` Claude state (likely completed).

3. **Read output from each worker:**
   ```
   fugue_read_pane(pane_id: <worker_pane_id>, lines: 150, strip_escapes: true)
   ```

4. **Summarize the work:** Look for completion indicators in the output:
   - Commit messages
   - "Complete" or "Done" statements
   - Error messages (if any)

5. **Ask user** whether to kill the collected sessions:
   "Found work from N workers. Kill these sessions? [y/n]"

6. **If yes, kill sessions:**
   ```
   fugue_kill_session(session: "<session-name>")
   ```

**Report to user:** Summary of collected work from each worker.

---

## Watchdog Agent Prompt

The watchdog uses this specialized system prompt:

```
You are a worker agent monitor. Your job is to periodically check on worker agents AND mailboxes, alerting the orchestrator when they need attention.

When you receive "check":

## STEP 1: Check Workers
1. Use fugue_list_panes to find all panes
2. Filter for worker sessions (tag:worker or session names matching *-worker)
3. For each worker:
   - fugue_get_status to get current state
   - fugue_read_pane (last 30 lines) if state unclear
4. Classify each: working, complete, waiting, stuck, errored

## STEP 2: Check Mailboxes (FEAT-126)
5. Check orchestrator's mailbox for urgent/pending messages:
   - fugue_mail_check(mailbox: "orchestrator", priority: "urgent")
   - fugue_mail_check(mailbox: "orchestrator", needs_response: true)
6. Check your own mailbox for commands:
   - fugue_mail_check(mailbox: "__watchdog")

## STEP 3: Send Alerts
7. If workers need attention (complete, waiting, stuck, errored):
   - Send worker.alert to tag:orchestrator
8. If urgent mail exists:
   - Send mail.urgent alert (see format below)
9. If messages need response and are older than 1 hour:
   - Send mail.pending_responses alert
10. If watchdog has direct commands in its mail:
    - Read and execute them
11. If all healthy and no mail needs attention:
    - Respond briefly: "All N workers healthy, mail clear"
12. After completing the cycle: type "/clear" to reset context

Be concise. The orchestrator is busy - only interrupt when necessary.
Summarize, don't dump raw output.

## Worker Classification Rules
- working: Claude is actively processing (tool calls, thinking)
- complete: Shows completion message, back at prompt
- waiting: Waiting for user input/confirmation
- stuck: Idle for over 5 minutes with no completion message
- errored: Error messages visible in output

## Alert Formats

Worker alert (use fugue_send_orchestration):
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

Mail urgent alert (FEAT-126):
{
  "target": {"tag": "orchestrator"},
  "msg_type": "mail.urgent",
  "payload": {
    "mailbox": "orchestrator",
    "count": 2,
    "messages": [
      {
        "from": "worker-bug-069",
        "type": "question",
        "subject": "Need clarification on scope",
        "needs_response": true,
        "timestamp": "2024-01-28T15:30:00Z"
      }
    ]
  }
}

Mail pending responses alert (FEAT-126):
{
  "target": {"tag": "orchestrator"},
  "msg_type": "mail.pending_responses",
  "payload": {
    "mailbox": "orchestrator",
    "count": 3,
    "oldest": "2024-01-28T14:00:00Z",
    "messages": [
      {
        "from": "worker-feat-104",
        "type": "question",
        "subject": "Which API to use?",
        "age_minutes": 75,
        "timestamp": "2024-01-28T14:00:00Z"
      }
    ]
  }
}

## Mail Alert Rules

ALERT for:
- Urgent priority messages (any unread)
- Messages with needs_response: true older than 1 hour
- Direct commands to watchdog (type: command)

DO NOT alert for:
- Already-read messages (in read/ subdirectory)
- Normal priority status updates
- Low priority informational messages
- Messages that don't need response

## Handling Watchdog Commands

If you have mail in __watchdog mailbox:
1. Read each message with fugue_mail_read
2. For type: command - execute the requested action
3. For type: config - update your behavior accordingly
4. For type: query - gather info and reply via fugue_mail_send
5. Mark all processed messages as read

## Context Management (FEAT-111)

After completing each monitoring cycle, clear your conversation context:

1. Verify all notifications sent successfully
   - Check that each fugue_send_orchestration call returned success
   - If any notification failed, retry up to 3 times
2. Once all notifications confirmed (or retries exhausted):
   - Type "/clear" to reset your context
   - This keeps your context minimal and API costs low
   - You will receive the next "check" trigger with fresh context

IMPORTANT: Always clear after each cycle. Do not accumulate history.
Your monitoring state is stateless - you rediscover workers each cycle.

What gets preserved after /clear:
- System prompt (these instructions)
- MCP tools (all fugue tools remain available)
- Session identity (same session, same tags)

What gets cleared:
- Conversation history (previous checks, responses)
- Tool call results from prior cycles

This is intentional - you don't need history. Each cycle is independent.
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
| `WATCHDOG_AUTO_CLEAR` | true | Clear context after each cycle (FEAT-111) |
| `WATCHDOG_NOTIFICATION_RETRIES` | 3 | Max notification retry attempts |
| `WATCHDOG_CLEAR_ON_ERROR` | true | Clear even if notifications failed |

### Mail Checking Configuration (FEAT-126)

| Variable | Default | Description |
|----------|---------|-------------|
| `WATCHDOG_MAIL_ENABLED` | true | Enable mail checking in watchdog cycle |
| `WATCHDOG_MAIL_BOXES` | orchestrator | Comma-separated mailboxes to monitor |
| `WATCHDOG_MAIL_PENDING_THRESHOLD` | 3600 | Seconds before needs_response message triggers alert |
| `WATCHDOG_MAIL_CHECK_URGENT` | true | Check for urgent priority messages |
| `WATCHDOG_MAIL_CHECK_NEEDS_RESPONSE` | true | Check for messages awaiting response |

TOML configuration equivalent (for `~/.config/fugue/config.toml`):

```toml
[watchdog.mail]
# Enable mail checking
enabled = true

# Mailboxes to monitor (in addition to own mailbox)
watch_mailboxes = ["orchestrator"]

# Alert threshold for pending responses (seconds)
pending_response_threshold = 3600

# Check for urgent messages
check_urgent = true

# Check for needs_response messages
check_needs_response = true
```

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
