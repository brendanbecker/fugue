# Implementation Plan: FEAT-067

**Work Item**: [FEAT-067: Client TCP connection support](PROMPT.md)
**Component**: ccmux-client
**Priority**: P2
**Created**: 2026-01-13

## Overview

Add TCP connection capability to ccmux-client, enabling connections to remote daemons over TCP alongside the existing Unix socket support. This completes Phase 2 of remote peering (Phase 1 = FEAT-066).

## Architecture Decisions

### Connection Address Format

**Decision**: Use URL-style scheme prefix (`tcp://`, `unix://`)

**Rationale**:
- Clear, unambiguous scheme identification
- Standard format familiar to users
- Easy to parse with existing libraries
- Allows future extension (e.g., `tls://`, `ws://`)

**Alternatives Considered**:
- Raw `host:port` - Ambiguous (what about IPv6? Unix paths with colons?)
- Flag-based (`--tcp` vs `--unix`) - Less flexible, more flags to maintain

### Connection Abstraction

**Decision**: Use trait objects (`Box<dyn AsyncRead + AsyncWrite>`) for transport abstraction

**Rationale**:
- Both `TcpStream` and `UnixStream` implement these traits
- Single message handler works for all transports
- Easy to test with mock streams

**Trade-offs**:
- Small allocation cost for Box
- Dynamic dispatch overhead (negligible for network I/O)
- Benefit: Clean, maintainable code structure

### Configuration Priority

**Decision**: CLI flag > Environment variable > Default (Unix socket)

**Rationale**:
- CLI flags are most explicit (highest priority)
- Environment variables useful for scripts
- Default ensures backward compatibility

### Message Framing

**Decision**: Start with raw bincode, add framing only if needed

**Rationale**:
- Unix socket version works with raw bincode
- TCP should work the same way (full-duplex, ordered)
- Only add `LengthDelimitedCodec` if message boundary issues arise
- Keep it simple initially

**Risk**: TCP may require framing if message boundaries are not preserved
**Mitigation**: Test thoroughly; add framing in follow-up if needed

## Affected Components

| Component | Type of Change | Risk Level |
|-----------|----------------|------------|
| ccmux-client/src/main.rs | Add CLI flag parsing | Low |
| ccmux-client/src/client.rs | Add TCP connection logic | Medium |
| ccmux-client/Cargo.toml | Add url crate dependency | Low |

## Dependencies

**Required**:
- FEAT-066: Daemon must support TCP before client can connect to it

**Optional**:
- Consider using `url` crate for robust URL parsing (recommended)

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Message framing issues over TCP | Medium | Medium | Test thoroughly; add LengthDelimitedCodec if needed |
| Breaking backward compatibility | Low | High | Default to Unix socket; extensive testing |
| Connection error handling gaps | Medium | Low | Clear error messages; handle all error cases |
| URL parsing edge cases (IPv6, etc.) | Low | Low | Use established url crate; add tests |

## Rollback Strategy

If implementation causes issues:
1. Revert commits associated with this work item
2. Verify existing Unix socket functionality unchanged
3. Document what went wrong in comments.md
4. Default behavior (Unix socket) unaffected, so rollback is low-risk

## Implementation Notes

### Phase 1: CLI and Parsing (Low Risk)
1. Add `--addr` flag to clap Args struct
2. Add `CCMUX_ADDR` environment variable support
3. Implement URL parsing (tcp://, unix://)
4. Unit tests for parsing logic

### Phase 2: Connection Logic (Medium Risk)
1. Refactor existing UnixStream connection to be transport-agnostic
2. Add TcpStream connection path
3. Ensure both return same interface (AsyncRead + AsyncWrite)
4. Integration tests with local TCP daemon

### Phase 3: Compatibility Testing (Critical)
1. Test with FEAT-066 TCP daemon
2. Test Unix socket (ensure no regression)
3. Test error cases (daemon not running, wrong port, etc.)
4. Test environment variable behavior

### Phase 4: Documentation
1. Update --help text
2. Add usage examples to README or docs
3. Document environment variable

### Dependency on FEAT-066

This feature **requires** FEAT-066 to be completed first, as there's no point in client TCP support without daemon TCP support. However, development can proceed in parallel and be tested with local daemons.

**Testing Strategy**:
- Test locally with `tcp://localhost:9999`
- Coordinate with FEAT-066 implementation to ensure protocol compatibility

---

## Code Structure (Proposed)

```rust
// main.rs
#[derive(Parser)]
struct Args {
    /// Connection address (tcp://host:port or unix://path)
    #[arg(long, env = "CCMUX_ADDR", default_value = "unix://~/.ccmux/ccmux.sock")]
    addr: String,
    // ... other args
}

// client.rs
enum ConnectionAddr {
    Tcp { host: String, port: u16 },
    Unix { path: PathBuf },
}

impl ConnectionAddr {
    fn parse(addr: &str) -> Result<Self> {
        // URL parsing logic
    }

    async fn connect(&self) -> Result<Box<dyn AsyncRead + AsyncWrite + Unpin + Send>> {
        match self {
            ConnectionAddr::Tcp { host, port } => {
                let stream = TcpStream::connect((host.as_str(), *port)).await?;
                Ok(Box::new(stream))
            }
            ConnectionAddr::Unix { path } => {
                let stream = UnixStream::connect(path).await?;
                Ok(Box::new(stream))
            }
        }
    }
}
```

---
*This plan should be updated as implementation progresses.*
