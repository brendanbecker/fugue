# FEAT-129: Auto SSH Tunneling

**Priority**: P1
**Component**: fugue-client/connection
**Effort**: Medium
**Status**: new
**Depends**: FEAT-127, FEAT-128

## Summary

Automatically establish SSH tunnels when connecting to servers with `ssh_tunnel = true` in config.

## Problem

Currently users must manually run `ssh -L 9999:127.0.0.1:9999 host -N` before connecting to remote servers. This should be automatic.

## Proposed Behavior

```bash
# Config has ssh_tunnel = true for polecats
fugue -s polecats

# Client automatically:
# 1. Checks if tunnel already exists (port in use)
# 2. If not, spawns: ssh -L 9999:127.0.0.1:9999 user@polecats -N
# 3. Waits for tunnel to be ready
# 4. Connects to tcp://127.0.0.1:9999
```

## Implementation

### Key Files

| File | Changes |
|------|---------|
| `fugue-client/src/tunnel.rs` | New - SSH tunnel management |
| `fugue-client/src/connection/client.rs` | Call tunnel setup before connect |
| `fugue-client/src/main.rs` | Handle tunnel lifecycle |

### Tunnel Manager

```rust
pub struct SshTunnel {
    local_port: u16,
    ssh_process: Child,
}

impl SshTunnel {
    pub fn establish(config: &ServerConfig) -> Result<Self> {
        // Parse local port from addr (tcp://127.0.0.1:9999 -> 9999)
        let local_port = parse_port(&config.addr)?;

        // Check if port already in use (tunnel exists)
        if is_port_in_use(local_port) {
            return Ok(Self::existing(local_port));
        }

        // Build SSH command
        let mut cmd = Command::new("ssh");
        cmd.arg("-L").arg(format!("{}:127.0.0.1:{}", local_port, local_port));
        cmd.arg("-N");  // No remote command
        cmd.arg("-o").arg("ExitOnForwardFailure=yes");
        cmd.arg("-o").arg("ServerAliveInterval=30");

        if let Some(identity) = &config.ssh_identity {
            cmd.arg("-i").arg(expand_tilde(identity));
        }

        if config.ssh_port != 22 {
            cmd.arg("-p").arg(config.ssh_port.to_string());
        }

        cmd.arg(&config.ssh_host.as_ref().unwrap());

        // Spawn in background
        let child = cmd.spawn()?;

        // Wait for tunnel to be ready (poll port)
        wait_for_port(local_port, Duration::from_secs(10))?;

        Ok(Self { local_port, ssh_process: child })
    }
}

impl Drop for SshTunnel {
    fn drop(&mut self) {
        // Optionally kill tunnel on client exit
        // Or leave it running for reconnects
    }
}
```

### Tunnel Persistence Options

Two strategies for tunnel lifecycle:

1. **Ephemeral**: Kill tunnel when client exits
   - Cleaner, no orphan processes
   - Slower reconnects

2. **Persistent**: Leave tunnel running
   - Fast reconnects
   - May accumulate orphan tunnels

Recommend: Persistent by default, with `fugue tunnel stop polecats` command.

## Edge Cases

- **Port conflict**: Another process using the local port
- **SSH auth failure**: Key not loaded, password required
- **Tunnel drops mid-session**: Detect and attempt reconnect
- **Multiple clients**: Share existing tunnel

## Acceptance Criteria

- [ ] Tunnel auto-established when `ssh_tunnel = true`
- [ ] Reuses existing tunnel if port already bound
- [ ] Respects `ssh_identity` and `ssh_port` config
- [ ] Clear error if SSH fails (auth, host unreachable)
- [ ] Timeout if tunnel doesn't come up
- [ ] Client can reconnect through existing tunnel

## Related

- FEAT-127: Server config schema (provides ssh_* fields)
- FEAT-128: Client --server flag (triggers tunnel setup)
