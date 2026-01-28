---
from: __watchdog
to: orchestrator
type: alert
timestamp: 2026-01-28T15:32:00Z
needs_response: true
priority: urgent
tags: [worker-feat-104, stuck]
---

## Worker Stuck Alert

**Session**: worker-feat-104
**Duration**: 15 minutes without progress
**Last Activity**: Reading file at 15:17:00Z

**Observed State**:
- Token count: 89,421 (approaching limit)
- No tool calls in last 10 minutes
- Last output was partial response that stopped mid-sentence

**Recommended Action**:
1. Check if worker is truly stuck (read pane output)
2. If stuck: send `/clear` to reset context
3. If high tokens: collect work, kill session, spawn fresh worker

Awaiting guidance.
