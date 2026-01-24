# FEAT-033: tmux-like Auto-Start Behavior

**Priority**: P1
**Component**: fugue-client
**Type**: enhancement
**Estimated Effort**: small
**Business Value**: high
**Status**: new

## Overview

When user runs `fugue`, the client should automatically start the server daemon if it's not already running, then connect. This provides the same UX as tmux where a single command handles everything.

## Problem Statement

Currently, users must manually start `fugue-server` before running the `fugue` client. This creates friction in the user experience:

1. User runs `fugue`
2. Connection fails because server isn't running
3. User must open another terminal, run `fugue-server`
4. User goes back to original terminal and runs `fugue` again

This is a poor UX compared to tmux, which "just works" with a single command.

## Solution

Implement auto-start behavior in the client:

1. Client attempts to connect to the Unix socket
2. If connection fails (ECONNREFUSED, ENOENT), assume server not running
3. Fork/spawn `fugue-server` as a background daemon
4. Wait briefly for server to initialize (e.g., 100-500ms with retries)
5. Retry connection to Unix socket
6. Proceed with normal client behavior

## Implementation Details

### Socket Connection Attempt

```rust
// Pseudo-code for connection logic
async fn connect_with_auto_start(socket_path: &Path) -> Result<Connection> {
    // First attempt
    match try_connect(socket_path).await {
        Ok(conn) => return Ok(conn),
        Err(e) if is_server_not_running(&e) => {
            // Server not running, start it
            start_server_daemon()?;
        }
        Err(e) => return Err(e),
    }

    // Retry with backoff
    for attempt in 0..MAX_RETRIES {
        tokio::time::sleep(RETRY_DELAY).await;
        match try_connect(socket_path).await {
            Ok(conn) => return Ok(conn),
            Err(e) if is_server_not_running(&e) && attempt < MAX_RETRIES - 1 => {
                continue;
            }
            Err(e) => return Err(e),
        }
    }

    Err(anyhow!("Failed to connect after starting server"))
}
```

### Server Daemon Spawning

```rust
fn start_server_daemon() -> Result<()> {
    use std::process::Command;

    // Find fugue-server binary (same directory as client, or in PATH)
    let server_path = find_server_binary()?;

    // Spawn as daemon (detached, no stdin/stdout/stderr)
    Command::new(&server_path)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;

    Ok(())
}
```

### Error Detection

Detect "server not running" from these error conditions:
- `std::io::ErrorKind::ConnectionRefused` - Socket exists but nobody listening
- `std::io::ErrorKind::NotFound` - Socket file doesn't exist
- Other platform-specific variants

### Configuration Options

Add optional config in `~/.config/fugue/config.toml`:

```toml
[client]
# Disable auto-start if user prefers manual control
auto_start_server = true

# How long to wait for server to start (milliseconds)
server_start_timeout = 2000

# Number of connection retry attempts
connection_retries = 5
```

### CLI Flag

Add `--no-auto-start` flag to disable this behavior:

```bash
# Normal usage - auto-starts if needed
fugue

# Disable auto-start (fail if server not running)
fugue --no-auto-start
```

## Files to Modify

### fugue-client/src/main.rs or connection module

- Add `start_server_daemon()` function
- Modify connection logic to retry with auto-start
- Add CLI flag `--no-auto-start`

### fugue-client/src/config.rs (if exists)

- Add `auto_start_server`, `server_start_timeout`, `connection_retries` options

## Implementation Tasks

### Section 1: Server Detection
- [ ] Add function to detect if server is running (socket exists and accepts connections)
- [ ] Distinguish between "server not running" vs other connection errors
- [ ] Handle both socket-not-found and connection-refused cases

### Section 2: Server Spawning
- [ ] Add function to find fugue-server binary (same dir, PATH, or config)
- [ ] Implement daemon spawning with detached process
- [ ] Ensure spawned server doesn't inherit client's stdin/stdout/stderr
- [ ] Handle platform differences (Unix daemonization)

### Section 3: Connection Retry Logic
- [ ] Implement retry loop with configurable timeout
- [ ] Add exponential backoff between retries
- [ ] Provide clear error message if server fails to start

### Section 4: Configuration
- [ ] Add config options for auto-start behavior
- [ ] Add `--no-auto-start` CLI flag
- [ ] Document configuration options

### Section 5: Testing
- [ ] Test auto-start when server not running
- [ ] Test normal connect when server already running
- [ ] Test --no-auto-start flag behavior
- [ ] Test timeout/retry behavior
- [ ] Test error handling when server binary not found

## Acceptance Criteria

- [ ] Running `fugue` when server is not running automatically starts the server
- [ ] Running `fugue` when server is already running connects normally (no duplicate servers)
- [ ] Server starts as a proper daemon (doesn't tie up terminal)
- [ ] Clear error message if server binary cannot be found
- [ ] Clear error message if server fails to start within timeout
- [ ] `--no-auto-start` flag disables auto-start behavior
- [ ] Config file can disable auto-start globally
- [ ] Works correctly on Linux (primary platform)

## Dependencies

- FEAT-011 (Client Connection - Unix Socket Client) - Must be complete for socket connection logic

## Edge Cases

1. **Multiple concurrent clients**: If multiple clients try to connect simultaneously when server is not running, only one should start the server. Others should detect the starting server and wait.

2. **Server crash during startup**: If server crashes immediately after starting, client should provide a helpful error message rather than retrying indefinitely.

3. **Permission issues**: Socket directory might not be writable, or server binary might not be executable.

4. **Server binary location**: Need strategy for finding `fugue-server`:
   - Same directory as `fugue` binary
   - In `$PATH`
   - Configurable path in config file

5. **Stale socket file**: If server crashed without cleanup, socket file might exist but nobody is listening. This should trigger auto-start (connection refused case).

## Notes

- This is a common pattern - tmux, screen, and most client/server tools do this
- Consider adding a `fugue --server` subcommand as alternative to separate `fugue-server` binary
- May want to add `fugue kill-server` command for stopping the daemon
- Log file location for daemonized server should be documented
