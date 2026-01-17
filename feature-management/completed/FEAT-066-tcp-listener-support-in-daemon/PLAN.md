# Implementation Plan: FEAT-066

**Work Item**: [FEAT-066: TCP listener support in daemon](PROMPT.md)
**Component**: ccmux-server
**Priority**: P2
**Created**: 2026-01-13

## Overview

Add optional TCP listener support to the ccmux daemon, enabling network-based client connections. This is Phase 1 of remote peering support, allowing the daemon to accept connections over TCP in addition to the default Unix socket.

## Architecture Decisions

### Transport-Agnostic Connection Handler

**Decision**: Abstract the connection handler to work with any transport implementing `AsyncRead + AsyncWrite`.

**Rationale**:
- Unix sockets and TCP sockets both implement these traits
- Single code path for message handling reduces bugs
- Easy to add more transports in the future (e.g., WebSocket, QUIC)

**Trade-offs**:
- Slight abstraction overhead
- Cannot leverage transport-specific optimizations
- Worth it for code simplicity and maintainability

### Concurrent Accept Loops

**Decision**: Spawn independent tokio tasks for Unix and TCP accept loops.

**Rationale**:
- Each listener can block independently without affecting the other
- Clean separation of concerns
- Easy to enable/disable TCP listener via config

**Implementation Pattern**:
```rust
// Spawn Unix socket accept loop
tokio::spawn(async move {
    accept_unix_loop(unix_listener, handler).await
});

// Conditionally spawn TCP accept loop
if let Some(tcp_addr) = config.listen_tcp {
    tokio::spawn(async move {
        accept_tcp_loop(tcp_listener, handler).await
    });
}
```

### Message Framing Over TCP

**Decision**: Use length-delimited framing for bincode messages over TCP.

**Rationale**:
- TCP is a byte stream (no message boundaries)
- Unix sockets have implicit message boundaries in some cases
- Length prefix allows receiver to know message size
- `tokio_util::codec::LengthDelimitedCodec` provides this

**Alternative Considered**:
- Raw bincode over TCP (no framing)
- Rejected because: No way to know message boundaries, could deadlock waiting for more bytes

**Implementation**:
```rust
use tokio_util::codec::{LengthDelimitedCodec, Framed};

let framed = Framed::new(tcp_stream, LengthDelimitedCodec::new());
// Read/write frames containing bincode messages
```

### Security-First Configuration

**Decision**: Default to localhost-only binding (127.0.0.1).

**Rationale**:
- Prevents accidental exposure to network
- Explicit opt-in required for remote access
- Aligns with security best practices (fail-safe defaults)

**Configuration Schema**:
```toml
# Default: TCP disabled
# listen_tcp = ""

# Explicit localhost (safe)
listen_tcp = "127.0.0.1:8888"

# Explicit remote binding (requires user awareness)
listen_tcp = "0.0.0.0:8888"
```

## Affected Components

| Component | Type of Change | Risk Level |
|-----------|----------------|------------|
| ccmux-server/src/main.rs | Add CLI flag parsing | Low |
| ccmux-server/src/server/mod.rs | Add TCP accept loop | Medium |
| ccmux-server/src/server/connection.rs | Make handler transport-agnostic | Medium |
| ccmux-server/config.toml | Add listen_tcp field | Low |
| ccmux-server/Cargo.toml | Add tokio_util dependency | Low |

## Dependencies

None - this is a standalone feature.

## Blocks

- FEAT-067: Client TCP connection support
  - Clients need to support TCP connections to use this feature
  - Without FEAT-067, only the server side is implemented

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Security exposure if bound to 0.0.0.0 | Medium | High | Default to 127.0.0.1, document risks clearly |
| Message framing bugs (truncation, corruption) | Medium | High | Comprehensive testing, use battle-tested codec |
| Regression in Unix socket handling | Low | High | Keep Unix socket code unchanged, test both paths |
| Performance degradation | Low | Medium | Profile both Unix and TCP paths, optimize if needed |
| Firewall/network issues | Medium | Low | Document requirements, provide diagnostic tools |

## Rollback Strategy

