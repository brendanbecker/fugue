# Task Breakdown: FEAT-069

**Work Item**: [FEAT-069: TLS/auth for direct TCP connections](PROMPT.md)
**Status**: Not Started
**Last Updated**: 2026-01-13

## Prerequisites

- [ ] Read and understand PROMPT.md
- [ ] Review PLAN.md and update if needed
- [ ] Verify FEAT-066, FEAT-067, FEAT-068 are complete
- [ ] Review rustls documentation
- [ ] Review fugue-tcp-implementation-plan.md Phase 4
- [ ] Review fugue-peering-design.md security considerations

## Phase 1: TLS Infrastructure (3-5 days)

- [ ] Add rustls dependency to workspace Cargo.toml
- [ ] Add tokio-rustls for async TLS support
- [ ] Create `fugue-server/src/tls.rs` module
- [ ] Create `fugue-client/src/tls.rs` module
- [ ] Implement TLS acceptor wrapper for daemon
- [ ] Implement TLS connector wrapper for client
- [ ] Add TLS configuration structs
- [ ] Support both TLS and non-TLS TCP connections
- [ ] Error handling for TLS handshake failures
- [ ] Logging for TLS connections
- [ ] Unit tests for TLS setup

## Phase 2: Authentication Mechanism (2-3 days)

- [ ] Design simple handshake protocol
- [ ] Create `fugue-protocol/src/auth.rs` module
- [ ] Add AUTH message type to protocol
- [ ] Implement token hashing (SHA-256)
- [ ] Implement auth validation in daemon
- [ ] Implement auth token in client
- [ ] Fail connections with invalid auth
- [ ] Add auth token to config file
- [ ] Add auth token to CLI parameter
- [ ] Log authentication failures
- [ ] Optional: Multiple auth tokens support
- [ ] Unit tests for authentication

## Phase 3: Certificate Management (2-3 days)

- [ ] Design certificate storage conventions
- [ ] Create certificate generation utility
- [ ] Implement self-signed certificate generation
- [ ] Implement certificate loading from files
- [ ] Implement certificate validation options
- [ ] Support certificate pinning for self-signed certs
- [ ] Document certificate file locations
- [ ] Add certificate rotation support
- [ ] Error handling for invalid certificates
- [ ] Optional: Let's Encrypt integration planning
- [ ] Unit tests for certificate operations

## Phase 4: URL Scheme Support (1-2 days)

- [ ] Add tls:// URL scheme parsing
- [ ] Update client --addr parameter handling
- [ ] Update daemon bind address handling
- [ ] Support tls://host:port format
- [ ] Maintain backward compatibility with tcp:// and unix://
- [ ] Clear error messages for invalid URLs
- [ ] Clear error messages for TLS failures
- [ ] Document all URL format options
- [ ] Unit tests for URL parsing

## Phase 5: Security Implementation (2-3 days)

- [ ] Implement 0.0.0.0 binding check (refuse without TLS)
- [ ] Implement explicit confirmation for public binding
- [ ] Default to localhost for non-TLS TCP
- [ ] Strict certificate validation by default
- [ ] Secure cipher suite selection
- [ ] Rate limiting for auth attempts (optional)
- [ ] Security logging and monitoring
- [ ] Document security configuration options

## Phase 6: Testing (3-5 days)

### Functional Testing
- [ ] Test TLS connection establishment
- [ ] Test authentication with valid token
- [ ] Test authentication with invalid token
- [ ] Test certificate validation (valid cert)
- [ ] Test certificate validation (invalid cert)
- [ ] Test backward compatibility with non-TLS TCP
- [ ] Test backward compatibility with Unix sockets
- [ ] Test multiple simultaneous TLS connections

### Security Testing
- [ ] Test authentication bypass attempts
- [ ] Test TLS downgrade protection
- [ ] Test certificate pinning (if implemented)
- [ ] Test invalid certificate rejection
- [ ] Test man-in-the-middle scenarios
- [ ] Test information leakage in errors
- [ ] Security review and audit

### Performance Testing
- [ ] Measure TLS connection overhead
- [ ] Compare with SSH tunnel performance
- [ ] Measure connection establishment time
- [ ] Measure throughput with TLS
- [ ] Identify performance bottlenecks

### Integration Testing
- [ ] Test with fugue-client and fugue-server
- [ ] Test error handling end-to-end
- [ ] Test configuration file loading
- [ ] Test CLI parameter handling
- [ ] Test mixed TLS/non-TLS environments

## Phase 7: Documentation (2-3 days)

- [ ] Write TLS setup guide (`docs/tls-setup.md`)
- [ ] Write certificate management guide
- [ ] Write authentication configuration guide
- [ ] Document security best practices
- [ ] Document when to use TLS vs SSH tunnels
- [ ] Write migration guide from SSH tunnels
- [ ] Write troubleshooting guide
- [ ] Add example configurations
- [ ] Document URL format options
- [ ] Update main README with TLS information
- [ ] Create security review document

## Optional: mDNS Discovery (Phase 8, future)

- [ ] Research mDNS libraries for Rust
- [ ] Design service advertisement format
- [ ] Implement mDNS service advertisement in daemon
- [ ] Implement mDNS client discovery
- [ ] Add service browsing UI
- [ ] Document discovery workflow
- [ ] Test discovery on various networks

## Completion Checklist

### Functionality
- [ ] TLS connections work with valid certificates
- [ ] Authentication prevents unauthorized access
- [ ] Backward compatible with non-TLS TCP
- [ ] Backward compatible with Unix sockets
- [ ] URL scheme parsing works correctly

### Security
- [ ] Cannot bind to 0.0.0.0 without TLS
- [ ] Certificate validation is strict by default
- [ ] Authentication cannot be bypassed
- [ ] No information leakage in errors
- [ ] Security review completed
- [ ] No known vulnerabilities

### Quality
- [ ] All tests passing
- [ ] Code coverage adequate
- [ ] Error messages are clear
- [ ] Logging is comprehensive
- [ ] Configuration is well-documented

### Documentation
- [ ] TLS setup guide complete
- [ ] Certificate management documented
- [ ] Authentication setup documented
- [ ] Security best practices documented
- [ ] Migration guide from SSH tunnels complete
- [ ] Troubleshooting guide complete
- [ ] Example configurations provided

### Performance
- [ ] TLS overhead is acceptable
- [ ] Performance within 10% of SSH tunnels
- [ ] No degradation of non-TLS connections
- [ ] Connection establishment is fast enough

### Integration
- [ ] Works with existing fugue infrastructure
- [ ] Config file format is consistent
- [ ] CLI interface is intuitive
- [ ] Error handling is robust

## Notes

- This is a large effort feature (13-21 days estimated)
- Security is critical - take time for thorough review
- Document security considerations clearly
- SSH tunnels (FEAT-068) remain the recommended MVP
- This feature is for advanced use cases only
- Consider implementing behind a feature flag initially
- Update PLAN.md with decisions made during implementation

---
*Check off tasks as you complete them. Update status field above.*
