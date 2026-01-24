# Implementation Plan: FEAT-069

**Work Item**: [FEAT-069: TLS/auth for direct TCP connections](PROMPT.md)
**Component**: fugue-server, fugue-client
**Priority**: P3
**Created**: 2026-01-13

## Overview

Add native TLS encryption and authentication for direct TCP connections as an alternative to SSH tunnels (FEAT-068) in trusted network environments. This is Phase 4 of remote peering support and is marked as a future enhancement.

## Architecture Decisions

### TLS Library Choice: rustls

**Decision**: Use rustls instead of native-tls or openssl

**Rationale**:
- Pure Rust implementation (no C dependencies)
- Memory safe by design
- Modern, secure defaults
- Good tokio integration
- Active maintenance

**Trade-offs**:
- Less mature than OpenSSL
- May have fewer features than OpenSSL
- Smaller ecosystem

### Authentication Approach: Simple Shared-Secret

**Decision**: Implement simple shared-secret authentication in handshake

**Rationale**:
- Sufficient for trusted network environments (VPN/LAN)
- Easy to configure and use
- Minimal overhead
- Clear security model

**Trade-offs**:
- Less secure than mTLS
- Token management burden on users
- No fine-grained access control

**Alternative considered**: mTLS (mutual TLS)
- Rejected: Too complex for terminal multiplexer use case
- May be added in future enhancement

### URL Scheme: tls://

**Decision**: Use `tls://host:port` URL format

**Rationale**:
- Clear indication of TLS usage
- Consistent with other protocols (http/https)
- Easy to parse and validate
- Backward compatible with tcp:// and unix://

**Trade-offs**:
- Need to update URL parsing logic
- Need to document all URL schemes clearly

### Certificate Management: Self-Signed by Default

**Decision**: Generate self-signed certificates by default, with option for custom certs

**Rationale**:
- Easy to get started (zero configuration)
- Sufficient for trusted environments
- Can upgrade to proper certs later
- Optional Let's Encrypt support can be added

**Trade-offs**:
- Self-signed certs require certificate pinning or trust on first use
- Users may need education about certificate validation

## Affected Components

| Component | Type of Change | Risk Level |
|-----------|----------------|------------|
| fugue-server | TLS acceptor, auth validation | High (security) |
| fugue-client | TLS connector, auth token | High (security) |
| fugue-protocol | Auth handshake messages | Medium |
| Cargo.toml | rustls dependency | Low |
| fugue.toml | TLS/auth configuration | Low |

## Dependencies

### Required (Must be completed first)
- **FEAT-066**: TCP listener support in daemon
- **FEAT-067**: Client TCP connection support
- **FEAT-068**: SSH tunnel integration (provides baseline for comparison)

### Related (Context)
- fugue-tcp-implementation-plan.md (Phase 4 design)
- fugue-peering-design.md (security considerations)

## Implementation Phases

### Phase 1: TLS Infrastructure (3-5 days)
1. Add rustls dependencies
2. Implement TLS acceptor wrapper for daemon
3. Implement TLS connector wrapper for client
4. Support both TLS and non-TLS on same codebase
5. Error handling and logging

**Deliverables**:
- TLS connection establishment working
- Clear error messages for TLS failures
- Unit tests for TLS setup

### Phase 2: Authentication (2-3 days)
1. Design handshake protocol
2. Add auth message to protocol
3. Implement auth validation in daemon
4. Implement auth token in client
5. Fail connections with invalid auth

**Deliverables**:
- Authentication prevents unauthorized access
- Auth token in config and CLI
- Auth failure logging

### Phase 3: Certificate Management (2-3 days)
1. Self-signed certificate generation utility
2. Certificate loading from files
3. Certificate validation options
4. Certificate storage conventions
5. Certificate rotation support

**Deliverables**:
- Certificate generation utility
- Certificate loading working
- Documentation for certificate setup

### Phase 4: URL Scheme (1-2 days)
1. Add tls:// URL parsing
2. Update --addr parameter handling
3. Maintain backward compatibility
4. Error messages for invalid URLs

**Deliverables**:
- tls:// URLs work correctly
- Backward compatibility maintained
- Clear documentation of URL formats

### Phase 5: Testing & Security Review (3-5 days)
1. Connection establishment tests
2. Authentication tests (valid/invalid)
3. Certificate validation tests
4. Backward compatibility tests
5. Security review and audit
6. Performance testing

**Deliverables**:
- Comprehensive test coverage
- Security review report
- Performance comparison with SSH tunnels
- No known vulnerabilities

### Phase 6: Documentation (2-3 days)
1. TLS setup guide
2. Certificate management guide
3. Authentication configuration
4. Security best practices
5. Migration guide from SSH tunnels
6. Troubleshooting guide

**Deliverables**:
- Complete documentation
- Example configurations
- Security recommendations

