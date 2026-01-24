# FEAT-069: TLS/auth for direct TCP connections

**Priority**: P3
**Component**: fugue-server, fugue-client
**Type**: enhancement
**Estimated Effort**: large
**Business Value**: medium

## Overview

Add native TLS encryption and authentication for direct TCP connections, eliminating the need for SSH tunnels in trusted network environments. This is Phase 4 (future enhancement) of remote peering support.

While SSH tunnels (FEAT-068) provide secure remote access, they require SSH setup and add a layer of indirection. In some environments (e.g., trusted LANs, VPNs), users may prefer direct TLS connections. This requires implementing TLS using rustls and a simple authentication mechanism.

## Problem Statement

Currently, remote fugue access requires either:
1. **Unix sockets** - Limited to local connections
2. **Unencrypted TCP** - Insecure for remote access
3. **SSH tunnels** - Secure but requires SSH configuration

SSH tunnels (FEAT-068) are the recommended MVP approach, but they have limitations:
- Require SSH server setup and key management
- Add an extra layer of indirection (local port forwarding)
- May be overkill for trusted network environments (VPNs, private LANs)

Users in trusted network environments may prefer direct TLS connections that provide:
- Native encryption without SSH overhead
- Simple shared-secret authentication
- Direct connection without port forwarding
- Backward compatibility with non-TLS TCP and Unix sockets

## Requested Feature

### Core Requirements

1. **TLS Support**
   - Use rustls for TLS implementation
   - Support both TLS and non-TLS TCP connections
   - Certificate loading and validation
   - Backward compatible with Unix sockets

2. **Authentication Mechanism**
   - Simple shared-secret authentication in handshake
   - Auth token in config file or CLI parameter
   - Fail connections with invalid auth
   - Optional: Multiple auth tokens support

3. **Certificate Management**
   - Self-signed certificate generation
   - Certificate storage and loading
   - Certificate validation options
   - Document certificate setup
   - Optional: Let's Encrypt integration

4. **URL Scheme Support**
   - Support `tls://host:port` URL format
   - Update client `--addr` parsing
   - Maintain backward compatibility
   - Clear error messages for TLS failures

5. **Discovery (Optional)**
   - mDNS service advertisement
   - Client discovery of local daemons
   - Service browsing and selection

## Implementation Tasks

### Section 1: TLS Infrastructure
- [ ] Add rustls dependency to Cargo.toml
- [ ] Implement TLS acceptor for daemon
- [ ] Implement TLS connector for client
- [ ] Support both TLS and non-TLS TCP connections
- [ ] Certificate loading and validation
- [ ] Error handling for TLS handshake failures

### Section 2: Authentication Mechanism
- [ ] Design simple handshake protocol
- [ ] Implement shared-secret authentication
- [ ] Add auth token to config/CLI
- [ ] Fail connections with invalid auth
- [ ] Optional: Multiple auth tokens support
- [ ] Document authentication flow

### Section 3: Certificate Management
- [ ] Self-signed certificate generation utility
- [ ] Certificate storage and loading
- [ ] Certificate validation options
- [ ] Document certificate setup
- [ ] Optional: Let's Encrypt integration
- [ ] Certificate rotation support

### Section 4: URL Scheme Support
- [ ] Support tls://host:port URL format
- [ ] Update client --addr parsing
- [ ] Maintain backward compatibility
- [ ] Clear error messages for TLS failures
- [ ] Document URL format options

### Section 5: Discovery (Optional)
- [ ] mDNS service advertisement
- [ ] Client discovery of local daemons
- [ ] Service browsing and selection
- [ ] Document discovery workflow

### Section 6: Testing
- [ ] Test TLS connection establishment
- [ ] Test authentication success/failure
- [ ] Test certificate validation
- [ ] Test backward compatibility
- [ ] Security testing and review
- [ ] Performance testing vs SSH tunnels

### Section 7: Documentation
- [ ] Document TLS setup and configuration
- [ ] Document certificate management
- [ ] Document authentication setup
- [ ] Security best practices
- [ ] Migration guide from SSH tunnels
- [ ] Troubleshooting guide

## Acceptance Criteria

- [ ] TLS connections work with valid certificates
- [ ] Authentication prevents unauthorized access
- [ ] Backward compatible with non-TLS TCP
- [ ] Clear error messages for TLS/auth failures
- [ ] Certificate management is well-documented
- [ ] Security review completed
- [ ] Performance is acceptable compared to SSH tunnels
- [ ] No security vulnerabilities introduced

## Dependencies

- **FEAT-066** - TCP listener support in daemon (Phase 1)
- **FEAT-067** - Client TCP connection support (Phase 2)
- **FEAT-068** - SSH tunnel integration (Phase 3 MVP)

This feature builds on the TCP infrastructure from FEAT-066/067 and provides an alternative to SSH tunnels from FEAT-068.

## Blocks

None - this is an optional enhancement that does not block other features.

## Related Files

- `fugue-server/src/tls.rs` (new)
- `fugue-client/src/tls.rs` (new)
- `fugue-protocol/src/auth.rs` (new)
- `Cargo.toml` (rustls dependency)
- `fugue-server/src/main.rs` (TLS listener setup)
- `fugue-client/src/main.rs` (TLS connection support)
- `docs/tls-setup.md` (new documentation)

## Notes

### Design Considerations

This is Phase 4 from `fugue-tcp-implementation-plan.md` and marked as "future" in `fugue-peering-design.md`. SSH tunnels (FEAT-068) are the recommended MVP approach. This feature is for advanced use cases where SSH tunnels are not preferred.

### Security Considerations

**CRITICAL**: Must never bind to 0.0.0.0 without TLS/auth enabled. Security requirements:

1. **Network Binding**
   - Refuse to bind to 0.0.0.0 without TLS
   - Default to localhost for non-TLS TCP
   - Explicit confirmation for public binding

2. **Certificate Validation**
   - Certificate validation should be strict by default
   - Consider certificate pinning for self-signed certs
   - Document validation options clearly

3. **Authentication**
   - Shared-secret should be cryptographically secure
   - Consider token rotation mechanism
   - Log authentication failures

4. **TLS Implementation**
   - Audit for common TLS implementation issues
   - Use secure cipher suites by default
   - Keep rustls updated

### Estimated Effort

Estimated effort is **large** due to:
- Security implications requiring careful design
- Proper TLS implementation and testing
- Certificate management complexity
- Authentication mechanism design
- Comprehensive security review needed
- Extensive documentation requirements

### Alternatives Considered

1. **SSH Tunnels** (FEAT-068) - Recommended MVP
   - Pros: Leverage existing SSH infrastructure, well-tested
   - Cons: Requires SSH setup, extra layer of indirection

2. **mTLS (Mutual TLS)**
   - Pros: Industry standard, strong authentication
   - Cons: More complex certificate management

3. **OAuth/JWT**
   - Pros: Standard authentication protocols
   - Cons: Overkill for terminal multiplexer use case

Decision: Simple shared-secret auth is sufficient for trusted network environments where this feature would be used. SSH tunnels remain the recommended approach for untrusted networks.

### Future Enhancements

- Integration with system keyring for token storage
- Certificate auto-renewal with Let's Encrypt
- Support for mTLS (mutual TLS) for stronger authentication
- Token-based access control (different permissions per token)
- Audit logging for security compliance
