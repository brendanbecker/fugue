# FEAT-066: TCP listener support in daemon

**Priority**: P2
**Component**: ccmux-server
**Type**: new_feature
**Estimated Effort**: medium
**Business Value**: high

## Overview

Add optional TCP listener support to the ccmux daemon, enabling network-based client connections. This is Phase 1 of remote peering support, allowing the daemon to accept connections over TCP in addition to the default Unix socket.

## Problem Statement

Currently, ccmux daemon only accepts connections via Unix domain socket (`~/.ccmux/ccmux.sock`). This limits client connections to the local machine. To support remote workflows (e.g., Mayor on laptop controlling polecats on gaming PC), the daemon needs optional TCP listener capability.

## Requested Feature

- Add configurable TCP listener to daemon
- Support both Unix socket and TCP simultaneously
- Default to 127.0.0.1 binding for security
- Use existing bincode message protocol over TCP

## Implementation Tasks

### Section 1: Configuration Support
- [ ] Add `listen_tcp` field to server config.toml
- [ ] Add `--listen-tcp` CLI flag
- [ ] Parse TCP address (host:port format)
- [ ] Default to empty (disabled) for backward compatibility

### Section 2: TCP Listener Implementation
- [ ] Implement `accept_tcp()` loop using `tokio::net::TcpListener`
- [ ] Spawn separate task for TCP accept loop
- [ ] Reuse existing connection handler logic
- [ ] Support concurrent Unix + TCP listeners

### Section 3: Connection Handling
- [ ] Frame bincode messages over TCP (use `tokio_util::codec::LengthDelimitedCodec` if needed)
- [ ] Test raw bincode over TCP stream first
- [ ] Ensure same message handling as Unix socket
- [ ] Handle TCP-specific errors (connection reset, etc.)

### Section 4: Testing
- [ ] Test local TCP (127.0.0.1) connections
- [ ] Test concurrent Unix + TCP connections
- [ ] Verify message round-trip over TCP
- [ ] Test daemon restart with TCP enabled

### Section 5: Documentation
- [ ] Document `listen_tcp` config option
- [ ] Document security considerations (only bind 127.0.0.1 by default)
- [ ] Add example configurations

## Acceptance Criteria

- [ ] Daemon accepts `--listen-tcp` or config.toml setting
- [ ] TCP listener binds to specified address
- [ ] Clients can connect via TCP
- [ ] Bincode messages work over TCP
- [ ] Unix socket continues to work unchanged
- [ ] No security regression (default to localhost only)

## Benefits

- **Remote Workflows**: Enables cross-machine ccmux control
- **SSH Tunnel Support**: Foundation for secure remote access via SSH tunnels (Phase 2)
- **Flexibility**: Users can choose Unix socket (local) or TCP (network) as needed
- **Security**: Default localhost-only binding prevents accidental exposure

## Dependencies

None (standalone feature)

## Blocks

- FEAT-067 (Client TCP connection support)

## Related Files

- `ccmux-server/src/main.rs` - CLI flag parsing and startup
- `ccmux-server/src/server/mod.rs` - Server listener implementation
- `ccmux-server/config.toml` - Configuration schema

## Notes

This is Phase 1 from `docs/architecture/REMOTE_PEERING.md`. The SSH tunnel use case (Phase 2) requires both this and FEAT-067.

**Security is critical** - default to 127.0.0.1 to prevent accidental exposure. Users who need to bind to 0.0.0.0 (e.g., for SSH tunnels) must explicitly configure it.

Consider using Rust's type system to enforce localhost-only binding by default:
```rust
pub enum TcpBindAddress {
    Localhost(u16),        // 127.0.0.1:port
    Explicit(SocketAddr),  // User-provided address
}
```

## Architecture Notes

The daemon should spawn two independent accept loops:
1. Unix socket accept loop (existing)
2. TCP accept loop (new)

Both should call the same connection handler logic. The connection handler should be transport-agnostic (works with any `AsyncRead + AsyncWrite`).

Reference the existing Unix socket implementation in `ccmux-server/src/server/mod.rs` for patterns.
