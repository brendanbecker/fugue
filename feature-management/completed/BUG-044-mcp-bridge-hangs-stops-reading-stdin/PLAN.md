# Implementation Plan: BUG-044

**Work Item**: [BUG-044: MCP bridge process hangs indefinitely, stops reading stdin](PROMPT.md)
**Component**: mcp-bridge
**Priority**: P1
**Created**: 2026-01-16

## Overview

The mcp-bridge process can enter a state where it stops reading from stdin entirely, causing all MCP tool calls to hang indefinitely. The 25-second daemon response timeout does not trigger. This is a high-severity issue that blocks the primary use case of AI-assisted terminal management.

## Architecture Decisions

### Approach: Async Stdin with Watchdog

Based on the diagnostic evidence (all threads on futex, stdin buffer filling up), the primary approach should be:

1. **Convert synchronous stdin reading to async** - Replace `stdin.lock().lines()` with `tokio::io::BufReader::new(tokio::io::stdin()).lines()`
2. **Add watchdog monitoring** - Implement a background task that detects processing stalls
3. **Improve timeout handling** - Ensure timeouts work correctly at all levels

### Trade-offs

| Decision | Pros | Cons |
|----------|------|------|
| Async stdin | Proper async/await integration, no blocking | Slightly more complex error handling |
| Watchdog thread | Can detect and recover from hangs | Additional complexity, resource usage |
| Aggressive timeout | Prevents indefinite hangs | May abort legitimate slow operations |

## Affected Components

| Component | Type of Change | Risk Level |
|-----------|----------------|------------|
| mcp-bridge/src/main.rs | Primary - stdin loop rewrite | High |
| mcp-bridge/src/connection.rs | Secondary - lock review | Medium |
| mcp-bridge/src/handlers.rs | Secondary - timeout review | Low |

## Investigation Plan

### Phase 1: Reproduce and Instrument
1. Add comprehensive logging at async/sync boundaries
2. Add lock acquisition timing logs
3. Add stdin read timing logs
4. Run under load until hang occurs

### Phase 2: Analyze
1. Review logs from hung instance
2. Confirm which code path is blocking
3. Identify exact sequence of events

### Phase 3: Fix
1. Implement async stdin reading
2. Add watchdog if needed
3. Fix any timeout issues discovered

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Async conversion breaks existing functionality | Medium | High | Comprehensive testing, incremental changes |
| Watchdog adds performance overhead | Low | Low | Make watchdog check interval configurable |
| Root cause is elsewhere | Medium | High | Thorough investigation before code changes |
| Fix doesn't fully resolve issue | Medium | High | Extensive soak testing |

## Rollback Strategy

If implementation causes issues:
1. Revert commits associated with this work item
2. Verify system returns to previous state
3. Document what went wrong in comments.md
4. Consider alternative approaches

## Implementation Notes

### Key Files to Modify

```
ccmux-server/src/bin/mcp-bridge/main.rs  # Main stdin loop
ccmux-server/src/bin/mcp-bridge/connection.rs  # ConnectionManager
ccmux-server/src/bin/mcp-bridge/handlers.rs  # Request handlers
```

### Async Stdin Pattern

```rust
// Before (synchronous)
for line in std::io::stdin().lock().lines() {
    // handle_request().await inside sync iterator = problem
}

// After (asynchronous)
let stdin = tokio::io::stdin();
let reader = tokio::io::BufReader::new(stdin);
let mut lines = reader.lines();

while let Some(line) = lines.next_line().await? {
    // Fully async, no blocking
}
```

### Watchdog Pattern

```rust
// Background task that checks for stalls
tokio::spawn(async move {
    let mut last_activity = Instant::now();
    loop {
        tokio::time::sleep(Duration::from_secs(5)).await;
        if last_activity.elapsed() > Duration::from_secs(30) {
            error!("Processing stall detected!");
            // Log diagnostics, possibly trigger recovery
        }
    }
});
```

---
*This plan should be updated as implementation progresses.*