If implementation causes issues:
1. Revert commits associated with FEAT-066
2. Remove `listen_tcp` config option
3. Verify Unix socket functionality unchanged
4. Document what went wrong in comments.md

Safe rollback is easy because:
- TCP listener is optional (disabled by default)
- Unix socket code remains unchanged
- No database or state migration required

## Implementation Notes

### Phase 1: Configuration (Low Risk)

Add configuration support without changing server behavior:
- Add `listen_tcp` field to config struct
- Add `--listen-tcp` CLI flag
- Parse and validate TCP address
- Log if TCP is enabled but don't start listener yet

**Validation**:
- Parse host:port format correctly
- Reject invalid addresses
- Handle edge cases (IPv6, domain names)

### Phase 2: TCP Listener (Medium Risk)

Implement TCP accept loop:
- Use `tokio::net::TcpListener`
- Spawn separate task
- Handle TCP-specific errors gracefully
- Log connections/disconnections

**Key Decisions**:
- Should `SO_REUSEADDR` be set? (Yes, for quick restarts)
- Should `TCP_NODELAY` be set? (Yes, for low latency)
- Should connection limits be enforced? (Not in Phase 1)

### Phase 3: Message Framing (High Risk)

This is the most complex part. Consider two approaches:

**Approach A**: Length-delimited framing
- Use `tokio_util::codec::LengthDelimitedCodec`
- Pros: Standard, well-tested, handles message boundaries correctly
- Cons: Extra dependency, slight overhead

**Approach B**: Fixed-size buffer with retry
- Read into fixed buffer, retry if incomplete message
- Pros: No extra dependencies
- Cons: Complex error handling, easy to get wrong

**Recommendation**: Use Approach A (length-delimited codec). The benefits far outweigh the cost of an extra dependency.

### Phase 4: Testing (Critical)

Test matrix:

| Test Case | Transport | Expected Result |
|-----------|-----------|-----------------|
| Unix only | Unix socket | Works as before |
| TCP only | TCP (127.0.0.1) | Works identically |
| Both enabled | Unix + TCP | Both work concurrently |
| Message round-trip | TCP | Request/response succeed |
| Connection reset | TCP | Graceful error handling |
| Daemon restart | TCP | Reconnection works |

### Phase 5: Documentation

Document in multiple places:
1. Config file comments (inline)
2. README.md (user-facing)
3. SECURITY.md (security considerations)
4. CHANGELOG.md (release notes)

Security documentation is critical. Include:
- Default is localhost-only
- How to bind to remote addresses (and why it's risky)
- Recommend SSH tunnels for remote access
- Firewall considerations

## Performance Considerations

TCP has different performance characteristics than Unix sockets:
- Higher latency (network stack overhead)
- Lower throughput for small messages
- May benefit from batching/buffering

**Optimization Strategy**:
1. Implement naively first (correctness over performance)
2. Profile under realistic workloads
3. Optimize only if measurements show bottlenecks

**Expected Performance**:
- Unix socket: <1ms latency, >100MB/s throughput
- TCP (localhost): ~2-5ms latency, ~50MB/s throughput
- TCP (remote): Depends on network (10-100ms latency)

For ccmux's use case (infrequent control messages), this is acceptable.

## Future Enhancements (Out of Scope)

These are explicitly NOT included in FEAT-066:
- TLS/encryption (Phase 3)
- Authentication (Phase 3)
- Connection limits/rate limiting
- WebSocket transport
- QUIC transport
- Connection pooling

FEAT-066 focuses solely on basic TCP listener support. Security features (TLS, auth) will be added in later phases.

## Testing Strategy

### Unit Tests
- Config parsing (valid/invalid addresses)
- Message framing (encode/decode)
- Error handling (connection reset, etc.)

### Integration Tests
- End-to-end message round-trip over TCP
- Concurrent Unix + TCP connections
- Daemon restart with TCP enabled

### Manual Testing
- Use `netcat` to test raw TCP connection
- Use `tcpdump` to inspect TCP traffic
- Test from different machines (if available)

---
*This plan should be updated as implementation progresses.*
