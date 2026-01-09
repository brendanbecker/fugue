# Implementation Plan: FEAT-033

**Work Item**: [FEAT-033: tmux-like Auto-Start Behavior](PROMPT.md)
**Component**: ccmux-client
**Priority**: P1
**Created**: 2026-01-09

## Overview

When user runs `ccmux`, the client should automatically start the server daemon if it's not already running, then connect. This provides the same UX as tmux where a single command handles everything.

## Architecture Decisions

### Decision 1: Server Binary Location Strategy

**Choice**: Check multiple locations in order: same directory as client, then PATH.

**Rationale**:
- Development: server and client binaries are in same target directory
- Installation: both binaries typically installed to same location (e.g., /usr/local/bin)
- PATH fallback covers custom installations
- No need for hardcoded paths

**Alternatives Considered**:
- Config file only - Too much setup for first-time users
- Compile-time path - Inflexible, breaks development workflow
- Search XDG directories - Overcomplicated

### Decision 2: Daemon Spawning Method

**Choice**: Use `std::process::Command` with null stdio handles.

**Rationale**:
- Simple and portable (works on all Unix-like systems)
- No additional dependencies needed
- Spawned process is automatically orphaned when client exits
- Server handles its own daemonization (PID file, etc.)

**Alternatives Considered**:
- Double-fork Unix daemon pattern - Server should handle this, not client
- Platform-specific APIs (systemd, launchd) - Overcomplicated for this use case
- nohup wrapper - Extra binary dependency

### Decision 3: Retry Strategy

**Choice**: Fixed delay retries with timeout, not exponential backoff.

**Rationale**:
- Server startup time is relatively consistent
- Total timeout matters more than individual retry timing
- Simpler implementation
- User shouldn't wait longer than ~2 seconds

**Parameters**:
- Initial delay: 100ms
- Retry interval: 200ms
- Max timeout: 2000ms (configurable)
- Results in ~10 retry attempts

**Alternatives Considered**:
- Exponential backoff - Delays too long for startup scenario
- Poll without delay - Wastes CPU
- Single attempt with long timeout - Poor UX

### Decision 4: Race Condition Handling

**Choice**: Let the OS handle races via socket binding.

**Rationale**:
- If multiple clients try to start server, first one wins (binds socket)
- Others get connection-refused briefly, then succeed on retry
- Server startup is idempotent from client's perspective
- No need for file locks or coordination

**Alternatives Considered**:
- File lock before spawning - Overcomplicated
- Check for running process first - Race window still exists
- Coordinated startup protocol - Server complexity

## Affected Components

| Component | Type of Change | Risk Level |
|-----------|----------------|------------|
| ccmux-client/src/main.rs | Add auto-start logic | Low |
| ccmux-client/src/connection.rs (or similar) | Add retry wrapper | Low |
| ccmux-client CLI | Add --no-auto-start flag | Low |

## Implementation Order

1. **Phase 1: Core Logic**
   - Implement server binary finder
   - Implement daemon spawner
   - Basic integration with connection

2. **Phase 2: Retry Logic**
   - Implement connection retry loop
   - Add timeout handling
   - Error message improvements

3. **Phase 3: Configuration**
   - Add CLI flag
   - Add config file options (optional)

4. **Phase 4: Polish**
   - Edge case handling
   - Testing
   - Documentation

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Server binary not found | Medium | High | Clear error message with suggestions |
| Permission denied on spawn | Low | Medium | Check executable bit, suggest fix |
| Infinite retry on broken server | Low | High | Hard timeout, error message |
| Platform-specific issues | Low | Medium | Test on target platforms |

## Rollback Strategy

If implementation causes issues:
1. Revert commits associated with this work item
2. Client returns to requiring manual server start
3. Document in release notes

## Testing Strategy

### Unit Tests
- `find_server_binary()` with various PATH configurations
- Retry loop timing and exit conditions
- Error classification (is_server_not_running)

### Integration Tests
- Full auto-start flow with real server
- --no-auto-start flag behavior
- Already-running server detection
- Timeout behavior

### Manual Tests
- Fresh start (no server running)
- Server already running
- Server crashes during startup
- Multiple concurrent clients

## Implementation Notes

### Server Binary Finding

```rust
fn find_server_binary() -> Result<PathBuf> {
    // 1. Same directory as current executable
    if let Ok(current_exe) = std::env::current_exe() {
        let server_path = current_exe.parent()
            .map(|p| p.join("ccmux-server"));
        if let Some(path) = server_path {
            if path.is_file() {
                return Ok(path);
            }
        }
    }

    // 2. Search PATH
    if let Ok(path) = which::which("ccmux-server") {
        return Ok(path);
    }

    Err(anyhow!("ccmux-server binary not found. Ensure it's in the same directory as ccmux or in your PATH."))
}
```

### Connection Wrapper

```rust
pub async fn connect_with_auto_start(
    socket_path: &Path,
    auto_start: bool,
) -> Result<Connection> {
    match connect(socket_path).await {
        Ok(conn) => Ok(conn),
        Err(e) if auto_start && is_server_not_running(&e) => {
            start_server_daemon()?;
            connect_with_retry(socket_path, Duration::from_secs(2)).await
        }
        Err(e) => Err(e),
    }
}

async fn connect_with_retry(socket_path: &Path, timeout: Duration) -> Result<Connection> {
    let start = Instant::now();
    let retry_delay = Duration::from_millis(200);

    // Initial delay for server startup
    tokio::time::sleep(Duration::from_millis(100)).await;

    loop {
        match connect(socket_path).await {
            Ok(conn) => return Ok(conn),
            Err(e) if start.elapsed() < timeout && is_server_not_running(&e) => {
                tokio::time::sleep(retry_delay).await;
            }
            Err(e) => {
                return Err(anyhow!(
                    "Failed to connect to server after auto-start: {}. \
                     Check server logs for details.",
                    e
                ));
            }
        }
    }
}
```

---
*This plan should be updated as implementation progresses.*
