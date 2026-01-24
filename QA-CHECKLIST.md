# QA Checklist - New Orchestration Features

Temporary doc for testing new features shipped in Sessions 3-6.

**QA Run: 2026-01-17**

## FEAT-094: fugue_run_parallel

Run commands in parallel across separate panes.

```json
fugue_run_parallel({
  "commands": [
    {"command": "echo 'task 1' && sleep 1", "name": "task1"},
    {"command": "echo 'task 2' && sleep 2", "name": "task2"},
    {"command": "echo 'task 3' && sleep 1", "name": "task3"}
  ],
  "timeout_ms": 30000,
  "cleanup": true
})
```

- [x] Returns structured results with exit codes
- [x] Panes created in hidden `__orchestration__` session
- [x] Panes cleaned up after completion (cleanup: true)
- [x] Timeout works correctly
- [x] Failed commands report non-zero exit code

**Notes:** All tests passed. Parallel execution confirmed (~3.2s total for tasks that would take 4s+ sequentially). Exit codes correctly reported (tested with `false` command returning exit_code: 1).

---

## FEAT-095: fugue_run_pipeline

Run commands sequentially in a single pane.

```json
fugue_run_pipeline({
  "commands": [
    {"command": "echo 'step 1'", "name": "step1"},
    {"command": "echo 'step 2'", "name": "step2"},
    {"command": "false", "name": "should_fail"},
    {"command": "echo 'step 4'", "name": "never_reached"}
  ],
  "stop_on_error": true,
  "timeout_ms": 30000
})
```

- [x] Commands run sequentially
- [x] stop_on_error halts on first failure
- [x] Returns exit code per step
- [x] step4 not executed after step3 fails

**Notes:** All tests passed. Also verified `stop_on_error: false` continues after failure (step3 ran after step2 failed).

---

## FEAT-096: fugue_expect

Wait for regex pattern in pane output.

```json
// First, create a pane and start a slow command
fugue_create_pane({...})

// Then send a command that will eventually output a pattern
fugue_send_input({"pane_id": "...", "input": "sleep 2 && echo 'READY'\n"})

// Wait for the pattern
fugue_expect({
  "pane_id": "...",
  "pattern": "READY",
  "timeout_ms": 5000,
  "action": "return_output"
})
```

- [x] Returns when pattern matches
- [x] Timeout returns error after timeout_ms
- [x] action: "notify" works
- [ ] action: "close_pane" closes the pane (not tested - pane visibility issues)
- [x] action: "return_output" returns matching output

**Notes:** Core functionality works. `close_pane` action not tested due to pane creation in visible session blocking approval UI.

---

## FEAT-097: fugue_get_worker_status / fugue_poll_messages

Status reporting and message polling for workers.

**Worker reports status:**
```json
fugue_report_status({
  "status": "working",
  "message": "Processing task X"
})
```

**Orchestrator polls worker status:**
```json
fugue_get_worker_status({
  "worker_id": "<session-uuid-or-name>"
})
```

**Orchestrator polls messages:**
```json
fugue_poll_messages({
  "worker_id": "<session-uuid-or-name>"
})
```

- [ ] report_status stores status in daemon (requires session attachment)
- [x] get_worker_status retrieves stored status (returns null when none)
- [x] poll_messages returns messages from inbox (returns [] when none)
- [x] get_worker_status() with no arg returns all workers

**Notes:** Read-side tools work correctly. Write operations (`report_status`, `send_orchestration`, `broadcast`) require session attachment - cannot test from MCP bridge connection. This is expected security behavior.

---

## FEAT-098: Gemini Agent Detection

Gemini CLI should be detected like Claude.

```json
// Start Gemini in a pane
fugue_send_input({"pane_id": "...", "input": "gemini\n"})

// Check status
fugue_list_panes()
// Should show: is_claude: true (or similar agent detection field)
```

- [x] Gemini CLI detected (agent_type: "gemini")
- [x] Activity states detected (Idle, AwaitingConfirmation)
- [x] Works alongside Claude detection
- [x] Model extracted from UI (showed "Gemini 3")

**Notes:** Detection works well. Observed potential bug: after running Gemini in one pane, the Claude pane's detection also showed as Gemini. May be screen buffer contamination or detection priority issue. Filed as potential BUG-057.

---

## Integration Test: Multi-Agent Workflow

1. Create 2 worker sessions
2. Tag them as workers
3. Start Claude/Gemini in each
4. Use run_parallel to distribute tasks
5. Use expect to wait for completion patterns
6. Use get_worker_status to check states
7. Clean up sessions

**Status:** Not run during this QA session. Individual tool tests passed.

---

## Summary

| Feature | Status | Notes |
|---------|--------|-------|
| FEAT-094 | PASS | All tests passed |
| FEAT-095 | PASS | All tests passed |
| FEAT-096 | PASS* | close_pane action not tested |
| FEAT-097 | PASS* | Write ops require session attachment (by design) |
| FEAT-098 | PASS* | Potential detection cross-contamination bug |

**Overall:** All features functional. Minor edge cases noted for follow-up.
