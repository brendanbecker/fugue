# FEAT-127: Server Configuration Schema

**Priority**: P1
**Component**: config
**Effort**: Small
**Status**: new

## Summary

Add `[servers.<name>]` sections to fugue config.toml for defining named server profiles with connection addresses and optional SSH tunnel configuration.

## Problem

Currently, connecting to a remote fugue server requires manually specifying `--addr tcp://...` and setting up SSH tunnels. Users with multiple machines want to define server profiles once and reference them by name.

## Proposed Config Schema

```toml
# ~/.config/fugue/config.toml

[servers.local]
addr = "unix:///run/user/1000/fugue.sock"
default = true  # Used when no --server specified

[servers.polecats]
addr = "tcp://127.0.0.1:9999"
ssh_host = "user@polecats.local"
ssh_tunnel = true  # Auto-establish tunnel on connect

[servers.workstation]
addr = "tcp://workstation.internal:9999"
# No SSH tunnel - direct connection (e.g., on same LAN/VPN)

[servers.cloud-dev]
addr = "tcp://127.0.0.1:9998"
ssh_host = "ubuntu@dev.example.com"
ssh_tunnel = true
ssh_port = 22
ssh_identity = "~/.ssh/cloud_dev_key"
```

## Config Fields

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `addr` | string | yes | Connection URL (unix:// or tcp://) |
| `default` | bool | no | Use this server when --server not specified |
| `ssh_host` | string | no | SSH host for tunneling (user@host) |
| `ssh_tunnel` | bool | no | Auto-establish SSH tunnel |
| `ssh_port` | int | no | SSH port (default: 22) |
| `ssh_identity` | string | no | Path to SSH key |

## Implementation

### Key Files

| File | Changes |
|------|---------|
| `fugue-client/src/config.rs` | Add `ServerConfig` struct, parse `[servers.*]` |
| `fugue-protocol/src/config.rs` | Shared config types if needed |

### Config Struct

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub addr: String,
    #[serde(default)]
    pub default: bool,
    pub ssh_host: Option<String>,
    #[serde(default)]
    pub ssh_tunnel: bool,
    #[serde(default = "default_ssh_port")]
    pub ssh_port: u16,
    pub ssh_identity: Option<PathBuf>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    // ... existing fields ...
    #[serde(default)]
    pub servers: HashMap<String, ServerConfig>,
}
```

## Acceptance Criteria

- [ ] Config parser accepts `[servers.<name>]` sections
- [ ] All fields parse correctly with appropriate defaults
- [ ] Invalid config produces clear error messages
- [ ] Backward compatible - missing `[servers]` section works (uses default socket)
- [ ] `default = true` on multiple servers produces warning/error

## Related

- FEAT-128: Client --server CLI flag (uses this config)
- FEAT-129: Auto SSH tunneling (uses ssh_* fields)
