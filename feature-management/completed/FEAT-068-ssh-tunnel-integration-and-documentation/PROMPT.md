# FEAT-068: SSH tunnel integration and documentation

**Priority**: P2
**Component**: documentation
**Type**: enhancement
**Estimated Effort**: small
**Business Value**: high

## Overview

Document and validate the complete SSH tunnel-based remote workflow for ccmux. This is the integration/validation phase following FEAT-066 (daemon TCP) and FEAT-067 (client TCP), enabling secure remote peering without requiring TLS implementation.

The SSH tunnel approach provides encryption and authentication via SSH keys, making it the recommended MVP for remote access.

## Problem Statement

With TCP support in both daemon and client (FEAT-066 and FEAT-067), users need clear documentation and validated workflows for the SSH tunnel use case. The SSH tunnel approach provides encryption and authentication via SSH keys, making it the recommended MVP for remote access without implementing TLS/auth (FEAT-069).

## Requested Feature

Comprehensive documentation and validation of SSH tunnel-based remote workflows:

- **Workflow Documentation**: Step-by-step setup instructions for SSH tunnel remote access
- **Example Configurations**: Sample config.toml files and wrapper scripts
- **Security Guidance**: Document SSH-based encryption and authentication benefits
- **Troubleshooting Guide**: Common issues and debugging steps
- **End-to-End Validation**: Test and validate the complete workflow
- **Optional Helper Scripts**: Convenience utilities for common workflows

## Benefits

1. **Clear Remote Setup Path**: Users can easily set up secure remote access
2. **Security Through SSH**: Leverages existing SSH infrastructure for encryption/auth
3. **No TLS Implementation Needed**: SSH tunnel provides secure channel without custom TLS
4. **Validated Workflow**: End-to-end tested and documented approach
5. **Troubleshooting Support**: Users can resolve common issues independently

## Implementation Tasks

### Section 1: Core Documentation
- [ ] Document complete SSH tunnel workflow
- [ ] Provide step-by-step setup instructions
- [ ] Include example configurations
- [ ] Document security considerations (SSH provides encryption + auth)
- [ ] Add troubleshooting section

### Section 2: Example Configurations
- [ ] Create example config.toml for remote daemon
- [ ] Create example wrapper scripts
- [ ] Document port selection best practices
- [ ] Provide systemd service examples (optional)

### Section 3: Workflow Validation
- [ ] Test daemon on remote host with TCP listener
- [ ] Test SSH tunnel setup (ssh -L port forwarding)
- [ ] Test client connection through tunnel
- [ ] Validate full workflow end-to-end
- [ ] Document any issues or limitations

### Section 4: Troubleshooting Guide
- [ ] Document common connection errors
- [ ] Add SSH tunnel debugging steps
- [ ] Provide network troubleshooting guidance
- [ ] Document firewall considerations

### Section 5: Optional Helper Scripts
- [ ] Create ccmux-remote wrapper script (optional)
- [ ] Add SSH tunnel management helpers (optional)
- [ ] Provide connection testing utilities (optional)

## Acceptance Criteria

- [ ] Complete workflow documentation exists
- [ ] SSH tunnel setup is clearly documented
- [ ] Example configurations are provided
- [ ] Workflow is validated end-to-end
- [ ] Troubleshooting guide is comprehensive
- [ ] Users can follow documentation to set up remote access

## Dependencies

- **FEAT-066**: TCP listener support in daemon (required)
- **FEAT-067**: Client TCP connection support (required)

## Blocks

- None (enables remote workflows)

## Related Files

- `docs/` - New documentation files
- `examples/` - Configuration examples
- `README.md` - Main documentation updates

## Example Workflow to Document

```bash
# Remote host: Start daemon with TCP listener
ccmux-server --listen-tcp 127.0.0.1:9999

# Local machine: Create SSH tunnel
ssh -L 9999:localhost:9999 remote-host

# Local machine: Connect client through tunnel
ccmux-client --addr tcp://localhost:9999
```

## Security Emphasis

SSH provides both encryption and authentication via SSH keys, making this a secure and practical solution. This approach:

- Uses battle-tested SSH encryption
- Leverages existing SSH key infrastructure
- Provides strong authentication
- Requires no custom TLS implementation
- Is the recommended approach from ccmux-peering-design.md

## Notes

This is the MVP completion for remote peering. The SSH tunnel approach avoids implementing TLS/auth (FEAT-069) while still providing secure, encrypted remote access. This completes the "Phase 1" remote support and enables users to access remote ccmux instances securely.

Future enhancement (FEAT-069) would add native TLS support, but SSH tunnels provide a complete and secure solution for most use cases.
