# FEAT-063: Add file-based logging to MCP bridge mode

**Priority**: P1
**Component**: ccmux-server (mcp module)
**Type**: enhancement
**Estimated Effort**: small
**Business Value**: high

## Overview

The MCP bridge (`ccmux-server mcp-bridge`) currently skips logging initialization entirely because it uses stdio for JSON-RPC communication. This means all tracing calls (`debug!`, `info!`, `error!`) in `bridge.rs` produce no output, making it impossible to debug issues like BUG-039 (MCP tools hanging through Claude Code).

File-based logging does NOT interfere with the stdio JSON-RPC protocol because logs go to a separate file descriptor, not stdout/stderr.

## Problem Statement

In `ccmux-server/src/main.rs` lines 1094-1106:

```rust
// Check for subcommands (don't init logging for MCP modes - they use stdio)
let args: Vec<String> = std::env::args().collect();

if args.len() > 1 {
    match args[1].as_str() {
        "mcp-server" => {
            // Legacy standalone MCP server (has its own session state)
            return run_mcp_server();
        }
        "mcp-bridge" => {
            // MCP bridge mode - connects to daemon (recommended)
            return run_mcp_bridge().await;
        }
        // ...
    }
}

// For daemon mode, initialize logging
ccmux_utils::init_logging()?;
```

The comment "don't init logging for MCP modes - they use stdio" is correct for stderr-based logging, but file-based logging is safe. The existing `LogConfig::server()` already uses `LogOutput::File`.

## Evidence

- `bridge.rs` contains extensive tracing calls that currently produce no output
- BUG-039 investigation is blocked by lack of visibility into mcp-bridge behavior
- Direct bash invocation works but Claude Code integration hangs intermittently
- No way to diagnose connection issues, hangs, or protocol errors

## Benefits

1. **Enables BUG-039 debugging**: Can trace request/response flow through mcp-bridge
2. **Future diagnostics**: Any MCP bridge issues become debuggable
3. **Minimal code change**: Just add `init_logging_with_config()` call
4. **No protocol interference**: File logging is independent of stdio

## Implementation Tasks

### Section 1: Enable File Logging

- [ ] Modify `run_mcp_bridge()` in `main.rs` to call `init_logging_with_config()`
- [ ] Use `LogConfig` with `LogOutput::File` (or create `LogConfig::mcp_bridge()` preset)
- [ ] Ensure logs go to `~/.local/share/ccmux/logs/` directory

### Section 2: Optional - MCP-specific Log Config

- [ ] Consider adding `LogConfig::mcp_bridge()` preset in `ccmux-utils/src/logging.rs`
- [ ] Consider separate log file `mcp-bridge.log` to avoid mixing with daemon logs
- [ ] Consider `CCMUX_MCP_LOG` env var for MCP-specific log level control

### Section 3: Log Rotation Considerations

- [ ] Ensure log rotation or size limits for long-running bridge processes
- [ ] Document log file location in help output

### Section 4: Testing

- [ ] Verify mcp-bridge produces log output to file
- [ ] Verify JSON-RPC protocol still works correctly (no stdout pollution)
- [ ] Test with `RUST_LOG=debug` to confirm debug-level messages appear
- [ ] Run integration tests to confirm no regression

### Section 5: Verification

- [ ] Confirm log messages appear when calling mcp-bridge tools
- [ ] Use logs to investigate BUG-039
- [ ] Document logging behavior in mcp-bridge help text

## Acceptance Criteria

- [ ] MCP bridge mode initializes file-based logging
- [ ] Log output goes to file (not stdout/stderr) to avoid protocol interference
- [ ] All existing tracing calls in bridge.rs produce visible output
- [ ] JSON-RPC protocol continues to work correctly
- [ ] Log level controllable via `RUST_LOG` environment variable
- [ ] No regression in existing MCP functionality
- [ ] BUG-039 can be investigated using generated logs

## Files to Modify

| File | Change |
|------|--------|
| `ccmux-server/src/main.rs` | Add `init_logging_with_config()` call in `run_mcp_bridge()` path |
| `ccmux-utils/src/logging.rs` | (Optional) Add `LogConfig::mcp_bridge()` preset |

## Implementation Approach

### Minimal Change (Recommended)

```rust
"mcp-bridge" => {
    // Initialize file-based logging for mcp-bridge (safe - doesn't use stdout)
    ccmux_utils::init_logging_with_config(ccmux_utils::LogConfig::server())?;
    return run_mcp_bridge().await;
}
```

### With Custom Preset (Optional Enhancement)

```rust
// In ccmux-utils/src/logging.rs
impl LogConfig {
    /// Configuration for MCP bridge mode
    /// Uses file logging to avoid stdio interference with JSON-RPC
    pub fn mcp_bridge() -> Self {
        Self {
            output: LogOutput::File,
            filter: std::env::var("CCMUX_MCP_LOG")
                .unwrap_or_else(|_| "ccmux=info".to_string()),
        }
    }
}

// In main.rs
"mcp-bridge" => {
    ccmux_utils::init_logging_with_config(ccmux_utils::LogConfig::mcp_bridge())?;
    return run_mcp_bridge().await;
}
```

## Related Work Items

- **BUG-039**: MCP tools hang intermittently through Claude Code (blocked by this)
- **FEAT-042**: Debug Logging for MCP Pane Broadcast Path (completed - similar pattern)

## Notes

The mcp-bridge process is typically short-lived (spawned per Claude Code session), so log rotation is less critical than for the daemon. However, if Claude Code maintains persistent connections, logs could grow over time.

Consider whether to:
1. Use the same log file as the daemon (simpler, single location)
2. Use a separate `mcp-bridge.log` file (cleaner separation, easier to tail)

The daemon log file is already at `~/.local/share/ccmux/logs/ccmux.log`, so option 1 (reuse) is simpler and keeps all ccmux logs in one place.
