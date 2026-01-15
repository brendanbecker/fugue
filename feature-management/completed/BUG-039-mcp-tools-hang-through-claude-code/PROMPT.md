# BUG-039: MCP tools hang intermittently through Claude Code

**Priority**: P1
**Component**: mcp-bridge
**Severity**: high
**Status**: new

## Problem Statement

MCP tool calls (list_sessions, create_window, list_windows, etc.) frequently hang when called through Claude Code's MCP integration, requiring manual abort. However, the exact same operations work reliably when calling the mcp-bridge directly via bash:

```bash
echo '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"ccmux_list_sessions","arguments":{}}}' | timeout 3 ccmux-server mcp-bridge
```

## Evidence

### Observations
- Multiple stale mcp-bridge processes accumulate (seen 10+ orphaned processes)
- The daemon itself is running and responsive
- Socket exists at correct location (`/run/user/1000/ccmux/ccmux.sock`)
- Direct bash invocation works every time
- Claude Code MCP calls hang unpredictably

### Key Difference
- **Direct bash**: Spawns mcp-bridge, sends single request, receives response, process exits
- **Claude Code**: Maintains persistent connection to mcp-bridge, may reuse connections

## Steps to Reproduce

1. Configure ccmux MCP server in Claude Code settings
2. Start ccmux daemon (`ccmux-server start`)
3. Use Claude Code to invoke MCP tools like `ccmux_list_sessions`
4. Observe intermittent hangs requiring manual abort
5. Check for orphaned mcp-bridge processes: `ps aux | grep mcp-bridge`

## Expected Behavior

MCP tool calls through Claude Code should complete reliably within a reasonable timeout, matching the behavior of direct bash invocation.

## Actual Behavior

MCP tool calls frequently hang indefinitely. Multiple stale mcp-bridge processes (10+) accumulate. Direct bash invocation works every time.

## Root Cause Hypotheses

### Hypothesis 1: Connection Lifecycle Mismatch
Claude Code's MCP client may keep the connection open expecting persistent operation, while mcp-bridge may not handle long-lived connections properly.

**Investigation**: Check if mcp-bridge is designed for single-request or persistent operation.

### Hypothesis 2: Stdin/Stdout Deadlock
The mcp-bridge communicates via stdin/stdout. If Claude Code's buffering differs from direct invocation, there could be a deadlock where:
- mcp-bridge is waiting for more input
- Claude Code is waiting for output

**Investigation**: Add timeout handling to mcp-bridge stdin reads.

### Hypothesis 3: Process Cleanup Failure
When Claude Code closes a connection or times out, the mcp-bridge process may not receive the signal to exit, leading to zombie processes.

**Investigation**: Check signal handling and stdin EOF detection in mcp-bridge.

### Hypothesis 4: Response Not Flushed
The response may be buffered and not flushed to stdout, causing Claude Code to wait indefinitely.

**Investigation**: Ensure explicit flush after writing responses.

## Investigation Tasks

### Section 1: Process Behavior Analysis
- [ ] Run `strace` on mcp-bridge during a hang to see what syscall it's blocked on
- [ ] Check if mcp-bridge has proper stdin EOF handling
- [ ] Verify stdout is line-buffered or explicitly flushed
- [ ] Check for any blocking reads without timeout

### Section 2: Code Review
- [ ] Review mcp-bridge main loop for connection handling
- [ ] Check if there's a read timeout on stdin
- [ ] Verify response serialization includes proper line termination
- [ ] Check for any mutex/lock contention with daemon communication

### Section 3: Comparison Testing
- [ ] Compare behavior with `cat | ccmux-server mcp-bridge` (interactive)
- [ ] Test with `timeout 5 ccmux-server mcp-bridge` and slow input
- [ ] Test multiple rapid requests to check for state accumulation

## Implementation Tasks

### Section 1: Diagnosis
- [ ] Add debug logging to mcp-bridge startup/shutdown
- [ ] Log each request received and response sent
- [ ] Add process ID to logs for correlation
- [ ] Instrument stdin/stdout operations

### Section 2: Potential Fixes
- [ ] Add read timeout to stdin (don't block indefinitely)
- [ ] Ensure stdout flush after each response
- [ ] Add proper signal handling (SIGPIPE, SIGHUP)
- [ ] Implement graceful shutdown on stdin EOF

### Section 3: Testing
- [ ] Create integration test simulating Claude Code behavior
- [ ] Test with varying request timing
- [ ] Verify no zombie processes after connection close
- [ ] Stress test with many rapid connections

### Section 4: Verification
- [ ] Test fix with actual Claude Code MCP integration
- [ ] Confirm no orphaned processes after extended use
- [ ] Verify direct bash invocation still works
- [ ] Document any behavioral changes

## Acceptance Criteria

- [ ] MCP tool calls through Claude Code complete reliably
- [ ] No orphaned mcp-bridge processes accumulate
- [ ] Direct bash invocation continues to work
- [ ] Reasonable timeout behavior (fail fast, don't hang)
- [ ] Debug logging available for future diagnosis

## Files to Investigate

| File | Reason |
|------|--------|
| `ccmux-server/src/mcp/bridge.rs` | Main mcp-bridge implementation |
| `ccmux-server/src/mcp/mod.rs` | MCP module organization |
| `ccmux-server/src/bin/ccmux-server.rs` | Entry point for mcp-bridge subcommand |

## Related Bugs

- BUG-029: MCP response synchronization bug (may share root cause with response routing)
- BUG-030: Daemon unresponsive after create_window (similar hang symptoms)

## Notes

This bug significantly impacts the primary use case of ccmux - AI-assisted terminal management through Claude Code. Priority should be high as it breaks the core value proposition.

The fact that direct bash invocation works perfectly suggests the issue is specific to how Claude Code maintains MCP connections, not a fundamental problem with the daemon or protocol implementation.
