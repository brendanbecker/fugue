---
from: worker-feat-104
to: orchestrator
type: question
timestamp: 2026-01-28T15:55:00Z
needs_response: true
priority: normal
tags: [FEAT-104, watchdog, design-question]
thread_id: feat-104-implementation
---

## Design Question: Watchdog Alert Threshold

I'm implementing the watchdog monitoring loop and need clarification on alert thresholds.

**Question**: How many consecutive "no progress" checks should trigger a `worker.stuck` alert?

**Options**:
1. **Immediate** (1 check) - Most responsive, but may false-positive during long tool calls
2. **3 checks** (~90 seconds at 30s interval) - Balanced
3. **5 checks** (~150 seconds) - Conservative, fewer false positives

**My recommendation**: Option 2 (3 checks) balances responsiveness with avoiding false alerts during legitimate long operations like file writes or API calls.

Please advise.
