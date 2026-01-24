# Comments: FEAT-066

## 2026-01-13 - Feature Created

Created FEAT-066 as Phase 1 of remote peering support for fugue.

**Context**: This feature enables the fugue daemon to accept TCP connections in addition to Unix socket connections. This is essential for remote workflows, such as controlling fugue sessions on a gaming PC from a laptop via SSH tunnels.

**Key Design Points**:
- Default to localhost-only binding (127.0.0.1) for security
- Support concurrent Unix + TCP listeners
- Use length-delimited framing for bincode messages over TCP
- Reuse existing connection handler logic (transport-agnostic)

**Related Work**:
- Follows `fugue-tcp-implementation-plan.md` Phase 1 design
- Blocks FEAT-067 (Client TCP connection support)
- Phase 2 (SSH tunnels) requires both FEAT-066 and FEAT-067

**Security Considerations**:
- Must never bind to 0.0.0.0 by default
- Explicit user configuration required for remote binding
- Documentation must emphasize risks of network exposure
- Recommend SSH tunnels for remote access

**Implementation Notes**:
- Consider using `tokio_util::codec::LengthDelimitedCodec` for message framing
- Set `SO_REUSEADDR` and `TCP_NODELAY` socket options
- Spawn independent accept loops for Unix and TCP
- Ensure backward compatibility (TCP disabled by default)

---
*Add implementation notes and progress updates below*
