# FEAT-128: Client --server CLI Flag

**Priority**: P1
**Component**: fugue-client/cli
**Effort**: Small
**Status**: new
**Depends**: FEAT-127

## Summary

Add `--server` / `-s` CLI flag to fugue client to select a named server from config.

## Problem

After FEAT-127 adds server profiles to config, users need a way to select which server to connect to from the command line.

## Proposed CLI

```bash
# Connect to named server from config
fugue --server polecats
fugue -s polecats

# Still works - explicit addr overrides config
fugue --addr tcp://127.0.0.1:9999

# No flags - uses server with default=true, or local socket
fugue
```

## Implementation

### Key Files

| File | Changes |
|------|---------|
| `fugue-client/src/cli.rs` | Add `--server` arg |
| `fugue-client/src/main.rs` | Resolve server name to addr from config |
| `fugue-client/src/config.rs` | Add `get_server()` method |

### CLI Args

```rust
#[derive(Parser, Debug)]
pub struct Args {
    /// Server name from config (e.g., "polecats")
    #[arg(short, long)]
    pub server: Option<String>,

    /// Connection address (overrides --server)
    #[arg(long)]
    pub addr: Option<String>,

    // ... existing args ...
}
```

### Resolution Logic

```rust
fn resolve_connection_addr(args: &Args, config: &Config) -> Result<String> {
    // Explicit --addr takes priority
    if let Some(addr) = &args.addr {
        return Ok(addr.clone());
    }

    // Named --server from config
    if let Some(name) = &args.server {
        let server = config.servers.get(name)
            .ok_or_else(|| anyhow!("Unknown server: {}", name))?;
        return Ok(server.addr.clone());
    }

    // Default server from config
    if let Some((_, server)) = config.servers.iter().find(|(_, s)| s.default) {
        return Ok(server.addr.clone());
    }

    // Fall back to local socket
    Ok(default_socket_addr())
}
```

## Acceptance Criteria

- [ ] `fugue -s name` connects to named server
- [ ] `fugue --server name` connects to named server
- [ ] Unknown server name produces clear error with available names
- [ ] `--addr` overrides `--server` if both provided
- [ ] Default server used when no flags
- [ ] Falls back to local socket if no default configured

## Related

- FEAT-127: Server config schema (provides the config)
- FEAT-129: Auto SSH tunneling (may trigger tunnel setup)
