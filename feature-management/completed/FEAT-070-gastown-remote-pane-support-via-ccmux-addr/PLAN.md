# Implementation Plan: FEAT-070

**Work Item**: [FEAT-070: gastown remote pane support via FUGUE_ADDR](PROMPT.md)
**Component**: external (gastown fork)
**Priority**: P2
**Created**: 2026-01-13

## Overview

Extend gastown to support remote pane execution via fugue TCP connections. This enables hybrid orchestration where the Mayor runs locally while polecats execute remotely, leveraging fugue remote peering for compute offload and token distribution.

## Architecture Decisions

### URL Scheme Design

**Decision**: Use standard URL schemes for fugue addressing
- `unix:///path/to/socket` - Unix domain socket (default: `unix:///home/user/.fugue/fugue.sock`)
- `tcp://host:port` - TCP connection (e.g., `tcp://localhost:9999`)

**Rationale**:
- Standard URL parsing available in Go stdlib
- Clear distinction between local and remote connections
- Easy to extend with additional schemes in future (e.g., `tls://`)

### Environment Variable Approach

**Decision**: Use `GASTOWN_FUGUE_ADDR` environment variable
- Inspected at agent spawn time
- Passed to fugue-client via `--addr` flag
- Falls back to default Unix socket if unset

**Rationale**:
- Non-invasive: No gastown CLI changes required
- Flexible: Can be set per-shell, per-agent, or globally
- Transparent: Existing gastown commands work unchanged
- Testable: Easy to test local vs remote scenarios

**Trade-offs**:
- Environment variables less discoverable than CLI flags
- Mitigation: Clear documentation and error messages

### State Synchronization Strategy

**Decision**: Delegate to external tooling (git, rsync)
- **Code sync**: Git worktrees with push/pull
- **Beads ledger**: Git commit hooks or rsync
- **WAL persistence**: Lives on remote host (survives disconnects)
- **Rig coordination**: Git or network filesystem

**Rationale**:
- Gastown doesn't need custom sync logic
- Leverage proven tools (git, rsync)
- Users have full control over sync strategy
- Avoids complex distributed state management

**Trade-offs**:
- Manual setup burden on users
- Potential for sync conflicts
- Mitigation: Document best practices and examples

## Affected Components

| Component | Type of Change | Risk Level |
|-----------|----------------|------------|
| gastown/internal/crew/ | Spawn logic modification | Medium |
| gastown/cmd/gt/ | Environment variable handling | Low |
| gastown/presets/ | New remote preset examples | Low |
| gastown/docs/ | New documentation | Low |

## Implementation Approach

### Phase 1: Environment Variable Infrastructure

1. Add `GASTOWN_FUGUE_ADDR` environment variable reading in gastown startup
2. Parse URL scheme (unix:// vs tcp://)
3. Validate URL format and connectivity
4. Store parsed address for use during agent spawn

**Code Location**: `cmd/gt/main.go` or `internal/config/`

### Phase 2: Spawn Logic Updates

1. Modify agent spawn commands to accept `--addr` parameter
2. Pass `GASTOWN_FUGUE_ADDR` value to fugue-client calls
3. Update all spawn sites (Mayor, polecats, convoys)
4. Maintain backward compatibility (default to Unix socket)

**Code Location**: `internal/crew/spawn.go` (or equivalent)

**Example**:
```go
// Before:
cmd := exec.Command("fugue-client", "new-pane", "--name", agentName)

// After:
args := []string{"new-pane", "--name", agentName}
if fugueAddr := os.Getenv("GASTOWN_FUGUE_ADDR"); fugueAddr != "" {
    args = append([]string{"--addr", fugueAddr}, args...)
}
cmd := exec.Command("fugue-client", args...)
```

### Phase 3: Remote Preset Examples

1. Create `presets/remote-polecat.toml` (or equivalent)
2. Document heavy vs light task classification
3. Provide example workflows for common scenarios
4. Update existing presets to show remote capability

**Example Preset**:
```toml
[polecat]
# Spawn remotely on gaming PC
fugue_addr = "tcp://localhost:9999"  # or use env var
tags = ["heavy-compute", "remote"]
```

### Phase 4: Documentation

1. Create `docs/remote-execution.md` guide
2. Document GASTOWN_FUGUE_ADDR usage
3. Provide SSH tunnel setup instructions
4. Add troubleshooting section
5. Document state sync strategies

## Dependencies

**Required (must be completed first)**:
- FEAT-066: TCP listener in daemon (completed or in-progress)
- FEAT-067: Client TCP connection support (completed or in-progress)
- FEAT-068: SSH tunnel documentation (completed or in-progress)

**Nice-to-have (future enhancements)**:
- FEAT-069: TLS/auth for direct TCP (not required for SSH tunnel approach)

## Testing Strategy

### Unit Tests
- URL parsing and validation
- Environment variable handling
- Spawn command construction

### Integration Tests
1. **Local execution** (baseline): Verify unchanged behavior when GASTOWN_FUGUE_ADDR is unset
2. **Remote execution**: Spawn agents via TCP to remote fugue
3. **Hybrid workflow**: Mayor local + polecats remote
4. **Error scenarios**: Invalid URLs, connection failures, disconnects

### Manual Testing
1. Set up SSH tunnel to remote fugue
2. Export GASTOWN_FUGUE_ADDR
3. Run Mayor and verify polecats spawn remotely
4. Test state sync (git push/pull)
5. Test disconnect/reconnect scenarios

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Network connectivity issues | Medium | High | Document error handling, test disconnect scenarios |
| State sync conflicts | Medium | Medium | Document best practices, provide examples |
| Breaking existing workflows | Low | High | Maintain backward compatibility, extensive testing |
| Performance impact (latency) | Medium | Low | Document trade-offs, provide guidance on task placement |

## Rollback Strategy

If implementation causes issues:
1. Unset `GASTOWN_FUGUE_ADDR` environment variable
2. Gastown reverts to local Unix socket behavior
3. No code changes required to roll back
4. Document rollback procedure in troubleshooting guide

## Implementation Checklist

- [ ] Environment variable reading and parsing
- [ ] URL validation (tcp:// and unix:// schemes)
- [ ] Update spawn logic with --addr parameter
- [ ] Test local execution (unchanged)
- [ ] Test remote execution via SSH tunnel
- [ ] Create remote preset examples
- [ ] Write documentation (setup, troubleshooting)
- [ ] Update existing docs to mention remote capability
- [ ] Test hybrid Mayor + remote polecats workflow
- [ ] Document state sync strategies
- [ ] Add error handling and user-friendly messages

## Future Enhancements

After FEAT-070 is complete, consider:
- Auto-discovery of available fugue daemons
- Load balancing across multiple remote daemons
- Built-in state sync (vs external git/rsync)
- TLS/auth for direct TCP (FEAT-069)
- Automatic SSH tunnel management

---
*This plan should be updated as implementation progresses.*
