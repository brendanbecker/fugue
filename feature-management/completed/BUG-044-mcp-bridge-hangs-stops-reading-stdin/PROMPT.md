# BUG-044: MCP bridge process hangs indefinitely, stops reading stdin

**Priority**: P1
**Component**: mcp-bridge
**Severity**: high
**Status**: new

## Problem Statement

The mcp-bridge process can enter a state where it stops reading from stdin entirely, causing all MCP tool calls to hang indefinitely. The 25-second daemon response timeout does not trigger. Killing the hung process and letting Claude Code spawn a fresh one resolves the issue.

## Evidence

### Socket State (Data Stuck in Queues)
```
# stdin has 1323 bytes queued but not being read
# Claude Code blocked trying to send more data (6912 bytes)
u_str ESTAB 1323   0    * 34100 (stdin)  * 34101
u_str ESTAB 0   6912    * 34101          * 34100

# stdout is clear (no backpressure)
# daemon connection established and clear (0 bytes queued)
```

### Thread Syscalls (All Waiting on Futex)
```
Thread 6696: 202 (futex)
Thread 6699: 232 (epoll_wait)
Thread 6700-6709: 202 (futex)
```

### Timing
- Tested: MCP call hung for 4.5+ minutes without timeout
- No error messages in mcp-bridge.log after initial connection

## Steps to Reproduce

1. Configure ccmux MCP server in Claude Code settings
2. Start ccmux daemon (`ccmux-server start`)
3. Use Claude Code to invoke MCP tools repeatedly
4. Eventually mcp-bridge enters hung state
5. Observe MCP tool calls hang for 4.5+ minutes with no timeout
6. Verify with socket analysis: `ss -x | grep $(pgrep -f mcp-bridge)`
7. Verify with thread analysis: `for t in /proc/$(pgrep -f mcp-bridge)/task/*; do echo "Thread $(basename $t): $(cat $t/syscall | cut -d' ' -f1)"; done`

## Expected Behavior

- MCP bridge should continuously read from stdin and process requests
- The 25-second daemon response timeout should trigger if processing stalls
- No MCP call should hang longer than timeout + grace period

## Actual Behavior

- MCP tool calls hang indefinitely (4.5+ minutes tested)
- stdin buffer fills up but data is not being read
- Process blocked on futex, not I/O
- 25-second timeout never triggers
- Only resolution is to kill the process

## Root Cause Hypotheses

### 1. Async/Sync Mixing Issue (Most Likely)
The main loop uses synchronous `stdin.lock().lines()` inside an async function. While `handle_request().await` is running, the stdin iterator may enter a bad state. The synchronous stdin read could be blocking the async runtime or vice versa.

### 2. Deadlock in ConnectionManager
The ConnectionManager has complex state with multiple channels (`daemon_tx`, `state_tx`, `state_rx`) and RwLocks. A deadlock between the health monitor task and request handling could cause the main loop to block waiting on a lock that will never be released.

### 3. Tokio Runtime Starvation
If a task doesn't yield properly (e.g., a tight loop without `.await`), it could starve other tasks including the one that should be reading stdin.

### 4. Response Handling Infinite Loop
Despite the 25-second timeout in `recv_filtered`, something might not be timing out correctly under certain message patterns. The filtering logic for broadcast messages might get stuck.

## Related Issues

- **BUG-039** (supposedly fixed) - MCP tools hang intermittently
- **BUG-043** - Sequenced message wrapper not unwrapped (may be contributing factor if message parsing fails silently)

## Implementation Tasks

### Section 1: Investigation
- [ ] Reproduce the bug consistently (or set up monitoring to catch it)
- [ ] Add diagnostic logging at key points:
  - Before/after stdin.lock().lines()
  - Before/after handle_request().await
  - In ConnectionManager lock acquisitions
  - In recv_filtered timeout handling
- [ ] Analyze thread dumps when hung to identify exact blocking point
- [ ] Review async/sync boundary in main loop

### Section 2: Root Cause Identification
- [ ] Confirm which hypothesis is correct through logging/debugging
- [ ] Document the exact sequence of events leading to hang
- [ ] Identify why 25-second timeout is not triggering

### Section 3: Fix Implementation
- [ ] If async/sync issue: Convert stdin reading to async (tokio::io::stdin())
- [ ] If deadlock: Refactor ConnectionManager to avoid lock contention
- [ ] If runtime starvation: Add yields in long-running operations
- [ ] If timeout issue: Fix recv_filtered timeout logic
- [ ] Add error handling for edge cases

### Section 4: Watchdog/Recovery
- [ ] Consider adding a watchdog thread that detects hangs
- [ ] Implement graceful recovery or restart mechanism
- [ ] Add heartbeat logging to detect stalls early

### Section 5: Testing
- [ ] Add stress test that invokes MCP tools rapidly
- [ ] Add test that simulates slow/stuck daemon responses
- [ ] Verify 25-second timeout actually triggers
- [ ] Run extended soak test to confirm fix

### Section 6: Verification
- [ ] Confirm hang no longer occurs under normal usage
- [ ] Verify all acceptance criteria met
- [ ] No performance regression from fix
- [ ] Update bug report with resolution details

## Acceptance Criteria

- [ ] Identify exact point where mcp-bridge gets stuck
- [ ] Add diagnostic logging to capture state when hang detected
- [ ] Ensure 25-second timeout actually triggers
- [ ] Consider watchdog to detect and recover from hangs
- [ ] No MCP call should hang longer than timeout + grace period (30 seconds max)
- [ ] Fix passes stress testing with rapid MCP tool invocations
- [ ] No regression in normal MCP tool performance

## Workaround

Kill the mcp-bridge process: `kill $(pgrep -f "mcp-bridge")`

Claude Code will spawn a fresh process on next tool call.

## Notes

This bug appears to be a regression or evolution of BUG-039, which was marked as fixed. The previous fix addressed connection recovery and infinite loop issues, but the underlying async/sync mixing issue in the main loop may still be present.

Key code locations to investigate:
- `mcp-bridge/src/main.rs` - main stdin reading loop
- `mcp-bridge/src/connection.rs` - ConnectionManager with channels/locks
- `mcp-bridge/src/handlers.rs` - request handling that might block
