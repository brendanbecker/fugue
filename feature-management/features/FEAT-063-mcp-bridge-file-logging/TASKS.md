# Task Breakdown: FEAT-063

**Work Item**: [FEAT-063: Add file-based logging to MCP bridge mode](PROMPT.md)
**Status**: Not Started
**Last Updated**: 2026-01-13

## Prerequisites

- [ ] Read and understand PROMPT.md
- [ ] Review PLAN.md and update if needed
- [ ] Verify existing logging infrastructure in ccmux-utils

## Implementation Tasks

### Phase 1: Enable File Logging (Core Change)

- [ ] Open `ccmux-server/src/main.rs`
- [ ] Locate the `"mcp-bridge"` match arm (line ~1103)
- [ ] Add logging initialization before `run_mcp_bridge().await`
- [ ] Use `ccmux_utils::init_logging_with_config(ccmux_utils::LogConfig::server())?;`
- [ ] Self-review the change

### Phase 2: Verify No Protocol Interference

- [ ] Build the project: `cargo build -p ccmux-server`
- [ ] Start mcp-bridge manually: `ccmux-server mcp-bridge`
- [ ] Send test JSON-RPC request via stdin
- [ ] Verify response appears on stdout (not corrupted)
- [ ] Verify log entries appear in `~/.local/share/ccmux/logs/ccmux.log`

### Phase 3: Test with Claude Code Integration

- [ ] Ensure ccmux daemon is running
- [ ] Invoke MCP tool through Claude Code
- [ ] Check log file for request/response tracing
- [ ] Verify tool calls complete successfully

## Testing Tasks

- [ ] Run existing MCP integration tests
- [ ] Manual test: direct bash invocation still works
- [ ] Manual test: JSON-RPC protocol not corrupted
- [ ] Manual test: Log file receives entries
- [ ] Verify RUST_LOG environment variable controls verbosity

## Documentation Tasks

- [ ] Update help text if needed (optional)
- [ ] Add note to DEPLOYMENT_PLAN.md if relevant (optional)

## Verification Tasks

- [ ] All acceptance criteria from PROMPT.md met
- [ ] BUG-039 investigation can proceed with logs
- [ ] Update feature_request.json status when complete
- [ ] Document completion in comments.md (if needed)

## Completion Checklist

- [ ] Core implementation complete (1 line change)
- [ ] All tests passing
- [ ] No protocol interference verified
- [ ] PLAN.md reflects final implementation
- [ ] Ready for review/merge

---
*Check off tasks as you complete them. Update status field above.*
