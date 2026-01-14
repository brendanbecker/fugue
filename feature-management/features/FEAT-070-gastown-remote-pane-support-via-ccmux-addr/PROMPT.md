# FEAT-070: gastown remote pane support via CCMUX_ADDR

**Priority**: P2
**Component**: external (gastown fork)
**Type**: enhancement
**Estimated Effort**: medium
**Business Value**: high

## Overview

Extend the gastown multi-agent system to support remote pane execution via ccmux TCP connections. This enables hybrid orchestration where the Mayor runs locally (laptop) while polecats/agents execute remotely (gaming PC), leveraging ccmux remote peering for compute offload and token distribution.

## Problem Statement

Currently, gastown agents (Mayor, polecats, convoys) all run on the same machine as ccmux. With ccmux remote peering support (FEAT-066, FEAT-067, FEAT-068), gastown needs integration to spawn remote panes. This enables:

- **Mayor orchestrator on low-latency local machine (laptop)**: Quick response times for coordination
- **Heavy compute agents on remote machine (gaming PC)**: Leverage more powerful hardware
- **Distributed token burn across multiple Claude instances**: Share API costs and avoid rate limits

## Requested Feature

Add support for the `GASTOWN_CCMUX_ADDR` environment variable to control where gastown agents spawn their panes:

- **GASTOWN_CCMUX_ADDR environment variable** for remote addressing
- Update spawn/attach logic to use remote ccmux when configured
- Support `tcp://host:port` and `unix://path` URL schemes
- Maintain local execution as default (backward compatible)
- Document remote workflow setup

## Benefits

1. **Hybrid orchestration**: Mayor local, polecats remote
2. **Compute offload**: Heavy tasks on powerful remote machine
3. **Token distribution**: Multiple Claude instances across machines
4. **Latency optimization**: Keep orchestrator local, move compute remote
5. **Resource flexibility**: Use different machines for different roles

## Implementation Tasks

### Section 1: Environment Variable Support
- [ ] Add GASTOWN_CCMUX_ADDR configuration
- [ ] Parse and validate connection URLs
- [ ] Support tcp:// and unix:// schemes
- [ ] Default to local Unix socket if unset
- [ ] Add URL validation and error handling

### Section 2: Spawn Logic Updates
- [ ] Update agent spawn commands to use CCMUX_ADDR
- [ ] Pass --addr flag to ccmux-client when remote
- [ ] Update preset/formula configuration
- [ ] Maintain backward compatibility with existing workflows
- [ ] Test local execution (unchanged behavior)

### Section 3: Remote-Aware Presets
- [ ] Create remote-polecat preset example
- [ ] Document remote agent configuration
- [ ] Add heavy/light task classification guidance
- [ ] Configure remote vs local selection logic
- [ ] Provide examples for common workflows

### Section 4: State Synchronization
- [ ] Document git worktree sync strategy
- [ ] Document beads ledger sync approach
- [ ] Ensure WAL persistence on remote host
- [ ] Handle disconnect/reconnect scenarios
- [ ] Document rig coordination via git/network filesystem

### Section 5: Testing and Validation
- [ ] Test local execution (unchanged)
- [ ] Test remote execution via SSH tunnel
- [ ] Test Mayor local + polecat remote workflow
- [ ] Validate state sync across hosts
- [ ] Test disconnect recovery scenarios
- [ ] Verify ccmux-client TCP connection handling

### Section 6: Documentation
- [ ] Document GASTOWN_CCMUX_ADDR usage
- [ ] Provide setup guide for remote workflow
- [ ] Document SSH tunnel setup (reference FEAT-068)
- [ ] Add troubleshooting guide
- [ ] Document state sync requirements
- [ ] Provide example commands for common scenarios

## Acceptance Criteria

- [ ] GASTOWN_CCMUX_ADDR controls remote pane spawning
- [ ] Mayor can run locally while polecats run remotely
- [ ] Remote panes spawn correctly via TCP connection
- [ ] State sync works across local/remote hosts
- [ ] Documentation covers complete setup workflow
- [ ] Backward compatibility maintained (local default)
- [ ] Error messages are clear when remote connection fails
- [ ] All existing gastown functionality works unchanged

## Dependencies

This feature depends on ccmux remote peering support:
- **FEAT-066**: TCP listener support in daemon (Phase 1)
- **FEAT-067**: Client TCP connection support (Phase 2)
- **FEAT-068**: SSH tunnel integration and documentation (Phase 3)

## Example Workflow

```bash
# On remote gaming PC:
ccmux-server --listen-tcp 127.0.0.1:9999

# On local laptop:
# Create SSH tunnel to remote daemon
ssh -L 9999:localhost:9999 gaming-pc &

# Configure gastown to use remote ccmux
export GASTOWN_CCMUX_ADDR=tcp://localhost:9999

# Start Mayor locally (spawns polecats remotely)
gt mayor attach
```

## State Synchronization Considerations

- **Remote WAL**: Survives disconnects on remote host
- **Git worktrees**: Push/pull from laptop to sync code
- **Beads ledger**: Needs rsync or git commit hooks for sync
- **Rig coordination**: Via git or network filesystem

## Related Files

- `gastown/cmd/gt/main.go` (or equivalent entry point)
- `gastown/internal/crew/` (agent spawn logic)
- `gastown/presets/` (remote preset configuration)
- `gastown/docs/` (documentation)
- Reference: `docs/architecture/GASTOWN_REMOTE_SUPPORT.md`

## Notes

- Design is based on gastown-ccmux-remote-support.md specification
- Uses GASTOWN_CCMUX_ADDR env var to transparently route ccmux-client calls to remote daemon
- SSH tunnel provides secure remote access without implementing TLS in ccmux itself
- Remote execution is opt-in via environment variable
- Default behavior (local execution) remains unchanged
