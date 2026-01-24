# FEAT-067: Client TCP connection support

**Priority**: P2
**Component**: fugue-client
**Type**: new_feature
**Estimated Effort**: small
**Business Value**: high

## Overview

Add TCP connection capability to the fugue client (fugue-client), enabling it to connect to remote daemons over TCP in addition to the default Unix socket. This is Phase 2 of remote peering support.

## Problem Statement

The fugue client currently only connects via Unix domain socket. With FEAT-066 adding TCP listener support to the daemon, the client needs corresponding TCP connection capability to complete the remote workflow.

Without this feature, users cannot:
- Connect to remote daemons over TCP
- Use SSH tunnels for remote fugue control
- Leverage FEAT-066's TCP listener functionality

## Requested Feature

- Add `--addr` flag to fugue-client for specifying connection address
- Support `tcp://host:port` and `unix://path` URL formats
- Maintain backward compatibility (default to Unix socket)
- Support `FUGUE_ADDR` environment variable for configuration

## Implementation Tasks

### Section 1: CLI and Configuration
- [ ] Add `--addr` flag to client CLI (clap)
- [ ] Support `FUGUE_ADDR` environment variable
- [ ] Parse connection URLs (tcp://host:port, unix://path)
- [ ] Default to `unix://~/.fugue/fugue.sock` if not specified

### Section 2: Connection Logic
- [ ] Implement TCP connection using `tokio::net::TcpStream`
- [ ] Implement URL parsing and validation
- [ ] Connect based on URL scheme (tcp vs unix)
- [ ] Handle connection errors with clear messages

### Section 3: Protocol Compatibility
- [ ] Ensure bincode messages work over TCP
- [ ] Add framing if needed (LengthDelimitedCodec)
- [ ] Test message round-trip over TCP
- [ ] Verify compatibility with daemon TCP implementation

### Section 4: Testing
- [ ] Test `tcp://localhost:9999` connections
- [ ] Test `unix://` connections (backward compat)
- [ ] Test `FUGUE_ADDR` environment variable
- [ ] Test connection errors and error messages
- [ ] Test with FEAT-066 TCP daemon

### Section 5: Documentation
- [ ] Document `--addr` flag usage
- [ ] Document `FUGUE_ADDR` environment variable
- [ ] Add examples for SSH tunnel workflow
- [ ] Update help text

## Acceptance Criteria

- [ ] Client accepts `--addr` flag or `FUGUE_ADDR` env var
- [ ] Client can connect via `tcp://host:port`
- [ ] Client can still connect via Unix socket (default)
- [ ] Bincode messages work over TCP
- [ ] Clear error messages on connection failure
- [ ] Help text documents new flag

## Benefits

- **Remote Workflows**: Complete the remote peering story started by FEAT-066
- **SSH Tunnel Support**: Enable secure remote access via `ssh -L` port forwarding
- **Flexibility**: Users choose connection method via simple URL scheme
- **Backward Compatible**: Existing workflows unchanged (Unix socket default)

## Dependencies

- FEAT-066 (TCP listener support in daemon) - **REQUIRED**

## Blocks

- FEAT-068 (SSH tunnel integration and documentation)

## Related Files

- `fugue-client/src/main.rs` - CLI flag parsing
- `fugue-client/src/client.rs` - Connection logic

## Notes

This is Phase 2 from `fugue-tcp-implementation-plan.md`. Combined with FEAT-066, this enables SSH tunnel workflows where daemon runs on remote host and client connects via `ssh -L` port forwarding.

**Example Workflow:**
```bash
# On remote machine (polecats):
fugue-server --listen-tcp 127.0.0.1:9999

# On local machine (Mayor):
ssh -L 9999:127.0.0.1:9999 polecats -N &
fugue-client --addr tcp://localhost:9999 list-sessions

# Or using environment variable:
export FUGUE_ADDR=tcp://localhost:9999
fugue-client list-sessions
```

## Implementation Guidance

### URL Parsing
Consider using the `url` crate for robust URL parsing:
```rust
use url::Url;

match Url::parse(addr) {
    Ok(url) => match url.scheme() {
        "tcp" => connect_tcp(url.host_str()?, url.port_or_known_default()?),
        "unix" => connect_unix(url.path()),
        scheme => Err(format!("Unsupported scheme: {}", scheme)),
    },
    Err(_) => {
        // Fallback: treat as host:port or path
        if addr.contains(':') {
            connect_tcp_raw(addr)
        } else {
            connect_unix(addr)
        }
    }
}
```

### Connection Abstraction
Both TCP and Unix streams implement `AsyncRead + AsyncWrite`, so the message handling code should be transport-agnostic:

```rust
async fn connect(addr: &str) -> Result<Box<dyn AsyncRead + AsyncWrite>> {
    match parse_url(addr)? {
        ConnectionType::Tcp(host, port) => {
            let stream = TcpStream::connect((host, port)).await?;
            Ok(Box::new(stream))
        }
        ConnectionType::Unix(path) => {
            let stream = UnixStream::connect(path).await?;
            Ok(Box::new(stream))
        }
    }
}
```

### Error Messages
Provide clear, actionable error messages:
- "Failed to connect to tcp://localhost:9999: Connection refused. Is the daemon running with --listen-tcp?"
- "Failed to parse address 'invalid': Expected format tcp://host:port or unix://path"
- "Failed to connect to unix socket: No such file or directory. Is the daemon running?"
