# Task Breakdown: FEAT-070

**Work Item**: [FEAT-070: gastown remote pane support via FUGUE_ADDR](PROMPT.md)
**Status**: Not Started
**Last Updated**: 2026-01-13

## Prerequisites

- [ ] Read and understand PROMPT.md
- [ ] Review PLAN.md and update if needed
- [ ] Verify FEAT-066, FEAT-067, FEAT-068 are complete
- [ ] Set up test environment with fugue TCP listener
- [ ] Clone/fork gastown repository

## Section 1: Environment Variable Support

- [ ] Add GASTOWN_FUGUE_ADDR environment variable reading
- [ ] Implement URL parsing (tcp:// and unix:// schemes)
- [ ] Add URL validation logic
- [ ] Handle missing/invalid URL gracefully
- [ ] Default to local Unix socket if unset
- [ ] Add unit tests for URL parsing
- [ ] Document environment variable in code comments

## Section 2: Spawn Logic Updates

- [ ] Locate agent spawn code in gastown
- [ ] Update spawn commands to accept --addr parameter
- [ ] Pass GASTOWN_FUGUE_ADDR to fugue-client calls
- [ ] Update Mayor spawn logic
- [ ] Update polecat spawn logic
- [ ] Update convoy spawn logic (if applicable)
- [ ] Maintain backward compatibility
- [ ] Add unit tests for spawn command construction
- [ ] Test local execution (verify unchanged behavior)

## Section 3: Remote-Aware Presets

- [ ] Create remote-polecat preset example
- [ ] Document heavy vs light task classification
- [ ] Provide example configurations for:
  - [ ] Mayor local + polecats remote
  - [ ] All agents remote
  - [ ] Mixed local/remote based on task type
- [ ] Update existing preset documentation
- [ ] Add comments explaining remote options

## Section 4: State Synchronization

- [ ] Document git worktree sync strategy
  - [ ] Push/pull workflow
  - [ ] Branch strategy
  - [ ] Conflict resolution
- [ ] Document beads ledger sync approach
  - [ ] Git commit hooks option
  - [ ] Rsync option
  - [ ] Manual sync procedures
- [ ] Document WAL persistence on remote host
- [ ] Document disconnect/reconnect handling
- [ ] Document rig coordination strategies
- [ ] Provide example sync scripts/commands

## Section 5: Testing and Validation

- [ ] Test local execution (GASTOWN_FUGUE_ADDR unset)
  - [ ] Mayor spawns locally
  - [ ] Polecats spawn locally
  - [ ] All functionality unchanged
- [ ] Test remote execution (GASTOWN_FUGUE_ADDR set)
  - [ ] Set up SSH tunnel to remote fugue
  - [ ] Export GASTOWN_FUGUE_ADDR=tcp://localhost:9999
  - [ ] Verify agents spawn on remote machine
- [ ] Test hybrid workflow
  - [ ] Run Mayor locally
  - [ ] Spawn polecats remotely
  - [ ] Verify communication works
  - [ ] Test state sync across hosts
- [ ] Test error scenarios
  - [ ] Invalid URL format
  - [ ] Connection refused
  - [ ] Network disconnect during operation
  - [ ] Reconnection after disconnect
- [ ] Validate state sync
  - [ ] Git push/pull workflow
  - [ ] Beads ledger consistency
  - [ ] WAL persistence on remote

## Section 6: Documentation

- [ ] Write GASTOWN_FUGUE_ADDR usage guide
  - [ ] Environment variable syntax
  - [ ] URL scheme documentation
  - [ ] Examples for common scenarios
- [ ] Write remote workflow setup guide
  - [ ] Prerequisites (fugue remote peering)
  - [ ] SSH tunnel setup (reference FEAT-068)
  - [ ] Configuration steps
  - [ ] Verification steps
- [ ] Write troubleshooting guide
  - [ ] Common errors and solutions
  - [ ] Network connectivity issues
  - [ ] State sync problems
  - [ ] Performance considerations
- [ ] Document state sync requirements
  - [ ] Git configuration
  - [ ] SSH key setup
  - [ ] Rsync configuration (if applicable)
- [ ] Provide example commands
  - [ ] SSH tunnel creation
  - [ ] Environment variable export
  - [ ] Agent spawn commands
  - [ ] State sync commands
- [ ] Update main gastown README
  - [ ] Mention remote execution capability
  - [ ] Link to detailed guides
  - [ ] Add to feature list

## Verification Tasks

- [ ] All acceptance criteria from PROMPT.md met
  - [ ] GASTOWN_FUGUE_ADDR controls remote pane spawning
  - [ ] Mayor can run locally while polecats run remotely
  - [ ] Remote panes spawn correctly via TCP
  - [ ] State sync works across hosts
  - [ ] Documentation covers complete setup
  - [ ] Backward compatibility maintained
  - [ ] Clear error messages for failures
- [ ] All tests passing
  - [ ] Unit tests
  - [ ] Integration tests
  - [ ] Manual test scenarios
- [ ] Documentation complete and accurate
- [ ] Code reviewed and polished
- [ ] Examples tested and verified

## Completion Checklist

- [ ] All implementation tasks complete
- [ ] All tests passing
- [ ] Documentation complete
- [ ] PLAN.md reflects final implementation
- [ ] Example workflows tested and documented
- [ ] Backward compatibility verified
- [ ] Ready for merge/release

## Notes

- **Dependencies**: Requires FEAT-066, FEAT-067, FEAT-068 to be complete
- **Gastown location**: External project (fork or separate repo)
- **Testing setup**: Need two machines or VMs for full integration testing
- **State sync**: Document clearly that users are responsible for sync strategy
- **Error handling**: Prioritize clear, actionable error messages

---
*Check off tasks as you complete them. Update status field above.*
