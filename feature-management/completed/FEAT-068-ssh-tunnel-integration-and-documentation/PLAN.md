# Implementation Plan: FEAT-068

**Work Item**: [FEAT-068: SSH tunnel integration and documentation](PROMPT.md)
**Component**: documentation
**Priority**: P2
**Created**: 2026-01-13

## Overview

Document and validate the complete SSH tunnel-based remote workflow for fugue, integrating FEAT-066 (daemon TCP) and FEAT-067 (client TCP) into a secure, production-ready remote access solution.

## Architecture Decisions

### SSH Tunnel Approach

**Decision**: Use SSH tunnels as the recommended MVP for remote access instead of implementing custom TLS.

**Rationale**:
- SSH provides battle-tested encryption and authentication
- Leverages existing SSH key infrastructure
- No need to implement certificate management
- Well-understood by users and sysadmins
- Simpler than custom TLS implementation (FEAT-069)

**Trade-offs**:
- Requires SSH access to remote host
- Slightly more setup than native TLS would require
- Dependent on SSH daemon availability
- Benefits: Zero custom security code, proven reliability

### Documentation Structure

**Decision**: Create comprehensive docs in multiple locations for discoverability.

**Structure**:
1. `docs/remote-access.md` - Primary detailed guide
2. `examples/remote-configs/` - Configuration examples
3. `README.md` updates - Quick start reference
4. Optional: `scripts/fugue-remote` wrapper

**Rationale**: Users need different entry points depending on use case.

## Affected Components

| Component | Type of Change | Risk Level |
|-----------|----------------|------------|
| docs/ | New documentation | Low |
| examples/ | New examples | Low |
| README.md | Documentation update | Low |
| scripts/ (optional) | New helper scripts | Low |

## Workflow to Document

### Basic SSH Tunnel Workflow

```bash
# 1. Remote host: Configure daemon with TCP listener
# config.toml:
# [server]
# listen_tcp = "127.0.0.1:9999"

# 2. Remote host: Start daemon
fugue-server

# 3. Local machine: Create SSH tunnel
ssh -L 9999:localhost:9999 user@remote-host

# 4. Local machine: Connect client
fugue-client --addr tcp://localhost:9999
```

### Security Model

- **Encryption**: Provided by SSH protocol
- **Authentication**: SSH key-based authentication
- **Authorization**: System-level (SSH access = fugue access)
- **Firewall**: TCP listener on localhost only (127.0.0.1)

## Validation Plan

### Test Scenarios

1. **Basic Connection**
   - Start daemon with TCP on remote
   - Create SSH tunnel
   - Connect client through tunnel
   - Verify full functionality

2. **Reconnection**
   - Break SSH tunnel
   - Verify client handles disconnection
   - Restore tunnel
   - Verify automatic reconnection

3. **Multiple Clients**
   - Multiple SSH tunnels to same daemon
   - Multiple client connections
   - Verify isolation and concurrency

4. **Error Scenarios**
   - Port already in use
   - SSH tunnel fails
   - Connection refused
   - Network timeout

## Documentation Outline

### docs/remote-access.md

```markdown
# Remote Access via SSH Tunnel

## Overview
- What this enables
- Security model
- Prerequisites

## Quick Start
- 3-step setup

## Detailed Setup
- Daemon configuration
- SSH tunnel setup
- Client connection
- Connection string format

## Example Configurations
- config.toml examples
- systemd service
- SSH config entries

## Security Considerations
- SSH key authentication
- Firewall configuration
- Port selection
- Multi-user scenarios

## Troubleshooting
- Connection refused
- SSH tunnel issues
- Port conflicts
- Network debugging

## Advanced Usage
- Custom ports
- Multiple tunnels
- Persistent tunnels (autossh)
- Helper scripts
```

## Dependencies

- FEAT-066: Daemon TCP listener (completed)
- FEAT-067: Client TCP connection (completed)

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Unclear documentation | Medium | Medium | User testing, examples |
| Missing edge cases | Low | Low | Comprehensive validation |
| Security misunderstanding | Low | High | Emphasize SSH security model |

## Rollback Strategy

Documentation changes are low-risk. If issues are found:
1. Update documentation based on feedback
2. Add clarifications to troubleshooting section
3. Provide additional examples

## Implementation Notes

### Phase 1: Core Documentation (Required)
- Write docs/remote-access.md
- Add examples/remote-configs/
- Update README.md

### Phase 2: Validation (Required)
- Test complete workflow
- Document any issues
- Verify troubleshooting steps

### Phase 3: Helper Scripts (Optional)
- fugue-remote wrapper
- SSH tunnel management
- Connection testing utilities

---
*This plan should be updated as implementation progresses.*
