# Implementation Plan: FEAT-063

**Work Item**: [FEAT-063: Add file-based logging to MCP bridge mode](PROMPT.md)
**Component**: ccmux-server
**Priority**: P1
**Created**: 2026-01-13

## Overview

Enable file-based logging for the MCP bridge mode so that tracing calls in `bridge.rs` produce visible output. This is critical for debugging BUG-039 (MCP tools hanging through Claude Code).

## Architecture Decisions

### Decision 1: Reuse Existing LogConfig::server()

**Rationale**: The `LogConfig::server()` preset already configures file-based logging with appropriate defaults. Creating a separate `LogConfig::mcp_bridge()` adds complexity without significant benefit.

**Trade-off**: Logs from mcp-bridge will be interleaved with daemon logs in the same file. This is acceptable because:
- Logs include process/thread IDs for differentiation
- Single log file is easier to manage
- Timestamps allow correlation

### Decision 2: Initialize Logging Before run_mcp_bridge()

**Rationale**: Logging must be initialized before any async runtime operations to capture all relevant tracing events.

**Implementation**:
```rust
"mcp-bridge" => {
    ccmux_utils::init_logging_with_config(ccmux_utils::LogConfig::server())?;
    return run_mcp_bridge().await;
}
```

### Decision 3: No Separate Environment Variable

**Rationale**: The standard `RUST_LOG` environment variable is sufficient for controlling log levels. Adding `CCMUX_MCP_LOG` creates unnecessary complexity. Users can use `RUST_LOG=ccmux_server::mcp=debug` for MCP-specific verbosity.

## Affected Components

| Component | Type of Change | Risk Level |
|-----------|----------------|------------|
| `ccmux-server/src/main.rs` | Add logging init | Low |
| `ccmux-utils/src/logging.rs` | No change needed | None |

## Dependencies

None - uses existing logging infrastructure.

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Log file permissions | Low | Low | Existing daemon already creates logs |
| Log pollution to stdout | Low | High | Using LogOutput::File explicitly |
| Performance overhead | Very Low | Low | File logging is async/buffered |

## Rollback Strategy

If implementation causes issues:
1. Revert the single-line change to main.rs
2. MCP bridge returns to no-logging behavior
3. No other components affected

## Implementation Notes

### Key Insight

The original comment "don't init logging for MCP modes - they use stdio" was overly broad. The concern is valid for stderr-based logging (which would corrupt JSON-RPC output), but file-based logging writes to a completely separate file descriptor and cannot interfere with the stdio protocol.

### Verification Steps

1. Start mcp-bridge: `ccmux-server mcp-bridge`
2. Check log file: `tail -f ~/.local/share/ccmux/logs/ccmux.log`
3. Send JSON-RPC request via stdin
4. Verify:
   - Log entries appear in file
   - JSON-RPC response appears on stdout (not corrupted)

---
*This plan should be updated as implementation progresses.*