**Total estimated time**: 13-21 days (large effort)

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| TLS implementation vulnerabilities | Medium | Critical | Use well-tested rustls, security review, keep dependencies updated |
| Certificate management complexity | High | Medium | Provide utilities, clear documentation, sensible defaults |
| Authentication bypass | Low | Critical | Thorough testing, security review, fail-secure design |
| Backward compatibility breakage | Low | High | Comprehensive testing, feature flags, clear migration path |
| Performance degradation | Low | Medium | Performance testing, optimization if needed |
| User confusion about security | High | Medium | Clear documentation, security warnings, sensible defaults |

## Security Considerations

### Critical Requirements

1. **Never bind to 0.0.0.0 without TLS**
   - Refuse to start with public binding without TLS
   - Require explicit confirmation for public TLS binding
   - Default to localhost for non-TLS

2. **Strict Certificate Validation**
   - Default to strict validation
   - Consider certificate pinning for self-signed
   - Log validation failures
   - Document validation options

3. **Secure Authentication**
   - Use cryptographically secure tokens
   - Log authentication failures
   - Rate limit authentication attempts
   - Consider token rotation

4. **Audit TLS Implementation**
   - Review for common TLS issues
   - Use secure cipher suites
   - Keep rustls updated
   - Monitor security advisories

### Security Testing Checklist

- [ ] TLS handshake cannot be bypassed
- [ ] Invalid certificates are rejected
- [ ] Authentication prevents unauthorized access
- [ ] No information leakage in error messages
- [ ] Rate limiting prevents brute force
- [ ] Secure defaults for all TLS options
- [ ] Clear security warnings in documentation

## Rollback Strategy

If implementation causes issues:

1. **Feature Flag**: Implement behind feature flag for easy disable
2. **Config Option**: Allow disabling TLS in config
3. **Graceful Degradation**: Fall back to SSH tunnel recommendation
4. **Revert Commits**: Document commit range for clean revert
5. **Migration Path**: Provide clear rollback instructions

## Implementation Notes

### rustls Integration

```rust
// Daemon side (acceptor)
use tokio_rustls::TlsAcceptor;

let acceptor = TlsAcceptor::from(Arc::new(tls_config));
let tls_stream = acceptor.accept(tcp_stream).await?;
```

```rust
// Client side (connector)
use tokio_rustls::TlsConnector;

let connector = TlsConnector::from(Arc::new(tls_config));
let tls_stream = connector.connect(domain, tcp_stream).await?;
```

### Authentication Protocol

```
Client → Server: AUTH <token_hash>
Server → Client: AUTH_OK | AUTH_FAIL
```

Token should be hashed (SHA-256) before transmission. Consider adding challenge-response for replay prevention.

### Configuration Example

```toml
[server.tls]
enabled = true
cert = "~/.config/fugue/cert.pem"
key = "~/.config/fugue/key.pem"
auth_tokens = ["path/to/tokens.txt"]
bind = "0.0.0.0:8585"  # Only allowed if TLS enabled

[client.tls]
verify_cert = true
cert_pinning = "sha256:..."  # Optional for self-signed
auth_token = "..."
```

### URL Format Examples

```bash
# TLS connection with authentication
fugue-client --addr tls://remote.host:8585

# Non-TLS TCP (local only)
fugue-client --addr tcp://localhost:8585

# Unix socket (default)
fugue-client --addr unix:///tmp/fugue.sock
```

## Testing Strategy

### Unit Tests
- TLS configuration parsing
- Certificate loading
- Authentication token validation
- URL parsing

### Integration Tests
- TLS connection establishment
- Authentication success/failure
- Certificate validation
- Backward compatibility

### Security Tests
- Invalid certificate rejection
- Authentication bypass attempts
- TLS downgrade protection
- Man-in-the-middle scenarios

### Performance Tests
- TLS overhead measurement
- Comparison with SSH tunnels
- Connection establishment time
- Throughput testing

## Documentation Plan

### User Documentation
1. **TLS Setup Guide** (`docs/tls-setup.md`)
   - Certificate generation
   - Configuration options
   - Security recommendations

2. **Authentication Guide** (`docs/tls-auth.md`)
   - Token generation
   - Token management
   - Multi-user scenarios

3. **Migration Guide** (`docs/ssh-to-tls.md`)
   - Comparison with SSH tunnels
   - When to use each approach
   - Migration steps

### Developer Documentation
1. **Architecture** (in PLAN.md)
   - TLS implementation details
   - Authentication protocol
   - Security considerations

2. **Security Review** (security-review.md)
   - Threat model
   - Security testing results
   - Known limitations

## Success Metrics

- [ ] TLS connections establish successfully
- [ ] Authentication prevents unauthorized access
- [ ] Zero known security vulnerabilities
- [ ] Performance within 10% of SSH tunnels
- [ ] Documentation rated clear by reviewers
- [ ] Backward compatibility maintained
- [ ] Security review passed

---
*This plan should be updated as implementation progresses.*
